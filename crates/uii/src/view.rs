//! The window. A slim toolbar holds the tools, zoom, and undo. Design,
//! Animation, and Function are the tabs, and Design is the one that opens.
//! Layers stay on the left. Properties stay on the right. The canvas is the
//! neutral surface in the middle. Assets and Components live in the
//! collapsible bottom dock. HTML and CSS stay closed until they are opened.

use dear_imgui_rs::{
    Condition, DrawListMut, DrawSegmentCount, Key, MouseButton, StyleColor, StyleVar, Ui,
    WindowFlags,
};
use uii_core::{
    command_for, export_source, import_source, library_file_path, parse_hex_color, read_library,
    resize_from_handle, shortcuts, write_library, AssetKind, Bind, BindKey, Command, DrawerPage,
    Editor, Handle, Motion, NodeId, NodeKind, Rect, WorkspaceTab, MARKUP_STARTS_CLOSED,
};
use uii_function::Session;

pub struct View {
    gesture: Gesture,
    guides: Vec<uii_core::Guide>,
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
    library_name: String,
    library_query: String,
    motion: uii_animation::AnimationTab,
    motion_path: std::path::PathBuf,
    status: String,
    canvas_size: [f32; 2],
    props_id: Option<u32>,
    props_editing: bool,
    prop_x: f32,
    prop_y: f32,
    prop_w: f32,
    prop_h: f32,
    prop_fill: String,
    prop_font: f32,
    prop_radius: f32,
    drawer_hovered: bool,
    functions: Session,
}

#[derive(Clone)]
enum Gesture {
    None,
    Pan {
        last: [f32; 2],
    },
    Move {
        primary: NodeId,
        start: [f32; 2],
        base: Vec<(NodeId, Rect)>,
        moved: bool,
    },
    Resize {
        id: NodeId,
        handle: Handle,
        last: [f32; 2],
    },
    Create {
        kind: NodeKind,
        start: [f32; 2],
    },
    Marquee {
        start: [f32; 2],
        add: bool,
    },
}

impl View {
    pub fn new() -> Self {
        let motion_path = uii_animation::animation_path();
        let mut motion = uii_animation::AnimationTab::new();
        let mut status = String::new();
        match uii_animation::load_animation(&motion_path) {
            Ok(file) => {
                motion.project = file.project;
                if !file.status.is_empty() {
                    status = file.status;
                }
            }
            Err(err) => status = err,
        }
        Self {
            gesture: Gesture::None,
            guides: Vec::new(),
            made: 1,
            markup_open: !MARKUP_STARTS_CLOSED,
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
            library_name: String::new(),
            library_query: String::new(),
            motion,
            motion_path,
            status,
            canvas_size: [800.0, 600.0],
            props_id: None,
            props_editing: false,
            prop_x: 0.0,
            prop_y: 0.0,
            prop_w: 0.0,
            prop_h: 0.0,
            prop_fill: String::new(),
            prop_font: 16.0,
            prop_radius: 0.0,
            drawer_hovered: false,
            functions: Session::open(function_path()),
        }
    }

    pub fn save_function(&mut self) {
        if let Err(err) = self.functions.save_if_changed() {
            self.status = err;
        }
    }

    pub fn load_library(&mut self, editor: &mut Editor) {
        match read_library(&library_file_path()) {
            Ok(Some(library)) => editor.replace_library(library),
            Ok(None) => {}
            Err(err) => self.status = err,
        }
    }

    pub fn save_library(&mut self, editor: &mut Editor) {
        if !editor.library_changed() {
            return;
        }
        match write_library(&library_file_path(), editor.library()) {
            Ok(()) => editor.clear_library_changed(),
            Err(err) => {
                editor.mark_library_changed();
                self.status = err;
            }
        }
    }

    pub fn save_motion(&mut self) {
        if !self.motion.is_dirty() {
            return;
        }
        match uii_animation::save_animation(&self.motion_path, &self.motion.project) {
            Ok(()) => {
                let _ = self.motion.take_dirty();
            }
            Err(err) => self.status = err,
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
                    | WindowFlags::NO_BRING_TO_FRONT_ON_FOCUS
                    | WindowFlags::NO_SCROLLBAR
                    | WindowFlags::NO_SCROLL_WITH_MOUSE,
            )
            .build(|| {
                let chord = ui.io().key_ctrl() && ui.is_key_down(Key::Space);
                editor.poll_drawer_chord(chord, ui.io().want_text_input());
                self.shortcuts(ui, editor);
                self.workspace(ui, editor, dt);
            });
        self.exchange_windows(ui, editor);
    }

    fn shortcuts(&mut self, ui: &Ui, editor: &mut Editor) {
        if ui.io().want_text_input() || editor.tab() == WorkspaceTab::Function {
            return;
        }
        let ctrl = ui.io().key_ctrl();
        let shift = ui.io().key_shift();
        let keys = [
            (Key::Z, BindKey::Z),
            (Key::Y, BindKey::Y),
            (Key::C, BindKey::C),
            (Key::V, BindKey::V),
            (Key::D, BindKey::D),
            (Key::A, BindKey::A),
            (Key::G, BindKey::G),
            (Key::Equal, BindKey::Equal),
            (Key::Minus, BindKey::Minus),
            (Key::Delete, BindKey::Delete),
            (Key::Backspace, BindKey::Backspace),
            (Key::Key1, BindKey::Digit1),
        ];
        for (key, bind) in keys {
            if ui.is_key_pressed(key) {
                if let Some(command) = command_for(Bind {
                    ctrl,
                    shift,
                    key: bind,
                }) {
                    if editor.tab() == WorkspaceTab::Animation
                        && matches!(command, Command::Undo | Command::Redo | Command::Delete)
                    {
                        continue;
                    }
                    editor.apply_command(command, self.canvas_size);
                }
            }
        }
    }

    fn workspace(&mut self, ui: &Ui, editor: &mut Editor, dt: f32) {
        self.tab_row(ui, editor);
        hairline(ui, "rule-tabs");
        match editor.tab() {
            WorkspaceTab::Design => self.workspace_body(ui, editor, dt, false),
            WorkspaceTab::Animation => self.workspace_body(ui, editor, dt, true),
            WorkspaceTab::Function => self.function_body(ui, editor),
        }
    }

    fn tab_row(&mut self, ui: &Ui, editor: &mut Editor) {
        let width = ui.content_region_avail()[0];
        let bg = ui.push_style_color(StyleColor::ChildBg, crate::theme::CHROME);
        let pad = ui.push_style_var(StyleVar::WindowPadding([crate::theme::INSET, 5.0]));
        ui.child_window("tabs")
            .size([width, crate::theme::TAB_H])
            .border(false)
            .flags(WindowFlags::NO_SCROLLBAR)
            .build(ui, || {
                for tab in Editor::tabs() {
                    let on = editor.tab() == tab;
                    if tool_button(ui, &format!("{}##tab", tab.label()), on) {
                        editor.set_tab(tab);
                    }
                    ui.same_line();
                }
            });
        pad.pop();
        bg.pop();
    }

    fn function_body(&mut self, ui: &Ui, editor: &mut Editor) {
        let avail = ui.content_region_avail();
        let body_h = (avail[1] - self.dock_reserve(editor) - self.status_reserve()).max(120.0);
        let bg = ui.push_style_color(StyleColor::ChildBg, crate::theme::PANEL);
        let pad = ui.push_style_var(StyleVar::WindowPadding([
            crate::theme::INSET,
            crate::theme::INSET,
        ]));
        ui.child_window("function-body")
            .size([avail[0], body_h])
            .border(false)
            .build(ui, || {
                if uii_function::draw_function_panel(ui, &mut self.functions) {
                    if let Err(err) = self.functions.save() {
                        self.status = err;
                    }
                }
            });
        pad.pop();
        bg.pop();
        self.dock_and_status(ui, editor);
    }

    fn workspace_body(&mut self, ui: &Ui, editor: &mut Editor, dt: f32, animate: bool) {
        if animate && editor.playing() {
            editor.tick(dt);
        }
        self.toolbar(ui, editor);
        hairline(ui, "rule-tools");
        if editor.tool() == uii_core::Tool::Inspect {
            self.inspect_strip(ui, editor);
        }
        if self.markup_open {
            self.markup_panel(ui, editor);
        }

        let avail = ui.content_region_avail();
        let timeline = if animate {
            crate::theme::TIMELINE_H
        } else {
            0.0
        };
        let body_h =
            (avail[1] - self.dock_reserve(editor) - timeline - self.status_reserve()).max(120.0);
        let columns = ui.push_style_color(StyleColor::ChildBg, crate::theme::DIVIDER);
        let column_pad = ui.push_style_var(StyleVar::WindowPadding([0.0, 0.0]));
        let column_gap = ui.push_style_var(StyleVar::ItemSpacing([1.0, 0.0]));
        ui.child_window("workspace-body")
            .size([avail[0], body_h])
            .border(false)
            .flags(WindowFlags::NO_SCROLLBAR)
            .build(ui, || {
                let inner = ui.content_region_avail();
                let layers_w = crate::theme::LAYERS_W;
                let props_w = crate::theme::PROPS_W;
                side_column(ui, "layers", [layers_w, inner[1]], || {
                    ui.text("Layers");
                    let roots = editor.document().roots().to_vec();
                    if roots.is_empty() {
                        ui.text_disabled("No layers yet");
                    }
                    self.layers(ui, editor, &roots, 0);
                });
                ui.same_line();
                let canvas_w = (inner[0] - layers_w - props_w - 2.0).max(80.0);
                let canvas_bg = ui.push_style_color(StyleColor::ChildBg, crate::theme::CANVAS);
                let canvas_pad = ui.push_style_var(StyleVar::WindowPadding([0.0, 0.0]));
                ui.child_window("canvas")
                    .size([canvas_w, inner[1]])
                    .border(false)
                    .flags(WindowFlags::NO_SCROLLBAR)
                    .build(ui, || {
                        self.canvas(ui, editor, animate);
                    });
                canvas_pad.pop();
                canvas_bg.pop();
                ui.same_line();
                side_column(ui, "properties", [props_w, inner[1]], || {
                    self.properties(ui, editor);
                    ui.separator();
                    self.shortcut_list(ui, editor);
                });
            });
        column_gap.pop();
        column_pad.pop();
        columns.pop();

        if animate {
            let host_bg = ui.push_style_color(StyleColor::ChildBg, crate::theme::PANEL);
            let host_pad = ui.push_style_var(StyleVar::WindowPadding([crate::theme::INSET, 8.0]));
            ui.child_window("animation-host")
                .size([ui.content_region_avail()[0], crate::theme::TIMELINE_H])
                .border(false)
                .build(ui, || {
                    self.animation_host(ui, editor, dt);
                });
            host_pad.pop();
            host_bg.pop();
        }
        self.dock_and_status(ui, editor);
    }

    fn dock_reserve(&self, editor: &Editor) -> f32 {
        if editor.drawer_collapsed() {
            crate::theme::DOCK_BAR
        } else {
            crate::theme::DOCK_BAR + crate::theme::DOCK_BODY
        }
    }

    fn status_reserve(&self) -> f32 {
        if self.status.is_empty() {
            0.0
        } else {
            22.0
        }
    }

    fn dock_and_status(&mut self, ui: &Ui, editor: &mut Editor) {
        self.drawer(ui, editor);
        if ui.is_mouse_clicked(MouseButton::Left) && !self.drawer_hovered {
            editor.dismiss_unpinned_drawer();
        }
        if !self.status.is_empty() {
            ui.text_colored(crate::theme::MUTED, &self.status);
        }
    }

    fn toolbar(&mut self, ui: &Ui, editor: &mut Editor) {
        let width = ui.content_region_avail()[0];
        let bg = ui.push_style_color(StyleColor::ChildBg, crate::theme::CHROME);
        let pad = ui.push_style_var(StyleVar::WindowPadding([crate::theme::INSET, 7.0]));
        ui.child_window("toolbar")
            .size([width, crate::theme::TOOLBAR_H])
            .border(false)
            .flags(WindowFlags::NO_SCROLLBAR)
            .build(ui, || {
                let tools = Editor::tools();
                for (index, tool) in tools.iter().enumerate() {
                    if index > 0 {
                        ui.same_line();
                    }
                    let label = format!("{}##tool{index}", tool.label());
                    if tool_button(ui, &label, editor.tool() == *tool) {
                        editor.set_tool(*tool);
                    }
                }
                ui.same_line();
                if tool_button(ui, "Delete", false) {
                    editor.delete_selected();
                }
                ui.same_line();
                if tool_button(ui, "Import", false) {
                    self.import_open = true;
                }
                ui.same_line();
                if tool_button(ui, "Export", false) {
                    self.export_open = true;
                    self.refresh_export(editor);
                }
                ui.same_line();
                let markup = if self.markup_open { "HTML on" } else { "HTML" };
                if tool_button(ui, markup, self.markup_open) {
                    self.markup_open = !self.markup_open;
                }

                ui.same_line();
                let cluster = ui.window_size()[0] - 348.0;
                if cluster > ui.cursor_pos()[0] + 8.0 {
                    ui.set_cursor_pos_x(cluster);
                }
                if tool_button(ui, "-##zoom-out", false) {
                    editor.zoom_out();
                }
                ui.same_line();
                ui.align_text_to_frame_padding();
                ui.text(format!("{:.0}%", editor.zoom() * 100.0));
                ui.same_line();
                if tool_button(ui, "+##zoom-in", false) {
                    editor.zoom_in();
                }
                ui.same_line();
                if tool_button(ui, "Fit", false) {
                    editor.fit(self.canvas_size[0], self.canvas_size[1]);
                }
                ui.same_line();
                if tool_button(ui, "Undo", false) {
                    editor.undo();
                }
                ui.same_line();
                if tool_button(ui, "Redo", false) {
                    editor.redo();
                }
            });
        pad.pop();
        bg.pop();
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

    fn properties(&mut self, ui: &Ui, editor: &mut Editor) {
        ui.text("Properties");
        let count = editor.selection().len();
        if count > 1 {
            ui.text_disabled(format!("{count} selected"));
        }
        let id = editor.selected();
        if !self.props_editing || self.props_id != id.map(|id| id.0) {
            self.props_id = id.map(|id| id.0);
            if let Some(props) = editor.properties() {
                self.prop_x = props.x;
                self.prop_y = props.y;
                self.prop_w = props.width;
                self.prop_h = props.height;
                self.prop_fill = props.fill;
                self.prop_font = props.font_size.unwrap_or(16.0);
                self.prop_radius = props.corner_radius.unwrap_or(0.0);
            }
        }
        self.props_editing = false;
        let Some(id) = id else {
            ui.text_disabled("Nothing selected");
            return;
        };
        let kind = editor.document().node(id).kind;
        ui.text_disabled("Type");
        ui.same_line();
        ui.text(format!("{kind}"));
        section_label(ui, "Position");
        let (x_changed, y_changed) = float_pair(
            ui,
            "X",
            "##x",
            &mut self.prop_x,
            "Y",
            "##y",
            &mut self.prop_y,
            &mut self.props_editing,
        );
        if x_changed {
            editor.move_node(id, self.prop_x, editor.document().node(id).rect.y);
            editor.finish_edit();
            self.props_editing = true;
        }
        if y_changed {
            editor.move_node(id, editor.document().node(id).rect.x, self.prop_y);
            editor.finish_edit();
            self.props_editing = true;
        }
        section_label(ui, "Size");
        let (w_changed, h_changed) = float_pair(
            ui,
            "W",
            "##w",
            &mut self.prop_w,
            "H",
            "##h",
            &mut self.prop_h,
            &mut self.props_editing,
        );
        if w_changed {
            editor.resize_node(id, self.prop_w, editor.document().node(id).rect.height);
            editor.finish_edit();
            self.props_editing = true;
        }
        if h_changed {
            editor.resize_node(id, editor.document().node(id).rect.width, self.prop_h);
            editor.finish_edit();
            self.props_editing = true;
        }
        section_label(ui, "Fill");
        let swatch = editor.document().node(id).fill;
        ui.color_button("##fill", color(swatch));
        ui.same_line();
        ui.set_next_item_width(ui.content_region_avail()[0]);
        if ui.input_text("##fill-hex", &mut self.prop_fill).build() {
            if let Some(parsed) = parse_hex_color(&self.prop_fill) {
                editor.set_fill(id, parsed);
                editor.finish_edit();
            }
            self.props_editing = true;
        }
        self.note_active(ui);
        if matches!(kind, NodeKind::Text | NodeKind::Button) {
            section_label(ui, "Font size");
            if labeled_float(ui, "##font", &mut self.prop_font, &mut self.props_editing) {
                editor.set_font_size(id, self.prop_font);
                editor.finish_edit();
                self.props_editing = true;
            }
        }
        if matches!(
            kind,
            NodeKind::Rectangle | NodeKind::Frame | NodeKind::Child | NodeKind::Dock
        ) {
            section_label(ui, "Corner radius");
            if labeled_float(
                ui,
                "##radius",
                &mut self.prop_radius,
                &mut self.props_editing,
            ) {
                editor.set_corner_radius(id, self.prop_radius);
                editor.finish_edit();
                self.props_editing = true;
            }
        }
    }

    fn note_active(&mut self, ui: &Ui) {
        if ui.is_item_active() {
            self.props_editing = true;
        }
    }

    fn shortcut_list(&mut self, ui: &Ui, editor: &Editor) {
        section_label(ui, "Shortcuts");
        for shortcut in shortcuts() {
            ui.text(shortcut.label);
            let keys_w = ui.calc_text_size(shortcut.keys)[0];
            ui.same_line();
            let edge = ui.window_size()[0] - keys_w - crate::theme::INSET;
            if edge > ui.cursor_pos()[0] {
                ui.set_cursor_pos_x(edge);
            }
            ui.text_disabled(shortcut.keys);
        }
        ui.text_disabled(format!("Zoom {:.0}%", editor.zoom() * 100.0));
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

    fn layers(&mut self, ui: &Ui, editor: &mut Editor, ids: &[NodeId], depth: usize) {
        for id in ids {
            let (label, children) = {
                let node = editor.document().node(*id);
                (
                    format!("{}  {}##layer{}", node.kind, node.name, id.0),
                    node.children.clone(),
                )
            };
            let selected = editor.selection().contains(id);
            let pad = "  ".repeat(depth);
            let text = format!("{pad}{label}");
            if ui
                .selectable_config(&text)
                .selected(selected)
                .size([0.0, crate::theme::ROW_H])
                .build()
            {
                if ui.io().key_shift() {
                    editor.toggle_in_selection(*id);
                } else {
                    editor.select(*id);
                }
            }
            if !children.is_empty() {
                self.layers(ui, editor, &children, depth + 1);
            }
        }
    }

    fn canvas(&mut self, ui: &Ui, editor: &mut Editor, animate: bool) {
        let size = ui.content_region_avail();
        self.canvas_size = size;
        {
            let origin = ui.window_pos();
            let extent = ui.window_size();
            let backdrop = ui.get_window_draw_list();
            backdrop
                .add_rect(
                    origin,
                    [origin[0] + extent[0], origin[1] + extent[1]],
                    crate::theme::CANVAS,
                )
                .filled(true)
                .build();
        }
        let origin = ui.get_cursor_screen_pos();
        ui.invisible_button("canvas-hit", size);
        let hovered = ui.is_item_hovered();
        let mouse = ui.mouse_pos();
        let zoom = editor.zoom().max(0.05);
        let pan = editor.pan();
        let local = doc_point(origin, pan, zoom, mouse);
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
                let zoom = editor.zoom().max(0.05);
                let mut pan = editor.pan();
                pan[0] -= (mouse[0] - last[0]) / zoom;
                pan[1] -= (mouse[1] - last[1]) / zoom;
                editor.set_pan(pan);
            }
            self.gesture = Gesture::Pan { last: mouse };
            return;
        }
        if matches!(self.gesture, Gesture::Pan { .. }) {
            self.gesture = Gesture::None;
        }

        if hovered && ui.is_mouse_clicked(MouseButton::Left) {
            self.begin_press(ui, editor, local, animate);
        }

        if ui.is_mouse_down(MouseButton::Left) {
            self.drag(editor, local, animate);
        } else if !matches!(self.gesture, Gesture::None) {
            self.finish(editor, local);
        }
    }

    fn begin_press(&mut self, ui: &Ui, editor: &mut Editor, local: [f32; 2], animate: bool) {
        let tool = editor.tool();
        if let Some(kind) = tool.kind() {
            self.gesture = Gesture::Create { kind, start: local };
            return;
        }
        let shift = ui.io().key_shift();
        if let Some(id) = editor.selected() {
            let rect = shown_rect(editor, id, animate);
            if let Some(handle) = hit_handle(rect, local, editor.zoom()) {
                self.gesture = Gesture::Resize {
                    id,
                    handle,
                    last: local,
                };
                return;
            }
        }
        if let Some(id) = hit_node(editor, local, animate) {
            if shift {
                editor.toggle_in_selection(id);
                self.gesture = Gesture::None;
                return;
            }
            if !editor.selection().contains(&id) {
                editor.select(id);
            }
            if tool == uii_core::Tool::Select && !animate {
                let base = selection_base(editor);
                let primary = if base.iter().any(|(item, _)| *item == id) {
                    id
                } else {
                    base.first().map(|(item, _)| *item).unwrap_or(id)
                };
                self.gesture = Gesture::Move {
                    primary,
                    start: local,
                    base,
                    moved: false,
                };
            } else if tool == uii_core::Tool::Select {
                self.gesture = Gesture::Move {
                    primary: id,
                    start: local,
                    base: vec![(id, local_rect(editor, id, true))],
                    moved: false,
                };
            } else {
                self.gesture = Gesture::None;
            }
        } else if tool == uii_core::Tool::Select && !shift {
            self.gesture = Gesture::Marquee {
                start: local,
                add: false,
            };
        } else if tool == uii_core::Tool::Select && shift {
            self.gesture = Gesture::Marquee {
                start: local,
                add: true,
            };
        } else {
            editor.clear_selection();
            self.gesture = Gesture::None;
        }
    }

    fn drag(&mut self, editor: &mut Editor, local: [f32; 2], animate: bool) {
        match self.gesture.clone() {
            Gesture::Move {
                primary,
                start,
                base,
                moved,
            } => {
                let dx = local[0] - start[0];
                let dy = local[1] - start[1];
                if !moved && dx.abs() + dy.abs() < 1.0 {
                    return;
                }
                if animate {
                    let current = local_rect(editor, primary, true);
                    apply_rect(
                        editor,
                        primary,
                        Rect::new(
                            current.x + dx,
                            current.y + dy,
                            current.width,
                            current.height,
                        ),
                        true,
                    );
                    self.gesture = Gesture::Move {
                        primary,
                        start: local,
                        base,
                        moved: true,
                    };
                } else {
                    self.guides = editor.place_moved(primary, &base, dx, dy);
                    self.gesture = Gesture::Move {
                        primary,
                        start,
                        base,
                        moved: true,
                    };
                }
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
        match self.gesture.clone() {
            Gesture::Create { kind, start } => {
                let rect = drag_rect(start, local);
                let rect = if rect.width < 4.0 || rect.height < 4.0 {
                    Rect::new(
                        start[0],
                        start[1],
                        default_size(kind).0,
                        default_size(kind).1,
                    )
                } else {
                    rect
                };
                let name = format!("{} {}", kind_name(kind), self.made);
                self.made += 1;
                editor.add_shape(kind, &name, rect);
            }
            Gesture::Marquee { start, add } => {
                let rect = drag_rect(start, local);
                let hits = editor.document().intersects_nodes(rect);
                if add {
                    for id in hits {
                        editor.add_to_selection(id);
                    }
                } else if hits.is_empty() {
                    editor.clear_selection();
                } else {
                    editor.select_ids(&hits);
                }
            }
            Gesture::Move { moved, .. } => {
                if moved && editor.tab() != WorkspaceTab::Animation {
                    editor.drop_selection();
                }
                editor.finish_edit();
                self.guides.clear();
            }
            Gesture::Resize { .. } => editor.finish_edit(),
            _ => {}
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
        let font = ui.current_font();
        self.paint_nodes(editor, draw, &roots, origin, animate, font);
        let zoom = editor.zoom().max(0.05);
        let pan = editor.pan();
        if let Gesture::Create { start, .. } | Gesture::Marquee { start, .. } = self.gesture {
            let mouse = ui.mouse_pos();
            let local = doc_point(origin, pan, zoom, mouse);
            let rect = drag_rect(start, local);
            let a = screen_point(origin, pan, zoom, rect.x, rect.y);
            let b = screen_point(origin, pan, zoom, rect.x + rect.width, rect.y + rect.height);
            draw.add_rect(a, b, crate::theme::ACCENT)
                .thickness(1.5)
                .build();
        }
        for guide in &self.guides {
            if guide.vertical {
                let x = screen_point(origin, pan, zoom, guide.at, 0.0)[0];
                draw.add_line(
                    [x, origin[1]],
                    [x, origin[1] + 4000.0],
                    crate::theme::ACCENT,
                )
                .build();
            } else {
                let y = screen_point(origin, pan, zoom, 0.0, guide.at)[1];
                draw.add_line(
                    [origin[0], y],
                    [origin[0] + 4000.0, y],
                    crate::theme::ACCENT,
                )
                .build();
            }
        }
    }

    fn paint_nodes(
        &self,
        editor: &Editor,
        draw: &DrawListMut<'_>,
        ids: &[NodeId],
        origin: [f32; 2],
        animate: bool,
        font: dear_imgui_rs::FontId,
    ) {
        let zoom = editor.zoom().max(0.05);
        let pan = editor.pan();
        for id in ids {
            let (kind, fill_rgba, ink_rgba, label, children, font_size, radius) = {
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
                    node.font_size.max(1.0),
                    node.corner_radius.max(0.0),
                )
            };
            let rect = shown_rect(editor, *id, animate);
            let min = screen_point(origin, pan, zoom, rect.x, rect.y);
            let max = screen_point(origin, pan, zoom, rect.x + rect.width, rect.y + rect.height);
            let fill = color(fill_rgba);
            let ink = color(ink_rgba);
            let screen_radius = radius * zoom;
            if kind == NodeKind::Ellipse {
                if rect.width > 0.0 && rect.height > 0.0 {
                    draw.add_ellipse_filled(
                        [(min[0] + max[0]) * 0.5, (min[1] + max[1]) * 0.5],
                        [rect.width * 0.5 * zoom, rect.height * 0.5 * zoom],
                        fill,
                        0.0,
                        DrawSegmentCount::new(32).unwrap_or(DrawSegmentCount::AUTO),
                    );
                }
            } else if fill_rgba.a > 0 {
                draw.add_rect(min, max, fill)
                    .rounding(screen_radius)
                    .filled(true)
                    .build();
            }
            draw.add_rect(min, max, ink).rounding(screen_radius).build();
            if !label.is_empty() && matches!(kind, NodeKind::Text | NodeKind::Button) {
                draw.add_text_with_font(
                    font,
                    font_size * zoom,
                    [min[0] + 6.0, min[1] + 4.0],
                    ink,
                    &label,
                    (rect.width * zoom - 8.0).max(0.0),
                    None,
                );
            } else if !label.is_empty() {
                draw.add_text([min[0] + 6.0, min[1] + 4.0], ink, &label);
            }
            let paint_children = |view: &View| {
                if !children.is_empty() {
                    view.paint_nodes(editor, draw, &children, origin, animate, font);
                }
            };
            if kind == NodeKind::Frame {
                let _clip = draw.push_clip_rect(min, max, true);
                paint_children(self);
            } else {
                paint_children(self);
            }
            if editor.selection().contains(id) {
                draw.add_rect(
                    [min[0] - 1.0, min[1] - 1.0],
                    [max[0] + 1.0, max[1] + 1.0],
                    crate::theme::ACCENT,
                )
                .thickness(2.0)
                .build();
                for (_, point) in handles(rect) {
                    let at = screen_point(origin, pan, zoom, point[0], point[1]);
                    let half = 4.0;
                    draw.add_rect_filled_multicolor(
                        [at[0] - half, at[1] - half],
                        [at[0] + half, at[1] + half],
                        crate::theme::WHITE,
                        crate::theme::WHITE,
                        crate::theme::WHITE,
                        crate::theme::WHITE,
                    );
                    draw.add_rect(
                        [at[0] - half, at[1] - half],
                        [at[0] + half, at[1] + half],
                        crate::theme::ACCENT,
                    )
                    .thickness(1.5)
                    .build();
                }
            }
        }
    }

    fn drawer(&mut self, ui: &Ui, editor: &mut Editor) {
        let cursor = ui.cursor_screen_pos();
        let width = ui.content_region_avail()[0];
        let collapsed = editor.drawer_collapsed();
        let pinned = editor.drawer_pinned();
        let bg = ui.push_style_color(StyleColor::ChildBg, crate::theme::CHROME);
        let pad = ui.push_style_var(StyleVar::WindowPadding([crate::theme::INSET, 5.0]));
        ui.child_window("dock-bar")
            .size([width, crate::theme::DOCK_BAR])
            .border(false)
            .flags(WindowFlags::NO_SCROLLBAR)
            .build(ui, || {
                let toggle = if collapsed { "Dock  >" } else { "Dock  v" };
                if tool_button(ui, toggle, !collapsed) {
                    editor.toggle_drawer();
                }
                ui.same_line();
                let pin = if pinned { "Pinned" } else { "Pin" };
                if tool_button(ui, pin, pinned) {
                    editor.set_drawer_pinned(!pinned);
                }
                for page in Editor::drawer_pages() {
                    ui.same_line();
                    let current = editor.drawer_page() == page && !collapsed;
                    if tool_button(ui, &format!("{}##dock", page.label()), current) {
                        editor.show_drawer(page);
                    }
                }
            });
        pad.pop();
        bg.pop();
        if editor.drawer_collapsed() {
            let after = ui.cursor_screen_pos();
            self.drawer_hovered = point_in(ui.mouse_pos(), cursor, cursor[0] + width, after[1]);
            return;
        }
        let body_bg = ui.push_style_color(StyleColor::ChildBg, crate::theme::PANEL);
        let body_pad = ui.push_style_var(StyleVar::WindowPadding([crate::theme::INSET, 8.0]));
        ui.child_window("dock-body")
            .size([width, crate::theme::DOCK_BODY])
            .border(false)
            .build(ui, || match editor.drawer_page() {
                DrawerPage::Assets => self.assets(ui, editor),
                DrawerPage::Components => self.components(ui, editor),
            });
        body_pad.pop();
        body_bg.pop();
        let end = ui.cursor_screen_pos();
        self.drawer_hovered = point_in(ui.mouse_pos(), cursor, cursor[0] + width, end[1]);
    }

    fn assets(&mut self, ui: &Ui, editor: &mut Editor) {
        if ui.input_text("Find", &mut self.library_query).build() {
            editor.set_library_query(&self.library_query);
        }
        ui.input_text("Folder or asset", &mut self.library_name)
            .build();
        if ui.button("New folder") {
            if let Err(err) = editor.create_named_folder(&self.library_name) {
                self.status = err.to_string();
            }
        }
        ui.same_line();
        if ui.button("New asset") {
            if let Err(err) = editor.create_named_asset(&self.library_name) {
                self.status = err.to_string();
            }
        }
        ui.same_line();
        if ui.button("Remove") {
            if let Err(err) = editor.delete_library_selection() {
                self.status = err.to_string();
            }
        }
        let rows = editor.library_rows();
        if rows.is_empty() {
            ui.text("No folders yet. Create a folder, then an asset inside it.");
        }
        for row in rows {
            let indent = "  ".repeat(row.depth as usize);
            let mark = if row.kind == AssetKind::Folder {
                "folder"
            } else {
                "asset"
            };
            if ui.button(format!("{indent}{mark} {}##lib{}", row.name, row.id.0)) {
                editor.activate_library_row(row.id);
            }
            if row.kind == AssetKind::Asset {
                ui.same_line();
                if ui.button(format!("Place##lib{}", row.id.0)) {
                    editor.place_asset(&row.name);
                }
            }
        }
        ui.separator();
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
    }

    fn animation_host(&mut self, ui: &Ui, editor: &mut Editor, dt: f32) {
        self.sync_motion_targets(editor);
        self.motion.tick(dt);
        uii_animation::draw_animation_tab(ui, &mut self.motion);
    }

    fn sync_motion_targets(&mut self, editor: &Editor) {
        let mut nodes = Vec::new();
        for node in editor.document().nodes() {
            let rect = editor.document().absolute_rect(node.id);
            nodes.push(uii_animation::LayoutNode {
                id: uii_animation::NodeId(node.id.0),
                name: node.name.clone(),
                x: rect.x,
                y: rect.y,
                width: rect.width,
                height: rect.height,
                rotation_deg: 0.0,
                opacity: node.fill.a as f32 / 255.0,
                fill: [
                    node.fill.r as f32 / 255.0,
                    node.fill.g as f32 / 255.0,
                    node.fill.b as f32 / 255.0,
                    node.fill.a as f32 / 255.0,
                ],
            });
        }
        self.motion.project.sync_targets(&nodes);
    }

    fn components(&mut self, ui: &Ui, editor: &mut Editor) {
        ui.input_text("Component", &mut self.component_name).build();
        ui.same_line();
        if ui.button("Save selection") {
            editor.save_component(&self.component_name);
        }
        let mut names = editor.library_component_names();
        for component in editor.components() {
            if !names.iter().any(|name| name == &component.name) {
                names.push(component.name.clone());
            }
        }
        for (index, name) in names.iter().enumerate() {
            let saved = editor
                .components()
                .iter()
                .any(|component| component.name == *name);
            if saved && ui.button(format!("Insert##component{index}")) {
                editor.insert_component(name);
            }
            if saved {
                ui.same_line();
            }
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
                        ui.input_text_multiline(
                            "##import-html",
                            &mut self.import_html,
                            [490.0, 120.0],
                        )
                        .build();
                        ui.text("CSS");
                        ui.input_text_multiline(
                            "##import-css",
                            &mut self.import_css,
                            [490.0, 80.0],
                        )
                        .build();
                        if ui.button("Import into canvas") {
                            editor.apply_markup(&self.import_html, &self.import_css);
                            self.markup_dirty = false;
                            self.status = "Imported HTML and CSS.".into();
                        }
                    } else {
                        ui.input_text_multiline(
                            "##import-source",
                            &mut self.import_source,
                            [490.0, 220.0],
                        )
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

fn function_path() -> std::path::PathBuf {
    let data = std::env::var_os("XDG_DATA_HOME").map(std::path::PathBuf::from);
    let home = std::env::var_os("HOME").map(std::path::PathBuf::from);
    uii_function::default_graph_path(data.as_deref(), home.as_deref())
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

fn selection_base(editor: &Editor) -> Vec<(NodeId, Rect)> {
    editor
        .selection()
        .iter()
        .copied()
        .filter(|id| editor.document().get(*id).is_some())
        .filter(|id| {
            !editor.selection().iter().any(|other| {
                *other != *id
                    && editor.document().get(*other).is_some()
                    && editor.document().is_descendant(*id, *other)
            })
        })
        .map(|id| (id, editor.document().node(id).rect))
        .collect()
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
        let node = editor.document().node(*id);
        let rect = shown_rect(editor, *id, true);
        if node.kind == NodeKind::Frame && !rect.contains(x, y) {
            continue;
        }
        let children = editor.document().children(*id).to_vec();
        if let Some(found) = hit_posed(editor, &children, x, y) {
            return Some(found);
        }
        if node.kind == NodeKind::Ellipse {
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

fn hit_handle(rect: Rect, local: [f32; 2], zoom: f32) -> Option<Handle> {
    let slop = 6.0 / zoom.max(0.05);
    handles(rect)
        .into_iter()
        .find(|(_, point)| {
            (local[0] - point[0]).abs() <= slop && (local[1] - point[1]).abs() <= slop
        })
        .map(|(handle, _)| handle)
}

fn drag_rect(start: [f32; 2], end: [f32; 2]) -> Rect {
    let x = start[0].min(end[0]);
    let y = start[1].min(end[1]);
    Rect::new(x, y, (end[0] - start[0]).abs(), (end[1] - start[1]).abs())
}

fn side_column(ui: &Ui, name: &str, size: [f32; 2], body: impl FnOnce()) {
    let bg = ui.push_style_color(StyleColor::ChildBg, crate::theme::PANEL);
    let pad = ui.push_style_var(StyleVar::WindowPadding([
        crate::theme::INSET,
        crate::theme::INSET,
    ]));
    let gap = ui.push_style_var(StyleVar::ItemSpacing([8.0, 6.0]));
    ui.child_window(name)
        .size(size)
        .border(false)
        .build(ui, body);
    gap.pop();
    pad.pop();
    bg.pop();
}

fn hairline(ui: &Ui, id: &str) {
    let width = ui.content_region_avail()[0];
    let color = ui.push_style_color(StyleColor::ChildBg, crate::theme::DIVIDER);
    let pad = ui.push_style_var(StyleVar::WindowPadding([0.0, 0.0]));
    ui.child_window(id)
        .size([width, 1.0])
        .border(false)
        .flags(WindowFlags::NO_SCROLLBAR)
        .build(ui, || {});
    pad.pop();
    color.pop();
}

fn section_label(ui: &Ui, title: &str) {
    ui.dummy([0.0, 2.0]);
    ui.text_disabled(title);
}

fn float_pair(
    ui: &Ui,
    left_label: &str,
    left_id: &str,
    left: &mut f32,
    right_label: &str,
    right_id: &str,
    right: &mut f32,
    editing: &mut bool,
) -> (bool, bool) {
    let field = ((ui.content_region_avail()[0] - 52.0) * 0.5).max(36.0);
    ui.align_text_to_frame_padding();
    ui.text(left_label);
    ui.same_line();
    ui.set_next_item_width(field);
    let left_changed = ui.input_float(left_id, left);
    if ui.is_item_active() {
        *editing = true;
    }
    ui.same_line();
    ui.align_text_to_frame_padding();
    ui.text(right_label);
    ui.same_line();
    ui.set_next_item_width(field);
    let right_changed = ui.input_float(right_id, right);
    if ui.is_item_active() {
        *editing = true;
    }
    (left_changed, right_changed)
}

fn labeled_float(ui: &Ui, id: &str, value: &mut f32, editing: &mut bool) -> bool {
    ui.set_next_item_width(ui.content_region_avail()[0]);
    let changed = ui.input_float(id, value);
    if ui.is_item_active() {
        *editing = true;
    }
    changed
}

fn tool_button(ui: &Ui, label: &str, active: bool) -> bool {
    let fill = if active {
        crate::theme::ACCENT
    } else {
        crate::theme::TOOL
    };
    let hover = if active {
        crate::theme::ACCENT_HOVER
    } else {
        crate::theme::HOVER
    };
    let text = if active {
        crate::theme::WHITE
    } else {
        crate::theme::TEXT
    };
    let button = ui.push_style_color(StyleColor::Button, fill);
    let hovered = ui.push_style_color(StyleColor::ButtonHovered, hover);
    let pressed = ui.push_style_color(StyleColor::ButtonActive, crate::theme::ACCENT);
    let ink = ui.push_style_color(StyleColor::Text, text);
    let hit = ui.button(label);
    ink.pop();
    pressed.pop();
    hovered.pop();
    button.pop();
    hit
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

fn doc_point(origin: [f32; 2], pan: [f32; 2], zoom: f32, mouse: [f32; 2]) -> [f32; 2] {
    [
        (mouse[0] - origin[0]) / zoom + pan[0],
        (mouse[1] - origin[1]) / zoom + pan[1],
    ]
}

fn screen_point(origin: [f32; 2], pan: [f32; 2], zoom: f32, x: f32, y: f32) -> [f32; 2] {
    [
        origin[0] + (x - pan[0]) * zoom,
        origin[1] + (y - pan[1]) * zoom,
    ]
}

fn point_in(point: [f32; 2], origin: [f32; 2], max_x: f32, max_y: f32) -> bool {
    point[0] >= origin[0] && point[1] >= origin[1] && point[0] <= max_x && point[1] <= max_y
}

fn color(value: uii_core::Rgba) -> [f32; 4] {
    [
        value.r as f32 / 255.0,
        value.g as f32 / 255.0,
        value.b as f32 / 255.0,
        value.a as f32 / 255.0,
    ]
}
