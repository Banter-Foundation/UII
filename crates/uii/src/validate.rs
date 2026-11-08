//! After a document is parsed, require the native calls the mapping asked for.
//! A record line alone is not an import of that toolkit.

use crate::error::Error;
use crate::model::{banter_region_variant, Axis, DockSide, Document, Format, Node, NodeKind};

pub(crate) fn check(format: Format, document: &Document, source: &str) -> Result<(), Error> {
    let mut missing = Vec::new();
    match format {
        Format::HtmlCss | Format::React | Format::Vue | Format::Svelte | Format::Angular => {
            check_markup(format, document, source, &mut missing);
        }
        Format::DearImgui => check_dear(document, source, &mut missing, false),
        Format::RavenImgui => check_dear(document, source, &mut missing, true),
        Format::BanterImgui => check_banter(document, source, &mut missing),
        Format::Egui => check_egui(document, source, &mut missing),
        Format::Slint => check_slint(document, source, &mut missing),
        Format::Qt => check_qt(document, source, &mut missing),
        Format::WinUi => check_winui(document, source, &mut missing),
        Format::SwiftUi => check_swift(document, source, &mut missing),
        Format::Gtk => check_gtk(document, source, &mut missing),
        Format::JetpackCompose => check_compose(document, source, &mut missing),
    }
    if missing.is_empty() {
        Ok(())
    } else {
        Err(Error::new(format!("missing {}", missing.join(", "))))
    }
}

fn check_markup(format: Format, document: &Document, source: &str, missing: &mut Vec<String>) {
    let need = |missing: &mut Vec<String>, ok: bool, needle: &str| {
        if !ok {
            missing.push(needle.to_string());
        }
    };
    match format {
        Format::React => need(
            missing,
            source.contains("export function"),
            "export function",
        ),
        Format::Vue => need(missing, source.contains("<template"), "<template"),
        Format::Svelte => need(missing, source.contains("uii-svelte"), "uii-svelte"),
        Format::Angular => need(missing, source.contains("@Component"), "@Component"),
        _ => {}
    }
    if has(document, |node| node.kind == NodeKind::Region) {
        need(
            missing,
            source.contains("<header")
                || source.contains("<nav")
                || source.contains("<main")
                || source.contains("<aside")
                || source.contains("<footer")
                || source.contains("<section")
                || source.contains("data-uii-kind=\"region\""),
            "HTML region",
        );
    }
    if has(document, |node| node.kind == NodeKind::Table) {
        need(missing, source.contains("<table"), "<table");
    }
    if has(document, |node| node.kind == NodeKind::Box) {
        need(missing, source.contains("<div"), "<div");
    }
    if has(document, |node| node.kind == NodeKind::Button) {
        need(missing, source.contains("<button"), "<button");
    }
    if has(document, |node| node.kind == NodeKind::Label) {
        need(
            missing,
            source.contains("<span")
                || source.contains("<p")
                || source.contains("data-uii-kind=\"label\""),
            "<span",
        );
    }
    if has(document, |node| node.kind == NodeKind::TextInput) {
        need(
            missing,
            source.contains("<input") || source.contains("data-uii-kind=\"text-input\""),
            "<input",
        );
    }
    if has(document, |node| node.kind == NodeKind::Checkbox) {
        need(
            missing,
            source.contains("type=\"checkbox\"") || source.contains("data-uii-kind=\"checkbox\""),
            "checkbox",
        );
    }
    if has(document, |node| node.kind == NodeKind::Slider) {
        need(
            missing,
            source.contains("type=\"range\"") || source.contains("data-uii-kind=\"slider\""),
            "slider",
        );
    }
    if has(document, |node| node.kind == NodeKind::Image) {
        need(
            missing,
            source.contains("<img") || source.contains("data-uii-kind=\"image\""),
            "<img",
        );
    }
    if has(document, |node| node.kind == NodeKind::Separator) {
        need(
            missing,
            source.contains("<hr") || source.contains("data-uii-kind=\"separator\""),
            "<hr",
        );
    }
    if has(document, |node| node.kind == NodeKind::HeaderBar) {
        need(
            missing,
            source.contains("data-uii-kind=\"header-bar\""),
            "data-uii-kind=\"header-bar\"",
        );
    }
    if has(document, |node| node.kind == NodeKind::Island) {
        need(
            missing,
            source.contains("data-uii-kind=\"island\""),
            "data-uii-kind=\"island\"",
        );
    }
}

fn check_dear(document: &Document, source: &str, missing: &mut Vec<String>, raven: bool) {
    let need = |missing: &mut Vec<String>, needle: &str| {
        if !source.contains(needle) {
            missing.push(needle.to_string());
        }
    };
    if has(document, |node| node.kind == NodeKind::Window) {
        need(missing, "ImGui::Begin(");
    }
    if has(document, |node| node.kind == NodeKind::Dock) {
        need(missing, "ImGui::DockSpace");
    }
    if has(document, |node| node.kind == NodeKind::Table) {
        need(missing, "ImGui::BeginTable");
    }
    if has(document, |node| node.kind == NodeKind::Box) {
        need(missing, "ImGui::BeginChild");
    }
    if has(document, |node| node.kind == NodeKind::Region) {
        need(missing, "ImGui::DockBuilderDockWindow");
    }
    if has_side(document, DockSide::Left) {
        need(missing, "ImGuiDir_Left");
    }
    if has_side(document, DockSide::Right) {
        need(missing, "ImGuiDir_Right");
    }
    if has_side(document, DockSide::Top) {
        need(missing, "ImGuiDir_Up");
    }
    if has_side(document, DockSide::Bottom) {
        need(missing, "ImGuiDir_Down");
    }
    if !raven {
        return;
    }
    need(missing, "#include <Raven.h>");
    if has(document, |node| node.kind == NodeKind::Dock) {
        need(missing, "Rdock_area_create");
        need(missing, "Rdock_area_add_panel");
    }
    if has_side(document, DockSide::Left) {
        need(missing, "R_DOCK_LEFT");
    }
    if has_side(document, DockSide::Right) {
        need(missing, "R_DOCK_RIGHT");
    }
    if has_side(document, DockSide::Top) {
        need(missing, "R_DOCK_TOP");
    }
    if has_side(document, DockSide::Bottom) {
        need(missing, "R_DOCK_BOTTOM");
    }
    if has(document, |node| node.kind == NodeKind::Table) {
        need(missing, "R_WIDGET_TABLE");
    }
    if has(document, |node| !node.columns.is_empty()) {
        need(missing, "Rtable_add_column");
    }
    if has(document, |node| !node.rows.is_empty()) {
        need(missing, "Rtable_add_row");
        need(missing, "Rtable_set_cell");
    }
    if has(document, |node| {
        node.kind == NodeKind::Region
            || (node.kind == NodeKind::Box && node.axis != Some(Axis::Horizontal))
    }) {
        need(missing, "R_WIDGET_VBOX");
    }
    if has(document, |node| {
        node.kind == NodeKind::Box && node.axis == Some(Axis::Horizontal)
    }) {
        need(missing, "R_WIDGET_HBOX");
    }
    if has(document, |node| node.kind == NodeKind::HeaderBar) {
        need(missing, "Rheader_bar_create");
    }
    if has(document, |node| node.kind == NodeKind::Island) {
        need(missing, "R_WIDGET_ISLAND");
    }
    if has(document, |node| node.kind == NodeKind::Button) {
        need(missing, "Rbutton_create");
    }
    if has(document, |node| node.kind == NodeKind::Checkbox) {
        need(missing, "Rcheckbox_create");
    }
    if has(document, |node| {
        node.kind == NodeKind::TextInput && node.props.contains_key("placeholder")
    }) {
        need(missing, "Rtext_input_set_placeholder");
    }
}

fn check_banter(document: &Document, source: &str, missing: &mut Vec<String>) {
    let need = |missing: &mut Vec<String>, needle: &str| {
        if !source.contains(needle) {
            missing.push(needle.to_string());
        }
    };
    need(missing, "dear_imgui_rs");
    need(missing, "ConfigFlags::DOCKING_ENABLE");
    if has(document, |node| node.kind == NodeKind::Dock) {
        need(missing, "ui.dockspace()");
    }
    if has(document, |node| node.kind == NodeKind::Table) {
        need(missing, "ui.table(");
    }
    if has(document, |node| node.kind == NodeKind::Box) {
        need(missing, "child_window(");
    }
    if has_side(document, DockSide::Left) {
        need(missing, "DockSplit::Left");
    }
    if has_side(document, DockSide::Right) {
        need(missing, "DockSplit::Right");
    }
    if has_side(document, DockSide::Top) {
        need(missing, "DockSplit::Up");
    }
    if has_side(document, DockSide::Bottom) {
        need(missing, "DockSplit::Down");
    }
    walk(document, &mut |node| {
        for slug in [node.id.as_str(), node.name.as_str()] {
            if let Some(variant) = banter_region_variant(slug) {
                let needle = format!("layout::Region::{variant}");
                if !source.contains(&needle) {
                    missing.push(needle);
                }
            }
        }
    });
}

fn check_egui(document: &Document, source: &str, missing: &mut Vec<String>) {
    let need = |missing: &mut Vec<String>, needle: &str| {
        if !source.contains(needle) {
            missing.push(needle.to_string());
        }
    };
    need(missing, "egui::");
    if has_side(document, DockSide::Left) {
        need(missing, "SidePanel::left");
    }
    if has_side(document, DockSide::Right) {
        need(missing, "SidePanel::right");
    }
    if has_side(document, DockSide::Top) {
        need(missing, "TopBottomPanel::top");
    }
    if has_side(document, DockSide::Bottom) {
        need(missing, "TopBottomPanel::bottom");
    }
    if has_side(document, DockSide::Center) || has_side(document, DockSide::Tab) {
        need(missing, "CentralPanel");
    }
    if has(document, |node| node.kind == NodeKind::Table) {
        need(missing, "TableBuilder");
    }
    if has(document, |node| {
        node.kind == NodeKind::Box && node.axis != Some(Axis::Horizontal)
    }) {
        need(missing, "ui.vertical");
    }
    if has(document, |node| {
        node.kind == NodeKind::Box && node.axis == Some(Axis::Horizontal)
    }) {
        need(missing, "ui.horizontal");
    }
}

fn check_slint(document: &Document, source: &str, missing: &mut Vec<String>) {
    let need = |missing: &mut Vec<String>, needle: &str| {
        if !source.contains(needle) {
            missing.push(needle.to_string());
        }
    };
    need(missing, "export component");
    if has(document, |node| {
        node.kind == NodeKind::Box && node.axis != Some(Axis::Horizontal)
    }) {
        need(missing, "VerticalBox");
    }
    if has(document, |node| {
        node.kind == NodeKind::Box && node.axis == Some(Axis::Horizontal)
    }) {
        need(missing, "HorizontalBox");
    }
}

fn check_qt(document: &Document, source: &str, missing: &mut Vec<String>) {
    let need = |missing: &mut Vec<String>, needle: &str| {
        if !source.contains(needle) {
            missing.push(needle.to_string());
        }
    };
    need(missing, "QMainWindow");
    if has(document, |node| node.kind == NodeKind::Region) {
        need(missing, "QDockWidget");
    }
    if has(document, |node| node.kind == NodeKind::Table) {
        need(missing, "QTableWidget");
    }
    if has(document, |node| {
        node.kind == NodeKind::Box && node.axis != Some(Axis::Horizontal)
    }) {
        need(missing, "QVBoxLayout");
    }
    if has(document, |node| {
        node.kind == NodeKind::Box && node.axis == Some(Axis::Horizontal)
    }) {
        need(missing, "QHBoxLayout");
    }
    if has_side(document, DockSide::Left) {
        need(missing, "Qt::LeftDockWidgetArea");
    }
    if has_side(document, DockSide::Right) {
        need(missing, "Qt::RightDockWidgetArea");
    }
    if has_side(document, DockSide::Top) {
        need(missing, "Qt::TopDockWidgetArea");
    }
    if has_side(document, DockSide::Bottom) {
        need(missing, "Qt::BottomDockWidgetArea");
    }
}

fn check_winui(document: &Document, source: &str, missing: &mut Vec<String>) {
    if !source.contains("<Window") {
        missing.push("<Window".into());
    }
    if has(document, |node| node.kind == NodeKind::Box) && !source.contains("StackPanel") {
        missing.push("StackPanel".into());
    }
}

fn check_swift(document: &Document, source: &str, missing: &mut Vec<String>) {
    if !source.contains("import SwiftUI") {
        missing.push("import SwiftUI".into());
    }
    if has(document, |node| {
        node.kind == NodeKind::Box && node.axis != Some(Axis::Horizontal)
    }) && !source.contains("VStack")
    {
        missing.push("VStack".into());
    }
    if has(document, |node| {
        node.kind == NodeKind::Box && node.axis == Some(Axis::Horizontal)
    }) && !source.contains("HStack")
    {
        missing.push("HStack".into());
    }
    if has(document, |node| {
        node.kind == NodeKind::Table && !node.columns.is_empty()
    }) && !source.contains("TableColumn")
    {
        missing.push("TableColumn".into());
    }
}

fn check_gtk(document: &Document, source: &str, missing: &mut Vec<String>) {
    if !source.contains("gtk_window_new") {
        missing.push("gtk_window_new".into());
    }
    if has(document, |node| node.kind == NodeKind::Box) && !source.contains("gtk_box_new") {
        missing.push("gtk_box_new".into());
    }
    if has(document, |node| node.kind == NodeKind::Table) && !source.contains("gtk_column_view_new")
    {
        missing.push("gtk_column_view_new".into());
    }
}

fn check_compose(document: &Document, source: &str, missing: &mut Vec<String>) {
    if !source.contains("@Composable") {
        missing.push("@Composable".into());
    }
    if has(document, |node| {
        node.kind == NodeKind::Box && node.axis != Some(Axis::Horizontal)
    }) && !source.contains("Column")
    {
        missing.push("Column".into());
    }
    if has(document, |node| {
        node.kind == NodeKind::Box && node.axis == Some(Axis::Horizontal)
    }) && !source.contains("Row")
    {
        missing.push("Row".into());
    }
}

fn has(document: &Document, pred: impl Fn(&Node) -> bool) -> bool {
    fn walk(node: &Node, pred: &impl Fn(&Node) -> bool) -> bool {
        pred(node) || node.children.iter().any(|child| walk(child, pred))
    }
    walk(&document.root, &pred)
}

fn has_side(document: &Document, side: DockSide) -> bool {
    has(document, |node| {
        node.kind == NodeKind::Region && node.dock == Some(side)
    })
}

fn walk(document: &Document, visit: &mut impl FnMut(&Node)) {
    fn go(node: &Node, visit: &mut impl FnMut(&Node)) {
        visit(node);
        for child in &node.children {
            go(child, visit);
        }
    }
    go(&document.root, visit);
}
