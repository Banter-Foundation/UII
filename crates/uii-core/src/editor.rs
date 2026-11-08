//! Editor state for the native app.
//! Design and Animation are the only top-level tabs. Assets and Components
//! live in a collapsible drawer. Inspect is a tool, not a tab.

use crate::document::{Document, NodeId, NodeKind, Rect, Rgba};
use crate::html::import_html_css;
use crate::exchange::html_and_css;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WorkspaceTab {
    Design,
    Animation,
}

impl WorkspaceTab {
    pub fn label(self) -> &'static str {
        match self {
            WorkspaceTab::Design => "Design",
            WorkspaceTab::Animation => "Animation",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DrawerPage {
    Assets,
    Components,
}

impl DrawerPage {
    pub fn label(self) -> &'static str {
        match self {
            DrawerPage::Assets => "Assets",
            DrawerPage::Components => "Components",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tool {
    Select,
    Frame,
    Text,
    Rectangle,
    Ellipse,
    Inspect,
}

impl Tool {
    pub fn label(self) -> &'static str {
        match self {
            Tool::Select => "Select",
            Tool::Frame => "Frame",
            Tool::Text => "Text",
            Tool::Rectangle => "Rectangle",
            Tool::Ellipse => "Ellipse",
            Tool::Inspect => "Inspect",
        }
    }

    pub fn kind(self) -> Option<NodeKind> {
        match self {
            Tool::Frame => Some(NodeKind::Frame),
            Tool::Text => Some(NodeKind::Text),
            Tool::Rectangle => Some(NodeKind::Rectangle),
            Tool::Ellipse => Some(NodeKind::Ellipse),
            Tool::Select | Tool::Inspect => None,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Motion {
    X,
    Y,
    Width,
    Height,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Inspection {
    pub name: String,
    pub kind: String,
    pub width: f32,
    pub height: f32,
    pub color: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Pose {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

#[derive(Clone, Debug)]
pub struct Component {
    pub name: String,
    pub html: String,
    pub css: String,
}

#[derive(Clone, Debug)]
struct Keyframe {
    node: NodeId,
    prop: Motion,
    time: f32,
    value: f32,
}

pub struct Editor {
    tab: WorkspaceTab,
    tool: Tool,
    drawer_open: bool,
    drawer_page: DrawerPage,
    document: Document,
    html: String,
    css: String,
    selected: Option<NodeId>,
    assets: Vec<String>,
    components: Vec<Component>,
    keys: Vec<Keyframe>,
    time: f32,
    playing: bool,
}

impl Editor {
    pub fn new() -> Self {
        Self {
            tab: WorkspaceTab::Design,
            tool: Tool::Select,
            drawer_open: false,
            drawer_page: DrawerPage::Assets,
            document: Document::new("Untitled"),
            html: String::new(),
            css: String::new(),
            selected: None,
            assets: Vec::new(),
            components: Vec::new(),
            keys: Vec::new(),
            time: 0.0,
            playing: false,
        }
    }

    pub fn tabs() -> [WorkspaceTab; 2] {
        [WorkspaceTab::Design, WorkspaceTab::Animation]
    }

    pub fn drawer_pages() -> [DrawerPage; 2] {
        [DrawerPage::Assets, DrawerPage::Components]
    }

    pub fn tools() -> [Tool; 6] {
        [
            Tool::Select,
            Tool::Frame,
            Tool::Text,
            Tool::Rectangle,
            Tool::Ellipse,
            Tool::Inspect,
        ]
    }

    pub fn tab(&self) -> WorkspaceTab {
        self.tab
    }

    pub fn set_tab(&mut self, tab: WorkspaceTab) {
        self.tab = tab;
    }

    pub fn tool(&self) -> Tool {
        self.tool
    }

    pub fn set_tool(&mut self, tool: Tool) {
        self.tool = tool;
    }

    pub fn drawer_collapsed(&self) -> bool {
        !self.drawer_open
    }

    pub fn open_drawer(&mut self) {
        self.drawer_open = true;
    }

    pub fn collapse_drawer(&mut self) {
        self.drawer_open = false;
    }

    pub fn toggle_drawer(&mut self) {
        self.drawer_open = !self.drawer_open;
    }

    pub fn drawer_page(&self) -> DrawerPage {
        self.drawer_page
    }

    pub fn show_drawer(&mut self, page: DrawerPage) {
        self.drawer_page = page;
        self.drawer_open = true;
    }

    pub fn document(&self) -> &Document {
        &self.document
    }

    pub fn document_mut(&mut self) -> &mut Document {
        &mut self.document
    }

    pub fn markup(&self) -> (&str, &str) {
        (&self.html, &self.css)
    }

    pub fn selected(&self) -> Option<NodeId> {
        self.selected
    }

    pub fn select(&mut self, id: NodeId) {
        if self.document.get(id).is_some() {
            self.selected = Some(id);
        }
    }

    pub fn clear_selection(&mut self) {
        self.selected = None;
    }

    pub fn add_shape(&mut self, kind: NodeKind, name: &str, rect: Rect) -> NodeId {
        let id = self.document.create(None, kind, name, rect);
        if kind == NodeKind::Text || kind == NodeKind::Button {
            self.document.set_text(id, name);
        }
        self.selected = Some(id);
        self.sync_markup();
        id
    }

    pub fn move_node(&mut self, id: NodeId, x: f32, y: f32) {
        self.document.move_node(id, x, y);
        self.sync_markup();
    }

    pub fn resize_node(&mut self, id: NodeId, width: f32, height: f32) {
        self.document.resize_node(id, width, height);
        self.sync_markup();
    }

    pub fn set_fill(&mut self, id: NodeId, color: Rgba) {
        self.document.node_mut(id).fill = color;
        self.sync_markup();
    }

    pub fn set_text(&mut self, id: NodeId, text: &str) {
        self.document.set_text(id, text);
        self.sync_markup();
    }

    pub fn delete_selected(&mut self) {
        if let Some(id) = self.selected.take() {
            self.keys.retain(|key| key.node != id);
            self.document.delete(id);
            self.sync_markup();
        }
    }

    pub fn inspection(&self) -> Option<Inspection> {
        if self.tool != Tool::Inspect {
            return None;
        }
        let id = self.selected?;
        let node = self.document.get(id)?;
        Some(Inspection {
            name: node.name.clone(),
            kind: node.kind.to_string(),
            width: node.rect.width,
            height: node.rect.height,
            color: visible_color(node),
        })
    }

    pub fn assets(&self) -> Vec<String> {
        let mut out = self.assets.clone();
        for node in self.document.nodes() {
            if node.kind == NodeKind::Image && !node.image_src.is_empty() && !out.iter().any(|path| path == &node.image_src) {
                out.push(node.image_src.clone());
            }
        }
        out
    }

    pub fn add_asset(&mut self, path: &str) {
        if path.is_empty() {
            return;
        }
        if !self.assets.iter().any(|item| item == path) {
            self.assets.push(path.to_string());
        }
    }

    pub fn place_asset(&mut self, path: &str) -> NodeId {
        self.add_asset(path);
        let name = path.rsplit(['/', '\\']).next().unwrap_or(path);
        let id = self.document.create(
            None,
            NodeKind::Image,
            name,
            Rect::new(16.0, 16.0, 64.0, 64.0),
        );
        self.document.set_image(id, path);
        self.selected = Some(id);
        self.sync_markup();
        id
    }

    pub fn components(&self) -> &[Component] {
        &self.components
    }

    pub fn save_component(&mut self, name: &str) {
        let Some(id) = self.selected else {
            return;
        };
        if self.document.get(id).is_none() {
            return;
        }
        let mut piece = Document::new(name);
        copy_subtree(&self.document, id, &mut piece, None);
        let (html, css) = html_and_css(&piece);
        if let Some(existing) = self.components.iter_mut().find(|component| component.name == name) {
            existing.html = html;
            existing.css = css;
        } else {
            self.components.push(Component { name: name.to_string(), html, css });
        }
    }

    pub fn insert_component(&mut self, name: &str) {
        let Some(component) = self.components.iter().find(|component| component.name == name) else {
            return;
        };
        let imported = import_html_css(&component.html, &component.css);
        let roots = imported.roots().to_vec();
        let mut last = None;
        for root in roots {
            last = Some(copy_subtree(&imported, root, &mut self.document, None));
        }
        if let Some(id) = last {
            self.selected = Some(id);
        }
        self.sync_markup();
    }

    pub fn add_key(&mut self, node: NodeId, prop: Motion, time: f32, value: f32) {
        if let Some(existing) = self.keys.iter_mut().find(|key| key.node == node && key.prop == prop && (key.time - time).abs() < 0.0001) {
            existing.value = value;
            return;
        }
        self.keys.push(Keyframe { node, prop, time, value });
    }

    pub fn set_time(&mut self, time: f32) {
        self.time = time.max(0.0);
    }

    pub fn time(&self) -> f32 {
        self.time
    }

    pub fn set_playing(&mut self, playing: bool) {
        self.playing = playing;
    }

    pub fn playing(&self) -> bool {
        self.playing
    }

    pub fn duration(&self) -> f32 {
        self.keys.iter().fold(1.0_f32, |end, key| end.max(key.time)).max(0.001)
    }

    pub fn tick(&mut self, dt: f32) {
        if !self.playing || dt <= 0.0 {
            return;
        }
        self.time += dt;
        let end = self.duration();
        if self.time > end {
            self.time %= end;
        }
    }

    pub fn pose(&self, id: NodeId) -> Pose {
        let rect = self.document.node(id).rect;
        Pose {
            x: sample(&self.keys, id, Motion::X, self.time, rect.x),
            y: sample(&self.keys, id, Motion::Y, self.time, rect.y),
            width: sample(&self.keys, id, Motion::Width, self.time, rect.width).max(1.0),
            height: sample(&self.keys, id, Motion::Height, self.time, rect.height).max(1.0),
        }
    }

    pub fn keys_for(&self, id: NodeId) -> Vec<(Motion, f32, f32)> {
        let mut out: Vec<_> = self
            .keys
            .iter()
            .filter(|key| key.node == id)
            .map(|key| (key.prop, key.time, key.value))
            .collect();
        out.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));
        out
    }

    pub fn replace_document(&mut self, document: Document) {
        self.document = document;
        self.selected = None;
        self.keys.clear();
        self.sync_markup();
    }

    pub fn apply_markup(&mut self, html: &str, css: &str) {
        self.document = import_html_css(html, css);
        self.selected = None;
        self.keys.clear();
        self.sync_markup();
    }

    pub fn cursor(&mut self, request: &str) -> String {
        let op = json_string(request, "op").unwrap_or_default();
        match op.as_str() {
            "get" => design_json(&self.document, &self.html, &self.css),
            "apply" => {
                let html = json_string(request, "html").unwrap_or_default();
                let css = json_string(request, "css").unwrap_or_default();
                self.apply_markup(&html, &css);
                design_json(&self.document, &self.html, &self.css)
            }
            _ => "{\"ok\":false,\"error\":\"unknown op\"}".to_string(),
        }
    }

    fn sync_markup(&mut self) {
        let (html, css) = html_and_css(&self.document);
        self.html = html;
        self.css = css;
    }
}

impl Default for Editor {
    fn default() -> Self {
        Self::new()
    }
}

fn visible_color(node: &crate::document::Node) -> String {
    let color = if node.fill.a > 0 { node.fill } else { node.color };
    format!("#{:02x}{:02x}{:02x}", color.r, color.g, color.b)
}

fn sample(keys: &[Keyframe], node: NodeId, prop: Motion, time: f32, base: f32) -> f32 {
    let mut mine: Vec<(f32, f32)> = keys
        .iter()
        .filter(|key| key.node == node && key.prop == prop)
        .map(|key| (key.time, key.value))
        .collect();
    if mine.is_empty() {
        return base;
    }
    mine.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));
    if time <= mine[0].0 {
        return mine[0].1;
    }
    if time >= mine[mine.len() - 1].0 {
        return mine[mine.len() - 1].1;
    }
    for pair in mine.windows(2) {
        if time >= pair[0].0 && time <= pair[1].0 {
            let span = pair[1].0 - pair[0].0;
            if span <= f32::EPSILON {
                return pair[1].1;
            }
            let t = (time - pair[0].0) / span;
            return pair[0].1 + (pair[1].1 - pair[0].1) * t;
        }
    }
    base
}

fn copy_subtree(src: &Document, id: NodeId, dst: &mut Document, parent: Option<NodeId>) -> NodeId {
    let node = src.node(id);
    let new_id = dst.create(parent, node.kind, node.name.clone(), node.rect);
    {
        let copied = dst.node_mut(new_id);
        copied.fill = node.fill;
        copied.color = node.color;
        copied.text = node.text.clone();
        copied.font_size = node.font_size;
        copied.image_src = node.image_src.clone();
        copied.dock_slot = node.dock_slot;
        copied.columns = node.columns.clone();
        copied.rows = node.rows.clone();
    }
    let children = src.children(id).to_vec();
    for child in children {
        copy_subtree(src, child, dst, Some(new_id));
    }
    new_id
}

fn purpose(node: &crate::document::Node) -> String {
    match node.kind {
        NodeKind::Dock => match node.dock_slot {
            Some(slot) => format!("{} dock for {}", slot_label(slot), node.name),
            None => format!("Dock for {}", node.name),
        },
        NodeKind::Child => format!("Child window {}", node.name),
        NodeKind::Text => format!("Text {}", node.name),
        NodeKind::Button => format!("Button {}", node.name),
        NodeKind::Image => format!("Image {}", node.name),
        NodeKind::Table => format!("Table {}", node.name),
        NodeKind::Rectangle => format!("Rectangle {}", node.name),
        NodeKind::Ellipse => format!("Ellipse {}", node.name),
        NodeKind::Frame => format!("Frame {}", node.name),
    }
}

fn slot_label(slot: crate::document::DockSlot) -> &'static str {
    match slot {
        crate::document::DockSlot::Top => "Top",
        crate::document::DockSlot::Bottom => "Bottom",
        crate::document::DockSlot::Left => "Left",
        crate::document::DockSlot::Right => "Right",
        crate::document::DockSlot::Center => "Center",
    }
}

fn design_json(doc: &Document, html: &str, css: &str) -> String {
    let mut regions = String::from("[");
    let mut first = true;
    for node in doc.nodes() {
        if !first {
            regions.push(',');
        }
        first = false;
        let parent = node.parent.map(|id| id.0.to_string()).unwrap_or_else(|| "null".into());
        let slot = node
            .dock_slot
            .map(|slot| format!("\"{}\"", slot.as_str()))
            .unwrap_or_else(|| "null".into());
        regions.push_str(&format!(
            "{{\"id\":{id},\"parent\":{parent},\"name\":{name},\"kind\":\"{kind}\",\"purpose\":{purpose},\"dock_slot\":{slot},\"text\":{text},\"image\":{image},\"x\":{x},\"y\":{y},\"width\":{width},\"height\":{height},\"color\":\"{color}\"}}",
            id = node.id.0,
            name = json_encode(&node.name),
            kind = node.kind,
            purpose = json_encode(&purpose(node)),
            text = json_encode(&node.text),
            image = json_encode(&node.image_src),
            x = json_num(node.rect.x),
            y = json_num(node.rect.y),
            width = json_num(node.rect.width),
            height = json_num(node.rect.height),
            color = visible_color(node),
        ));
    }
    regions.push(']');
    format!(
        "{{\"ok\":true,\"html\":{},\"css\":{},\"regions\":{regions}}}",
        json_encode(html),
        json_encode(css)
    )
}

fn json_encode(text: &str) -> String {
    let mut out = String::from("\"");
    for ch in text.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            other => out.push(other),
        }
    }
    out.push('"');
    out
}

fn json_num(value: f32) -> String {
    if value.is_finite() {
        let rounded = (value * 1000.0).round() / 1000.0;
        if (rounded - rounded.round()).abs() < 0.0001 {
            format!("{}", rounded.round() as i32)
        } else {
            format!("{rounded}")
        }
    } else {
        "0".into()
    }
}

fn json_string(src: &str, key: &str) -> Option<String> {
    let needle = format!("\"{key}\"");
    let at = src.find(&needle)?;
    let rest = src[at + needle.len()..].trim_start();
    let rest = rest.strip_prefix(':')?.trim_start();
    let rest = rest.strip_prefix('"')?;
    let mut out = String::new();
    let mut chars = rest.chars();
    while let Some(ch) = chars.next() {
        if ch == '\\' {
            match chars.next() {
                Some('n') => out.push('\n'),
                Some('r') => out.push('\r'),
                Some('t') => out.push('\t'),
                Some('u') => {
                    let hex: String = chars.by_ref().take(4).collect();
                    if let Ok(code) = u32::from_str_radix(&hex, 16) {
                        if let Some(decoded) = char::from_u32(code) {
                            out.push(decoded);
                        }
                    }
                }
                Some(other) => out.push(other),
                None => break,
            }
        } else if ch == '"' {
            return Some(out);
        } else {
            out.push(ch);
        }
    }
    None
}
