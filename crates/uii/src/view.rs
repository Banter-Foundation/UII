//! The window. Design and Animation are the only top tabs. Inspect is a
//! tool. Assets and Components live in the collapsible content drawer.
//! HTML and CSS stay off the main canvas until they are opened.

use dear_imgui_rs::{
    Condition, DrawListMut, DrawSegmentCount, MouseButton, Ui, WindowFlags,
};
use uii_core::{
    export_source, import_source, resize_from_handle, DrawerPage, Editor, Handle, Motion, NodeId,
    NodeKind, Rect, WorkspaceTab,
};

pub struct View {
    pan: [f32; 2],
    gesture: Gesture,
    made: u32,
    markup_open: bool,
    markup_dirty: bool,
    html_draft: String,
    css_draft: String,
    import_open: bool,
    export_open: bool,
    format_index: usize,
    import_html: String,
    import_css: String,
    import_source: String,
    export_text: String,
    asset_path: String,
    component_name: String,
    status: String,
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
            import_open: false,
            export_open: false,
            format_index: 0,
            import_html: String::new(),
            import_css: String::new(),
            import_source: String::new(),
            export_text: String::new(),
            asset_path: String::new(),
            component_name: String::from("Card"),
            status: String::new(),
        }
    }

    pub fn draw(&mut self, ui: &Ui, editor: &mut Editor, dt: f32) {
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
                self.workspace(ui, editor, dt);
            });
        self.exchange_windows(ui, editor);
    }

    fn workspace(&mut self, ui: &Ui, editor: &mut Editor, dt: f32) {
        if let Some(_bar) = ui.tab_bar("workspace") {
            if let Some(_tab) = ui.tab_item("Design") {
                editor.set_tab(WorkspaceTab::Design);
                self.workspace_body(ui, editor, dt, false);
            }
            if let Some(_tab) = ui.tab_item("Animation") {
                editor.set_tab(WorkspaceTab::Animation);
                self.workspace_body(ui, editor, dt, true);
            }
        }
    }

    fn workspace_body(&mut self, ui: &Ui, editor: &mut Editor, dt: f32, animate: bool) {
        if animate && editor.playing() {
            editor.tick(dt);
        }
        self.toolbar(ui, editor);
        if editor.tool() == uii_core::Tool::Inspect {
            self.inspect_strip(ui, editor);
        }
        if self.markup_open {
            self.markup_panel(ui, editor);
        }

        let avail = ui.content_region_avail();
        let drawer_body = if editor.drawer_collapsed() { 0.0 } else { 168.0 };
        let timeline = if animate { 88.0 } else { 0.0 };
        let body_h = (avail[1] - drawer_body - timeline - 32.0).max(120.0);
        ui.child_window("workspace-body")
            .size([avail[0], body_h])
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
                        self.canvas(ui, editor, animate);
                    });
            });

        if animate {
            self.timeline(ui, editor);
        }
        self.drawer(ui, editor);
        if !self.status.is_empty() {
            ui.text(&self.status);
        }
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
        if ui.button("Import") {
            self.import_open = true;
        }
        ui.same_line();
        if ui.button("Export") {
            self.export_open = true;
            self.refresh_export(editor);
        }
        ui.same_line();
        let markup = if self.markup_open { "Hide HTML / CSS" } else { "HTML / CSS" };
        if ui.button(markup) {
            self.markup_open = !self.markup_open;
        }
    }

    fn inspect_strip(&mut self, ui: &Ui, editor: &Editor) {
        match editor.inspection() {
            Some(info) => ui.text(format!(
                "Inspect   {}   {}   {:.0} x {:.0}   {}",
                info.name, info.kind, info.width, info.height, info.color
            )),
            None => ui.text("Inspect   select an object to see its type, size, and color"),
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
            self.status.clear();
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

    fn canvas(&mut self, ui: &Ui, editor: &mut Editor, animate: bool) {
        let size = ui.content_region_avail();
        let origin = ui.get_cursor_screen_pos();
        ui.invisible_button("canvas-hit", size);
        let hovered = ui.is_item_hovered();
        let mouse = ui.mouse_pos();
        let local = [
            mouse[0] - origin[0] + self.pan[0],
            mouse[1] - origin[1] + self.pan[1],
        ];
        self.pointer(ui, editor, hovered, mouse, local, animate);
        let draw = ui.get_window_draw_list();
        self.paint(ui, editor, &draw, origin, animate);
    }

    fn pointer(
        &mut self,
        ui: &Ui,
        editor: &mut Editor,
        hovered: bool,
        mouse: [f32; 2],
        local: [f32; 2],
        animate: bool,
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
            self.begin_press(editor, local, animate);
        }

        if ui.is_mouse_down(MouseButton::Left) {
            self.drag(editor, local, animate);
        } else if !matches!(self.gesture, Gesture::None) {
            self.finish(editor, local);
        }
    }

    fn begin_press(&mut self, editor: &mut Editor, local: [f32; 2], animate: bool) {
        let tool = editor.tool();
        if let Some(kind) = tool.kind() {
            self.gesture = Gesture::Create {
                kind,
                start: local,
            };
            return;
        }
        if let Some(id) = editor.selected() {
            let rect = shown_rect(editor, id, animate);
            if let Some(handle) = hit_handle(rect, local) {
                self.gesture = Gesture::Resize {
                    id,
                    handle,
                    last: local,
                };
                return;
            }
        }
        if let Some(id) = hit_node(editor, local, animate) {
            editor.select(id);
            if tool == uii_core::Tool::Select {
                self.gesture = Gesture::Move { id, last: local };
            } else {
                self.gesture = Gesture::None;
            }
        } else {
            editor.clear_selection();
            self.gesture = Gesture::None;
        }
    }

    fn drag(&mut self, editor: &mut Editor, local: [f32; 2], animate: bool) {
        match self.gesture {
            Gesture::Move { id, last } => {
                let dx = local[0] - last[0];
                let dy = local[1] - last[1];
                let current = local_rect(editor, id, animate);
                apply_rect(
                    editor,
                    id,
                    Rect::new(current.x + dx, current.y + dy, current.width, current.height),
                    animate,
                );
                self.gesture = Gesture::Move { id, last: local };
            }
            Gesture::Resize { id, handle, last } => {
                let dx = local[0] - last[0];
                let dy = local[1] - last[1];
                let current = local_rect(editor, id, animate);
                let next = resize_from_handle(current, handle, dx, dy);
                apply_rect(editor, id, next, animate);
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
                Rect::new(start[0], start[1], default_size(kind).0, default_size(kind).1)
            } else {
                rect
            };
            let name = format!("{} {}", kind_name(kind), self.made);
            self.made += 1;
            editor.add_shape(kind, &name, rect);
        }
        self.gesture = Gesture::None;
    }

    fn paint(
        &self,
        ui: &Ui,
        editor: &Editor,
        draw: &DrawListMut<'_>,
        origin: [f32; 2],
        animate: bool,
    ) {
        let roots = editor.document().roots().to_vec();
        self.paint_nodes(editor, draw, &roots, origin, animate);
        if let Gesture::Create { start, .. } = self.gesture {
            let mouse = ui.mouse_pos();
            let local = [
                mouse[0] - origin[0] + self.pan[0],
                mouse[1] - origin[1] + self.pan[1],
            ];
            let rect = drag_rect(start, local);
            let a = screen(origin, self.pan, rect.x, rect.y);
            let b = screen(origin, self.pan, rect.x + rect.width, rect.y + rect.height);
            draw.add_rect(a, b, [0.35, 0.55, 1.0, 1.0]).build();
        }
    }

    fn paint_nodes(
        &self,
        editor: &Editor,
        draw: &DrawListMut<'_>,
        ids: &[NodeId],
        origin: [f32; 2],
        animate: bool,
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
            let rect = shown_rect(editor, *id, animate);
            let min = screen(origin, self.pan, rect.x, rect.y);
            let max = screen(origin, self.pan, rect.x + rect.width, rect.y + rect.height);
            let fill = color(fill_rgba);
            let ink = color(ink_rgba);
            if kind == NodeKind::Ellipse {
                if rect.width > 0.0 && rect.height > 0.0 {
                    draw.add_ellipse_filled(
                        [
                            (min[0] + max[0]) * 0.5,
                            (min[1] + max[1]) * 0.5,
                        ],
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
                self.paint_nodes(editor, draw, &children, origin, animate);
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

    fn timeline(&mut self, ui: &Ui, editor: &mut Editor) {
        ui.separator();
        let playing = editor.playing();
        if ui.button(if playing { "Pause" } else { "Play" }) {
            editor.set_playing(!playing);
        }
        ui.same_line();
        if ui.button("Key") {
            if let Some(id) = editor.selected() {
                let rect = editor.document().node(id).rect;
                let time = editor.time();
                editor.add_key(id, Motion::X, time, rect.x);
                editor.add_key(id, Motion::Y, time, rect.y);
                editor.add_key(id, Motion::Width, time, rect.width);
                editor.add_key(id, Motion::Height, time, rect.height);
            }
        }
        ui.same_line();
        let mut time = editor.time();
        let end = editor.duration().max(time);
        if ui.slider_f32("Time", &mut time, 0.0, end) {
            editor.set_time(time);
        }
        if let Some(id) = editor.selected() {
            let keys = editor.keys_for(id);
            ui.text(format!("{} keys on the selection", keys.len()));
        }
    }

    fn drawer(&mut self, ui: &Ui, editor: &mut Editor) {
        ui.separator();
        let toggle = if editor.drawer_collapsed() {
            "Content  >"
        } else {
            "Content  v"
        };
        if ui.button(toggle) {
            editor.toggle_drawer();
        }
        if editor.drawer_collapsed() {
            return;
        }
        ui.same_line();
        for page in Editor::drawer_pages() {
            let current = editor.drawer_page() == page;
            let label = if current {
                format!("[{}]##drawer{}", page.label(), page.label())
            } else {
                format!("{}##drawer{}", page.label(), page.label())
            };
            if ui.button(&label) {
                editor.show_drawer(page);
            }
            ui.same_line();
        }
        ui.new_line();
        match editor.drawer_page() {
            DrawerPage::Assets => self.assets(ui, editor),
            DrawerPage::Components => self.components(ui, editor),
        }
    }

    fn assets(&mut self, ui: &Ui, editor: &mut Editor) {
        ui.input_text("Asset path", &mut self.asset_path).build();
        ui.same_line();
        if ui.button("Add asset") {
            editor.add_asset(&self.asset_path);
        }
        let paths = editor.assets();
        for (index, path) in paths.iter().enumerate() {
            if ui.button(format!("Place##asset{index}")) {
                editor.place_asset(path);
            }
            ui.same_line();
            ui.text(path);
        }
        if paths.is_empty() {
            ui.text("No assets yet. Add a path, then place it on the canvas.");
        }
    }

    fn components(&mut self, ui: &Ui, editor: &mut Editor) {
        ui.input_text("Component", &mut self.component_name).build();
        ui.same_line();
        if ui.button("Save selection") {
            editor.save_component(&self.component_name);
        }
        let names: Vec<String> = editor
            .components()
            .iter()
            .map(|component| component.name.clone())
            .collect();
        for (index, name) in names.iter().enumerate() {
            if ui.button(format!("Insert##component{index}")) {
                editor.insert_component(name);
            }
            ui.same_line();
            ui.text(name);
        }
        if names.is_empty() {
            ui.text("Select a layer and save it as a component.");
        }
    }

    fn exchange_windows(&mut self, ui: &Ui, editor: &mut Editor) {
        if self.import_open {
            let mut open = self.import_open;
            ui.window("Import")
                .size([520.0, 420.0], Condition::FirstUseEver)
                .opened(&mut open)
                .build(|| {
                    self.format_picker(ui);
                    let id = self.format_id();
                    if id == "html-css" {
                        ui.text("HTML");
                        ui.input_text_multiline("##import-html", &mut self.import_html, [490.0, 120.0])
                            .build();
                        ui.text("CSS");
                        ui.input_text_multiline("##import-css", &mut self.import_css, [490.0, 80.0])
                            .build();
                        if ui.button("Import into canvas") {
                            editor.apply_markup(&self.import_html, &self.import_css);
                            self.markup_dirty = false;
                            self.status = "Imported HTML and CSS.".into();
                        }
                    } else {
                        ui.input_text_multiline("##import-source", &mut self.import_source, [490.0, 220.0])
                            .build();
                        if ui.button("Import into canvas") {
                            match import_source(&id, &self.import_source) {
                                Ok(document) => {
                                    editor.replace_document(document);
                                    self.markup_dirty = false;
                                    self.status = format!("Imported {id}.");
                                }
                                Err(err) => self.status = err.message,
                            }
                        }
                    }
                });
            self.import_open = open;
        }
        if self.export_open {
            let mut open = self.export_open;
            ui.window("Export")
                .size([520.0, 420.0], Condition::FirstUseEver)
                .opened(&mut open)
                .build(|| {
                    if self.format_picker(ui) {
                        self.refresh_export(editor);
                    }
                    if ui.button("Refresh export") {
                        self.refresh_export(editor);
                    }
                    ui.input_text_multiline("##export", &mut self.export_text, [490.0, 280.0])
                        .build();
                });
            self.export_open = open;
        }
    }

    fn format_picker(&mut self, ui: &Ui) -> bool {
        let formats = uii_core::formats();
        if self.format_index >= formats.len() {
            self.format_index = 0;
        }
        let mut changed = false;
        for (index, format) in formats.iter().enumerate() {
            if index > 0 && index % 4 != 0 {
                ui.same_line();
            }
            let label = if index == self.format_index {
                format!("[{}]##fmt{index}", format.label)
            } else {
                format!("{}##fmt{index}", format.label)
            };
            if ui.button(&label) {
                self.format_index = index;
                changed = true;
            }
        }
        changed
    }

    fn format_id(&self) -> String {
        uii_core::formats()
            .get(self.format_index)
            .map(|format| format.id.to_string())
            .unwrap_or_else(|| "html-css".into())
    }

    fn refresh_export(&mut self, editor: &Editor) {
        let id = self.format_id();
        match export_source(editor.document(), &id) {
            Ok(text) => {
                self.export_text = text;
                self.status = format!("Exported {id}.");
            }
            Err(err) => self.status = err.message,
        }
    }
}

fn apply_rect(editor: &mut Editor, id: NodeId, rect: Rect, animate: bool) {
    if animate {
        let time = editor.time();
        editor.add_key(id, Motion::X, time, rect.x);
        editor.add_key(id, Motion::Y, time, rect.y);
        editor.add_key(id, Motion::Width, time, rect.width);
        editor.add_key(id, Motion::Height, time, rect.height);
    } else {
        editor.move_node(id, rect.x, rect.y);
        editor.resize_node(id, rect.width, rect.height);
    }
}

fn local_rect(editor: &Editor, id: NodeId, animate: bool) -> Rect {
    if animate {
        let pose = editor.pose(id);
        Rect::new(pose.x, pose.y, pose.width, pose.height)
    } else {
        editor.document().node(id).rect
    }
}

fn shown_rect(editor: &Editor, id: NodeId, animate: bool) -> Rect {
    if !animate {
        return editor.document().absolute_rect(id);
    }
    let pose = editor.pose(id);
    let mut x = pose.x;
    let mut y = pose.y;
    let mut parent = editor.document().node(id).parent;
    while let Some(parent_id) = parent {
        let parent_pose = editor.pose(parent_id);
        x += parent_pose.x;
        y += parent_pose.y;
        parent = editor.document().node(parent_id).parent;
    }
    Rect::new(x, y, pose.width, pose.height)
}

fn hit_node(editor: &Editor, local: [f32; 2], animate: bool) -> Option<NodeId> {
    if !animate {
        return editor.document().hit_test(local[0], local[1]);
    }
    let roots = editor.document().roots().to_vec();
    hit_posed(editor, &roots, local[0], local[1])
}

fn hit_posed(editor: &Editor, ids: &[NodeId], x: f32, y: f32) -> Option<NodeId> {
    for id in ids.iter().rev() {
        let children = editor.document().children(*id).to_vec();
        if let Some(found) = hit_posed(editor, &children, x, y) {
            return Some(found);
        }
        let rect = shown_rect(editor, *id, true);
        let kind = editor.document().node(*id).kind;
        if kind == NodeKind::Ellipse {
            let cx = rect.x + rect.width * 0.5;
            let cy = rect.y + rect.height * 0.5;
            if rect.width > 0.0 && rect.height > 0.0 {
                let dx = (x - cx) / (rect.width * 0.5);
                let dy = (y - cy) / (rect.height * 0.5);
                if dx * dx + dy * dy <= 1.0 {
                    return Some(*id);
                }
            }
        } else if rect.contains(x, y) {
            return Some(*id);
        }
    }
    None
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
    handles(rect).into_iter().find(|(_, point)| {
        (local[0] - point[0]).abs() <= 6.0 && (local[1] - point[1]).abs() <= 6.0
    }).map(|(handle, _)| handle)
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
