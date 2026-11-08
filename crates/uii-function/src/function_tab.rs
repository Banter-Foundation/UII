//! Function tab. Where a behavior lives, how it works, and the wires between
//! functions. Design, Animation, the assets dock, Inspect, import/export, and
//! the Cursor plugin are other parts of UII and are not drawn here.

use std::path::PathBuf;

use dear_imgui_rs::{Color, Condition, Key, MouseButton, Ui, WindowFlags};

use crate::{to_text, Button, Direction, Editor, FunctionId, FunctionKind, Graph, PORT_RADIUS};

pub struct Session {
    pub editor: Editor,
    pub path: PathBuf,
    pub notice: String,
    pub(crate) baseline: String,
    pub(crate) protect_file: bool,
    styled: bool,
}

impl Session {
    pub fn open(path: PathBuf) -> Self {
        match std::fs::read_to_string(&path) {
            Ok(text) => match crate::from_text(&text) {
                Ok(graph) => Self {
                    editor: Editor::with_graph(graph),
                    path,
                    notice: String::new(),
                    baseline: text,
                    protect_file: false,
                    styled: false,
                },
                Err(err) => Self {
                    editor: Editor::new(),
                    path,
                    notice: format!("Could not read the function graph: {err}"),
                    baseline: String::new(),
                    protect_file: true,
                    styled: false,
                },
            },
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => Self {
                editor: Editor::new(),
                path,
                notice: String::new(),
                baseline: to_text(&Graph::new()),
                protect_file: false,
                styled: false,
            },
            Err(err) => Self {
                editor: Editor::new(),
                path,
                notice: format!("Could not read the function graph: {err}"),
                baseline: String::new(),
                protect_file: true,
                styled: false,
            },
        }
    }

    pub fn save(&mut self) -> Result<(), String> {
        if let Some(parent) = self.path.parent() {
            if !parent.as_os_str().is_empty() {
                std::fs::create_dir_all(parent).map_err(|err| err.to_string())?;
            }
        }
        let text = to_text(self.editor.graph());
        std::fs::write(&self.path, text.as_bytes()).map_err(|err| err.to_string())?;
        self.baseline = text;
        self.protect_file = false;
        self.notice = format!("Saved {}", self.path.display());
        Ok(())
    }

    pub fn save_if_changed(&mut self) -> Result<(), String> {
        if self.protect_file {
            return Ok(());
        }
        let text = to_text(self.editor.graph());
        if text == self.baseline {
            return Ok(());
        }
        self.save()
    }
}

/// Draw the graph inside the shell's Function tab. The caller owns the window.
pub fn draw_function_panel(ui: &Ui, session: &mut Session) -> bool {
    session.editor.retain_selection();
    draw_function(ui, session)
}

pub fn draw_function_tab(ui: &Ui, session: &mut Session) -> bool {
    if !session.styled {
        ui.style_colors_dark();
        session.styled = true;
    }
    session.editor.retain_selection();
    let mut save = false;
    let display = ui.io().display_size();
    ui.window("UII")
        .flags(
            WindowFlags::NO_DECORATION
                | WindowFlags::NO_MOVE
                | WindowFlags::NO_BRING_TO_FRONT_ON_FOCUS,
        )
        .position([0.0, 0.0], Condition::Always)
        .size(display, Condition::Always)
        .build(|| {
            if let Some(_bar) = ui.tab_bar("uii-top") {
                if let Some(_tab) = ui.tab_item("Function") {
                    save = draw_function(ui, session);
                }
            }
        });
    save
}

fn draw_function(ui: &Ui, session: &mut Session) -> bool {
    let mut save = ui.io().key_ctrl() && ui.is_key_pressed(Key::S);
    if ui.button("New trigger") {
        add_staggered(session, FunctionKind::Trigger);
    }
    ui.same_line();
    if ui.button("New action") {
        add_staggered(session, FunctionKind::Action);
    }
    ui.same_line();
    if ui.button("New value") {
        add_staggered(session, FunctionKind::Value);
    }
    ui.same_line();
    if ui.button("Save") {
        save = true;
    }

    let avail = ui.content_region_avail();
    let where_w = 240.0_f32.min((avail[0] * 0.28).max(140.0));
    let how_w = 280.0_f32.min((avail[0] * 0.32).max(160.0));
    let canvas_w = (avail[0] - where_w - how_w - 16.0).max(80.0);
    let body_h = (avail[1] - 36.0).max(80.0);

    let rows = place_rows(session.editor.graph());
    ui.child_window("where")
        .size([where_w, body_h])
        .border(true)
        .build(ui, || {
            ui.text("Where");
            if rows.is_empty() {
                ui.text_wrapped("Functions group by the place you name.");
            }
            for (place, items) in &rows {
                ui.separator();
                ui.text(place);
                for (id, label) in items {
                    let on = session.editor.selected() == Some(*id);
                    if ui.selectable_config(label).selected(on).build() {
                        focus_function(session, *id);
                    }
                }
            }
        });
    ui.same_line();
    ui.child_window("canvas")
        .size([canvas_w, body_h])
        .border(true)
        .flags(WindowFlags::NO_SCROLLBAR | WindowFlags::NO_SCROLL_WITH_MOUSE)
        .build(ui, || draw_canvas(ui, session));
    ui.same_line();
    ui.child_window("how")
        .size([how_w, body_h])
        .border(true)
        .build(ui, || draw_how(ui, session));

    if !ui.io().want_text_input() {
        if ui.is_key_pressed(Key::Delete) || ui.is_key_pressed(Key::Backspace) {
            session.editor.delete_selection();
        }
        if ui.is_key_pressed(Key::Escape) {
            session.editor.clear_selection();
        }
    }

    let path = session.path.display().to_string();
    if session.notice.is_empty() {
        ui.text(path);
    } else {
        ui.text(format!("{path}  {}", session.notice));
    }
    if let Some(err) = session.editor.last_error() {
        ui.text(err.to_string());
    }
    save
}

fn add_staggered(session: &mut Session, kind: FunctionKind) {
    let n = session.editor.graph().len() as f32;
    // Step past a two-port node (header, two rows, pad) so the new node is visible.
    session
        .editor
        .add_function_at(48.0 + n * 36.0, 48.0 + n * 96.0, kind);
}

fn focus_function(session: &mut Session, id: FunctionId) {
    session.editor.select(Some(id));
    if let Some(function) = session.editor.graph().get(id) {
        let (x, y) = function.position();
        let zoom = session.editor.camera().zoom();
        session
            .editor
            .camera_mut()
            .set_view(x - 40.0, y - 40.0, zoom);
    }
}

fn place_rows(graph: &Graph) -> Vec<(String, Vec<(FunctionId, String)>)> {
    let mut rows = Vec::new();
    for place in graph.places() {
        let items = graph
            .at_place(&place)
            .into_iter()
            .map(|id| (id, display(graph, id)))
            .collect();
        rows.push((place, items));
    }
    let loose = graph.at_place("");
    if !loose.is_empty() {
        let items = loose
            .into_iter()
            .map(|id| (id, display(graph, id)))
            .collect();
        rows.push(("No place".to_string(), items));
    }
    rows
}

fn display(graph: &Graph, id: FunctionId) -> String {
    let Some(function) = graph.get(id) else {
        return String::new();
    };
    let name = if function.name().is_empty() {
        "Untitled"
    } else {
        function.name()
    };
    format!("{}  {name}", function.kind().label())
}

fn draw_canvas(ui: &Ui, session: &mut Session) {
    let origin = ui.cursor_screen_pos();
    let size = ui.content_region_avail();
    if size[0] < 1.0 || size[1] < 1.0 {
        return;
    }
    paint_canvas(ui, session, origin, size);
    ui.set_cursor_screen_pos(origin);
    let _ = ui.invisible_button("function-canvas", size);
    let hovered = ui.is_item_hovered();
    let mouse = ui.io().mouse_pos();
    let x = mouse[0] - origin[0];
    let y = mouse[1] - origin[1];
    if hovered && ui.is_mouse_double_clicked(MouseButton::Left) {
        session.editor.add_function_at(x, y, FunctionKind::Trigger);
    } else if hovered && ui.is_mouse_clicked(MouseButton::Left) {
        session
            .editor
            .pointer_down(x, y, Button::Primary, ui.io().key_alt());
    }
    if hovered && ui.is_mouse_clicked(MouseButton::Middle) {
        session.editor.pointer_down(x, y, Button::Middle, false);
    }
    if ui.is_mouse_down(MouseButton::Left) || ui.is_mouse_down(MouseButton::Middle) {
        session.editor.pointer_move(x, y);
    }
    if ui.is_mouse_released(MouseButton::Left) {
        session.editor.pointer_up(x, y, Button::Primary);
    }
    if ui.is_mouse_released(MouseButton::Middle) {
        session.editor.pointer_up(x, y, Button::Middle);
    }
    if hovered && ui.io().mouse_wheel() != 0.0 {
        session.editor.scroll(x, y, ui.io().mouse_wheel());
    }
}

fn paint_canvas(ui: &Ui, session: &Session, origin: [f32; 2], size: [f32; 2]) {
    let draw = ui.get_window_draw_list();
    let canvas = Color::rgb(0.10, 0.11, 0.13);
    let grid = Color::new(1.0, 1.0, 1.0, 0.07);
    let node = Color::rgb(0.18, 0.19, 0.22);
    let wire = Color::rgb(0.74, 0.76, 0.82);
    let text = Color::rgb(0.94, 0.94, 0.96);
    let port_color = Color::rgb(0.93, 0.93, 0.95);
    let selected = Color::rgb(0.95, 0.84, 0.42);
    draw.add_rect(origin, [origin[0] + size[0], origin[1] + size[1]], canvas)
        .filled(true)
        .build();

    let camera = *session.editor.camera();
    let (world_min_x, world_min_y) = camera.canvas_to_world(0.0, 0.0);
    let (world_max_x, world_max_y) = camera.canvas_to_world(size[0], size[1]);
    let step = 24.0;
    let mut x = (world_min_x / step).floor() * step;
    while x <= world_max_x {
        let screen = to_screen(origin, &camera, x, 0.0);
        let sx = screen[0];
        draw.add_line([sx, origin[1]], [sx, origin[1] + size[1]], grid)
            .build();
        x += step;
    }
    let mut y = (world_min_y / step).floor() * step;
    while y <= world_max_y {
        let screen = to_screen(origin, &camera, 0.0, y);
        let sy = screen[1];
        draw.add_line([origin[0], sy], [origin[0] + size[0], sy], grid)
            .build();
        y += step;
    }

    let graph = session.editor.graph();
    let mut ids = graph.at_place("");
    for place in graph.places() {
        ids.extend(graph.at_place(&place));
    }
    ids.sort_by_key(|id| id.0);
    for connection in graph.connections() {
        let Some(from) = graph.port_center(connection.from_fn, connection.from_port) else {
            continue;
        };
        let Some(to) = graph.port_center(connection.to_fn, connection.to_port) else {
            continue;
        };
        curve(&draw, origin, &camera, from, to, wire);
    }
    if let Some(link) = session.editor.linking() {
        if let Some(from) = graph.port_center(link.from, link.from_port) {
            curve(&draw, origin, &camera, from, (link.x, link.y), selected);
        }
    }

    for id in ids {
        let Some(function) = graph.get(id) else {
            continue;
        };
        let rect = graph.node_rect(function.id()).unwrap();
        let screen = to_screen(origin, &camera, rect.x, rect.y);
        let sx = screen[0];
        let sy = screen[1];
        let w = rect.width * camera.zoom();
        let h = rect.height * camera.zoom();
        draw.add_rect([sx, sy], [sx + w, sy + h], node)
            .filled(true)
            .rounding(4.0)
            .build();
        let stripe = kind_color(function.kind());
        draw.add_rect([sx, sy], [sx + w, sy + 4.0], stripe)
            .filled(true)
            .build();
        if session.editor.selected() == Some(function.id()) {
            draw.add_rect([sx, sy], [sx + w, sy + h], selected)
                .thickness(2.0)
                .rounding(4.0)
                .build();
        }
        let title = if function.name().is_empty() {
            "Untitled".to_string()
        } else {
            short(function.name())
        };
        draw.add_text([sx + 10.0, sy + 8.0], text, title);
        for port in function.ports() {
            let Some((px, py)) = graph.port_center(function.id(), port.id) else {
                continue;
            };
            let center = to_screen(origin, &camera, px, py);
            let cx = center[0];
            let cy = center[1];
            draw.add_circle([cx, cy], (PORT_RADIUS * camera.zoom()).max(3.0), port_color)
                .filled(true)
                .build();
            let label_x = if port.direction == Direction::In {
                cx + 8.0
            } else {
                cx - 8.0 - port.name.len() as f32 * 7.0
            };
            draw.add_text([label_x, cy - 7.0], text, &port.name);
        }
    }

    if graph.len() == 0 {
        draw.add_text(
            [origin[0] + 24.0, origin[1] + 24.0],
            text,
            "Add a function to say how and where something works, then connect it.",
        );
    }
}

fn curve(
    draw: &dear_imgui_rs::DrawListMut<'_>,
    origin: [f32; 2],
    camera: &crate::Camera,
    from: (f32, f32),
    to: (f32, f32),
    color: Color,
) {
    let start = to_screen(origin, camera, from.0, from.1);
    let end = to_screen(origin, camera, to.0, to.1);
    let bend = 80.0;
    draw.add_bezier_curve(
        start,
        [start[0] + bend, start[1]],
        [end[0] - bend, end[1]],
        end,
        color,
    )
    .thickness(2.0)
    .build();
}

fn to_screen(origin: [f32; 2], camera: &crate::Camera, x: f32, y: f32) -> [f32; 2] {
    let (cx, cy) = camera.world_to_canvas(x, y);
    [origin[0] + cx, origin[1] + cy]
}

fn kind_color(kind: FunctionKind) -> Color {
    match kind {
        FunctionKind::Trigger => Color::rgb(0.36, 0.58, 0.86),
        FunctionKind::Action => Color::rgb(0.86, 0.55, 0.28),
        FunctionKind::Value => Color::rgb(0.42, 0.70, 0.48),
    }
}

fn short(text: &str) -> String {
    let mut chars = text.chars();
    let mut out = String::new();
    for _ in 0..24 {
        let Some(ch) = chars.next() else {
            return out;
        };
        out.push(ch);
    }
    if chars.next().is_some() {
        out.push('…');
    }
    out
}

fn draw_how(ui: &Ui, session: &mut Session) {
    ui.text("How");
    let Some(id) = session.editor.selected() else {
        ui.text_wrapped("Select a function to name it, say where it works, and how.");
        return;
    };
    if session.editor.graph().get(id).is_none() {
        ui.text("That function is gone.");
        return;
    }
    let (mut name, mut place, mut detail, kind) = {
        let function = session.editor.graph().get(id).unwrap();
        (
            function.name().to_string(),
            function.place().to_string(),
            function.detail().to_string(),
            function.kind(),
        )
    };
    ui.text(format!("Kind: {}", kind.label()));
    if ui.button("Trigger") {
        let _ = session
            .editor
            .graph_mut()
            .set_kind(id, FunctionKind::Trigger);
    }
    ui.same_line();
    if ui.button("Action") {
        let _ = session
            .editor
            .graph_mut()
            .set_kind(id, FunctionKind::Action);
    }
    ui.same_line();
    if ui.button("Value") {
        let _ = session.editor.graph_mut().set_kind(id, FunctionKind::Value);
    }
    if ui.input_text("Name", &mut name).build() {
        let _ = session.editor.graph_mut().set_name(id, &name);
    }
    if ui.input_text("Where", &mut place).build() {
        let _ = session.editor.graph_mut().set_place(id, &place);
    }
    if ui.input_text("How", &mut detail).build() {
        let _ = session.editor.graph_mut().set_detail(id, &detail);
    }
    ui.separator();
    ui.text("Ports");
    let ports: Vec<_> = session
        .editor
        .graph()
        .get(id)
        .unwrap()
        .ports()
        .map(|port| (port.id, port.direction, port.name.clone()))
        .collect();
    for (port_id, direction, mut port_name) in ports {
        ui.text(direction_label(direction));
        ui.same_line();
        let field = format!("##port-{}", port_id.0);
        if ui.input_text(&field, &mut port_name).build() {
            let _ = session
                .editor
                .graph_mut()
                .rename_port(id, port_id, &port_name);
        }
        ui.same_line();
        let remove = format!("Remove##{}", port_id.0);
        if ui.button(remove) {
            let _ = session.editor.graph_mut().remove_port(id, port_id);
        }
    }
    if ui.button("Add input") {
        if let Some(function) = session.editor.graph().get(id) {
            let name = next_port_name(function, Direction::In);
            let _ = session
                .editor
                .graph_mut()
                .add_port(id, Direction::In, &name);
        }
    }
    ui.same_line();
    if ui.button("Add output") {
        if let Some(function) = session.editor.graph().get(id) {
            let name = next_port_name(function, Direction::Out);
            let _ = session
                .editor
                .graph_mut()
                .add_port(id, Direction::Out, &name);
        }
    }
    ui.separator();
    ui.text("Connects");
    let incoming: Vec<String> = session
        .editor
        .graph()
        .wires_to(id)
        .into_iter()
        .map(|wire| display(session.editor.graph(), wire.from_fn))
        .collect();
    let outgoing: Vec<String> = session
        .editor
        .graph()
        .downstream(id)
        .into_iter()
        .map(|next| display(session.editor.graph(), next))
        .collect();
    if incoming.is_empty() && outgoing.is_empty() {
        ui.text_wrapped("Drag from an output dot to an input dot.");
    }
    for name in incoming {
        ui.text(format!("From {name}"));
    }
    for name in outgoing {
        ui.text(format!("To {name}"));
    }
}

fn direction_label(direction: Direction) -> &'static str {
    match direction {
        Direction::In => "in",
        Direction::Out => "out",
    }
}

fn next_port_name(function: &crate::Function, direction: Direction) -> String {
    let base = direction_label(direction);
    let taken = |name: &str| function.ports().any(|port| port.name == name);
    if !taken(base) {
        return base.to_string();
    }
    let mut index = 2;
    loop {
        let candidate = format!("{base} {index}");
        if !taken(&candidate) {
            return candidate;
        }
        index += 1;
    }
}
