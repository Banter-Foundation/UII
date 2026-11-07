//! Design canvas state. Animation and Function are placeholder tabs.
//! The content dock and Inspect are hooks in the window, not this model.

use crate::document::{Document, NodeId, NodeKind, Rect, Rgba};
use crate::html::{self, import_html_css};

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

    pub fn is_placeholder(self) -> bool {
        matches!(self, WorkspaceTab::Animation | WorkspaceTab::Function)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tool {
    Select,
    Frame,
    Text,
    Rectangle,
    Ellipse,
}

impl Tool {
    pub fn label(self) -> &'static str {
        match self {
            Tool::Select => "Select",
            Tool::Frame => "Frame",
            Tool::Text => "Text",
            Tool::Rectangle => "Rectangle",
            Tool::Ellipse => "Ellipse",
        }
    }

    pub fn kind(self) -> Option<NodeKind> {
        match self {
            Tool::Frame => Some(NodeKind::Frame),
            Tool::Text => Some(NodeKind::Text),
            Tool::Rectangle => Some(NodeKind::Rectangle),
            Tool::Ellipse => Some(NodeKind::Ellipse),
            Tool::Select => None,
        }
    }
}

pub struct Editor {
    tab: WorkspaceTab,
    tool: Tool,
    document: Document,
    html: String,
    css: String,
    selected: Option<NodeId>,
}

impl Editor {
    pub fn new() -> Self {
        Self {
            tab: WorkspaceTab::Design,
            tool: Tool::Select,
            document: Document::new("Untitled"),
            html: String::new(),
            css: String::new(),
            selected: None,
        }
    }

    pub fn tabs() -> [WorkspaceTab; 3] {
        [
            WorkspaceTab::Design,
            WorkspaceTab::Animation,
            WorkspaceTab::Function,
        ]
    }

    pub fn tools() -> [Tool; 5] {
        [
            Tool::Select,
            Tool::Frame,
            Tool::Text,
            Tool::Rectangle,
            Tool::Ellipse,
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
            self.document.delete(id);
            self.sync_markup();
        }
    }

    pub fn apply_markup(&mut self, html: &str, css: &str) {
        self.document = import_html_css(html, css);
        self.selected = None;
        self.sync_markup();
    }

    fn sync_markup(&mut self) {
        let (html, css) = html::html_and_css_parts(&self.document);
        self.html = html;
        self.css = css;
    }
}

impl Default for Editor {
    fn default() -> Self {
        Self::new()
    }
}
