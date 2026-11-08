//! Editor state for the native app.
//! Design and Animation are the only top-level tabs. Assets and Components
//! live in a collapsible drawer. Inspect is a tool, not a tab.

use std::collections::BTreeSet;

use crate::commands::Command;
use crate::document::{snap_rect, Document, NodeId, NodeKind, Rect, Rgba};
use crate::easing::Easing;
use crate::exchange::html_and_css;
use crate::folders::{folder_rows, FolderRow};
use crate::html::import_html_css;
use crate::library::{AssetId, AssetKind, Library, LibraryError};
use crate::{format_color, SNAP_PX};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WorkspaceTab {
    Design,
    Animation,
    Function,
}

impl WorkspaceTab {
    pub fn label(self) -> &'static str {
        match self {
            WorkspaceTab::Design => "Design",
            WorkspaceTab::Animation => "Animation",
            WorkspaceTab::Function => "Function",
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
    ease: Easing,
}

struct Snapshot {
    document: Document,
    selection: Vec<NodeId>,
    keys: Vec<Keyframe>,
    time: f32,
    playing: bool,
    zoom: f32,
    pan: [f32; 2],
}

#[derive(Clone, Debug, PartialEq)]
pub struct Properties {
    pub name: String,
    pub kind: String,
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
    pub fill: String,
    pub font_size: Option<f32>,
    pub corner_radius: Option<f32>,
}

pub struct Editor {
    tab: WorkspaceTab,
    tool: Tool,
    drawer_open: bool,
    drawer_page: DrawerPage,
    document: Document,
    html: String,
    css: String,
    selection: Vec<NodeId>,
    undo: Vec<Snapshot>,
    redo: Vec<Snapshot>,
    editing: bool,
    clipboard: Option<Document>,
    zoom: f32,
    pan: [f32; 2],
    ease: Easing,
    drawer_pinned: bool,
    chord_down: bool,
    revision: u64,
    assets: Vec<String>,
    components: Vec<Component>,
    library: Library,
    library_query: String,
    library_selected: Option<AssetId>,
    library_expanded: BTreeSet<AssetId>,
    library_dirty: bool,
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
            selection: Vec::new(),
            undo: Vec::new(),
            redo: Vec::new(),
            editing: false,
            clipboard: None,
            zoom: 1.0,
            pan: [0.0, 0.0],
            ease: Easing::Linear,
            drawer_pinned: false,
            chord_down: false,
            revision: 0,
            assets: Vec::new(),
            components: Vec::new(),
            library: Library::new(),
            library_query: String::new(),
            library_selected: None,
            library_expanded: BTreeSet::new(),
            library_dirty: false,
            keys: Vec::new(),
            time: 0.0,
            playing: false,
        }
    }

    pub fn tabs() -> [WorkspaceTab; 3] {
        [
            WorkspaceTab::Design,
            WorkspaceTab::Animation,
            WorkspaceTab::Function,
        ]
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
        self.selection
            .iter()
            .rev()
            .copied()
            .find(|id| self.document.get(*id).is_some())
    }

    pub fn selection(&self) -> &[NodeId] {
        &self.selection
    }

    pub fn select(&mut self, id: NodeId) {
        if self.document.get(id).is_some() {
            self.selection.clear();
            self.selection.push(id);
        }
    }

    pub fn add_to_selection(&mut self, id: NodeId) {
        if self.document.get(id).is_some() && !self.selection.contains(&id) {
            self.selection.push(id);
        }
    }

    pub fn toggle_in_selection(&mut self, id: NodeId) {
        if self.selection.contains(&id) {
            self.selection.retain(|item| *item != id);
        } else {
            self.add_to_selection(id);
        }
    }

    pub fn select_ids(&mut self, ids: &[NodeId]) {
        self.selection.clear();
        for id in ids {
            self.add_to_selection(*id);
        }
    }

    pub fn select_all(&mut self) {
        self.selection = self.document.nodes().map(|node| node.id).collect();
    }

    pub fn clear_selection(&mut self) {
        self.selection.clear();
    }

    pub fn add_shape(&mut self, kind: NodeKind, name: &str, rect: Rect) -> NodeId {
        self.finish_edit();
        self.touch();
        let parent = self.frame_parent_for_new();
        let local = self.to_parent_local(parent, rect);
        let id = self.document.create(parent, kind, name, local);
        if kind == NodeKind::Text || kind == NodeKind::Button {
            self.document.set_text(id, name);
        }
        self.selection.clear();
        self.selection.push(id);
        self.finish_edit();
        self.sync_markup();
        id
    }

    pub fn move_node(&mut self, id: NodeId, x: f32, y: f32) {
        self.touch();
        self.document.move_node(id, x, y);
        self.sync_markup();
    }

    pub fn resize_node(&mut self, id: NodeId, width: f32, height: f32) {
        self.touch();
        self.document.resize_node(id, width, height);
        self.sync_markup();
    }

    pub fn set_fill(&mut self, id: NodeId, color: Rgba) {
        self.touch();
        self.document.node_mut(id).fill = color;
        self.sync_markup();
    }

    pub fn set_font_size(&mut self, id: NodeId, size: f32) {
        self.touch();
        self.document.node_mut(id).font_size = size.max(1.0);
        self.sync_markup();
    }

    pub fn set_corner_radius(&mut self, id: NodeId, radius: f32) {
        self.touch();
        self.document.node_mut(id).corner_radius = radius.max(0.0);
        self.sync_markup();
    }

    pub fn set_text(&mut self, id: NodeId, text: &str) {
        self.touch();
        self.document.set_text(id, text);
        self.sync_markup();
    }

    pub fn delete_selected(&mut self) {
        let doomed = self.top_selected();
        if doomed.is_empty() {
            return;
        }
        self.finish_edit();
        self.touch();
        for id in doomed {
            self.document.delete(id);
        }
        self.keys
            .retain(|key| self.document.get(key.node).is_some());
        self.selection.clear();
        self.finish_edit();
        self.sync_markup();
    }

    pub fn inspection(&self) -> Option<Inspection> {
        if self.tool != Tool::Inspect {
            return None;
        }
        let id = self.selected()?;
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
            if node.kind == NodeKind::Image
                && !node.image_src.is_empty()
                && !out.iter().any(|path| path == &node.image_src)
            {
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
        self.note_library_asset(path);
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
        self.selection.clear();
        self.selection.push(id);
        self.sync_markup();
        id
    }

    pub fn components(&self) -> &[Component] {
        &self.components
    }

    pub fn save_component(&mut self, name: &str) {
        let Some(id) = self.selected() else {
            return;
        };
        if self.document.get(id).is_none() {
            return;
        }
        let mut piece = Document::new(name);
        copy_subtree(&self.document, id, &mut piece, None);
        let (html, css) = html_and_css(&piece);
        if let Some(existing) = self
            .components
            .iter_mut()
            .find(|component| component.name == name)
        {
            existing.html = html;
            existing.css = css;
        } else {
            self.components.push(Component {
                name: name.to_string(),
                html,
                css,
            });
        }
        self.note_library_component(name);
    }

    pub fn insert_component(&mut self, name: &str) {
        let Some(component) = self
            .components
            .iter()
            .find(|component| component.name == name)
        else {
            return;
        };
        let imported = import_html_css(&component.html, &component.css);
        let roots = imported.roots().to_vec();
        let mut last = None;
        for root in roots {
            last = Some(copy_subtree(&imported, root, &mut self.document, None));
        }
        if let Some(id) = last {
            self.selection.clear();
            self.selection.push(id);
        }
        self.sync_markup();
    }

    pub fn add_key(&mut self, node: NodeId, prop: Motion, time: f32, value: f32) {
        if let Some(existing) = self
            .keys
            .iter_mut()
            .find(|key| key.node == node && key.prop == prop && (key.time - time).abs() < 0.0001)
        {
            existing.value = value;
            return;
        }
        self.keys.push(Keyframe {
            node,
            prop,
            time,
            value,
            ease: self.ease,
        });
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
        self.keys
            .iter()
            .fold(1.0_f32, |end, key| end.max(key.time))
            .max(0.001)
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

    pub fn set_ease(&mut self, ease: Easing) {
        self.ease = ease;
    }

    pub fn ease(&self) -> Easing {
        self.ease
    }

    pub fn set_key_easing(&mut self, node: NodeId, prop: Motion, time: f32, ease: Easing) {
        if let Some(existing) = self
            .keys
            .iter_mut()
            .find(|key| key.node == node && key.prop == prop && (key.time - time).abs() < 0.0001)
        {
            existing.ease = ease;
        }
    }

    pub fn replace_document(&mut self, document: Document) {
        self.finish_edit();
        self.touch();
        self.document = document;
        self.selection.clear();
        self.keys.clear();
        self.finish_edit();
        self.sync_markup();
    }

    pub fn apply_markup(&mut self, html: &str, css: &str) {
        self.finish_edit();
        self.touch();
        self.document = import_html_css(html, css);
        self.selection.clear();
        self.keys.clear();
        self.finish_edit();
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

    pub fn revision(&self) -> u64 {
        self.revision
    }

    pub fn zoom(&self) -> f32 {
        self.zoom
    }

    pub fn pan(&self) -> [f32; 2] {
        self.pan
    }

    pub fn set_pan(&mut self, pan: [f32; 2]) {
        self.pan = pan;
    }

    pub fn zoom_in(&mut self) {
        self.zoom = (self.zoom * 1.1).clamp(0.1, 8.0);
    }

    pub fn zoom_out(&mut self) {
        self.zoom = (self.zoom / 1.1).clamp(0.1, 8.0);
    }

    pub fn fit(&mut self, view_w: f32, view_h: f32) {
        let mut bounds: Option<Rect> = None;
        for node in self.document.nodes() {
            let abs = self.document.absolute_rect(node.id);
            bounds = Some(match bounds {
                None => abs,
                Some(acc) => union_rect(acc, abs),
            });
        }
        let Some(bounds) = bounds else {
            self.zoom = 1.0;
            self.pan = [0.0, 0.0];
            return;
        };
        let pad = 32.0;
        let usable_w = (view_w - pad * 2.0).max(1.0);
        let usable_h = (view_h - pad * 2.0).max(1.0);
        self.zoom = (usable_w / bounds.width.max(1.0))
            .min(usable_h / bounds.height.max(1.0))
            .clamp(0.1, 8.0);
        self.pan = [bounds.x - pad / self.zoom, bounds.y - pad / self.zoom];
    }

    pub fn drawer_pinned(&self) -> bool {
        self.drawer_pinned
    }

    pub fn set_drawer_pinned(&mut self, pinned: bool) {
        self.drawer_pinned = pinned;
    }

    pub fn poll_drawer_chord(&mut self, down: bool, text_input: bool) {
        let edge = down && !self.chord_down;
        self.chord_down = down;
        if edge && !text_input {
            self.toggle_drawer();
        }
    }

    pub fn dismiss_unpinned_drawer(&mut self) {
        if self.drawer_open && !self.drawer_pinned {
            self.drawer_open = false;
        }
    }

    pub fn library(&self) -> &Library {
        &self.library
    }

    pub fn library_changed(&self) -> bool {
        self.library_dirty
    }

    pub fn clear_library_changed(&mut self) {
        self.library_dirty = false;
    }

    pub fn mark_library_changed(&mut self) {
        self.library_dirty = true;
    }

    pub fn replace_library(&mut self, library: Library) {
        self.library = library;
        self.library_selected = None;
        self.library_expanded.clear();
        self.library_dirty = false;
    }

    pub fn set_library_query(&mut self, query: &str) {
        self.library_query = query.to_string();
    }

    pub fn library_rows(&self) -> Vec<FolderRow> {
        folder_rows(&self.library, &self.library_query, &self.library_expanded)
    }

    pub fn library_component_names(&self) -> Vec<String> {
        let mut ids: Vec<_> = self
            .library
            .components()
            .iter()
            .map(|component| component.id)
            .collect();
        ids.sort_by(|left, right| self.library.cmp_components(*left, *right));
        ids.into_iter()
            .filter_map(|id| {
                self.library
                    .component(id)
                    .map(|component| component.name.clone())
            })
            .collect()
    }

    pub fn activate_library_row(&mut self, id: AssetId) {
        self.library_selected = Some(id);
        if self.library.is_folder(id) && !self.library_expanded.insert(id) {
            self.library_expanded.remove(&id);
        }
    }

    pub fn create_named_folder(&mut self, name: &str) -> Result<AssetId, LibraryError> {
        let parent = self.library_parent();
        let id = self.library.create_folder(parent, name)?;
        if let Some(parent) = parent {
            self.library_expanded.insert(parent);
        }
        self.library_expanded.insert(id);
        self.library_selected = Some(id);
        self.library_dirty = true;
        Ok(id)
    }

    pub fn create_named_asset(&mut self, name: &str) -> Result<AssetId, LibraryError> {
        let parent = self.library_parent();
        let id = self.library.create_asset(parent, name)?;
        if let Some(parent) = parent {
            self.library_expanded.insert(parent);
        }
        self.library_selected = Some(id);
        self.library_dirty = true;
        Ok(id)
    }

    pub fn delete_library_selection(&mut self) -> Result<(), LibraryError> {
        let id = self.library_selected.ok_or(LibraryError::Missing)?;
        self.library.delete_asset(id)?;
        self.library_selected = None;
        self.library_expanded
            .retain(|folder| self.library.is_folder(*folder));
        self.library_dirty = true;
        Ok(())
    }

    pub fn selected_library_asset(&self) -> Option<&str> {
        let id = self.library_selected?;
        let asset = self.library.asset(id)?;
        if asset.kind == AssetKind::Asset {
            Some(asset.name.as_str())
        } else {
            None
        }
    }

    pub fn properties(&self) -> Option<Properties> {
        self.properties_for(self.selected()?)
    }

    pub fn properties_for(&self, id: NodeId) -> Option<Properties> {
        let node = self.document.get(id)?;
        let font_size =
            matches!(node.kind, NodeKind::Text | NodeKind::Button).then_some(node.font_size);
        let corner_radius = matches!(
            node.kind,
            NodeKind::Rectangle | NodeKind::Frame | NodeKind::Child | NodeKind::Dock
        )
        .then_some(node.corner_radius);
        Some(Properties {
            name: node.name.clone(),
            kind: node.kind.to_string(),
            x: node.rect.x,
            y: node.rect.y,
            width: node.rect.width,
            height: node.rect.height,
            fill: format_color(node.fill),
            font_size,
            corner_radius,
        })
    }

    pub fn finish_edit(&mut self) {
        self.editing = false;
    }

    pub fn undo(&mut self) -> bool {
        self.finish_edit();
        let Some(prev) = self.undo.pop() else {
            return false;
        };
        self.redo.push(self.snapshot());
        self.restore(prev);
        true
    }

    pub fn redo(&mut self) -> bool {
        self.finish_edit();
        let Some(next) = self.redo.pop() else {
            return false;
        };
        self.undo.push(self.snapshot());
        self.restore(next);
        true
    }

    pub fn move_selection_by(&mut self, dx: f32, dy: f32) {
        if dx == 0.0 && dy == 0.0 {
            return;
        }
        self.touch();
        for id in self.top_selected() {
            let rect = self.document.node(id).rect;
            self.document.move_node(id, rect.x + dx, rect.y + dy);
        }
        self.sync_markup();
    }

    pub fn place_moved(
        &mut self,
        primary: NodeId,
        base: &[(NodeId, Rect)],
        dx: f32,
        dy: f32,
    ) -> Vec<crate::Guide> {
        let Some((_, primary_base)) = base.iter().find(|(id, _)| *id == primary) else {
            return Vec::new();
        };
        let parent = self.document.node(primary).parent;
        let origin = parent
            .map(|id| self.document.absolute_rect(id))
            .map(|rect| (rect.x, rect.y))
            .unwrap_or((0.0, 0.0));
        let proposed = Rect::new(
            origin.0 + primary_base.x + dx,
            origin.1 + primary_base.y + dy,
            primary_base.width,
            primary_base.height,
        );
        let ignore = self.top_selected();
        let (snapped, guides) = snap_rect(&self.document, &ignore, proposed, SNAP_PX);
        let applied_dx = snapped.x - origin.0 - primary_base.x;
        let applied_dy = snapped.y - origin.1 - primary_base.y;
        self.touch();
        for (id, rect) in base {
            if !self.top_selected().contains(id) {
                continue;
            }
            self.document
                .move_node(*id, rect.x + applied_dx, rect.y + applied_dy);
        }
        self.sync_markup();
        guides
    }

    pub fn drop_selection(&mut self) {
        let ids = self.top_selected();
        for id in ids {
            let abs = self.document.absolute_rect(id);
            let target = deepest_frame_at(
                &self.document,
                abs.x + abs.width * 0.5,
                abs.y + abs.height * 0.5,
                id,
            );
            if target != self.document.node(id).parent {
                self.document.reparent(id, target);
            }
        }
        self.sync_markup();
    }

    pub fn group(&mut self) -> Option<NodeId> {
        let ids = self.top_selected();
        if ids.is_empty() {
            return None;
        }
        self.finish_edit();
        self.touch();
        let parent = shared_parent(&self.document, &ids);
        let mut bounds: Option<Rect> = None;
        for id in &ids {
            let abs = self.document.absolute_rect(*id);
            let local = self.to_parent_local(parent, abs);
            bounds = Some(match bounds {
                None => local,
                Some(acc) => union_rect(acc, local),
            });
        }
        let bounds = bounds?;
        let frame = self
            .document
            .create(parent, NodeKind::Frame, "Group", bounds);
        for id in &ids {
            self.document.reparent(*id, Some(frame));
        }
        self.selection.clear();
        self.selection.push(frame);
        self.finish_edit();
        self.sync_markup();
        Some(frame)
    }

    pub fn copy(&mut self) {
        let ids = self.top_selected();
        if ids.is_empty() {
            return;
        }
        let mut clip = Document::new("Clipboard");
        for id in ids {
            let copied = copy_subtree(&self.document, id, &mut clip, None);
            let abs = self.document.absolute_rect(id);
            clip.move_node(copied, abs.x, abs.y);
        }
        self.clipboard = Some(clip);
    }

    pub fn paste(&mut self) -> Vec<NodeId> {
        let Some(clip) = self.clipboard.clone() else {
            return Vec::new();
        };
        self.finish_edit();
        self.touch();
        let mut created = Vec::new();
        for root in clip.roots().to_vec() {
            let id = copy_subtree(&clip, root, &mut self.document, None);
            let rect = self.document.node(id).rect;
            self.document.move_node(id, rect.x + 16.0, rect.y + 16.0);
            created.push(id);
        }
        self.selection = created.clone();
        self.finish_edit();
        self.sync_markup();
        created
    }

    pub fn duplicate(&mut self) {
        let ids = self.top_selected();
        if ids.is_empty() {
            return;
        }
        self.finish_edit();
        self.touch();
        let mut scratch = Document::new("Duplicate");
        let mut staged = Vec::new();
        for id in ids {
            let abs = self.document.absolute_rect(id);
            let parent = self.document.node(id).parent;
            let copied = copy_subtree(&self.document, id, &mut scratch, None);
            scratch.move_node(copied, abs.x + 16.0, abs.y + 16.0);
            staged.push((copied, parent));
        }
        let mut created = Vec::new();
        for (id, parent) in staged {
            let abs = scratch.absolute_rect(id);
            let copied = copy_subtree(&scratch, id, &mut self.document, parent);
            let local = self.to_parent_local(parent, abs);
            self.document.move_node(copied, local.x, local.y);
            created.push(copied);
        }
        self.selection = created;
        self.finish_edit();
        self.sync_markup();
    }

    pub fn apply_command(&mut self, command: Command, viewport: [f32; 2]) {
        match command {
            Command::Undo => {
                self.undo();
            }
            Command::Redo => {
                self.redo();
            }
            Command::Copy => self.copy(),
            Command::Paste => {
                self.paste();
            }
            Command::Duplicate => self.duplicate(),
            Command::Delete => self.delete_selected(),
            Command::SelectAll => self.select_all(),
            Command::Group => {
                self.group();
            }
            Command::ZoomIn => self.zoom_in(),
            Command::ZoomOut => self.zoom_out(),
            Command::Fit => self.fit(viewport[0], viewport[1]),
        }
    }

    fn frame_parent_for_new(&self) -> Option<NodeId> {
        let id = self.selected()?;
        let node = self.document.get(id)?;
        if node.kind == NodeKind::Frame {
            return Some(id);
        }
        let parent = node.parent?;
        if self.document.get(parent)?.kind == NodeKind::Frame {
            Some(parent)
        } else {
            None
        }
    }

    fn to_parent_local(&self, parent: Option<NodeId>, absolute: Rect) -> Rect {
        let (ox, oy) = parent
            .map(|id| self.document.absolute_rect(id))
            .map(|rect| (rect.x, rect.y))
            .unwrap_or((0.0, 0.0));
        Rect::new(
            absolute.x - ox,
            absolute.y - oy,
            absolute.width,
            absolute.height,
        )
    }

    fn top_selected(&self) -> Vec<NodeId> {
        self.selection
            .iter()
            .copied()
            .filter(|id| self.document.get(*id).is_some())
            .filter(|id| {
                !self.selection.iter().any(|other| {
                    *other != *id
                        && self.document.get(*other).is_some()
                        && self.document.is_descendant(*id, *other)
                })
            })
            .collect()
    }

    fn touch(&mut self) {
        if self.editing {
            return;
        }
        self.undo.push(self.snapshot());
        if self.undo.len() > 64 {
            self.undo.remove(0);
        }
        self.redo.clear();
        self.editing = true;
    }

    fn snapshot(&self) -> Snapshot {
        Snapshot {
            document: self.document.clone(),
            selection: self.selection.clone(),
            keys: self.keys.clone(),
            time: self.time,
            playing: self.playing,
            zoom: self.zoom,
            pan: self.pan,
        }
    }

    fn restore(&mut self, snapshot: Snapshot) {
        self.document = snapshot.document;
        self.selection = snapshot.selection;
        self.keys = snapshot.keys;
        self.time = snapshot.time;
        self.playing = snapshot.playing;
        self.zoom = snapshot.zoom;
        self.pan = snapshot.pan;
        self.editing = false;
        self.sync_markup();
    }

    fn sync_markup(&mut self) {
        let (html, css) = html_and_css(&self.document);
        self.html = html;
        self.css = css;
        self.revision = self.revision.wrapping_add(1);
    }

    fn library_parent(&self) -> Option<AssetId> {
        let id = self.library_selected?;
        let asset = self.library.asset(id)?;
        if asset.kind == AssetKind::Folder {
            Some(id)
        } else {
            asset.parent
        }
    }

    fn note_library_asset(&mut self, path: &str) {
        let name = path.rsplit(['/', '\\']).next().unwrap_or(path);
        if name.trim().is_empty() {
            return;
        }
        if self.library.assets().iter().any(|asset| asset.name == name) {
            return;
        }
        if self.library.create_asset(None, name).is_ok() {
            self.library_dirty = true;
        }
    }

    fn note_library_component(&mut self, name: &str) {
        if name.trim().is_empty() {
            return;
        }
        if self
            .library
            .components()
            .iter()
            .any(|component| component.name == name)
        {
            return;
        }
        if self.library.create_component(name).is_ok() {
            self.library_dirty = true;
        }
    }
}

impl Default for Editor {
    fn default() -> Self {
        Self::new()
    }
}

fn visible_color(node: &crate::document::Node) -> String {
    let color = if node.fill.a > 0 {
        node.fill
    } else {
        node.color
    };
    format_color(color)
}

fn sample(keys: &[Keyframe], node: NodeId, prop: Motion, time: f32, base: f32) -> f32 {
    let mut mine: Vec<(f32, f32, Easing)> = keys
        .iter()
        .filter(|key| key.node == node && key.prop == prop)
        .map(|key| (key.time, key.value, key.ease))
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
            let t = pair[0].2.apply((time - pair[0].0) / span);
            return pair[0].1 + (pair[1].1 - pair[0].1) * t;
        }
    }
    base
}

fn union_rect(a: Rect, b: Rect) -> Rect {
    let x = a.x.min(b.x);
    let y = a.y.min(b.y);
    Rect::new(
        x,
        y,
        (a.x + a.width).max(b.x + b.width) - x,
        (a.y + a.height).max(b.y + b.height) - y,
    )
}

fn shared_parent(doc: &Document, ids: &[NodeId]) -> Option<NodeId> {
    let parent = doc.node(ids[0]).parent;
    if ids.iter().all(|id| doc.node(*id).parent == parent) {
        parent
    } else {
        None
    }
}

fn deepest_frame_at(doc: &Document, x: f32, y: f32, moving: NodeId) -> Option<NodeId> {
    fn walk(
        doc: &Document,
        ids: &[NodeId],
        x: f32,
        y: f32,
        moving: NodeId,
        best: &mut Option<NodeId>,
    ) {
        for id in ids {
            if *id == moving || doc.is_descendant(*id, moving) {
                continue;
            }
            let node = doc.node(*id);
            let abs = doc.absolute_rect(*id);
            if node.kind == NodeKind::Frame && abs.contains(x, y) {
                *best = Some(*id);
            }
            walk(doc, &node.children, x, y, moving, best);
        }
    }
    let mut best = None;
    walk(doc, doc.roots(), x, y, moving, &mut best);
    best
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
        copied.corner_radius = node.corner_radius;
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

pub fn node_purpose(node: &crate::document::Node) -> String {
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
        let parent = node
            .parent
            .map(|id| id.0.to_string())
            .unwrap_or_else(|| "null".into());
        let slot = node
            .dock_slot
            .map(|slot| format!("\"{}\"", slot.as_str()))
            .unwrap_or_else(|| "null".into());
        regions.push_str(&format!(
            "{{\"id\":{id},\"parent\":{parent},\"name\":{name},\"kind\":\"{kind}\",\"purpose\":{purpose},\"dock_slot\":{slot},\"text\":{text},\"image\":{image},\"x\":{x},\"y\":{y},\"width\":{width},\"height\":{height},\"color\":\"{color}\"}}",
            id = node.id.0,
            name = json_encode(&node.name),
            kind = node.kind,
            purpose = json_encode(&node_purpose(node)),
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
