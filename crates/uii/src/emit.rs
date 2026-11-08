//! Native snippets for every non-HTML format.
//!
//! Each snippet is real toolkit source (Dear ImGui, dear-imgui-rs as Banter
//! links it, Raven's C API, egui, Slint, Qt, WinUI, SwiftUI, GTK, Compose)
//! plus one `uii` record per node so the import can rebuild the tree and then
//! reject a file whose native calls were stripped.

use crate::codec::{
    c_ident, c_string, document_line, document_xml, line_comment, record_body, rust_string,
    type_name, xml_attr, xml_comment,
};
use crate::error::Error;
use crate::model::{banter_region_variant, Axis, DockSide, Document, Node, NodeKind};

struct Buf {
    text: String,
    indent: usize,
    seq: usize,
}

impl Buf {
    fn new() -> Self {
        Self {
            text: String::new(),
            indent: 0,
            seq: 0,
        }
    }

    fn line(&mut self, line: &str) {
        for _ in 0..self.indent {
            self.text.push_str("    ");
        }
        self.text.push_str(line);
        self.text.push('\n');
    }

    fn next(&mut self) -> usize {
        self.seq += 1;
        self.seq
    }
}

fn visible(node: &Node) -> String {
    if node.text.is_empty() {
        node.name.clone()
    } else {
        node.text.clone()
    }
}

fn group_sides(dock: &Node) -> (Vec<&Node>, Vec<&Node>, Vec<&Node>, Vec<&Node>, Vec<&Node>) {
    let mut left = Vec::new();
    let mut right = Vec::new();
    let mut top = Vec::new();
    let mut bottom = Vec::new();
    let mut center = Vec::new();
    for child in &dock.children {
        if child.kind != NodeKind::Region {
            continue;
        }
        match child.dock.unwrap_or(DockSide::Center) {
            DockSide::Left => left.push(child),
            DockSide::Right => right.push(child),
            DockSide::Top => top.push(child),
            DockSide::Bottom => bottom.push(child),
            DockSide::Center | DockSide::Tab => center.push(child),
        }
    }
    (left, right, top, bottom, center)
}

fn tabs_expr(nodes: &[&Node]) -> String {
    let inner = nodes
        .iter()
        .map(|node| rust_string(&node.id))
        .collect::<Vec<_>>()
        .join(", ");
    format!("DockLayout::tabs([{inner}])")
}

fn banter_layout(dock: &Node) -> String {
    let (left, right, top, bottom, center) = group_sides(dock);
    let mut expr = tabs_expr(&center);
    if !bottom.is_empty() {
        expr = format!(
            "DockLayout::split(DockSplit::Down, 0.18_f32, {}, {expr})",
            tabs_expr(&bottom)
        );
    }
    if !right.is_empty() {
        expr = format!(
            "DockLayout::split(DockSplit::Right, 0.22_f32, {}, {expr})",
            tabs_expr(&right)
        );
    }
    if !top.is_empty() {
        expr = format!(
            "DockLayout::split(DockSplit::Up, 0.18_f32, {}, {expr})",
            tabs_expr(&top)
        );
    }
    if !left.is_empty() {
        expr = format!(
            "DockLayout::split(DockSplit::Left, 0.22_f32, {}, {expr})",
            tabs_expr(&left)
        );
    }
    expr
}

pub(crate) fn export_banter(document: &Document) -> Result<String, Error> {
    let mut out = Buf::new();
    out.line(&document_line(&document.name));
    out.line("// Banter enables docking with ConfigFlags::DOCKING_ENABLE before the first frame");
    out.line("// (crates/banter/src/platform.rs). Named shell slugs follow layout::Region.");
    out.line(
        "use dear_imgui_rs::{Condition, DockLayout, DockLayoutApply, DockSplit, WindowFlags};",
    );
    out.line("");
    out.line("pub fn draw(ui: &dear_imgui_rs::Ui) {");
    out.indent += 1;
    emit_banter(&document.root, None, true, &mut out);
    out.indent -= 1;
    out.line("}");
    Ok(out.text)
}

fn emit_banter(node: &Node, parent: Option<&str>, is_root: bool, out: &mut Buf) {
    if let Some(variant) =
        banter_region_variant(&node.id).or_else(|| banter_region_variant(&node.name))
    {
        out.line(&format!(
            "// layout::Region::{variant} — crates/banter/src/layout.rs"
        ));
    }
    out.line(&line_comment(&record_body(node, parent)));
    match node.kind {
        NodeKind::Window | NodeKind::Region => {
            if is_root {
                out.line(&format!(
                    "ui.window({}).flags(WindowFlags::NO_DECORATION | WindowFlags::NO_MOVE | WindowFlags::NO_RESIZE | WindowFlags::NO_BRING_TO_FRONT_ON_FOCUS | WindowFlags::NO_SAVED_SETTINGS | WindowFlags::NO_DOCKING | WindowFlags::NO_SCROLLBAR).position([0.0, 0.0], Condition::Always).size(ui.io().display_size(), Condition::Always).build(|| {{",
                    rust_string(&node.id)
                ));
            } else {
                out.line(&format!("ui.window({}).build(|| {{", rust_string(&node.id)));
            }
            out.indent += 1;
            emit_banter_children(&node.children, &node.id, out);
            out.indent -= 1;
            out.line("});");
        }
        NodeKind::Box => {
            out.line(&format!(
                "ui.child_window({}).build(&ui, || {{",
                rust_string(&node.id)
            ));
            out.indent += 1;
            emit_banter_children(&node.children, &node.id, out);
            out.indent -= 1;
            out.line("});");
        }
        NodeKind::Dock => {
            let var = format!("dock_layout_{}", c_ident(&node.id));
            out.line(&format!("let {var} = {};", banter_layout(node)));
            out.line(&format!(
                "let _{var} = ui.dockspace().current_window(ui.content_region_avail()).layout(&{var}, DockLayoutApply::Replace).build();"
            ));
            emit_banter_children(&node.children, &node.id, out);
        }
        NodeKind::Table => emit_banter_table(node, out),
        NodeKind::Label | NodeKind::HeaderBar | NodeKind::Image => {
            out.line(&format!("ui.text({});", rust_string(&visible(node))));
            emit_banter_children(&node.children, &node.id, out);
        }
        NodeKind::Button => {
            out.line(&format!(
                "let _ = ui.button({});",
                rust_string(&visible(node))
            ));
        }
        NodeKind::TextInput => {
            let var = c_ident(&node.id);
            out.line(&format!(
                "let mut {var} = String::from({});",
                rust_string(&node.text)
            ));
            out.line(&format!(
                "ui.input_text({}, &mut {var});",
                rust_string(&node.id)
            ));
        }
        NodeKind::Checkbox => {
            let var = c_ident(&node.id);
            let on = node.props.get("checked").map(String::as_str) == Some("true");
            out.line(&format!("let mut {var} = {on};"));
            out.line(&format!(
                "ui.checkbox({}, &mut {var});",
                rust_string(&visible(node))
            ));
        }
        NodeKind::Slider => {
            let var = c_ident(&node.id);
            let value = node
                .props
                .get("value")
                .and_then(|v| v.parse::<f32>().ok())
                .unwrap_or(0.0);
            out.line(&format!("let mut {var} = {value}_f32;"));
            out.line(&format!(
                "ui.slider_f32({}, &mut {var}, 0.0, 1.0);",
                rust_string(&visible(node))
            ));
        }
        NodeKind::Separator => out.line("ui.separator();"),
        NodeKind::Island => {
            out.line(&format!("ui.text({});", rust_string(&visible(node))));
            emit_banter_children(&node.children, &node.id, out);
        }
    }
}

fn emit_banter_children(children: &[Node], parent: &str, out: &mut Buf) {
    for child in children {
        emit_banter(child, Some(parent), false, out);
    }
}

fn emit_banter_table(node: &Node, out: &mut Buf) {
    out.line(&format!("ui.table({})", rust_string(&node.id)));
    out.indent += 1;
    if node.columns.is_empty() {
        out.line(".column(\"\").done()");
    }
    for column in &node.columns {
        out.line(&format!(".column({}).done()", rust_string(column)));
    }
    out.line(".headers(true)");
    out.line(".build(|ui| {");
    out.indent += 1;
    for row in &node.rows {
        out.line("ui.table_next_row();");
        for cell in row {
            out.line("ui.table_next_column();");
            out.line(&format!("ui.text({});", rust_string(cell)));
        }
    }
    out.indent -= 1;
    out.line("});");
    out.indent -= 1;
}

pub(crate) fn export_dear(document: &Document) -> Result<String, Error> {
    let mut out = Buf::new();
    out.line(&document_line(&document.name));
    out.line("#include <imgui.h>");
    out.line("");
    out.line("static void uii_draw(void) {");
    out.indent += 1;
    emit_imgui(&document.root, None, &mut out);
    out.indent -= 1;
    out.line("}");
    Ok(out.text)
}

pub(crate) fn export_raven(document: &Document) -> Result<String, Error> {
    let mut out = Buf::new();
    out.line(&document_line(&document.name));
    out.line("#include <imgui.h>");
    out.line("#include <Raven.h>");
    out.line("");
    out.line("// Dear ImGui docking, tables, and child windows.");
    out.line("static void uii_imgui(void) {");
    out.indent += 1;
    emit_imgui(&document.root, None, &mut out);
    out.indent -= 1;
    out.line("}");
    out.line("");
    out.line("// Raven (oscar-soirey/Raven) widgets for the same tree.");
    out.line("// Banter does not ship Raven. Calls match include/raven/raven_widget.h,");
    out.line("// raven_controls.h, and raven_shell.h.");
    out.line("static void uii_raven(void) {");
    out.indent += 1;
    emit_raven_window(document, &mut out);
    out.indent -= 1;
    out.line("}");
    Ok(out.text)
}

fn emit_imgui(node: &Node, parent: Option<&str>, out: &mut Buf) {
    out.line(&line_comment(&record_body(node, parent)));
    match node.kind {
        NodeKind::Window | NodeKind::Region => {
            out.line(&format!("ImGui::Begin({});", c_string(&node.id)));
            out.indent += 1;
            if !node.text.is_empty() {
                out.line(&format!(
                    "ImGui::TextUnformatted({});",
                    c_string(&node.text)
                ));
            }
            emit_imgui_children(&node.children, &node.id, out);
            out.indent -= 1;
            out.line("ImGui::End();");
        }
        NodeKind::Box => {
            out.line(&format!("ImGui::BeginChild({});", c_string(&node.id)));
            out.indent += 1;
            emit_imgui_children(&node.children, &node.id, out);
            out.indent -= 1;
            out.line("ImGui::EndChild();");
        }
        NodeKind::Dock => {
            emit_dock_builder(node, out);
            emit_imgui_children(&node.children, &node.id, out);
        }
        NodeKind::Table => emit_imgui_table(node, out),
        NodeKind::Label | NodeKind::Image | NodeKind::Island => {
            out.line(&format!(
                "ImGui::TextUnformatted({});",
                c_string(&visible(node))
            ));
            emit_imgui_children(&node.children, &node.id, out);
        }
        NodeKind::HeaderBar => {
            out.line(&format!(
                "ImGui::SeparatorText({});",
                c_string(&visible(node))
            ));
            emit_imgui_children(&node.children, &node.id, out);
        }
        NodeKind::Button => {
            out.line(&format!("ImGui::Button({});", c_string(&visible(node))));
        }
        NodeKind::TextInput => {
            let var = c_ident(&node.id);
            out.line(&format!(
                "static char {var}[1024] = {};",
                c_string(&node.text)
            ));
            out.line(&format!(
                "ImGui::InputText({}, {var}, sizeof {var});",
                c_string(&node.id)
            ));
        }
        NodeKind::Checkbox => {
            let var = c_ident(&node.id);
            let on = if node.props.get("checked").map(String::as_str) == Some("true") {
                "true"
            } else {
                "false"
            };
            out.line(&format!("static bool {var} = {on};"));
            out.line(&format!(
                "ImGui::Checkbox({}, &{var});",
                c_string(&visible(node))
            ));
        }
        NodeKind::Slider => {
            let var = c_ident(&node.id);
            let value = node
                .props
                .get("value")
                .and_then(|v| v.parse::<f32>().ok())
                .unwrap_or(0.0);
            out.line(&format!("static float {var} = {value}f;"));
            out.line(&format!(
                "ImGui::SliderFloat({}, &{var}, 0.0f, 1.0f);",
                c_string(&visible(node))
            ));
        }
        NodeKind::Separator => out.line("ImGui::Separator();"),
    }
}

fn emit_imgui_children(children: &[Node], parent: &str, out: &mut Buf) {
    for child in children {
        emit_imgui(child, Some(parent), out);
    }
}

fn emit_dock_builder(dock: &Node, out: &mut Buf) {
    let (left, right, top, bottom, center) = group_sides(dock);
    let id = c_ident(&dock.id);
    out.line(&format!(
        "ImGuiID {id} = ImGui::GetID({});",
        c_string(&dock.id)
    ));
    out.line(&format!("ImGui::DockBuilderRemoveNode({id});"));
    out.line(&format!(
        "ImGui::DockBuilderAddNode({id}, ImGuiDockNodeFlags_DockSpace);"
    ));
    out.line(&format!(
        "ImGui::DockBuilderSetNodeSize({id}, ImGui::GetMainViewport()->Size);"
    ));
    let mut rest = id.clone();
    for (dir, ratio, nodes) in [
        ("ImGuiDir_Left", "0.22", left),
        ("ImGuiDir_Up", "0.18", top),
        ("ImGuiDir_Right", "0.22", right),
        ("ImGuiDir_Down", "0.18", bottom),
    ] {
        if nodes.is_empty() {
            continue;
        }
        let n = out.next();
        let part = format!("part_{n}");
        let next = format!("rest_{n}");
        out.line(&format!("ImGuiID {part} = 0, {next} = 0;"));
        out.line(&format!(
            "ImGui::DockBuilderSplitNode({rest}, {dir}, {ratio}f, &{part}, &{next});"
        ));
        for node in nodes {
            out.line(&format!(
                "ImGui::DockBuilderDockWindow({}, {part});",
                c_string(&node.id)
            ));
        }
        rest = next;
    }
    for node in center {
        out.line(&format!(
            "ImGui::DockBuilderDockWindow({}, {rest});",
            c_string(&node.id)
        ));
    }
    out.line(&format!("ImGui::DockBuilderFinish({id});"));
    out.line(&format!(
        "ImGui::DockSpace({id}, ImVec2(0.0f, 0.0f), ImGuiDockNodeFlags_PassthruCentralNode);"
    ));
}

fn emit_imgui_table(node: &Node, out: &mut Buf) {
    let cols = node.columns.len().max(1);
    out.line(&format!(
        "if (ImGui::BeginTable({}, {cols}, ImGuiTableFlags_Borders | ImGuiTableFlags_RowBg)) {{",
        c_string(&node.id)
    ));
    out.indent += 1;
    for column in &node.columns {
        out.line(&format!("ImGui::TableSetupColumn({});", c_string(column)));
    }
    if !node.columns.is_empty() {
        out.line("ImGui::TableHeadersRow();");
    }
    for row in &node.rows {
        out.line("ImGui::TableNextRow();");
        for (index, cell) in row.iter().enumerate() {
            out.line(&format!("ImGui::TableSetColumnIndex({index});"));
            out.line(&format!("ImGui::TextUnformatted({});", c_string(cell)));
        }
    }
    out.line("ImGui::EndTable();");
    out.indent -= 1;
    out.line("}");
}

fn emit_raven_window(document: &Document, out: &mut Buf) {
    let root = c_ident(&document.root.id);
    out.line(&format!(
        "Rid window = Rwindow_create({}, 960, 640, R_WINDOW_DEFAULT);",
        c_string(&document.name)
    ));
    out.line(&format!("Rid {root} = Rwidget_create(R_WIDGET_VBOX);"));
    out.line(&format!(
        "Rwidget_set_name({root}, {});",
        c_string(&document.root.id)
    ));
    out.line(&format!("Rwindow_set_content(window, {root});"));
    for child in &document.root.children {
        emit_raven(child, &root, false, out);
    }
}

fn emit_raven(node: &Node, parent: &str, parent_is_dock: bool, out: &mut Buf) {
    let var = c_ident(&node.id);
    if parent_is_dock && node.kind == NodeKind::Region {
        out.line(&format!("Rid {var} = Rwidget_create(R_WIDGET_VBOX);"));
        out.line(&format!("Rwidget_set_name({var}, {});", c_string(&node.id)));
        let (side, ratio) = raven_side(node.dock.unwrap_or(DockSide::Center));
        out.line(&format!(
            "Rdock_area_add_panel({parent}, {var}, {}, {side}, R_INVALID_ID, {ratio}f);",
            c_string(&node.id)
        ));
        for child in &node.children {
            emit_raven(child, &var, false, out);
        }
        return;
    }
    match node.kind {
        NodeKind::Dock => {
            out.line(&format!("Rid {var} = Rdock_area_create();"));
            out.line(&format!("Rwidget_set_name({var}, {});", c_string(&node.id)));
            out.line(&format!("Rwidget_add_child({parent}, {var});"));
            // Center / tab panel first so other sides split against a real panel.
            let mut ordered: Vec<&Node> = node
                .children
                .iter()
                .filter(|child| {
                    child.kind == NodeKind::Region
                        && matches!(
                            child.dock.unwrap_or(DockSide::Center),
                            DockSide::Center | DockSide::Tab
                        )
                })
                .collect();
            ordered.extend(node.children.iter().filter(|child| {
                !(child.kind == NodeKind::Region
                    && matches!(
                        child.dock.unwrap_or(DockSide::Center),
                        DockSide::Center | DockSide::Tab
                    ))
            }));
            for child in ordered {
                emit_raven(child, &var, true, out);
            }
        }
        NodeKind::Box => {
            let kind = if node.axis == Some(Axis::Horizontal) {
                "R_WIDGET_HBOX"
            } else {
                "R_WIDGET_VBOX"
            };
            out.line(&format!("Rid {var} = Rwidget_create({kind});"));
            out.line(&format!("Rwidget_set_name({var}, {});", c_string(&node.id)));
            out.line(&format!("Rwidget_add_child({parent}, {var});"));
            for child in &node.children {
                emit_raven(child, &var, false, out);
            }
        }
        NodeKind::Table => {
            out.line(&format!("Rid {var} = Rwidget_create(R_WIDGET_TABLE);"));
            out.line(&format!("Rwidget_set_name({var}, {});", c_string(&node.id)));
            out.line(&format!("Rwidget_add_child({parent}, {var});"));
            for column in &node.columns {
                out.line(&format!(
                    "Rtable_add_column({var}, {}, 0.0f);",
                    c_string(column)
                ));
            }
            for (index, row) in node.rows.iter().enumerate() {
                let row_var = format!("{var}_row_{index}");
                out.line(&format!("int {row_var} = Rtable_add_row({var}, NULL);"));
                for (col, cell) in row.iter().enumerate() {
                    out.line(&format!(
                        "Rtable_set_cell({var}, {row_var}, {col}, {});",
                        c_string(cell)
                    ));
                }
            }
        }
        NodeKind::Label => {
            out.line(&format!(
                "Rid {var} = Rlabel_create({});",
                c_string(&visible(node))
            ));
            out.line(&format!("Rwidget_set_name({var}, {});", c_string(&node.id)));
            out.line(&format!("Rwidget_add_child({parent}, {var});"));
        }
        NodeKind::Button => {
            out.line(&format!(
                "Rid {var} = Rbutton_create({});",
                c_string(&visible(node))
            ));
            out.line(&format!("Rwidget_set_name({var}, {});", c_string(&node.id)));
            out.line(&format!("Rwidget_add_child({parent}, {var});"));
        }
        NodeKind::Checkbox => {
            out.line(&format!(
                "Rid {var} = Rcheckbox_create({});",
                c_string(&visible(node))
            ));
            let on = if node.props.get("checked").map(String::as_str) == Some("true") {
                1
            } else {
                0
            };
            out.line(&format!("Rcheckbox_set_checked({var}, {on});"));
            out.line(&format!("Rwidget_set_name({var}, {});", c_string(&node.id)));
            out.line(&format!("Rwidget_add_child({parent}, {var});"));
        }
        NodeKind::TextInput => {
            out.line(&format!("Rid {var} = Rwidget_create(R_WIDGET_TEXT_INPUT);"));
            out.line(&format!("Rwidget_set_name({var}, {});", c_string(&node.id)));
            out.line(&format!(
                "Rtext_input_set_text({var}, {});",
                c_string(&node.text)
            ));
            if let Some(placeholder) = node.props.get("placeholder") {
                out.line(&format!(
                    "Rtext_input_set_placeholder({var}, {});",
                    c_string(placeholder)
                ));
            }
            out.line(&format!("Rwidget_add_child({parent}, {var});"));
        }
        NodeKind::Slider => {
            out.line(&format!("Rid {var} = Rwidget_create(R_WIDGET_SLIDER);"));
            out.line(&format!("Rwidget_set_name({var}, {});", c_string(&node.id)));
            if let Some(value) = node.props.get("value").and_then(|v| v.parse::<f32>().ok()) {
                out.line(&format!("Rslider_set_value({var}, {value}f);"));
            }
            out.line(&format!("Rwidget_add_child({parent}, {var});"));
        }
        NodeKind::Image => {
            out.line(&format!("Rid {var} = Rwidget_create(R_WIDGET_IMAGE);"));
            out.line(&format!("Rwidget_set_name({var}, {});", c_string(&node.id)));
            out.line(&format!("Rwidget_add_child({parent}, {var});"));
        }
        NodeKind::Separator => {
            out.line(&format!("Rid {var} = Rwidget_create(R_WIDGET_SEPARATOR);"));
            out.line(&format!("Rwidget_set_name({var}, {});", c_string(&node.id)));
            out.line(&format!("Rwidget_add_child({parent}, {var});"));
        }
        NodeKind::HeaderBar => {
            out.line(&format!(
                "Rid {var} = Rheader_bar_create({});",
                c_string(&visible(node))
            ));
            out.line(&format!("Rwidget_set_name({var}, {});", c_string(&node.id)));
            out.line(&format!("Rwidget_add_child({parent}, {var});"));
            for child in &node.children {
                emit_raven(child, &var, false, out);
            }
        }
        NodeKind::Island => {
            out.line(&format!("Rid {var} = Rwidget_create(R_WIDGET_ISLAND);"));
            out.line(&format!("Rwidget_set_name({var}, {});", c_string(&node.id)));
            out.line(&format!("Rwidget_add_child({parent}, {var});"));
            for child in &node.children {
                emit_raven(child, &var, false, out);
            }
        }
        NodeKind::Window | NodeKind::Region => {
            out.line(&format!("Rid {var} = Rwidget_create(R_WIDGET_VBOX);"));
            out.line(&format!("Rwidget_set_name({var}, {});", c_string(&node.id)));
            out.line(&format!("Rwidget_add_child({parent}, {var});"));
            for child in &node.children {
                emit_raven(child, &var, false, out);
            }
        }
    }
}

fn raven_side(side: DockSide) -> (&'static str, &'static str) {
    match side {
        DockSide::Left => ("R_DOCK_LEFT", "0.22"),
        DockSide::Right => ("R_DOCK_RIGHT", "0.22"),
        DockSide::Top => ("R_DOCK_TOP", "0.18"),
        DockSide::Bottom => ("R_DOCK_BOTTOM", "0.18"),
        DockSide::Center | DockSide::Tab => ("R_DOCK_TAB", "0.0"),
    }
}

pub(crate) fn export_egui(document: &Document) -> Result<String, Error> {
    let mut out = Buf::new();
    out.line(&document_line(&document.name));
    out.line("// egui lays regions out as panels. It has no DockSpace and no child window.");
    out.line("pub fn draw(ctx: &egui::Context) {");
    out.indent += 1;
    out.line(&line_comment(&record_body(&document.root, None)));
    emit_egui_children(&document.root.children, &document.root.id, &mut out);
    out.indent -= 1;
    out.line("}");
    Ok(out.text)
}

fn emit_egui_children(children: &[Node], parent: &str, out: &mut Buf) {
    for child in children {
        emit_egui(child, parent, out);
    }
}

fn emit_egui(node: &Node, parent: &str, out: &mut Buf) {
    out.line(&line_comment(&record_body(node, Some(parent))));
    match node.kind {
        NodeKind::Dock => {
            out.line("// egui panels stand in for this dock.");
            emit_egui_children(&node.children, &node.id, out);
        }
        NodeKind::Region => {
            let open = match node.dock.unwrap_or(DockSide::Center) {
                DockSide::Left => format!(
                    "egui::SidePanel::left({}).show(ctx, |ui| {{",
                    rust_string(&node.id)
                ),
                DockSide::Right => format!(
                    "egui::SidePanel::right({}).show(ctx, |ui| {{",
                    rust_string(&node.id)
                ),
                DockSide::Top => format!(
                    "egui::TopBottomPanel::top({}).show(ctx, |ui| {{",
                    rust_string(&node.id)
                ),
                DockSide::Bottom => format!(
                    "egui::TopBottomPanel::bottom({}).show(ctx, |ui| {{",
                    rust_string(&node.id)
                ),
                DockSide::Center | DockSide::Tab => {
                    "egui::CentralPanel::default().show(ctx, |ui| {".to_string()
                }
            };
            out.line(&open);
            out.indent += 1;
            if !node.text.is_empty() {
                out.line(&format!("ui.label({});", rust_string(&node.text)));
            }
            emit_egui_children(&node.children, &node.id, out);
            out.indent -= 1;
            out.line("});");
        }
        NodeKind::Box => {
            if node.axis == Some(Axis::Horizontal) {
                out.line("ui.horizontal(|ui| {");
            } else {
                out.line("ui.vertical(|ui| {");
            }
            out.indent += 1;
            emit_egui_children(&node.children, &node.id, out);
            out.indent -= 1;
            out.line("});");
        }
        NodeKind::Table => {
            out.line("egui_extras::TableBuilder::new(ui)");
            out.indent += 1;
            for _ in &node.columns {
                out.line(".column(egui_extras::Column::remainder())");
            }
            if node.columns.is_empty() {
                out.line(".column(egui_extras::Column::remainder())");
            }
            out.line(".header(20.0, |mut header| {");
            out.indent += 1;
            for column in &node.columns {
                out.line(&format!(
                    "header.col(|ui| {{ ui.label({}); }});",
                    rust_string(column)
                ));
            }
            out.indent -= 1;
            out.line("})");
            out.line(".body(|mut body| {");
            out.indent += 1;
            for row in &node.rows {
                out.line("body.row(18.0, |mut row| {");
                out.indent += 1;
                for cell in row {
                    out.line(&format!(
                        "row.col(|ui| {{ ui.label({}); }});",
                        rust_string(cell)
                    ));
                }
                out.indent -= 1;
                out.line("});");
            }
            out.indent -= 1;
            out.line("});");
            out.indent -= 1;
        }
        NodeKind::Label | NodeKind::HeaderBar | NodeKind::Image | NodeKind::Island => {
            out.line(&format!("ui.label({});", rust_string(&visible(node))));
            if !node.children.is_empty() {
                out.line("ui.group(|ui| {");
                out.indent += 1;
                emit_egui_children(&node.children, &node.id, out);
                out.indent -= 1;
                out.line("});");
            }
        }
        NodeKind::Button => {
            out.line(&format!(
                "let _ = ui.button({});",
                rust_string(&visible(node))
            ));
        }
        NodeKind::TextInput => {
            let var = c_ident(&node.id);
            out.line(&format!(
                "let mut {var} = {}.to_string();",
                rust_string(&node.text)
            ));
            out.line(&format!("ui.text_edit_singleline(&mut {var});"));
        }
        NodeKind::Checkbox => {
            let var = c_ident(&node.id);
            let on = node.props.get("checked").map(String::as_str) == Some("true");
            out.line(&format!("let mut {var} = {on};"));
            out.line(&format!(
                "ui.checkbox(&mut {var}, {});",
                rust_string(&visible(node))
            ));
        }
        NodeKind::Slider => {
            let var = c_ident(&node.id);
            let value = node
                .props
                .get("value")
                .and_then(|v| v.parse::<f32>().ok())
                .unwrap_or(0.0);
            out.line(&format!("let mut {var} = {value}_f32;"));
            out.line(&format!(
                "ui.add(egui::Slider::new(&mut {var}, 0.0..=1.0).text({}));",
                rust_string(&visible(node))
            ));
        }
        NodeKind::Separator => out.line("ui.separator();"),
        NodeKind::Window => {
            emit_egui_children(&node.children, &node.id, out);
        }
    }
}

pub(crate) fn export_slint(document: &Document) -> Result<String, Error> {
    let mut out = Buf::new();
    out.line(&document_line(&document.name));
    out.line(&format!(
        "export component {} inherits Window {{",
        type_name(&document.name)
    ));
    out.indent += 1;
    emit_slint(&document.root, None, &mut out);
    out.indent -= 1;
    out.line("}");
    Ok(out.text)
}

fn emit_slint(node: &Node, parent: Option<&str>, out: &mut Buf) {
    out.line(&line_comment(&record_body(node, parent)));
    match node.kind {
        NodeKind::Box if node.axis == Some(Axis::Horizontal) => {
            out.line("HorizontalBox {");
            slint_children(node, out);
        }
        NodeKind::Box | NodeKind::Window | NodeKind::Dock | NodeKind::Region | NodeKind::Island => {
            let open = if node.kind == NodeKind::Box {
                "VerticalBox {"
            } else if node.kind == NodeKind::Region || node.kind == NodeKind::Dock {
                "Rectangle {"
            } else {
                "VerticalBox {"
            };
            out.line(open);
            slint_children(node, out);
        }
        NodeKind::Table => {
            out.line("GridLayout {");
            out.indent += 1;
            for column in &node.columns {
                out.line(&format!("Text {{ text: {}; }}", rust_string(column)));
            }
            for row in &node.rows {
                for cell in row {
                    out.line(&format!("Text {{ text: {}; }}", rust_string(cell)));
                }
            }
            out.indent -= 1;
            out.line("}");
        }
        NodeKind::Label | NodeKind::HeaderBar => {
            out.line(&format!(
                "Text {{ text: {}; }}",
                rust_string(&visible(node))
            ));
        }
        NodeKind::Button => {
            out.line(&format!(
                "Button {{ text: {}; }}",
                rust_string(&visible(node))
            ));
        }
        NodeKind::TextInput => {
            let placeholder = node
                .props
                .get("placeholder")
                .map(String::as_str)
                .unwrap_or("");
            out.line(&format!(
                "LineEdit {{ text: {}; placeholder: {}; }}",
                rust_string(&node.text),
                rust_string(placeholder)
            ));
        }
        NodeKind::Checkbox => {
            let on = node.props.get("checked").map(String::as_str) == Some("true");
            out.line(&format!(
                "CheckBox {{ text: {}; checked: {on}; }}",
                rust_string(&visible(node))
            ));
        }
        NodeKind::Slider => {
            let value = node
                .props
                .get("value")
                .and_then(|v| v.parse::<f32>().ok())
                .unwrap_or(0.0);
            out.line(&format!("Slider {{ value: {value}; }}"));
        }
        NodeKind::Image => out.line("Image { }"),
        NodeKind::Separator => out.line("Rectangle { height: 1px; }"),
    }
}

fn slint_children(node: &Node, out: &mut Buf) {
    out.indent += 1;
    if !node.text.is_empty()
        && !matches!(
            node.kind,
            NodeKind::Label | NodeKind::Button | NodeKind::HeaderBar
        )
    {
        out.line(&format!("Text {{ text: {}; }}", rust_string(&node.text)));
    }
    for child in &node.children {
        emit_slint(child, Some(&node.id), out);
    }
    out.indent -= 1;
    out.line("}");
}

pub(crate) fn export_qt(document: &Document) -> Result<String, Error> {
    let mut out = Buf::new();
    out.line(&document_line(&document.name));
    for header in [
        "QMainWindow",
        "QDockWidget",
        "QTableWidget",
        "QTableWidgetItem",
        "QVBoxLayout",
        "QHBoxLayout",
        "QLabel",
        "QPushButton",
        "QLineEdit",
        "QCheckBox",
        "QSlider",
        "QFrame",
        "QGroupBox",
    ] {
        out.line(&format!("#include <{header}>"));
    }
    out.line("");
    out.line("static void uii_qt(void) {");
    out.indent += 1;
    let root = c_ident(&document.root.id);
    out.line(&line_comment(&record_body(&document.root, None)));
    out.line("auto *window = new QMainWindow();");
    out.line(&format!(
        "window->setObjectName({});",
        c_string(&document.root.id)
    ));
    out.line(&format!(
        "window->setWindowTitle({});",
        c_string(&document.name)
    ));
    out.line(&format!("auto *{root}_body = new QWidget(window);"));
    out.line(&format!(
        "auto *{root}_layout = new QVBoxLayout({root}_body);"
    ));
    out.line(&format!("window->setCentralWidget({root}_body);"));
    for child in &document.root.children {
        emit_qt(
            child,
            &document.root.id,
            "window",
            &format!("{root}_body"),
            &format!("{root}_layout"),
            &mut out,
        );
    }
    out.indent -= 1;
    out.line("}");
    Ok(out.text)
}

fn emit_qt(
    node: &Node,
    record_parent: &str,
    window: &str,
    parent_widget: &str,
    parent_layout: &str,
    out: &mut Buf,
) {
    out.line(&line_comment(&record_body(node, Some(record_parent))));
    let var = c_ident(&node.id);
    match node.kind {
        NodeKind::Dock => {
            for child in &node.children {
                emit_qt(child, &node.id, window, parent_widget, parent_layout, out);
            }
        }
        NodeKind::Region => {
            let side = node.dock.unwrap_or(DockSide::Center);
            if matches!(side, DockSide::Center | DockSide::Tab) {
                out.line(&format!("auto *{var} = new QWidget({parent_widget});"));
                out.line(&format!("{var}->setObjectName({});", c_string(&node.id)));
                out.line(&format!("auto *{var}_layout = new QVBoxLayout({var});"));
                out.line(&format!("{parent_layout}->addWidget({var});"));
            } else {
                out.line(&format!(
                    "auto *{var} = new QDockWidget({}, {window});",
                    c_string(&node.id)
                ));
                out.line(&format!("{var}->setObjectName({});", c_string(&node.id)));
                out.line(&format!(
                    "{window}->addDockWidget({}, {var});",
                    qt_area(side)
                ));
                out.line(&format!("auto *{var}_body = new QWidget({var});"));
                out.line(&format!(
                    "auto *{var}_layout = new QVBoxLayout({var}_body);"
                ));
                out.line(&format!("{var}->setWidget({var}_body);"));
            }
            if !node.text.is_empty() {
                let layout = if matches!(side, DockSide::Center | DockSide::Tab) {
                    format!("{var}_layout")
                } else {
                    format!("{var}_layout")
                };
                out.line(&format!(
                    "auto *{var}_text = new QLabel({});",
                    c_string(&node.text)
                ));
                out.line(&format!("{layout}->addWidget({var}_text);"));
            }
            let (widget, layout) = if matches!(side, DockSide::Center | DockSide::Tab) {
                (var.clone(), format!("{var}_layout"))
            } else {
                (format!("{var}_body"), format!("{var}_layout"))
            };
            for child in &node.children {
                emit_qt(child, &node.id, window, &widget, &layout, out);
            }
        }
        NodeKind::Box | NodeKind::Island | NodeKind::Window => {
            let layout_kind = if node.axis == Some(Axis::Horizontal) {
                "QHBoxLayout"
            } else {
                "QVBoxLayout"
            };
            let widget = if node.kind == NodeKind::Island {
                format!(
                    "auto *{var} = new QGroupBox({}, {parent_widget});",
                    c_string(&visible(node))
                )
            } else {
                format!("auto *{var} = new QWidget({parent_widget});")
            };
            out.line(&widget);
            out.line(&format!("{var}->setObjectName({});", c_string(&node.id)));
            out.line(&format!("auto *{var}_layout = new {layout_kind}({var});"));
            out.line(&format!("{parent_layout}->addWidget({var});"));
            for child in &node.children {
                emit_qt(child, &node.id, window, &var, &format!("{var}_layout"), out);
            }
        }
        NodeKind::Table => {
            let rows = node.rows.len();
            let cols = node.columns.len().max(1);
            out.line(&format!(
                "auto *{var} = new QTableWidget({rows}, {cols}, {parent_widget});"
            ));
            out.line(&format!("{var}->setObjectName({});", c_string(&node.id)));
            if !node.columns.is_empty() {
                let headers = node
                    .columns
                    .iter()
                    .map(|column| c_string(column))
                    .collect::<Vec<_>>()
                    .join(", ");
                out.line(&format!("{var}->setHorizontalHeaderLabels({{{headers}}});"));
            }
            for (r, row) in node.rows.iter().enumerate() {
                for (c, cell) in row.iter().enumerate() {
                    out.line(&format!(
                        "{var}->setItem({r}, {c}, new QTableWidgetItem({}));",
                        c_string(cell)
                    ));
                }
            }
            out.line(&format!("{parent_layout}->addWidget({var});"));
        }
        NodeKind::Label | NodeKind::HeaderBar | NodeKind::Image => {
            out.line(&format!(
                "auto *{var} = new QLabel({}, {parent_widget});",
                c_string(&visible(node))
            ));
            out.line(&format!("{var}->setObjectName({});", c_string(&node.id)));
            out.line(&format!("{parent_layout}->addWidget({var});"));
        }
        NodeKind::Button => {
            out.line(&format!(
                "auto *{var} = new QPushButton({}, {parent_widget});",
                c_string(&visible(node))
            ));
            out.line(&format!("{var}->setObjectName({});", c_string(&node.id)));
            out.line(&format!("{parent_layout}->addWidget({var});"));
        }
        NodeKind::TextInput => {
            out.line(&format!("auto *{var} = new QLineEdit({parent_widget});"));
            out.line(&format!("{var}->setObjectName({});", c_string(&node.id)));
            out.line(&format!("{var}->setText({});", c_string(&node.text)));
            if let Some(placeholder) = node.props.get("placeholder") {
                out.line(&format!(
                    "{var}->setPlaceholderText({});",
                    c_string(placeholder)
                ));
            }
            out.line(&format!("{parent_layout}->addWidget({var});"));
        }
        NodeKind::Checkbox => {
            out.line(&format!(
                "auto *{var} = new QCheckBox({}, {parent_widget});",
                c_string(&visible(node))
            ));
            out.line(&format!("{var}->setObjectName({});", c_string(&node.id)));
            if node.props.get("checked").map(String::as_str) == Some("true") {
                out.line(&format!("{var}->setChecked(true);"));
            }
            out.line(&format!("{parent_layout}->addWidget({var});"));
        }
        NodeKind::Slider => {
            out.line(&format!(
                "auto *{var} = new QSlider(Qt::Horizontal, {parent_widget});"
            ));
            out.line(&format!("{var}->setObjectName({});", c_string(&node.id)));
            out.line(&format!("{parent_layout}->addWidget({var});"));
        }
        NodeKind::Separator => {
            out.line(&format!("auto *{var} = new QFrame({parent_widget});"));
            out.line(&format!("{var}->setFrameShape(QFrame::HLine);"));
            out.line(&format!("{var}->setObjectName({});", c_string(&node.id)));
            out.line(&format!("{parent_layout}->addWidget({var});"));
        }
    }
}

fn qt_area(side: DockSide) -> &'static str {
    match side {
        DockSide::Left => "Qt::LeftDockWidgetArea",
        DockSide::Right => "Qt::RightDockWidgetArea",
        DockSide::Top => "Qt::TopDockWidgetArea",
        DockSide::Bottom => "Qt::BottomDockWidgetArea",
        DockSide::Center | DockSide::Tab => "Qt::LeftDockWidgetArea",
    }
}

pub(crate) fn export_winui(document: &Document) -> Result<String, Error> {
    let mut out = Buf::new();
    out.line(&document_xml(&document.name));
    out.line("<Window");
    out.line("    xmlns=\"http://schemas.microsoft.com/winfx/2006/xaml/presentation\"");
    out.line("    xmlns:x=\"http://schemas.microsoft.com/winfx/2006/xaml\">");
    out.indent += 1;
    emit_winui(&document.root, None, &mut out);
    out.indent -= 1;
    out.line("</Window>");
    Ok(out.text)
}

fn emit_winui(node: &Node, parent: Option<&str>, out: &mut Buf) {
    out.line(&xml_comment(&record_body(node, parent)));
    let name = xml_attr(&node.id);
    match node.kind {
        NodeKind::Box if node.axis == Some(Axis::Horizontal) => {
            out.line(&format!(
                "<StackPanel x:Name=\"{name}\" Orientation=\"Horizontal\">"
            ));
            winui_children(node, out);
            out.line("</StackPanel>");
        }
        NodeKind::Box | NodeKind::Window | NodeKind::Dock | NodeKind::Region | NodeKind::Island => {
            if node.kind == NodeKind::Window
                || node.kind == NodeKind::Dock
                || node.kind == NodeKind::Region
            {
                out.line(&format!("<Grid x:Name=\"{name}\">"));
                winui_children(node, out);
                out.line("</Grid>");
            } else {
                out.line(&format!(
                    "<StackPanel x:Name=\"{name}\" Orientation=\"Vertical\">"
                ));
                winui_children(node, out);
                out.line("</StackPanel>");
            }
        }
        NodeKind::Table => {
            out.line(&format!("<Grid x:Name=\"{name}\">"));
            out.indent += 1;
            for column in &node.columns {
                out.line(&format!("<TextBlock Text=\"{}\"/>", xml_attr(column)));
            }
            for row in &node.rows {
                for cell in row {
                    out.line(&format!("<TextBlock Text=\"{}\"/>", xml_attr(cell)));
                }
            }
            out.indent -= 1;
            out.line("</Grid>");
        }
        NodeKind::Label | NodeKind::HeaderBar | NodeKind::Image => {
            out.line(&format!(
                "<TextBlock x:Name=\"{name}\" Text=\"{}\"/>",
                xml_attr(&visible(node))
            ));
        }
        NodeKind::Button => {
            out.line(&format!(
                "<Button x:Name=\"{name}\" Content=\"{}\"/>",
                xml_attr(&visible(node))
            ));
        }
        NodeKind::TextInput => {
            let placeholder = node
                .props
                .get("placeholder")
                .map(String::as_str)
                .unwrap_or("");
            out.line(&format!(
                "<TextBox x:Name=\"{name}\" Text=\"{}\" PlaceholderText=\"{}\"/>",
                xml_attr(&node.text),
                xml_attr(placeholder)
            ));
        }
        NodeKind::Checkbox => {
            out.line(&format!(
                "<CheckBox x:Name=\"{name}\" Content=\"{}\"/>",
                xml_attr(&visible(node))
            ));
        }
        NodeKind::Slider => out.line(&format!("<Slider x:Name=\"{name}\"/>")),
        NodeKind::Separator => out.line(&format!("<Rectangle x:Name=\"{name}\" Height=\"1\"/>")),
    }
}

fn winui_children(node: &Node, out: &mut Buf) {
    out.indent += 1;
    if !node.text.is_empty()
        && !matches!(
            node.kind,
            NodeKind::Label | NodeKind::Button | NodeKind::HeaderBar | NodeKind::Image
        )
    {
        out.line(&format!("<TextBlock Text=\"{}\"/>", xml_attr(&node.text)));
    }
    for child in &node.children {
        emit_winui(child, Some(&node.id), out);
    }
    out.indent -= 1;
}

pub(crate) fn export_swift(document: &Document) -> Result<String, Error> {
    let mut out = Buf::new();
    out.line(&document_line(&document.name));
    out.line("import SwiftUI");
    out.line("");
    out.line(&format!("struct {}: View {{", type_name(&document.name)));
    out.indent += 1;
    out.line("var body: some View {");
    out.indent += 1;
    emit_swift(&document.root, None, &mut out);
    out.indent -= 1;
    out.line("}");
    out.indent -= 1;
    out.line("}");
    Ok(out.text)
}

fn emit_swift(node: &Node, parent: Option<&str>, out: &mut Buf) {
    out.line(&line_comment(&record_body(node, parent)));
    match node.kind {
        NodeKind::Box if node.axis == Some(Axis::Horizontal) => {
            out.line("HStack {");
            swift_children(node, out);
        }
        NodeKind::Box | NodeKind::Window | NodeKind::Dock | NodeKind::Region | NodeKind::Island => {
            out.line("VStack {");
            swift_children(node, out);
        }
        NodeKind::Table => {
            out.line("Table {");
            out.indent += 1;
            for column in &node.columns {
                out.line(&format!(
                    "TableColumn({}) {{ (_: String) in Text(\"\") }}",
                    rust_string(column)
                ));
            }
            out.indent -= 1;
            out.line("}");
        }
        NodeKind::Label | NodeKind::HeaderBar | NodeKind::Image => {
            out.line(&format!("Text({})", rust_string(&visible(node))));
        }
        NodeKind::Button => {
            out.line(&format!("Button({}) {{ }}", rust_string(&visible(node))));
        }
        NodeKind::TextInput => {
            let placeholder = node
                .props
                .get("placeholder")
                .map(String::as_str)
                .unwrap_or("");
            out.line(&format!(
                "TextField({}, text: .constant({}))",
                rust_string(placeholder),
                rust_string(&node.text)
            ));
        }
        NodeKind::Checkbox => {
            let on = node.props.get("checked").map(String::as_str) == Some("true");
            out.line(&format!(
                "Toggle({}, isOn: .constant({on}))",
                rust_string(&visible(node))
            ));
        }
        NodeKind::Slider => {
            out.line(&format!(
                "Slider(value: .constant({}))",
                node.props
                    .get("value")
                    .and_then(|v| v.parse::<f32>().ok())
                    .unwrap_or(0.0)
            ));
        }
        NodeKind::Separator => out.line("Divider()"),
    }
}

fn swift_children(node: &Node, out: &mut Buf) {
    out.indent += 1;
    if !node.text.is_empty()
        && !matches!(
            node.kind,
            NodeKind::Label | NodeKind::Button | NodeKind::HeaderBar
        )
    {
        out.line(&format!("Text({})", rust_string(&node.text)));
    }
    for child in &node.children {
        emit_swift(child, Some(&node.id), out);
    }
    out.indent -= 1;
    out.line("}");
}

pub(crate) fn export_gtk(document: &Document) -> Result<String, Error> {
    let mut out = Buf::new();
    out.line(&document_line(&document.name));
    out.line("#include <gtk/gtk.h>");
    out.line("");
    out.line("static void uii_gtk(void) {");
    out.indent += 1;
    emit_gtk(&document.root, None, "NULL", &mut out);
    out.indent -= 1;
    out.line("}");
    Ok(out.text)
}

fn emit_gtk(node: &Node, parent: Option<&str>, parent_var: &str, out: &mut Buf) {
    out.line(&line_comment(&record_body(node, parent)));
    let var = c_ident(&node.id);
    match node.kind {
        NodeKind::Window => {
            out.line(&format!("GtkWidget *{var} = gtk_window_new();"));
            out.line(&format!(
                "gtk_window_set_title(GTK_WINDOW({var}), {});",
                c_string(&node.name)
            ));
            for child in &node.children {
                emit_gtk(child, Some(&node.id), &var, out);
            }
        }
        NodeKind::Box | NodeKind::Dock | NodeKind::Region | NodeKind::Island => {
            let orient = if node.axis == Some(Axis::Horizontal) {
                "GTK_ORIENTATION_HORIZONTAL"
            } else {
                "GTK_ORIENTATION_VERTICAL"
            };
            out.line(&format!("GtkWidget *{var} = gtk_box_new({orient}, 0);"));
            if parent_var != "NULL" {
                out.line(&format!("gtk_box_append(GTK_BOX({parent_var}), {var});"));
            }
            if !node.text.is_empty() {
                out.line(&format!(
                    "gtk_box_append(GTK_BOX({var}), gtk_label_new({}));",
                    c_string(&node.text)
                ));
            }
            for child in &node.children {
                emit_gtk(child, Some(&node.id), &var, out);
            }
        }
        NodeKind::Table => {
            out.line(&format!("GtkWidget *{var} = gtk_column_view_new(NULL);"));
            for column in &node.columns {
                out.line(&format!(
                    "gtk_column_view_append_column(GTK_COLUMN_VIEW({var}), gtk_column_view_column_new({}, NULL));",
                    c_string(column)
                ));
            }
            if parent_var != "NULL" {
                out.line(&format!("gtk_box_append(GTK_BOX({parent_var}), {var});"));
            }
        }
        NodeKind::Label | NodeKind::HeaderBar | NodeKind::Image => {
            out.line(&format!(
                "GtkWidget *{var} = gtk_label_new({});",
                c_string(&visible(node))
            ));
            if parent_var != "NULL" {
                out.line(&format!("gtk_box_append(GTK_BOX({parent_var}), {var});"));
            }
        }
        NodeKind::Button => {
            out.line(&format!(
                "GtkWidget *{var} = gtk_button_new_with_label({});",
                c_string(&visible(node))
            ));
            if parent_var != "NULL" {
                out.line(&format!("gtk_box_append(GTK_BOX({parent_var}), {var});"));
            }
        }
        NodeKind::TextInput | NodeKind::Checkbox | NodeKind::Slider | NodeKind::Separator => {
            out.line(&format!(
                "GtkWidget *{var} = gtk_label_new({});",
                c_string(&visible(node))
            ));
            if parent_var != "NULL" {
                out.line(&format!("gtk_box_append(GTK_BOX({parent_var}), {var});"));
            }
        }
    }
}

pub(crate) fn export_compose(document: &Document) -> Result<String, Error> {
    let mut out = Buf::new();
    out.line(&document_line(&document.name));
    out.line("import androidx.compose.runtime.Composable");
    out.line("import androidx.compose.foundation.layout.Column");
    out.line("import androidx.compose.foundation.layout.Row");
    out.line("import androidx.compose.material3.Text");
    out.line("import androidx.compose.material3.Button");
    out.line("");
    out.line("@Composable");
    out.line(&format!("fun {}() {{", type_name(&document.name)));
    out.indent += 1;
    emit_compose(&document.root, None, &mut out);
    out.indent -= 1;
    out.line("}");
    Ok(out.text)
}

fn emit_compose(node: &Node, parent: Option<&str>, out: &mut Buf) {
    out.line(&line_comment(&record_body(node, parent)));
    match node.kind {
        NodeKind::Box if node.axis == Some(Axis::Horizontal) => {
            out.line("Row {");
            compose_children(node, out);
        }
        NodeKind::Box | NodeKind::Window | NodeKind::Dock | NodeKind::Region | NodeKind::Island => {
            out.line("Column {");
            compose_children(node, out);
        }
        NodeKind::Table => {
            out.line("Column {");
            out.indent += 1;
            for column in &node.columns {
                out.line(&format!("Text({})", rust_string(column)));
            }
            for row in &node.rows {
                for cell in row {
                    out.line(&format!("Text({})", rust_string(cell)));
                }
            }
            out.indent -= 1;
            out.line("}");
        }
        NodeKind::Label | NodeKind::HeaderBar | NodeKind::Image => {
            out.line(&format!("Text({})", rust_string(&visible(node))));
        }
        NodeKind::Button => {
            out.line(&format!(
                "Button(onClick = {{}}) {{ Text({}) }}",
                rust_string(&visible(node))
            ));
        }
        NodeKind::TextInput => {
            out.line(&format!("Text({})", rust_string(&node.text)));
        }
        NodeKind::Checkbox | NodeKind::Slider | NodeKind::Separator => {
            out.line(&format!("Text({})", rust_string(&visible(node))));
        }
    }
}

fn compose_children(node: &Node, out: &mut Buf) {
    out.indent += 1;
    if !node.text.is_empty()
        && !matches!(
            node.kind,
            NodeKind::Label | NodeKind::Button | NodeKind::HeaderBar
        )
    {
        out.line(&format!("Text({})", rust_string(&node.text)));
    }
    for child in &node.children {
        emit_compose(child, Some(&node.id), out);
    }
    out.indent -= 1;
    out.line("}");
}
