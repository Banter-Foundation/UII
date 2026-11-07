//! The window shell and the Design canvas.
//! Animation and Function are empty tabs. The content dock and Inspect are hooks.

use crate::{animation, content_dock, function, inspect};
use dear_imgui_rs::{
    Condition, DrawListMut, DrawSegmentCount, MouseButton, Ui, WindowFlags,
};
use uii_core::{resize_from_handle, Editor, Handle, NodeId, NodeKind, Rect, Tool, WorkspaceTab};

pub struct View {
    pan: [f32; 2],
    gesture: Gesture,
    made: u32,
    markup_open: bool,
    markup_dirty: bool,
    html_draft: String,
    css_draft: String,
}

#[derive(Clone, Copy)]
enum Gesture {
    None,
    Pan { last: [f32; 2] },
    Move { id: NodeId, last: [f32; 2] },
    Resize { id: NodeId, handle: Handle, last: [f32; 2] },
    Create { kind: NodeKind, start: [f32; 2] },
}

impl View {
    pub fn new() -> Self {
        Self {
            pan: [0.0, 0.0],
            gesture: Gesture::None,
            made: 1,
            markup_open: false,
            markup_dirty: false,
            html_draft: String::new(),
            css_draft: String::new(),
        }
    }

    pub fn draw(&mut self, ui: &Ui, editor: &mut Editor) {
        if !self.markup_dirty {
            let (html, css) = editor.markup();
            if html != self.html_draft {
                self.html_draft = html.to_string();
            }
            if css != self.css_draft {
                self.css_draft = css.to_string();
            }
        }

        let display = ui.io().display_size();
        ui.window("UII")
            .position([0.0, 0.0], Condition::Always)
            .size(display, Condition::Always)
            .flags(
                WindowFlags::NO_TITLE_BAR
                    | WindowFlags::NO_RESIZE
                    | WindowFlags::NO_MOVE
                    | WindowFlags::NO_COLLAPSE
                    | WindowFlags::NO_BRING_TO_FRONT_ON_FOCUS,
            )
            .build(|| {
                self.workspace(ui, editor);
                inspect::draw(ui, editor);
                content_dock::draw(ui, editor);
            });
    }

    fn workspace(&mut self, ui: &Ui, editor: &mut Editor) {
        if let Some(_bar) = ui.tab_bar("workspace") {
            if let Some(_tab) = ui.tab_item("Design") {
                editor.set_tab(WorkspaceTab::Design);
                self.design(ui, editor);
            }
            if let Some(_tab) = ui.tab_item("Animation") {
                editor.set_tab(WorkspaceTab::Animation);
                animation::draw(ui, editor);
            }
            if let Some(_tab) = ui.tab_item("Function") {
                editor.set_tab(WorkspaceTab::Function);
                function::draw(ui, editor);
            }
        }
    }

    fn design(&mut self, ui: &Ui, editor: &mut Editor) {
        self.toolbar(ui, editor);
        if self.markup_open {
            self.markup_panel(ui, editor);
        }
        let avail = ui.content_region_avail();
        ui.child_window("design-body")
            .size([avail[0], avail[1].max(120.0)])
            .border(false)
            .flags(WindowFlags::NO_SCROLLBAR)
            .build(ui, || {
                let inner = ui.content_region_avail();
                ui.child_window("layers")
                    .size([200.0, inner[1]])
                    .border(true)
                    .build(ui, || {
                        ui.text("Layers");
                        let roots = editor.document().roots().to_vec();
                        self.layers(ui, editor, &roots);
                    });
                ui.same_line();
                let canvas_w = (inner[0] - 208.0).max(80.0);
                ui.child_window("canvas")
                    .size([canvas_w, inner[1]])
                    .border(true)
                    .flags(WindowFlags::NO_SCROLLBAR)
                    .build(ui, || {
                        self.canvas(ui, editor);
                    });
            });
    }

    fn toolbar(&mut self, ui: &Ui, editor: &mut Editor) {
        let tools = Editor::tools();
        for (index, tool) in tools.iter().enumerate() {
            if index > 0 {
                ui.same_line();
            }
            let active = editor.tool() == *tool;
            let label = if active {
                format!("[{}]##tool{index}", tool.label())
            } else {
                format!("{}##tool{index}", tool.label())
            };
            if ui.button(&label) {
                editor.set_tool(*tool);
            }
        }
        ui.same_line();
        if ui.button("Delete") {
            editor.delete_selected();
        }
        ui.same_line();
        let markup = if self.markup_open {
            "Hide HTML / CSS"
        } else {
            "HTML / CSS"
        };
        if ui.button(markup) {
            self.markup_open = !self.markup_open;
        }
    }

    fn markup_panel(&mut self, ui: &Ui, editor: &mut Editor) {
        let width = ui.content_region_avail()[0];
        ui.text("HTML");
        if ui
            .input_text_multiline("##html", &mut self.html_draft, [width, 90.0])
            .build()
        {
            self.markup_dirty = true;
        }
        ui.text("CSS");
        if ui
            .input_text_multiline("##css", &mut self.css_draft, [width, 70.0])
            .build()
        {
            self.markup_dirty = true;
        }
        if ui.button("Apply markup") {
            editor.apply_markup(&self.html_draft, &self.css_draft);
            self.markup_dirty = false;
        }
        ui.same_line();
        if ui.button("Reload from canvas") {
            self.markup_dirty = false;
        }
    }

    fn layers(&mut self, ui: &Ui, editor: &mut Editor, ids: &[NodeId]) {
        for id in ids {
            let (label, children) = {
                let node = editor.document().node(*id);
                (
                    format!("{}  {}##layer{}", node.kind, node.name, id.0),
                    node.children.clone(),
                )
            };
            let selected = editor.selected() == Some(*id);
            let text = if selected {
                format!("> {label}")
            } else {
                label
            };
            if ui.button(&text) {
                editor.select(*id);
            }
            if !children.is_empty() {
                self.layers(ui, editor, &children);
            }
        }
    }

    fn canvas(&mut self, ui: &Ui, editor: &mut Editor) {
        let size = ui.content_region_avail();
        let origin = ui.get_cursor_screen_pos();
        ui.invisible_button("canvas-hit", size);
        let hovered = ui.is_item_hovered();
        let mouse = ui.mouse_pos();
        let local = [
            mouse[0] - origin[0] + self.pan[0],
            mouse[1] - origin[1] + self.pan[1],
        ];
        self.pointer(ui, editor, hovered, mouse, local);
        let draw = ui.get_window_draw_list();
        self.paint(ui, editor, &draw, origin);
    }

    fn pointer(
        &mut self,
        ui: &Ui,
        editor: &mut Editor,
        hovered: bool,
        mouse: [f32; 2],
        local: [f32; 2],
    ) {
        if hovered && ui.is_mouse_down(MouseButton::Right) {
            if let Gesture::Pan { last } = self.gesture {
                self.pan[0] -= mouse[0] - last[0];
                self.pan[1] -= mouse[1] - last[1];
            }
            self.gesture = Gesture::Pan { last: mouse };
            return;
        }
        if matches!(self.gesture, Gesture::Pan { .. }) {
            self.gesture = Gesture::None;
        }

        if hovered && ui.is_mouse_clicked(MouseButton::Left) {
            self.begin_press(editor, local);
        }

        if ui.is_mouse_down(MouseButton::Left) {
            self.drag(editor, local);
        } else if !matches!(self.gesture, Gesture::None) {
            self.finish(editor, local);
        }
    }

    fn begin_press(&mut self, editor: &mut Editor, local: [f32; 2]) {
        let tool = editor.tool();
        if let Some(kind) = tool.kind() {
            self.gesture = Gesture::Create {
                kind,
                start: local,
            };
            return;
        }
        if let Some(id) = editor.selected() {
            let rect = editor.document().absolute_rect(id);
            if let Some(handle) = hit_handle(rect, local) {
                self.gesture = Gesture::Resize {
                    id,
                    handle,
                    last: local,
                };
                return;
            }
        }
        if let Some(id) = editor.document().hit_test(local[0], local[1]) {
            editor.select(id);
            if tool == Tool::Select {
                self.gesture = Gesture::Move { id, last: local };
            } else {
                self.gesture = Gesture::None;
            }
        } else {
            editor.clear_selection();
            self.gesture = Gesture::None;
        }
    }

    fn drag(&mut self, editor: &mut Editor, local: [f32; 2]) {
        match self.gesture {
            Gesture::Move { id, last } => {
                let dx = local[0] - last[0];
                let dy = local[1] - last[1];
                let current = editor.document().node(id).rect;
                editor.move_node(id, current.x + dx, current.y + dy);
                self.gesture = Gesture::Move { id, last: local };
            }
            Gesture::Resize { id, handle, last } => {
                let dx = local[0] - last[0];
                let dy = local[1] - last[1];
                let current = editor.document().node(id).rect;
                let next = resize_from_handle(current, handle, dx, dy);
                editor.move_node(id, next.x, next.y);
                editor.resize_node(id, next.width, next.height);
                self.gesture = Gesture::Resize {
                    id,
                    handle,
                    last: local,
                };
            }
            _ => {}
        }
    }

    fn finish(&mut self, editor: &mut Editor, local: [f32; 2]) {
        if let Gesture::Create { kind, start } = self.gesture {
            let rect = drag_rect(start, local);
            let rect = if rect.width < 4.0 || rect.height < 4.0 {
                let (width, height) = default_size(kind);
                Rect::new(start[0], start[1], width, height)
            } else {
                rect
            };
            let name = format!("{} {}", kind_name(kind), self.made);
            self.made += 1;
            editor.add_shape(kind, &name, rect);
        }
        self.gesture = Gesture::None;
    }

    fn paint(&self, ui: &Ui, editor: &Editor, draw: &DrawListMut<'_>, origin: [f32; 2]) {
        let roots = editor.document().roots().to_vec();
        self.paint_nodes(editor, draw, &roots, origin);
        if let Gesture::Create { start, .. } = self.gesture {
            let mouse = ui.mouse_pos();
            let local = [
                mouse[0] - origin[0] + self.pan[0],
                mouse[1] - origin[1] + self.pan[1],
            ];
            let rect = drag_rect(start, local);
            let a = screen(origin, self.pan, rect.x, rect.y);
            let b = screen(
                origin,
                self.pan,
                rect.x + rect.width,
                rect.y + rect.height,
            );
            draw.add_rect(a, b, [0.35, 0.55, 1.0, 1.0]).build();
        }
    }

    fn paint_nodes(
        &self,
        editor: &Editor,
        draw: &DrawListMut<'_>,
        ids: &[NodeId],
        origin: [f32; 2],
    ) {
        for id in ids {
            let (kind, fill_rgba, ink_rgba, label, children) = {
                let node = editor.document().node(*id);
                let label = match node.kind {
                    NodeKind::Text | NodeKind::Button => node.text.clone(),
                    NodeKind::Image => node.image_src.clone(),
                    NodeKind::Table => node.name.clone(),
                    _ => node.text.clone(),
                };
                (
                    node.kind,
                    node.fill,
                    node.color,
                    label,
                    node.children.clone(),
                )
            };
            let rect = editor.document().absolute_rect(*id);
            let min = screen(origin, self.pan, rect.x, rect.y);
            let max = screen(
                origin,
                self.pan,
                rect.x + rect.width,
                rect.y + rect.height,
            );
            let fill = color(fill_rgba);
            let ink = color(ink_rgba);
            if kind == NodeKind::Ellipse {
                if rect.width > 0.0 && rect.height > 0.0 {
                    draw.add_ellipse_filled(
                        [(min[0] + max[0]) * 0.5, (min[1] + max[1]) * 0.5],
                        [rect.width * 0.5, rect.height * 0.5],
                        fill,
                        0.0,
                        DrawSegmentCount::new(32).unwrap_or(DrawSegmentCount::AUTO),
                    );
                }
            } else if fill_rgba.a > 0 {
                draw.add_rect_filled_multicolor(min, max, fill, fill, fill, fill);
            }
            draw.add_rect(min, max, ink).build();
            if !label.is_empty() {
                draw.add_text([min[0] + 6.0, min[1] + 4.0], ink, &label);
            }
            if !children.is_empty() {
                self.paint_nodes(editor, draw, &children, origin);
            }
            if editor.selected() == Some(*id) {
                draw.add_rect(
                    [min[0] - 1.0, min[1] - 1.0],
                    [max[0] + 1.0, max[1] + 1.0],
                    [0.24, 0.49, 1.0, 1.0],
                )
                .build();
                for (_, point) in handles(rect) {
                    let at = screen(origin, self.pan, point[0], point[1]);
                    draw.add_rect_filled_multicolor(
                        [at[0] - 3.0, at[1] - 3.0],
                        [at[0] + 3.0, at[1] + 3.0],
                        [0.24, 0.49, 1.0, 1.0],
                        [0.24, 0.49, 1.0, 1.0],
                        [0.24, 0.49, 1.0, 1.0],
                        [0.24, 0.49, 1.0, 1.0],
                    );
                }
            }
        }
    }
}

fn handles(rect: Rect) -> [(Handle, [f32; 2]); 8] {
    let left = rect.x;
    let top = rect.y;
    let right = rect.x + rect.width;
    let bottom = rect.y + rect.height;
    let mid_x = (left + right) * 0.5;
    let mid_y = (top + bottom) * 0.5;
    [
        (Handle::NorthWest, [left, top]),
        (Handle::North, [mid_x, top]),
        (Handle::NorthEast, [right, top]),
        (Handle::East, [right, mid_y]),
        (Handle::SouthEast, [right, bottom]),
        (Handle::South, [mid_x, bottom]),
        (Handle::SouthWest, [left, bottom]),
        (Handle::West, [left, mid_y]),
    ]
}

fn hit_handle(rect: Rect, local: [f32; 2]) -> Option<Handle> {
    handles(rect)
        .into_iter()
        .find(|(_, point)| (local[0] - point[0]).abs() <= 6.0 && (local[1] - point[1]).abs() <= 6.0)
        .map(|(handle, _)| handle)
}

fn drag_rect(start: [f32; 2], end: [f32; 2]) -> Rect {
    let x = start[0].min(end[0]);
    let y = start[1].min(end[1]);
    Rect::new(x, y, (end[0] - start[0]).abs(), (end[1] - start[1]).abs())
}

fn default_size(kind: NodeKind) -> (f32, f32) {
    match kind {
        NodeKind::Text => (120.0, 28.0),
        NodeKind::Ellipse => (80.0, 80.0),
        _ => (160.0, 96.0),
    }
}

fn kind_name(kind: NodeKind) -> &'static str {
    match kind {
        NodeKind::Frame => "Frame",
        NodeKind::Text => "Text",
        NodeKind::Rectangle => "Rectangle",
        NodeKind::Ellipse => "Ellipse",
        NodeKind::Button => "Button",
        NodeKind::Image => "Image",
        NodeKind::Table => "Table",
        NodeKind::Dock => "Dock",
        NodeKind::Child => "Child",
    }
}

fn screen(origin: [f32; 2], pan: [f32; 2], x: f32, y: f32) -> [f32; 2] {
    [origin[0] + x - pan[0], origin[1] + y - pan[1]]
}

fn color(value: uii_core::Rgba) -> [f32; 4] {
    [
        value.r as f32 / 255.0,
        value.g as f32 / 255.0,
        value.b as f32 / 255.0,
        value.a as f32 / 255.0,
    ]
}
