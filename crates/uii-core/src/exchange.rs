//! Import and export. HTML and CSS are the editor's layout. Every other
//! format reads that tree and writes it back.

use std::collections::HashMap;

use crate::document::{DockSlot, Document, NodeId, NodeKind, Rect};
use crate::html::{self, MarkupNode};
use crate::scan::{self, take_brace, Arg};

#[derive(Debug)]
pub struct FormatError {
    pub message: String,
}

pub struct FormatInfo {
    pub id: &'static str,
    pub label: &'static str,
    pub import: bool,
    pub export: bool,
}

pub fn formats() -> Vec<FormatInfo> {
    vec![
        info("html-css", "HTML and CSS"),
        info("react", "React (JSX and CSS)"),
        info("vue", "Vue"),
        info("svelte", "Svelte"),
        info("angular", "Angular"),
        info("banter-imgui", "Banter imgui"),
        info("raven-imgui", "Raven imgui"),
        info("imgui-cpp", "Dear ImGui C++"),
        info("egui", "egui"),
        info("slint", "Slint"),
        info("qt-widgets", "Qt Widgets"),
        info("winui", "WinUI"),
        info("swiftui", "SwiftUI"),
        info("gtk", "GTK"),
        info("compose", "Jetpack Compose"),
    ]
}

fn info(id: &'static str, label: &'static str) -> FormatInfo {
    FormatInfo {
        id,
        label,
        import: true,
        export: true,
    }
}

pub fn html_and_css(doc: &Document) -> (String, String) {
    html::html_and_css_parts(doc)
}

pub fn export_source(doc: &Document, format: &str) -> Result<String, FormatError> {
    Ok(match format {
        "html-css" => html::export_html_document(doc),
        "react" => export_react(doc),
        "vue" => export_vue(doc),
        "svelte" => export_svelte(doc),
        "angular" => export_angular(doc),
        "banter-imgui" => export_banter(doc),
        "raven-imgui" => export_raven(doc),
        "imgui-cpp" => wrap_calls(doc, CPP_SUPPORT),
        "egui" => wrap_calls(doc, EGUI_SUPPORT),
        "slint" => export_slint(doc),
        "qt-widgets" => wrap_calls(doc, QT_SUPPORT),
        "winui" => export_winui(doc),
        "swiftui" => wrap_calls(doc, SWIFT_SUPPORT),
        "gtk" => wrap_calls(doc, GTK_SUPPORT),
        "compose" => wrap_calls(doc, COMPOSE_SUPPORT),
        other => {
            return Err(FormatError {
                message: format!("unknown format {other}"),
            })
        }
    })
}

pub fn import_source(format: &str, source: &str) -> Result<Document, FormatError> {
    Ok(match format {
        "html-css" => html::import_html_css(source, ""),
        "react" => import_react(source),
        "vue" => import_vue(source),
        "svelte" => html::import_html_css(source, ""),
        "angular" => import_angular(source),
        "banter-imgui" => import_banter(source),
        "raven-imgui" => import_raven(source),
        "imgui-cpp" | "egui" | "qt-widgets" | "swiftui" | "gtk" | "compose" => import_calls(source),
        "slint" => import_slint(source),
        "winui" => import_winui(source),
        other => {
            return Err(FormatError {
                message: format!("unknown format {other}"),
            })
        }
    })
}

fn export_react(doc: &Document) -> String {
    let (html, css) = html::html_and_css_parts(doc);
    let jsx = html.replace(" class=\"", " className=\"");
    format!("import \"./Design.css\";\n\nexport function Design() {{\n  return (\n{jsx}  );\n}}\n\n/* Design.css */\n{css}\n")
}

fn import_react(src: &str) -> Document {
    let (code, css) = split_once(src, "/* Design.css */");
    let jsx = if let Some(at) = code.find("return (") {
        let start = at + "return (".len();
        if let Some(rel) = code[start..].rfind(");") {
            code[start..start + rel].to_string()
        } else {
            code[start..].to_string()
        }
    } else {
        code
    };
    html::import_html_css(&jsx.replace("className=", "class="), &css)
}

fn export_vue(doc: &Document) -> String {
    let (html, css) = html::html_and_css_parts(doc);
    format!("<template>\n{html}</template>\n\n<style>\n{css}</style>\n")
}

fn import_vue(src: &str) -> Document {
    let html = between(src, "<template>", "</template>").unwrap_or_else(|| src.to_string());
    let css = between(src, "<style>", "</style>").unwrap_or_default();
    html::import_html_css(&html, &css)
}

fn export_svelte(doc: &Document) -> String {
    let (html, css) = html::html_and_css_parts(doc);
    format!("<script>\n  // UII layout. The markup below is HTML and CSS.\n</script>\n{html}<style>\n{css}</style>\n")
}

fn export_angular(doc: &Document) -> String {
    let (html, css) = html::html_and_css_parts(doc);
    format!(
        "import {{ Component }} from \"@angular/core\";\n\n@Component({{\n  selector: \"uii-design\",\n  template: `\n{html}\n  `,\n  styles: [`\n{css}\n  `],\n}})\nexport class UiiDesignComponent {{}}\n",
        html = escape_tick(&html),
        css = escape_tick(&css),
    )
}

fn import_angular(src: &str) -> Document {
    let html = backtick_after(src, "template:").unwrap_or_default();
    let css = backtick_after(src, "styles:").unwrap_or_default();
    html::import_html_css(&html, &css)
}

fn split_once(src: &str, marker: &str) -> (String, String) {
    if let Some(at) = src.find(marker) {
        (src[..at].to_string(), src[at + marker.len()..].to_string())
    } else {
        (src.to_string(), String::new())
    }
}

fn between(src: &str, start: &str, end: &str) -> Option<String> {
    let at = src.find(start)? + start.len();
    let rel = src[at..].find(end)?;
    Some(src[at..at + rel].to_string())
}

fn escape_tick(text: &str) -> String {
    text.replace('\\', "\\\\").replace('`', "\\`").replace("${", "\\${")
}

fn backtick_after(src: &str, marker: &str) -> Option<String> {
    let at = src.find(marker)? + marker.len();
    let rest = &src[at..];
    let start = rest.find('`')? + 1;
    let bytes = rest.as_bytes();
    let mut out = String::new();
    let mut i = start;
    while i < bytes.len() {
        if bytes[i] == b'\\' && i + 1 < bytes.len() {
            out.push(bytes[i + 1] as char);
            i += 2;
            continue;
        }
        if bytes[i] == b'`' {
            return Some(out);
        }
        out.push(bytes[i] as char);
        i += 1;
    }
    None
}

struct Pending {
    name: Option<String>,
    kind: Option<NodeKind>,
    slot: Option<DockSlot>,
    image: Option<String>,
    x: f32,
    y: f32,
    w: f32,
    h: f32,
}

impl Pending {
    fn new() -> Self {
        Self {
            name: None,
            kind: None,
            slot: None,
            image: None,
            x: 0.0,
            y: 0.0,
            w: 0.0,
            h: 0.0,
        }
    }

    fn take_name(&mut self, fallback: &str) -> String {
        self.name.take().filter(|name| !name.is_empty()).unwrap_or_else(|| fallback.to_string())
    }

    fn clear_box(&mut self) {
        self.name = None;
        self.kind = None;
        self.slot = None;
        self.image = None;
        self.w = 0.0;
        self.h = 0.0;
    }
}

fn apply_comment(line: &str, pending: &mut Pending) {
    let line = line.trim();
    if let Some(value) = line.strip_prefix("uii-name:") {
        pending.name = Some(value.trim().to_string());
    } else if let Some(value) = line.strip_prefix("uii-kind:") {
        pending.kind = parse_kind(value.trim());
    } else if let Some(value) = line.strip_prefix("uii-slot:") {
        pending.slot = DockSlot::parse(value.trim());
    } else if let Some(value) = line.strip_prefix("uii-image:") {
        pending.image = Some(value.trim().to_string());
    }
}

fn parse_kind(text: &str) -> Option<NodeKind> {
    Some(match text {
        "frame" => NodeKind::Frame,
        "text" => NodeKind::Text,
        "rectangle" => NodeKind::Rectangle,
        "ellipse" => NodeKind::Ellipse,
        "button" => NodeKind::Button,
        "image" => NodeKind::Image,
        "table" => NodeKind::Table,
        "dock" => NodeKind::Dock,
        "child" => NodeKind::Child,
        _ => return None,
    })
}

fn kind_name(kind: NodeKind) -> &'static str {
    match kind {
        NodeKind::Frame => "frame",
        NodeKind::Text => "text",
        NodeKind::Rectangle => "rectangle",
        NodeKind::Ellipse => "ellipse",
        NodeKind::Button => "button",
        NodeKind::Image => "image",
        NodeKind::Table => "table",
        NodeKind::Dock => "dock",
        NodeKind::Child => "child",
    }
}

fn slot_name(slot: Option<DockSlot>) -> &'static str {
    slot.unwrap_or(DockSlot::Center).as_str()
}

fn one_line(text: &str) -> String {
    text.replace(['\n', '\r'], " ")
}

fn add_node(doc: &mut Document, parent: Option<NodeId>, kind: NodeKind, name: &str, x: f32, y: f32, w: f32, h: f32) -> NodeId {
    doc.create(parent, kind, name, Rect::new(x, y, w.max(1.0), h.max(1.0)))
}

fn export_banter(doc: &Document) -> String {
    let mut keys = String::new();
    let mut body = String::new();
    let mut wins: Vec<(String, DockSlot)> = Vec::new();
    let mut counter = 0;
    for id in doc.roots() {
        emit_banter_root(doc, *id, &mut keys, &mut body, &mut wins, &mut counter);
    }
    let layout = dock_layout(&wins);
    format!(
        "use dear_imgui_rs::{{Condition, DockLayout, DockLayoutApply, DockSplit, TableColumnFlags, TableFlags, Ui, WindowKey}};\n\n\
pub fn draw(ui: &Ui, mut image: impl FnMut(&str) -> Option<dear_imgui_rs::texture::TextureId>) {{\n\
{keys}\
    let layout = {layout};\n\
    ui.dockspace().layout(&layout, DockLayoutApply::IfMissing).build().ok();\n\
{body}}}\n"
    )
}

fn dock_layout(wins: &[(String, DockSlot)]) -> String {
    fn tabs(vars: &[String]) -> String {
        let inner = vars.iter().map(|var| format!("&{var}")).collect::<Vec<_>>().join(", ");
        format!("DockLayout::tabs([{inner}])")
    }
    let group = |slot: DockSlot| -> Vec<String> {
        wins.iter().filter(|(_, item)| *item == slot).map(|(var, _)| var.clone()).collect()
    };
    let mut expr = tabs(&group(DockSlot::Center));
    for (slot, dir) in [
        (DockSlot::Right, "Right"),
        (DockSlot::Left, "Left"),
        (DockSlot::Bottom, "Down"),
        (DockSlot::Top, "Up"),
    ] {
        let vars = group(slot);
        if vars.is_empty() {
            continue;
        }
        expr = format!("DockLayout::split(DockSplit::{dir}, 0.2, {}, {expr})", tabs(&vars));
    }
    expr
}

fn emit_banter_root(
    doc: &Document,
    id: NodeId,
    keys: &mut String,
    body: &mut String,
    wins: &mut Vec<(String, DockSlot)>,
    counter: &mut u32,
) {
    let kind = doc.node(id).kind;
    if matches!(kind, NodeKind::Dock | NodeKind::Frame) {
        let var = format!("k{counter}");
        *counter += 1;
        let name = doc.node(id).name.clone();
        keys.push_str(&format!(
            "    let {var} = WindowKey::new({}, {}).unwrap();\n",
            scan::rust_string(&var),
            scan::rust_string(&name)
        ));
        if kind == NodeKind::Dock {
            wins.push((var.clone(), doc.node(id).dock_slot.unwrap_or(DockSlot::Center)));
        } else {
            body.push_str(&format!("    // uii-kind: {}\n", kind_name(kind)));
        }
        let rect = doc.node(id).rect;
        body.push_str(&format!(
            "    ui.window(&{var})\n        .position([{}, {}], Condition::Always)\n        .size([{}, {}], Condition::Always)\n        .build(|| {{\n",
            scan::num(rect.x),
            scan::num(rect.y),
            scan::num(rect.width),
            scan::num(rect.height)
        ));
        for child in doc.children(id).to_vec() {
            emit_banter_widget(doc, child, body, 2);
        }
        body.push_str("    });\n");
    } else {
        emit_banter_widget(doc, id, body, 1);
    }
}

fn emit_banter_widget(doc: &Document, id: NodeId, out: &mut String, depth: usize) {
    let pad = "    ".repeat(depth);
    let node = doc.node(id);
    let kind = node.kind;
    let name = one_line(&node.name);
    let text = node.text.clone();
    let src = node.image_src.clone();
    let rect = node.rect;
    let columns = node.columns.clone();
    let rows = node.rows.clone();
    let slot = node.dock_slot;
    let children = doc.children(id).to_vec();
    let x = scan::num(rect.x);
    let y = scan::num(rect.y);
    let w = scan::num(rect.width);
    let h = scan::num(rect.height);
    match kind {
        NodeKind::Text => {
            out.push_str(&format!("{pad}// uii-name: {name}\n"));
            out.push_str(&format!("{pad}ui.set_cursor_pos([{x}, {y}]);\n"));
            out.push_str(&format!("{pad}ui.dummy([{w}, {h}]);\n"));
            out.push_str(&format!("{pad}ui.set_cursor_pos([{x}, {y}]);\n"));
            out.push_str(&format!("{pad}ui.text({});\n", scan::rust_string(&text)));
        }
        NodeKind::Button => {
            out.push_str(&format!("{pad}// uii-name: {name}\n"));
            out.push_str(&format!("{pad}ui.set_cursor_pos([{x}, {y}]);\n"));
            out.push_str(&format!(
                "{pad}let _ = ui.button_with_size({}, [{w}, {h}]);\n",
                scan::rust_string(&text)
            ));
        }
        NodeKind::Image => {
            out.push_str(&format!("{pad}// uii-name: {name}\n"));
            out.push_str(&format!("{pad}// uii-image: {}\n", one_line(&src)));
            out.push_str(&format!("{pad}ui.set_cursor_pos([{x}, {y}]);\n"));
            out.push_str(&format!("{pad}ui.dummy([{w}, {h}]);\n"));
            out.push_str(&format!("{pad}if let Some(tex) = image({}) {{\n", scan::rust_string(&src)));
            out.push_str(&format!("{pad}    ui.image(tex, [{w}, {h}]);\n"));
            out.push_str(&format!("{pad}}}\n"));
        }
        NodeKind::Table => {
            out.push_str(&format!("{pad}// uii-name: {name}\n"));
            out.push_str(&format!("{pad}ui.set_cursor_pos([{x}, {y}]);\n"));
            out.push_str(&format!(
                "{pad}if let Some(_table) = ui.begin_table_with_sizing({}, {}, TableFlags::BORDERS, [{w}, {h}], 0.0) {{\n",
                scan::rust_string(&name),
                columns.len().max(1)
            ));
            for column in &columns {
                out.push_str(&format!(
                    "{pad}    ui.table_setup_column({}, TableColumnFlags::WIDTH_STRETCH, None);\n",
                    scan::rust_string(column)
                ));
            }
            out.push_str(&format!("{pad}    ui.table_headers_row();\n"));
            for row in &rows {
                out.push_str(&format!("{pad}    ui.table_next_row();\n"));
                for cell in row {
                    out.push_str(&format!("{pad}    ui.table_next_column();\n"));
                    out.push_str(&format!("{pad}    ui.text({});\n", scan::rust_string(cell)));
                }
            }
            out.push_str(&format!("{pad}}}\n"));
        }
        NodeKind::Rectangle | NodeKind::Ellipse => {
            out.push_str(&format!("{pad}// uii-kind: {}\n", kind_name(kind)));
            out.push_str(&format!("{pad}// uii-name: {name}\n"));
            out.push_str(&format!("{pad}ui.set_cursor_pos([{x}, {y}]);\n"));
            out.push_str(&format!("{pad}ui.dummy([{w}, {h}]);\n"));
            if kind == NodeKind::Ellipse {
                out.push_str(&format!(
                    "{pad}ui.get_window_draw_list().add_ellipse_filled([{x}, {y}], [{w}, {h}], 0, 0.0, 24);\n"
                ));
            } else {
                let x2 = scan::num(rect.x + rect.width);
                let y2 = scan::num(rect.y + rect.height);
                out.push_str(&format!(
                    "{pad}ui.get_window_draw_list().add_rect_filled_multicolor([{x}, {y}], [{x2}, {y2}], 0, 0, 0, 0);\n"
                ));
            }
        }
        NodeKind::Dock | NodeKind::Frame | NodeKind::Child => {
            out.push_str(&format!("{pad}// uii-name: {name}\n"));
            out.push_str(&format!("{pad}// uii-kind: {}\n", kind_name(kind)));
            if kind == NodeKind::Dock {
                out.push_str(&format!("{pad}// uii-slot: {}\n", slot_name(slot)));
            }
            out.push_str(&format!("{pad}ui.set_cursor_pos([{x}, {y}]);\n"));
            out.push_str(&format!(
                "{pad}ui.child_window({}).size([{w}, {h}]).build(ui, || {{\n",
                scan::rust_string(&name)
            ));
            for child in children {
                emit_banter_widget(doc, child, out, depth + 1);
            }
            out.push_str(&format!("{pad}}});\n"));
        }
    }
}

fn import_banter(src: &str) -> Document {
    let keys = window_keys(src);
    let slots = layout_slots(src);
    let mut doc = Document::new("Imported");
    walk_banter(&mut doc, None, src, &keys, &slots);
    doc
}

fn window_keys(src: &str) -> HashMap<String, String> {
    let mut map = HashMap::new();
    let mut i = 0;
    while i < src.len() {
        if path_at(src, i, "WindowKey::new") {
            if let Some((_, args, end)) = scan::read_call(src, i) {
                let strings = scan::strings(&args);
                if strings.len() >= 2 {
                    let title = strings[1].clone();
                    map.insert(strings[0].clone(), title.clone());
                    if let Some(var) = let_var_before(src, i) {
                        map.insert(var, title);
                    }
                }
                i = end;
                continue;
            }
        }
        i += 1;
    }
    map
}

fn let_var_before(src: &str, i: usize) -> Option<String> {
    let prefix = &src[..i];
    let rel = prefix.rfind("let ")?;
    let after = prefix[rel + 4..].trim_start();
    let var: String = after.chars().take_while(|ch| ch.is_ascii_alphanumeric() || *ch == '_').collect();
    if var.is_empty() {
        None
    } else {
        Some(var)
    }
}

fn layout_slots(src: &str) -> HashMap<String, DockSlot> {
    let mut slots = HashMap::new();
    let mut i = 0;
    while i < src.len() {
        let path = if path_at(src, i, "DockLayout::split") {
            Some("DockLayout::split")
        } else if path_at(src, i, "DockLayout::tabs") {
            Some("DockLayout::tabs")
        } else {
            None
        };
        if let Some(_) = path {
            if let Some((name, args, _)) = scan::read_call(src, i) {
                walk_layout(&Arg::Call(name, args), &mut slots, DockSlot::Center);
            }
            break;
        }
        i += 1;
    }
    slots
}

fn walk_layout(arg: &Arg, slots: &mut HashMap<String, DockSlot>, inherited: DockSlot) {
    let Arg::Call(name, args) = arg else {
        return;
    };
    if name.ends_with("tabs") {
        for key in idents_in(args) {
            slots.insert(key, inherited);
        }
    } else if name.ends_with("split") {
        let dir = args.iter().find_map(|arg| match arg {
            Arg::Ident(text) => Some(text.clone()),
            _ => None,
        });
        let first_slot = match dir.as_deref() {
            Some(text) if text.ends_with("Up") => DockSlot::Top,
            Some(text) if text.ends_with("Down") => DockSlot::Bottom,
            Some(text) if text.ends_with("Left") => DockSlot::Left,
            Some(text) if text.ends_with("Right") => DockSlot::Right,
            _ => inherited,
        };
        let calls: Vec<&Arg> = args.iter().filter(|arg| matches!(arg, Arg::Call(_, _))).collect();
        if let Some(first) = calls.first() {
            walk_layout(first, slots, first_slot);
        }
        if let Some(second) = calls.get(1) {
            walk_layout(second, slots, inherited);
        }
    }
}

fn idents_in(args: &[Arg]) -> Vec<String> {
    let mut out = Vec::new();
    for arg in args {
        match arg {
            Arg::Ident(text) => out.push(text.trim_start_matches('&').to_string()),
            Arg::List(items) => out.extend(idents_in(items)),
            _ => {}
        }
    }
    out
}

fn walk_banter(doc: &mut Document, parent: Option<NodeId>, src: &str, keys: &HashMap<String, String>, slots: &HashMap<String, DockSlot>) {
    let mut pending = Pending::new();
    let mut i = 0;
    let paths = [
        "ui.begin_table_with_sizing",
        "ui.begin_table",
        "ui.button_with_size",
        "ui.button",
        "ui.child_window",
        "ui.set_cursor_pos",
        "ui.dummy",
        "ui.window",
        "ui.text",
        "ui.image",
        "add_rect_filled_multicolor",
        "add_ellipse_filled",
        "image",
    ];
    while i < src.len() {
        let byte = src.as_bytes()[i];
        if byte.is_ascii_whitespace() {
            i += 1;
            continue;
        }
        if byte == b'/' && i + 1 < src.len() && src.as_bytes()[i + 1] == b'/' {
            let end = src[i..].find('\n').map(|rel| i + rel).unwrap_or(src.len());
            apply_comment(&src[i + 2..end], &mut pending);
            i = end;
            continue;
        }
        if byte == b'/' && i + 1 < src.len() && src.as_bytes()[i + 1] == b'*' {
            i = src[i + 2..].find("*/").map(|rel| i + 2 + rel + 2).unwrap_or(src.len());
            continue;
        }
        if byte == b'"' || byte == b'\'' {
            i = skip_string(src, i);
            continue;
        }
        if let Some(path) = longest_path(src, i, &paths) {
            if let Some(end) = handle_banter(doc, parent, src, i, path, &mut pending, keys, slots) {
                i = end;
                continue;
            }
        }
        i += 1;
    }
}

fn handle_banter(
    doc: &mut Document,
    parent: Option<NodeId>,
    src: &str,
    at: usize,
    path: &str,
    pending: &mut Pending,
    keys: &HashMap<String, String>,
    slots: &HashMap<String, DockSlot>,
) -> Option<usize> {
    let (_, args, end) = scan::read_call(src, at)?;
    match path {
        "ui.set_cursor_pos" => {
            if let Some((x, y)) = first_pair(&args) {
                pending.x = x;
                pending.y = y;
            }
            Some(end)
        }
        "ui.dummy" => {
            if let Some((w, h)) = first_pair(&args) {
                pending.w = w;
                pending.h = h;
            }
            Some(end)
        }
        "image" => {
            if let Some(src) = scan::strings(&args).into_iter().next() {
                pending.image = Some(src);
            }
            Some(end)
        }
        "ui.text" => {
            let text = scan::strings(&args).into_iter().next().unwrap_or_default();
            let name = pending.take_name(&text);
            let w = if pending.w > 0.0 { pending.w } else { 8.0 };
            let h = if pending.h > 0.0 { pending.h } else { 16.0 };
            let id = add_node(doc, parent, NodeKind::Text, &name, pending.x, pending.y, w, h);
            doc.set_text(id, text);
            pending.clear_box();
            Some(end)
        }
        "ui.button_with_size" | "ui.button" => {
            let text = scan::strings(&args).into_iter().next().unwrap_or_default();
            let name = pending.take_name(&text);
            let (w, h) = first_pair(&args).unwrap_or((pending.w.max(1.0), pending.h.max(1.0)));
            let id = add_node(doc, parent, NodeKind::Button, &name, pending.x, pending.y, w, h);
            doc.set_text(id, text);
            pending.clear_box();
            Some(end)
        }
        "ui.image" => {
            let src_name = pending.image.clone().unwrap_or_default();
            let name = pending.take_name(if src_name.is_empty() { "image" } else { &src_name });
            let (w, h) = first_pair(&args).unwrap_or((pending.w.max(1.0), pending.h.max(1.0)));
            let id = add_node(doc, parent, NodeKind::Image, &name, pending.x, pending.y, w, h);
            doc.set_image(id, pending.image.clone().unwrap_or_default());
            pending.clear_box();
            Some(end)
        }
        "add_rect_filled_multicolor" | "add_ellipse_filled" => {
            let kind = if path.ends_with("ellipse_filled") {
                NodeKind::Ellipse
            } else {
                NodeKind::Rectangle
            };
            let kind = pending.kind.take().unwrap_or(kind);
            let name = pending.take_name(kind_name(kind));
            let (w, h) = if pending.w > 0.0 && pending.h > 0.0 {
                (pending.w, pending.h)
            } else {
                first_pair(&args).unwrap_or((1.0, 1.0))
            };
            add_node(doc, parent, kind, &name, pending.x, pending.y, w, h);
            pending.clear_box();
            Some(end)
        }
        "ui.child_window" => {
            let fallback = scan::strings(&args).into_iter().next().unwrap_or_else(|| "child".into());
            let name = pending.take_name(&fallback);
            let kind = pending.kind.take().unwrap_or(NodeKind::Child);
            let slot = pending.slot.take();
            let (w, h) = method_pair(src, end, "size").unwrap_or((pending.w.max(1.0), pending.h.max(1.0)));
            let (body, body_end) = method_body(src, end, "build");
            let id = add_node(doc, parent, kind, &name, pending.x, pending.y, w, h);
            if kind == NodeKind::Dock {
                doc.set_dock_slot(id, slot.unwrap_or(DockSlot::Center));
            }
            pending.clear_box();
            walk_banter(doc, Some(id), &body, keys, slots);
            Some(body_end)
        }
        "ui.window" => {
            let var = idents_in(&args).into_iter().next().unwrap_or_default();
            let title = keys.get(&var).cloned().unwrap_or_else(|| var.clone());
            let name = pending.take_name(&title);
            let (x, y) = method_pair(src, end, "position").unwrap_or((0.0, 0.0));
            let (w, h) = method_pair(src, end, "size").unwrap_or((100.0, 40.0));
            let (body, body_end) = method_body(src, end, "build");
            let layout_slot = slots.get(&var).copied();
            let kind = if layout_slot.is_some() {
                pending.kind.take();
                NodeKind::Dock
            } else {
                pending.kind.take().unwrap_or(NodeKind::Frame)
            };
            let slot = layout_slot.or(pending.slot.take());
            let id = add_node(doc, parent, kind, &name, x, y, w, h);
            if kind == NodeKind::Dock {
                doc.set_dock_slot(id, slot.unwrap_or(DockSlot::Center));
            }
            pending.clear_box();
            walk_banter(doc, Some(id), &body, keys, slots);
            Some(body_end)
        }
        "ui.begin_table_with_sizing" | "ui.begin_table" => {
            let fallback = scan::strings(&args).into_iter().next().unwrap_or_else(|| "table".into());
            let name = pending.take_name(&fallback);
            let (body, body_end) = following_brace(src, end);
            let columns = scan::calls(&body, "table_setup_column")
                .into_iter()
                .filter_map(|args| scan::strings(&args).into_iter().next())
                .collect::<Vec<_>>();
            let rows = table_rows(&body);
            let (w, h) = first_pair(&args).unwrap_or_else(|| {
                if pending.w > 0.0 && pending.h > 0.0 {
                    (pending.w, pending.h)
                } else {
                    ((columns.len().max(1) as f32) * 120.0, ((rows.len() + 1) as f32) * 28.0)
                }
            });
            let id = add_node(doc, parent, NodeKind::Table, &name, pending.x, pending.y, w, h);
            doc.set_table(id, columns, rows);
            pending.clear_box();
            Some(body_end)
        }
        _ => Some(end),
    }
}

fn table_rows(body: &str) -> Vec<Vec<String>> {
    let mut rows = Vec::new();
    for (index, part) in body.split("table_next_row").enumerate() {
        let cells = scan::calls(part, "ui.text")
            .into_iter()
            .filter_map(|args| scan::strings(&args).into_iter().next())
            .collect::<Vec<_>>();
        if !cells.is_empty() && (index > 0 || part.contains("table_next_column")) {
            rows.push(cells);
        }
    }
    rows
}

fn method_pair(src: &str, from: usize, method: &str) -> Option<(f32, f32)> {
    let at = find_method(src, from, method)?;
    let (_, args, _) = scan::read_call(src, at + 1)?;
    first_pair(&args)
}

fn method_body(src: &str, from: usize, method: &str) -> (String, usize) {
    let Some(at) = find_method(src, from, method) else {
        return (String::new(), from);
    };
    let Some((_, args, end)) = scan::read_call(src, at + 1) else {
        return (String::new(), from);
    };
    (scan::body_of(&args).unwrap_or("").to_string(), end)
}

fn find_method(src: &str, mut i: usize, method: &str) -> Option<usize> {
    let needle = format!(".{method}");
    let bytes = src.as_bytes();
    let mut depth = 0;
    while i < src.len() {
        if bytes[i] == b'"' || bytes[i] == b'\'' {
            i = skip_string(src, i);
            continue;
        }
        if bytes[i] == b'(' {
            depth += 1;
            i += 1;
            continue;
        }
        if bytes[i] == b')' {
            if depth == 0 {
                return None;
            }
            depth -= 1;
            i += 1;
            continue;
        }
        if depth == 0 && (bytes[i] == b';' || bytes[i] == b'{') {
            return None;
        }
        if depth == 0 && src[i..].starts_with(&needle) {
            return Some(i);
        }
        i += 1;
    }
    None
}

fn following_brace(src: &str, mut i: usize) -> (String, usize) {
    let bytes = src.as_bytes();
    let mut depth = 0;
    while i < src.len() {
        if bytes[i] == b'"' || bytes[i] == b'\'' {
            i = skip_string(src, i);
            continue;
        }
        if bytes[i] == b'(' {
            depth += 1;
            i += 1;
            continue;
        }
        if bytes[i] == b')' {
            if depth == 0 {
                break;
            }
            depth -= 1;
            i += 1;
            continue;
        }
        if depth == 0 && bytes[i] == b'{' {
            let (body, n) = take_brace(&src[i..]);
            return (body, i + n);
        }
        if depth == 0 && bytes[i] == b';' {
            return (String::new(), i);
        }
        i += 1;
    }
    (String::new(), i)
}

fn first_pair(args: &[Arg]) -> Option<(f32, f32)> {
    for arg in args {
        if let Arg::List(items) = arg {
            let nums = scan::nums(items);
            if nums.len() >= 2 {
                return Some((nums[0], nums[1]));
            }
        }
    }
    let nums = scan::nums(args);
    if nums.len() >= 2 {
        Some((nums[0], nums[1]))
    } else {
        None
    }
}

fn path_at(src: &str, i: usize, path: &str) -> bool {
    if i > 0 {
        let prev = src.as_bytes()[i - 1];
        if prev.is_ascii_alphanumeric() || prev == b'_' {
            return false;
        }
    }
    if !src[i..].starts_with(path) {
        return false;
    }
    let after = i + path.len();
    if after < src.len() {
        let ch = src.as_bytes()[after];
        if ch.is_ascii_alphanumeric() || ch == b'_' {
            return false;
        }
    }
    true
}

fn longest_path<'a>(src: &str, i: usize, paths: &[&'a str]) -> Option<&'a str> {
    let mut found: Option<&str> = None;
    for path in paths {
        if path_at(src, i, path) && found.map(|item| path.len() > item.len()).unwrap_or(true) {
            found = Some(*path);
        }
    }
    found
}

fn skip_string(src: &str, i: usize) -> usize {
    let bytes = src.as_bytes();
    if i >= bytes.len() {
        return i;
    }
    let quote = bytes[i];
    let mut j = i + 1;
    while j < bytes.len() {
        if bytes[j] == b'\\' {
            j += 2;
            continue;
        }
        if bytes[j] == quote {
            return j + 1;
        }
        j += 1;
    }
    src.len()
}

fn export_raven(doc: &Document) -> String {
    let mut xml = String::from("<Window Title=\"Design\">\n");
    for id in doc.roots() {
        emit_raven_xml(doc, *id, &mut xml, 1);
    }
    xml.push_str("</Window>\n");
    let imgui = export_banter(doc);
    format!(
        "// Custom imgui for this layout.\n\
// Dear ImGui is dear-imgui-rs 0.22 and dear-imgui-glow 0.22, the same crates\n\
// and docking branch Banter uses (https://github.com/Banter-Foundation/Banter).\n\
// Raven features follow https://github.com/oscar-soirey/Raven\n\
// commit 7f4411c376cec8fad2d071d63696b8444ec2640d:\n\
// DockArea with Slot.Dock, VBox, Table, Button, Label, and Image\n\
// (include/raven/raven_shell.h and raven_controls.h).\n\
// The XML is that Raven window. The Rust below draws the same layout with\n\
// Banter's Dear ImGui. libraven is not linked into the designer.\n\n\
use raven::window_from_xml;\n\n\
pub fn raven_window() -> raven::Rid {{\n    \
unsafe {{ window_from_xml(r###\"{xml}\"###) }}\n\
}}\n\n\
{imgui}"
    )
}

fn emit_raven_xml(doc: &Document, id: NodeId, out: &mut String, depth: usize) {
    let node = doc.node(id);
    let pad = "  ".repeat(depth);
    let (tag, extra) = match node.kind {
        NodeKind::Dock => ("DockArea", String::new()),
        NodeKind::Frame => ("VBox", " UiiKind=\"frame\"".to_string()),
        NodeKind::Child => ("VBox", " UiiKind=\"child\"".to_string()),
        NodeKind::Text => ("Label", format!(" Text=\"{}\"", xml_escape(&node.text))),
        NodeKind::Button => ("Button", format!(" Text=\"{}\"", xml_escape(&node.text))),
        NodeKind::Image => ("Image", format!(" Source=\"{}\"", xml_escape(&node.image_src))),
        NodeKind::Table => ("Table", String::new()),
        NodeKind::Rectangle => ("Rectangle", String::new()),
        NodeKind::Ellipse => ("Ellipse", String::new()),
    };
    let slot = match (node.kind, node.dock_slot) {
        (NodeKind::Dock, Some(slot)) if slot != DockSlot::Center => {
            let name = match slot {
                DockSlot::Top => "Top",
                DockSlot::Bottom => "Bottom",
                DockSlot::Left => "Left",
                DockSlot::Right => "Right",
                DockSlot::Center => "Tab",
            };
            format!(" Slot.Dock=\"{name}\"")
        }
        _ => String::new(),
    };
    let open = format!(
        "{pad}<{tag} Name=\"{}\"{extra}{slot} X=\"{}\" Y=\"{}\" Width=\"{}\" Height=\"{}\"",
        xml_escape(&node.name),
        scan::num(node.rect.x),
        scan::num(node.rect.y),
        scan::num(node.rect.width),
        scan::num(node.rect.height)
    );
    if node.kind == NodeKind::Table {
        out.push_str(&open);
        out.push_str(">\n");
        for column in &node.columns {
            out.push_str(&format!("{pad}  <Column Header=\"{}\"/>\n", xml_escape(column)));
        }
        for row in &node.rows {
            let cells = row.iter().map(|cell| xml_escape(cell)).collect::<Vec<_>>().join("|");
            out.push_str(&format!("{pad}  <Row>{cells}</Row>\n"));
        }
        out.push_str(&format!("{pad}</Table>\n"));
        return;
    }
    let children = doc.children(id).to_vec();
    if children.is_empty() {
        out.push_str(&open);
        out.push_str("/>\n");
        return;
    }
    out.push_str(&open);
    out.push_str(">\n");
    for child in children {
        emit_raven_xml(doc, child, out, depth + 1);
    }
    out.push_str(&format!("{pad}</{tag}>\n"));
}

fn string_list(items: &[String]) -> String {
    let inner = items.iter().map(|item| scan::rust_string(item)).collect::<Vec<_>>().join(", ");
    format!("[{inner}]")
}

fn import_raven(src: &str) -> Document {
    let mut doc = Document::new("Imported");
    walk_raven_xml(&mut doc, None, &html::parse_markup(raven_xml(src)));
    doc
}

fn raven_xml(src: &str) -> &str {
    let start = src.find("<Window").or_else(|| src.find("<window")).unwrap_or(0);
    let rest = &src[start..];
    if let Some(end) = rest.find("</Window>") {
        return &rest[..end + "</Window>".len()];
    }
    if let Some(end) = rest.find("</window>") {
        return &rest[..end + "</window>".len()];
    }
    rest
}

fn walk_raven_xml(doc: &mut Document, parent: Option<NodeId>, nodes: &[MarkupNode]) {
    for node in nodes {
        if node.tag == "window" {
            walk_raven_xml(doc, parent, &node.children);
            continue;
        }
        if node.tag == "column" || node.tag == "row" {
            continue;
        }
        let kind = match node.tag.as_str() {
            "dockarea" => NodeKind::Dock,
            "vbox" => match markup_attr(node, "uiikind").as_deref() {
                Some("frame") => NodeKind::Frame,
                _ => NodeKind::Child,
            },
            "label" => NodeKind::Text,
            "button" => NodeKind::Button,
            "image" => NodeKind::Image,
            "table" => NodeKind::Table,
            "rectangle" => NodeKind::Rectangle,
            "ellipse" => NodeKind::Ellipse,
            _ => continue,
        };
        let name = markup_attr(node, "name").unwrap_or_else(|| node.tag.clone());
        let x = attr_f32(node, "x", 0.0);
        let y = attr_f32(node, "y", 0.0);
        let w = attr_f32(node, "width", 1.0);
        let h = attr_f32(node, "height", 1.0);
        let id = add_node(doc, parent, kind, &name, x, y, w, h);
        if matches!(kind, NodeKind::Text | NodeKind::Button) {
            let text = markup_attr(node, "text").unwrap_or_else(|| node.text.clone());
            doc.set_text(id, text);
        }
        if kind == NodeKind::Image {
            doc.set_image(id, markup_attr(node, "source").unwrap_or_default());
        }
        if kind == NodeKind::Dock {
            let slot = markup_attr(node, "slot.dock")
                .as_deref()
                .map(|value| if value.eq_ignore_ascii_case("tab") { "center" } else { value })
                .and_then(DockSlot::parse)
                .unwrap_or(DockSlot::Center);
            doc.set_dock_slot(id, slot);
        }
        if kind == NodeKind::Table {
            let columns = node
                .children
                .iter()
                .filter(|child| child.tag == "column")
                .map(|child| markup_attr(child, "header").unwrap_or_default())
                .collect();
            let rows = node
                .children
                .iter()
                .filter(|child| child.tag == "row")
                .map(|child| split_cells(&child.text))
                .collect();
            doc.set_table(id, columns, rows);
        }
        walk_raven_xml(doc, Some(id), &node.children);
    }
}

fn attr_f32(node: &MarkupNode, name: &str, fallback: f32) -> f32 {
    markup_attr(node, name).and_then(|value| value.parse().ok()).unwrap_or(fallback)
}


fn list_strings(arg: &Arg) -> Vec<String> {
    match arg {
        Arg::List(items) => items.iter().filter_map(|item| match item {
            Arg::Str(text) => Some(text.clone()),
            _ => None,
        }).collect(),
        Arg::Str(text) => vec![text.clone()],
        _ => Vec::new(),
    }
}

fn list_rows(arg: &Arg) -> Vec<Vec<String>> {
    match arg {
        Arg::List(items) => items.iter().map(list_strings).filter(|row| !row.is_empty()).collect(),
        _ => Vec::new(),
    }
}

const CALLS: &[&str] = &[
    "uii_rectangle",
    "uii_ellipse",
    "uii_button",
    "uii_image",
    "uii_table",
    "uii_frame",
    "uii_child",
    "uii_dock",
    "uii_text",
];

fn wrap_calls(doc: &Document, support: &str) -> String {
    format!("{support}\n{}\n", emit_calls(doc))
}

fn emit_calls(doc: &Document) -> String {
    let mut out = String::from("pub fn uii_tree() {\n");
    for id in doc.roots() {
        emit_call(doc, *id, &mut out, 1);
    }
    out.push_str("}\n");
    out
}

fn emit_call(doc: &Document, id: NodeId, out: &mut String, depth: usize) {
    let pad = "    ".repeat(depth);
    let node = doc.node(id);
    let name = scan::rust_string(&node.name);
    let x = scan::num(node.rect.x);
    let y = scan::num(node.rect.y);
    let w = scan::num(node.rect.width);
    let h = scan::num(node.rect.height);
    let children = doc.children(id).to_vec();
    match node.kind {
        NodeKind::Dock => {
            out.push_str(&format!(
                "{pad}uii_dock({name}, {}, {x}, {y}, {w}, {h}) {{\n",
                scan::rust_string(slot_name(node.dock_slot))
            ));
            for child in children {
                emit_call(doc, child, out, depth + 1);
            }
            out.push_str(&format!("{pad}}}\n"));
        }
        NodeKind::Frame | NodeKind::Child => {
            let call = if node.kind == NodeKind::Frame { "uii_frame" } else { "uii_child" };
            out.push_str(&format!("{pad}{call}({name}, {x}, {y}, {w}, {h}) {{\n"));
            for child in children {
                emit_call(doc, child, out, depth + 1);
            }
            out.push_str(&format!("{pad}}}\n"));
        }
        NodeKind::Text | NodeKind::Button => {
            let call = if node.kind == NodeKind::Button { "uii_button" } else { "uii_text" };
            out.push_str(&format!(
                "{pad}{call}({name}, {}, {x}, {y}, {w}, {h});\n",
                scan::rust_string(&node.text)
            ));
        }
        NodeKind::Image => {
            out.push_str(&format!(
                "{pad}uii_image({name}, {}, {x}, {y}, {w}, {h});\n",
                scan::rust_string(&node.image_src)
            ));
        }
        NodeKind::Table => {
            out.push_str(&format!(
                "{pad}uii_table({name}, {x}, {y}, {w}, {h}, {}, {});\n",
                string_list(&node.columns),
                string_list_rows(&node.rows)
            ));
        }
        NodeKind::Rectangle => {
            out.push_str(&format!("{pad}uii_rectangle({name}, {x}, {y}, {w}, {h});\n"));
        }
        NodeKind::Ellipse => {
            out.push_str(&format!("{pad}uii_ellipse({name}, {x}, {y}, {w}, {h});\n"));
        }
    }
}

fn string_list_rows(rows: &[Vec<String>]) -> String {
    let inner = rows.iter().map(|row| string_list(row)).collect::<Vec<_>>().join(", ");
    format!("[{inner}]")
}

fn import_calls(src: &str) -> Document {
    let mut doc = Document::new("Imported");
    walk_calls(&mut doc, None, src);
    doc
}

fn walk_calls(doc: &mut Document, parent: Option<NodeId>, src: &str) {
    let mut i = 0;
    while i < src.len() {
        if let Some(path) = longest_path(src, i, CALLS) {
            if let Some((_, args, end)) = scan::read_call(src, i) {
                let (body, body_end) = trailing_brace(src, end);
                let strings = scan::strings(&args);
                let nums = scan::nums(&args);
                let named = |index: usize, fallback: &str| strings.get(index).cloned().unwrap_or_else(|| fallback.to_string());
                let rect = if path == "uii_dock" {
                    quad(&nums, 0)
                } else if matches!(path, "uii_text" | "uii_button" | "uii_image") {
                    quad(&nums, 0)
                } else {
                    quad(&nums, 0)
                };
                let (x, y, w, h) = rect;
                match path {
                    "uii_dock" => {
                        let id = add_node(doc, parent, NodeKind::Dock, &named(0, "dock"), x, y, w, h);
                        doc.set_dock_slot(id, DockSlot::parse(strings.get(1).map(String::as_str).unwrap_or("center")).unwrap_or(DockSlot::Center));
                        walk_calls(doc, Some(id), &body);
                    }
                    "uii_frame" | "uii_child" => {
                        let kind = if path == "uii_frame" { NodeKind::Frame } else { NodeKind::Child };
                        let id = add_node(doc, parent, kind, &named(0, kind_name(kind)), x, y, w, h);
                        walk_calls(doc, Some(id), &body);
                    }
                    "uii_text" | "uii_button" => {
                        let kind = if path == "uii_button" { NodeKind::Button } else { NodeKind::Text };
                        let id = add_node(doc, parent, kind, &named(0, kind_name(kind)), x, y, w, h);
                        doc.set_text(id, named(1, ""));
                    }
                    "uii_image" => {
                        let id = add_node(doc, parent, NodeKind::Image, &named(0, "image"), x, y, w, h);
                        doc.set_image(id, named(1, ""));
                    }
                    "uii_table" => {
                        let lists: Vec<&Arg> = args.iter().filter(|arg| matches!(arg, Arg::List(_))).collect();
                        let columns = lists.first().map(|arg| list_strings(arg)).unwrap_or_default();
                        let rows = lists.get(1).map(|arg| list_rows(arg)).unwrap_or_default();
                        let id = add_node(doc, parent, NodeKind::Table, &named(0, "table"), x, y, w, h);
                        doc.set_table(id, columns, rows);
                    }
                    "uii_rectangle" | "uii_ellipse" => {
                        let kind = if path == "uii_ellipse" { NodeKind::Ellipse } else { NodeKind::Rectangle };
                        add_node(doc, parent, kind, &named(0, kind_name(kind)), x, y, w, h);
                    }
                    _ => {}
                }
                i = if body_end > end { body_end } else { end };
                continue;
            }
        }
        i += 1;
    }
}

fn quad(nums: &[f32], at: usize) -> (f32, f32, f32, f32) {
    (
        nums.get(at).copied().unwrap_or(0.0),
        nums.get(at + 1).copied().unwrap_or(0.0),
        nums.get(at + 2).copied().unwrap_or(1.0),
        nums.get(at + 3).copied().unwrap_or(1.0),
    )
}

fn trailing_brace(src: &str, mut i: usize) -> (String, usize) {
    let bytes = src.as_bytes();
    while i < src.len() && bytes[i].is_ascii_whitespace() {
        i += 1;
    }
    if i < src.len() && bytes[i] == b'{' {
        let (body, n) = take_brace(&src[i..]);
        (body, i + n)
    } else {
        (String::new(), i)
    }
}

const CPP_SUPPORT: &str = r#"// Dear ImGui C++ 1.93 docking, the same generation as Banter's dear-imgui-rs 0.22.
static void uii_imgui_notes() {
    ImGuiID node = 0, first = 0, second = 0;
    ImGui::DockBuilderSplitNode(node, ImGuiDir_Up, 0.2f, &first, &second);
    ImGui::DockSpace(node);
    ImGui::BeginChild("card", ImVec2(1, 1), ImGuiChildFlags_Borders);
    ImGui::EndChild();
    ImGui::BeginTable("people", 2, ImGuiTableFlags_Borders);
    ImGui::EndTable();
}
"#;

const EGUI_SUPPORT: &str = r#"// egui has no built-in dockspace. Docking uses egui_dock.
fn uii_egui_notes(state: &mut egui_dock::DockState<String>) {
    state.main_surface_mut().split_above(egui_dock::NodeIndex::root(), 0.2, vec!["header".to_string()]);
}
"#;

const QT_SUPPORT: &str = r#"// Qt Widgets, not QML. QDockWidget and QTableWidget are the docking and table widgets.
static void uii_qt_notes(QMainWindow *window) {
    auto *dock = new QDockWidget("notes", window);
    window->addDockWidget(Qt::TopDockWidgetArea, dock);
    auto *table = new QTableWidget(1, 1, dock);
    auto *child = new QWidget(dock);
    (void)table;
    (void)child;
}
"#;

const SWIFT_SUPPORT: &str = r#"import SwiftUI
// SwiftUI has no dockspace. NavigationSplitView is the split. Table is the table.
struct UiiNotes: View {
    var body: some View {
        NavigationSplitView {
            Table(of: String.self) {
                TableColumn("Name") { row in Text(row) }
            }
        } detail: {
            EmptyView()
        }
    }
}
"#;

const GTK_SUPPORT: &str = r#"/* GTK 4 has no GtkDock. GtkPaned is the split. A static table is a GtkGrid. */
static void uii_gtk_notes(void) {
    GtkWidget *paned = gtk_paned_new(GTK_ORIENTATION_VERTICAL);
    GtkWidget *grid = gtk_grid_new();
    gtk_grid_attach(grid, paned, 0, 0, 1, 1);
}
"#;

const COMPOSE_SUPPORT: &str = r#"// Compose has no dockspace. Scaffold is the top/bottom split. Rows stand in for a table.
@Composable
fun UiiNotes() {
    Scaffold(topBar = {}, bottomBar = {}) { _ -> }
}
"#;

fn export_slint(doc: &Document) -> String {
    let mut body = String::new();
    for id in doc.roots() {
        emit_slint(doc, *id, &mut body, 1);
    }
    format!(
        "import {{ Button, StandardTableView }} from \"std-widgets.slint\";\n\nexport component Design inherits Window {{\n{body}}}\n"
    )
}

fn emit_slint(doc: &Document, id: NodeId, out: &mut String, depth: usize) {
    let pad = "    ".repeat(depth);
    let node = doc.node(id);
    let type_name = match node.kind {
        NodeKind::Text => "Text",
        NodeKind::Button => "Button",
        NodeKind::Image => "Image",
        NodeKind::Table => "StandardTableView",
        _ => "Rectangle",
    };
    let children = doc.children(id).to_vec();
    out.push_str(&format!("{pad}{} := {type_name} {{\n", slint_ident(&node.name)));
    out.push_str(&format!("{pad}    x: {}px;\n", scan::num(node.rect.x)));
    out.push_str(&format!("{pad}    y: {}px;\n", scan::num(node.rect.y)));
    out.push_str(&format!("{pad}    width: {}px;\n", scan::num(node.rect.width)));
    out.push_str(&format!("{pad}    height: {}px;\n", scan::num(node.rect.height)));
    out.push_str(&format!("{pad}    // uii-kind: {}\n", kind_name(node.kind)));
    out.push_str(&format!("{pad}    // uii-name: {}\n", one_line(&node.name)));
    if node.kind == NodeKind::Dock {
        out.push_str(&format!("{pad}    // uii-slot: {}\n", slot_name(node.dock_slot)));
    }
    if matches!(node.kind, NodeKind::Text | NodeKind::Button) {
        out.push_str(&format!("{pad}    text: {};\n", scan::rust_string(&node.text)));
    }
    if node.kind == NodeKind::Image {
        out.push_str(&format!("{pad}    // uii-src: {}\n", one_line(&node.image_src)));
    }
    if node.kind == NodeKind::Table {
        out.push_str(&format!("{pad}    // uii-columns: {}\n", node.columns.join("|")));
        for row in &node.rows {
            out.push_str(&format!("{pad}    // uii-row: {}\n", row.join("|")));
        }
    }
    for child in children {
        emit_slint(doc, child, out, depth + 1);
    }
    out.push_str(&format!("{pad}}}\n"));
}

fn slint_ident(name: &str) -> String {
    let mut out = String::new();
    for (index, ch) in name.chars().enumerate() {
        if ch.is_ascii_alphanumeric() || ch == '_' || ch == '-' {
            if index == 0 && ch.is_ascii_digit() {
                out.push('_');
            }
            out.push(ch);
        } else {
            out.push('_');
        }
    }
    if out.is_empty() {
        out.push_str("node");
    }
    out
}

struct SProps {
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    text: String,
    kind: Option<NodeKind>,
    slot: Option<DockSlot>,
    src: String,
    name: Option<String>,
    columns: Vec<String>,
    rows: Vec<Vec<String>>,
}

impl SProps {
    fn new() -> Self {
        Self {
            x: 0.0,
            y: 0.0,
            w: 1.0,
            h: 1.0,
            text: String::new(),
            kind: None,
            slot: None,
            src: String::new(),
            name: None,
            columns: Vec::new(),
            rows: Vec::new(),
        }
    }
}

struct SEl {
    name: String,
    type_name: String,
    props: SProps,
    children: Vec<SEl>,
}

fn import_slint(src: &str) -> Document {
    let body = slint_root(src);
    let (_props, children) = parse_slint_block(&body);
    let mut doc = Document::new("Imported");
    for child in children {
        emit_slint_el(&mut doc, None, &child);
    }
    doc
}

fn slint_root(src: &str) -> String {
    let at = src.find("inherits Window").unwrap_or(0);
    let rest = &src[at..];
    if let Some(rel) = rest.find('{') {
        take_brace(&rest[rel..]).0
    } else {
        String::new()
    }
}

fn parse_slint_block(src: &str) -> (SProps, Vec<SEl>) {
    let mut props = SProps::new();
    let mut children = Vec::new();
    let bytes = src.as_bytes();
    let mut i = 0;
    while i < src.len() {
        while i < src.len() && bytes[i].is_ascii_whitespace() {
            i += 1;
        }
        if i >= src.len() {
            break;
        }
        if bytes[i] == b'/' && i + 1 < src.len() && bytes[i + 1] == b'/' {
            let end = src[i..].find('\n').map(|rel| i + rel).unwrap_or(src.len());
            apply_slint_comment(&src[i + 2..end], &mut props);
            i = end;
            continue;
        }
        if bytes[i] == b'/' && i + 1 < src.len() && bytes[i + 1] == b'*' {
            i = src[i + 2..].find("*/").map(|rel| i + 4 + rel).unwrap_or(src.len());
            continue;
        }
        let ident_start = i;
        if !is_ident_char(bytes[i]) {
            i += 1;
            continue;
        }
        while i < src.len() && is_ident_char(bytes[i]) {
            i += 1;
        }
        let ident = src[ident_start..i].to_string();
        while i < src.len() && bytes[i].is_ascii_whitespace() {
            i += 1;
        }
        if src[i..].starts_with(":=") {
            i += 2;
            while i < src.len() && bytes[i].is_ascii_whitespace() {
                i += 1;
            }
            let type_start = i;
            while i < src.len() && is_ident_char(bytes[i]) {
                i += 1;
            }
            let type_name = src[type_start..i].to_string();
            while i < src.len() && bytes[i].is_ascii_whitespace() {
                i += 1;
            }
            if i < src.len() && bytes[i] == b'{' {
                let (body, n) = take_brace(&src[i..]);
                let (child_props, child_children) = parse_slint_block(&body);
                children.push(SEl {
                    name: ident,
                    type_name,
                    props: child_props,
                    children: child_children,
                });
                i += n;
            }
            continue;
        }
        if src[i..].starts_with(':') {
            i += 1;
            while i < src.len() && bytes[i].is_ascii_whitespace() {
                i += 1;
            }
            if i < src.len() && (bytes[i] == b'"' || bytes[i] == b'\'') {
                let (text, next) = read_quoted(src, i);
                if ident == "text" {
                    props.text = text;
                }
                i = next;
            } else {
                let start = i;
                while i < src.len() && bytes[i] != b';' && bytes[i] != b'\n' {
                    i += 1;
                }
                let token = src[start..i].trim().trim_end_matches(';').trim();
                let value = token.strip_suffix("px").unwrap_or(token).trim().parse().unwrap_or(0.0);
                match ident.as_str() {
                    "x" => props.x = value,
                    "y" => props.y = value,
                    "width" => props.w = value,
                    "height" => props.h = value,
                    _ => {}
                }
            }
            if i < src.len() && bytes[i] == b';' {
                i += 1;
            }
            continue;
        }
    }
    (props, children)
}

fn apply_slint_comment(line: &str, props: &mut SProps) {
    let line = line.trim();
    if let Some(value) = line.strip_prefix("uii-kind:") {
        props.kind = parse_kind(value.trim());
    } else if let Some(value) = line.strip_prefix("uii-name:") {
        props.name = Some(value.trim().to_string());
    } else if let Some(value) = line.strip_prefix("uii-slot:") {
        props.slot = DockSlot::parse(value.trim());
    } else if let Some(value) = line.strip_prefix("uii-src:") {
        props.src = value.trim().to_string();
    } else if let Some(value) = line.strip_prefix("uii-columns:") {
        props.columns = split_cells(value.trim());
    } else if let Some(value) = line.strip_prefix("uii-row:") {
        props.rows.push(split_cells(value.trim()));
    }
}

fn split_cells(text: &str) -> Vec<String> {
    if text.is_empty() {
        Vec::new()
    } else {
        text.split('|').map(|cell| cell.to_string()).collect()
    }
}

fn is_ident_char(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || byte == b'_' || byte == b'-'
}

fn read_quoted(src: &str, i: usize) -> (String, usize) {
    let bytes = src.as_bytes();
    let quote = bytes[i];
    let mut out = String::new();
    let mut j = i + 1;
    while j < bytes.len() {
        if bytes[j] == b'\\' && j + 1 < bytes.len() {
            out.push(bytes[j + 1] as char);
            j += 2;
            continue;
        }
        if bytes[j] == quote {
            return (out, j + 1);
        }
        out.push(bytes[j] as char);
        j += 1;
    }
    (out, src.len())
}

fn emit_slint_el(doc: &mut Document, parent: Option<NodeId>, el: &SEl) {
    let kind = el.props.kind.unwrap_or(match el.type_name.as_str() {
        "Text" => NodeKind::Text,
        "Button" => NodeKind::Button,
        "Image" => NodeKind::Image,
        "StandardTableView" => NodeKind::Table,
        _ => NodeKind::Frame,
    });
    let name = el.props.name.clone().unwrap_or_else(|| el.name.clone());
    let id = add_node(doc, parent, kind, &name, el.props.x, el.props.y, el.props.w, el.props.h);
    if matches!(kind, NodeKind::Text | NodeKind::Button) {
        doc.set_text(id, &el.props.text);
    }
    if kind == NodeKind::Image {
        doc.set_image(id, &el.props.src);
    }
    if kind == NodeKind::Table {
        doc.set_table(id, el.props.columns.clone(), el.props.rows.clone());
    }
    if kind == NodeKind::Dock {
        doc.set_dock_slot(id, el.props.slot.unwrap_or(DockSlot::Center));
    }
    for child in &el.children {
        emit_slint_el(doc, Some(id), child);
    }
}

fn export_winui(doc: &Document) -> String {
    let mut body = String::new();
    for id in doc.roots() {
        emit_winui(doc, *id, &mut body, 1);
    }
    format!("<DockPanel>\n{body}</DockPanel>\n")
}

fn emit_winui(doc: &Document, id: NodeId, out: &mut String, depth: usize) {
    let pad = "  ".repeat(depth);
    let node = doc.node(id);
    let attrs = format!(
        "x-name=\"{}\" canvas-left=\"{}\" canvas-top=\"{}\" width=\"{}\" height=\"{}\"",
        xml_escape(&node.name),
        scan::num(node.rect.x),
        scan::num(node.rect.y),
        scan::num(node.rect.width),
        scan::num(node.rect.height)
    );
    let children = doc.children(id).to_vec();
    match node.kind {
        NodeKind::Text => {
            out.push_str(&format!("{pad}<TextBlock {attrs} text=\"{}\"/>\n", xml_escape(&node.text)));
        }
        NodeKind::Button => {
            out.push_str(&format!("{pad}<Button {attrs} content=\"{}\"/>\n", xml_escape(&node.text)));
        }
        NodeKind::Image => {
            out.push_str(&format!("{pad}<Image {attrs} source=\"{}\"/>\n", xml_escape(&node.image_src)));
        }
        NodeKind::Table => {
            out.push_str(&format!(
                "{pad}<StackPanel uii-kind=\"table\" {attrs} columns=\"{}\" rows=\"{}\"/>\n",
                xml_escape(&node.columns.join("|")),
                xml_escape(&node.rows.iter().map(|row| row.join("|")).collect::<Vec<_>>().join(";"))
            ));
        }
        NodeKind::Rectangle | NodeKind::Ellipse => {
            out.push_str(&format!(
                "{pad}<Border uii-kind=\"{}\" {attrs}/>\n",
                kind_name(node.kind)
            ));
        }
        NodeKind::Dock | NodeKind::Frame | NodeKind::Child => {
            let slot = if node.kind == NodeKind::Dock {
                format!(" uii-slot=\"{}\"", slot_name(node.dock_slot))
            } else {
                String::new()
            };
            out.push_str(&format!(
                "{pad}<StackPanel uii-kind=\"{}\"{slot} {attrs}>\n",
                kind_name(node.kind)
            ));
            for child in children {
                emit_winui(doc, child, out, depth + 1);
            }
            out.push_str(&format!("{pad}</StackPanel>\n"));
        }
    }
}

fn xml_escape(text: &str) -> String {
    text.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;").replace('"', "&quot;")
}

fn import_winui(src: &str) -> Document {
    let mut doc = Document::new("Imported");
    walk_winui(&mut doc, None, &html::parse_markup(src));
    doc
}

fn walk_winui(doc: &mut Document, parent: Option<NodeId>, nodes: &[MarkupNode]) {
    for node in nodes {
        let kind_attr = markup_attr(node, "uii-kind");
        let transparent = kind_attr.is_none() && matches!(node.tag.as_str(), "dockpanel" | "canvas" | "page" | "window" | "grid");
        if transparent {
            walk_winui(doc, parent, &node.children);
            continue;
        }
        let kind = kind_attr.as_deref().and_then(parse_kind).unwrap_or(match node.tag.as_str() {
            "textblock" => NodeKind::Text,
            "button" => NodeKind::Button,
            "image" => NodeKind::Image,
            _ => NodeKind::Frame,
        });
        let name = markup_attr(node, "x-name").unwrap_or_else(|| node.tag.clone());
        let x = markup_attr(node, "canvas-left").and_then(|v| v.parse().ok()).unwrap_or(0.0);
        let y = markup_attr(node, "canvas-top").and_then(|v| v.parse().ok()).unwrap_or(0.0);
        let w = markup_attr(node, "width").and_then(|v| v.parse().ok()).unwrap_or(1.0);
        let h = markup_attr(node, "height").and_then(|v| v.parse().ok()).unwrap_or(1.0);
        let id = add_node(doc, parent, kind, &name, x, y, w, h);
        if matches!(kind, NodeKind::Text | NodeKind::Button) {
            let text = markup_attr(node, "text").or_else(|| markup_attr(node, "content")).unwrap_or(node.text.clone());
            doc.set_text(id, text);
        }
        if kind == NodeKind::Image {
            doc.set_image(id, markup_attr(node, "source").unwrap_or_default());
        }
        if kind == NodeKind::Table {
            let columns = markup_attr(node, "columns").map(|v| split_cells(&v)).unwrap_or_default();
            let rows = markup_attr(node, "rows")
                .map(|v| {
                    if v.is_empty() {
                        Vec::new()
                    } else {
                        v.split(';').map(|row| split_cells(row)).collect()
                    }
                })
                .unwrap_or_default();
            doc.set_table(id, columns, rows);
        }
        if kind == NodeKind::Dock {
            doc.set_dock_slot(id, markup_attr(node, "uii-slot").and_then(|v| DockSlot::parse(&v)).unwrap_or(DockSlot::Center));
        }
        walk_winui(doc, Some(id), &node.children);
    }
}

fn markup_attr(node: &MarkupNode, name: &str) -> Option<String> {
    node.attrs.iter().find(|(key, _)| key == name).map(|(_, value)| value.clone())
}
