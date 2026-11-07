//! Editable design tree. Coordinates are parent-relative.

use std::fmt;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct NodeId(pub u32);

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Rect {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

impl Rect {
    pub fn new(x: f32, y: f32, width: f32, height: f32) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }

    pub fn contains(self, px: f32, py: f32) -> bool {
        px >= self.x && py >= self.y && px < self.x + self.width && py < self.y + self.height
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NodeKind {
    Frame,
    Text,
    Rectangle,
    Ellipse,
    Button,
    Image,
    Table,
    Dock,
    Child,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DockSlot {
    Top,
    Bottom,
    Left,
    Right,
    Center,
}

impl DockSlot {
    pub fn as_str(self) -> &'static str {
        match self {
            DockSlot::Top => "top",
            DockSlot::Bottom => "bottom",
            DockSlot::Left => "left",
            DockSlot::Right => "right",
            DockSlot::Center => "center",
        }
    }

    pub fn parse(text: &str) -> Option<Self> {
        match text {
            "top" | "Top" => Some(DockSlot::Top),
            "bottom" | "Bottom" => Some(DockSlot::Bottom),
            "left" | "Left" => Some(DockSlot::Left),
            "right" | "Right" => Some(DockSlot::Right),
            "center" | "Center" => Some(DockSlot::Center),
            _ => None,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Handle {
    North,
    South,
    East,
    West,
    NorthEast,
    NorthWest,
    SouthEast,
    SouthWest,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Rgba {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

impl Rgba {
    pub const fn rgb(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b, a: 255 }
    }

    pub const fn rgba(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self { r, g, b, a }
    }
}

#[derive(Clone, Debug)]
pub struct Node {
    pub id: NodeId,
    pub parent: Option<NodeId>,
    pub children: Vec<NodeId>,
    pub kind: NodeKind,
    pub name: String,
    pub rect: Rect,
    pub fill: Rgba,
    pub color: Rgba,
    pub text: String,
    pub font_size: f32,
    pub image_src: String,
    pub dock_slot: Option<DockSlot>,
    pub columns: Vec<String>,
    pub rows: Vec<Vec<String>>,
    alive: bool,
}

pub struct Document {
    pub name: String,
    nodes: Vec<Node>,
    roots: Vec<NodeId>,
}

impl Document {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            nodes: Vec::new(),
            roots: Vec::new(),
        }
    }

    pub fn create(
        &mut self,
        parent: Option<NodeId>,
        kind: NodeKind,
        name: impl Into<String>,
        rect: Rect,
    ) -> NodeId {
        let id = NodeId(self.nodes.len() as u32);
        let (fill, color) = default_colors(kind);
        self.nodes.push(Node {
            id,
            parent,
            children: Vec::new(),
            kind,
            name: name.into(),
            rect,
            fill,
            color,
            text: String::new(),
            font_size: 16.0,
            image_src: String::new(),
            dock_slot: None,
            columns: Vec::new(),
            rows: Vec::new(),
            alive: true,
        });
        if let Some(parent) = parent {
            self.nodes[parent.0 as usize].children.push(id);
        } else {
            self.roots.push(id);
        }
        id
    }

    pub fn node(&self, id: NodeId) -> &Node {
        let node = &self.nodes[id.0 as usize];
        assert!(node.alive, "node {} was deleted", id.0);
        node
    }

    pub fn node_mut(&mut self, id: NodeId) -> &mut Node {
        let node = &mut self.nodes[id.0 as usize];
        assert!(node.alive, "node {} was deleted", id.0);
        node
    }

    pub fn get(&self, id: NodeId) -> Option<&Node> {
        self.nodes.get(id.0 as usize).filter(|node| node.alive)
    }

    pub fn roots(&self) -> &[NodeId] {
        &self.roots
    }

    pub fn children(&self, id: NodeId) -> &[NodeId] {
        &self.node(id).children
    }

    pub fn nodes(&self) -> impl Iterator<Item = &Node> {
        self.nodes.iter().filter(|node| node.alive)
    }

    pub fn find_named(&self, name: &str) -> Option<&Node> {
        self.nodes().find(|node| node.name == name)
    }

    pub fn move_node(&mut self, id: NodeId, x: f32, y: f32) {
        let rect = &mut self.node_mut(id).rect;
        rect.x = x;
        rect.y = y;
    }

    pub fn resize_node(&mut self, id: NodeId, width: f32, height: f32) {
        let rect = &mut self.node_mut(id).rect;
        rect.width = width.max(1.0);
        rect.height = height.max(1.0);
    }

    pub fn set_text(&mut self, id: NodeId, text: impl Into<String>) {
        self.node_mut(id).text = text.into();
    }

    pub fn set_image(&mut self, id: NodeId, src: impl Into<String>) {
        self.node_mut(id).image_src = src.into();
    }

    pub fn set_dock_slot(&mut self, id: NodeId, slot: DockSlot) {
        self.node_mut(id).dock_slot = Some(slot);
    }

    pub fn set_table(&mut self, id: NodeId, columns: Vec<String>, rows: Vec<Vec<String>>) {
        let node = self.node_mut(id);
        node.columns = columns;
        node.rows = rows;
    }

    pub fn delete(&mut self, id: NodeId) {
        let mut stack = vec![id];
        let mut doomed = Vec::new();
        while let Some(current) = stack.pop() {
            if !self.nodes[current.0 as usize].alive {
                continue;
            }
            doomed.push(current);
            stack.extend(self.nodes[current.0 as usize].children.iter().copied());
        }
        for dead in &doomed {
            self.nodes[dead.0 as usize].alive = false;
            self.nodes[dead.0 as usize].children.clear();
        }
        if let Some(parent) = self.nodes[id.0 as usize].parent {
            self.nodes[parent.0 as usize]
                .children
                .retain(|child| child.0 != id.0);
        } else {
            self.roots.retain(|root| root.0 != id.0);
        }
    }

    pub fn absolute_rect(&self, id: NodeId) -> Rect {
        let mut rect = self.node(id).rect;
        let mut parent = self.node(id).parent;
        while let Some(id) = parent {
            let node = self.node(id);
            rect.x += node.rect.x;
            rect.y += node.rect.y;
            parent = node.parent;
        }
        rect
    }

    pub fn hit_test(&self, x: f32, y: f32) -> Option<NodeId> {
        fn hit(doc: &Document, ids: &[NodeId], x: f32, y: f32) -> Option<NodeId> {
            for id in ids.iter().rev() {
                let node = doc.node(*id);
                if let Some(found) = hit(doc, &node.children, x, y) {
                    return Some(found);
                }
                let abs = doc.absolute_rect(*id);
                if contains_kind(node.kind, abs, x, y) {
                    return Some(*id);
                }
            }
            None
        }
        hit(self, &self.roots, x, y)
    }
}

fn contains_kind(kind: NodeKind, rect: Rect, x: f32, y: f32) -> bool {
    if rect.width <= 0.0 || rect.height <= 0.0 {
        return false;
    }
    if kind == NodeKind::Ellipse {
        let cx = rect.x + rect.width * 0.5;
        let cy = rect.y + rect.height * 0.5;
        let dx = (x - cx) / (rect.width * 0.5);
        let dy = (y - cy) / (rect.height * 0.5);
        return dx * dx + dy * dy <= 1.0;
    }
    rect.contains(x, y)
}

fn default_colors(kind: NodeKind) -> (Rgba, Rgba) {
    let ink = Rgba::rgb(0x1a, 0x1a, 0x1a);
    let fill = match kind {
        NodeKind::Text => Rgba::rgba(0, 0, 0, 0),
        NodeKind::Button => Rgba::rgb(0x3d, 0x7e, 0xff),
        NodeKind::Rectangle | NodeKind::Ellipse => Rgba::rgb(0xd0, 0xd0, 0xd0),
        NodeKind::Image => Rgba::rgb(0xee, 0xee, 0xee),
        _ => Rgba::rgb(0xff, 0xff, 0xff),
    };
    (fill, ink)
}

const MIN_SIZE: f32 = 1.0;

pub fn resize_from_handle(rect: Rect, handle: Handle, dx: f32, dy: f32) -> Rect {
    let mut x = rect.x;
    let mut y = rect.y;
    let mut width = rect.width;
    let mut height = rect.height;
    let east = matches!(handle, Handle::East | Handle::NorthEast | Handle::SouthEast);
    let west = matches!(handle, Handle::West | Handle::NorthWest | Handle::SouthWest);
    let south = matches!(handle, Handle::South | Handle::SouthEast | Handle::SouthWest);
    let north = matches!(handle, Handle::North | Handle::NorthEast | Handle::NorthWest);
    if east {
        width += dx;
    }
    if west {
        x += dx;
        width -= dx;
    }
    if south {
        height += dy;
    }
    if north {
        y += dy;
        height -= dy;
    }
    if width < MIN_SIZE {
        if west {
            x -= MIN_SIZE - width;
        }
        width = MIN_SIZE;
    }
    if height < MIN_SIZE {
        if north {
            y -= MIN_SIZE - height;
        }
        height = MIN_SIZE;
    }
    Rect::new(x, y, width, height)
}

impl fmt::Display for NodeKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let label = match self {
            NodeKind::Frame => "frame",
            NodeKind::Text => "text",
            NodeKind::Rectangle => "rectangle",
            NodeKind::Ellipse => "ellipse",
            NodeKind::Button => "button",
            NodeKind::Image => "image",
            NodeKind::Table => "table",
            NodeKind::Dock => "dock",
            NodeKind::Child => "child",
        };
        f.write_str(label)
    }
}
