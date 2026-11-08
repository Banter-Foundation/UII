//! Function graph contract. These tests are the API: where a function lives,
//! how it works, and which output feeds which input.

use std::path::Path;

use uii_function::{
    default_graph_path, from_text, to_text, Button, Camera, ConnectionId, Direction, EditError,
    Editor, FunctionId, FunctionKind, Graph, Hit, PortId, HEADER_HEIGHT, NODE_WIDTH, PORT_PAD,
    PORT_RADIUS, PORT_ROW, PORT_SLOP,
};

fn out_at(x: f32, y: f32, index: u32) -> (f32, f32) {
    (
        x + NODE_WIDTH,
        y + HEADER_HEIGHT + PORT_ROW * (index as f32 + 0.5),
    )
}

fn in_at(x: f32, y: f32, index: u32) -> (f32, f32) {
    (x, y + HEADER_HEIGHT + PORT_ROW * (index as f32 + 0.5))
}

fn height(rows: u32) -> f32 {
    HEADER_HEIGHT + PORT_ROW * (rows.max(1) as f32) + PORT_PAD
}

fn port(graph: &Graph, id: FunctionId, direction: Direction) -> PortId {
    graph
        .get(id)
        .unwrap()
        .ports()
        .find(|port| port.direction == direction)
        .unwrap()
        .id
}

#[test]
fn new_graph_is_empty() {
    let graph = Graph::new();
    assert_eq!(graph.len(), 0);
    assert_eq!(graph.connections().count(), 0);
    assert!(graph.places().is_empty());
}

#[test]
fn create_stores_place_how_and_default_ports() {
    let mut graph = Graph::new();
    let trigger = graph.create(FunctionKind::Trigger, " Save ", " Home ", 10.0, 20.0);
    assert_eq!(trigger, FunctionId(1));
    let function = graph.get(trigger).unwrap();
    assert_eq!(function.name(), " Save ");
    assert_eq!(function.place(), " Home ");
    assert_eq!(function.detail(), "");
    assert_eq!(function.kind(), FunctionKind::Trigger);
    assert_eq!(function.position(), (10.0, 20.0));
    let ports: Vec<_> = function
        .ports()
        .map(|port| (port.id, port.direction, port.name.as_str()))
        .collect();
    assert_eq!(ports, vec![(PortId(1), Direction::Out, "when")]);

    let action = graph.create(FunctionKind::Action, "", "", 0.0, 0.0);
    let ports: Vec<_> = graph
        .get(action)
        .unwrap()
        .ports()
        .map(|port| (port.id, port.direction, port.name.as_str()))
        .collect();
    assert_eq!(
        ports,
        vec![
            (PortId(1), Direction::In, "run"),
            (PortId(2), Direction::Out, "then"),
        ]
    );

    let value = graph.create(FunctionKind::Value, "", "", 0.0, 0.0);
    let ports: Vec<_> = graph
        .get(value)
        .unwrap()
        .ports()
        .map(|port| (port.id, port.direction, port.name.as_str()))
        .collect();
    assert_eq!(ports, vec![(PortId(1), Direction::Out, "value")]);
    assert_eq!(graph.len(), 3);
}

#[test]
fn kind_labels_round_trip_through_the_file_words() {
    assert_eq!(FunctionKind::Trigger.as_str(), "trigger");
    assert_eq!(FunctionKind::Action.as_str(), "action");
    assert_eq!(FunctionKind::Value.as_str(), "value");
    assert_eq!(FunctionKind::Trigger.label(), "Trigger");
    assert_eq!(FunctionKind::Action.label(), "Action");
    assert_eq!(FunctionKind::Value.label(), "Value");
    assert_eq!(FunctionKind::parse("trigger"), Some(FunctionKind::Trigger));
    assert_eq!(FunctionKind::parse("action"), Some(FunctionKind::Action));
    assert_eq!(FunctionKind::parse("value"), Some(FunctionKind::Value));
    assert_eq!(FunctionKind::parse("Trigger"), None);
}

#[test]
fn deleted_ids_are_not_reused_while_the_graph_is_open() {
    let mut graph = Graph::new();
    let first = graph.create(FunctionKind::Trigger, "", "", 0.0, 0.0);
    let second = graph.create(FunctionKind::Trigger, "", "", 0.0, 0.0);
    graph.delete(first).unwrap();
    assert!(graph.get(first).is_none());
    assert!(graph.get(second).is_some());
    let third = graph.create(FunctionKind::Trigger, "", "", 0.0, 0.0);
    assert_eq!(third, FunctionId(3));
    assert_eq!(graph.delete(FunctionId(9)), Err(EditError::MissingFunction));
}

#[test]
fn edits_keep_ports_and_reject_a_missing_function() {
    let mut graph = Graph::new();
    let id = graph.create(FunctionKind::Trigger, "A", "Home", 1.0, 2.0);
    let before: Vec<_> = graph.get(id).unwrap().ports().map(|port| port.id).collect();
    graph.set_name(id, "").unwrap();
    graph.set_place(id, "Settings").unwrap();
    graph.set_detail(id, "click").unwrap();
    graph.set_kind(id, FunctionKind::Action).unwrap();
    graph.move_to(id, 4.0, 5.0).unwrap();
    let function = graph.get(id).unwrap();
    assert_eq!(function.name(), "");
    assert_eq!(function.place(), "Settings");
    assert_eq!(function.detail(), "click");
    assert_eq!(function.kind(), FunctionKind::Action);
    assert_eq!(function.position(), (4.0, 5.0));
    let after: Vec<_> = function.ports().map(|port| port.id).collect();
    assert_eq!(after, before);
    assert_eq!(
        graph.set_name(FunctionId(8), "x"),
        Err(EditError::MissingFunction)
    );
    assert_eq!(
        graph.move_to(FunctionId(8), 0.0, 0.0),
        Err(EditError::MissingFunction)
    );
}

#[test]
fn ports_can_be_added_renamed_and_removed() {
    let mut graph = Graph::new();
    let action = graph.create(FunctionKind::Action, "", "", 0.0, 0.0);
    let trigger = graph.create(FunctionKind::Trigger, "", "", 300.0, 0.0);
    assert_eq!(
        graph.add_port(action, Direction::In, "   "),
        Err(EditError::EmptyPortName)
    );
    assert_eq!(
        graph.add_port(action, Direction::In, "run"),
        Err(EditError::DuplicatePortName)
    );
    let extra = graph.add_port(action, Direction::In, "  extra  ").unwrap();
    assert_eq!(extra, PortId(3));
    assert_eq!(
        graph
            .get(action)
            .unwrap()
            .ports()
            .find(|port| port.id == extra)
            .unwrap()
            .name,
        "extra"
    );
    graph.rename_port(action, extra, "extra").unwrap();
    assert_eq!(
        graph.rename_port(action, extra, "run"),
        Err(EditError::DuplicatePortName)
    );
    assert_eq!(
        graph
            .get(action)
            .unwrap()
            .ports()
            .find(|port| port.id == extra)
            .unwrap()
            .name,
        "extra"
    );
    assert_eq!(
        graph.rename_port(action, extra, "  "),
        Err(EditError::EmptyPortName)
    );

    let from = port(&graph, trigger, Direction::Out);
    let to = port(&graph, action, Direction::In);
    graph.connect(trigger, from, action, to).unwrap();
    graph.remove_port(action, to).unwrap();
    assert_eq!(graph.connections().count(), 0);
    assert!(graph.get(trigger).is_some());
    let added = graph.add_port(action, Direction::In, "next").unwrap();
    assert_eq!(added, PortId(4));
    assert_eq!(
        graph.remove_port(action, PortId(9)),
        Err(EditError::MissingPort)
    );
}

#[test]
fn connections_run_from_an_output_to_an_input() {
    let mut graph = Graph::new();
    let action = graph.create(FunctionKind::Action, "A", "", 0.0, 0.0);
    let other = graph.create(FunctionKind::Action, "B", "", 0.0, 0.0);
    let third = graph.create(FunctionKind::Action, "C", "", 0.0, 0.0);
    let out_a = port(&graph, action, Direction::Out);
    let in_a = port(&graph, action, Direction::In);
    let out_b = port(&graph, other, Direction::Out);
    let in_b = port(&graph, other, Direction::In);
    let in_c = port(&graph, third, Direction::In);
    let also = graph.add_port(action, Direction::Out, "also").unwrap();

    assert_eq!(
        graph.connect(action, out_a, action, also),
        Err(EditError::WrongDirection)
    );
    assert_eq!(
        graph.connect(action, in_a, action, out_a),
        Err(EditError::WrongDirection)
    );
    assert_eq!(
        graph.connect(action, out_a, action, in_a),
        Err(EditError::SameFunction)
    );
    assert_eq!(
        graph.connect(FunctionId(9), out_a, other, in_b),
        Err(EditError::MissingFunction)
    );
    assert_eq!(
        graph.connect(action, PortId(9), other, in_b),
        Err(EditError::MissingPort)
    );

    let first = graph.connect(action, out_a, other, in_b).unwrap();
    assert_eq!(first, ConnectionId(1));
    assert_eq!(
        graph.connect(action, also, other, in_b),
        Err(EditError::InputTaken)
    );
    let second = graph.connect(action, also, third, in_c).unwrap();
    assert_eq!(second, ConnectionId(2));
    let wires: Vec<_> = graph
        .connections()
        .map(|wire| {
            (
                wire.id,
                wire.from_fn,
                wire.from_port,
                wire.to_fn,
                wire.to_port,
            )
        })
        .collect();
    assert_eq!(
        wires,
        vec![
            (ConnectionId(1), action, out_a, other, in_b),
            (ConnectionId(2), action, also, third, in_c),
        ]
    );
    assert_eq!(
        graph
            .wires_from(action)
            .into_iter()
            .map(|wire| wire.id)
            .collect::<Vec<_>>(),
        vec![ConnectionId(1), ConnectionId(2)]
    );
    assert_eq!(graph.wires_to(other).len(), 1);
    assert!(graph.wires_to(action).is_empty());

    graph.disconnect(first).unwrap();
    assert_eq!(graph.disconnect(first), Err(EditError::MissingConnection));
    let third_wire = graph.connect(other, out_b, third, in_c);
    assert_eq!(third_wire, Err(EditError::InputTaken));
    graph.disconnect(second).unwrap();
    let again = graph.connect(other, out_b, third, in_c).unwrap();
    assert_eq!(again, ConnectionId(3));
}

#[test]
fn a_connection_that_loops_is_rejected() {
    let mut graph = Graph::new();
    let a = graph.create(FunctionKind::Action, "A", "", 0.0, 0.0);
    let b = graph.create(FunctionKind::Action, "B", "", 0.0, 0.0);
    let c = graph.create(FunctionKind::Action, "C", "", 0.0, 0.0);
    graph
        .connect(
            a,
            port(&graph, a, Direction::Out),
            b,
            port(&graph, b, Direction::In),
        )
        .unwrap();
    graph
        .connect(
            b,
            port(&graph, b, Direction::Out),
            c,
            port(&graph, c, Direction::In),
        )
        .unwrap();
    assert_eq!(
        graph.connect(
            c,
            port(&graph, c, Direction::Out),
            a,
            port(&graph, a, Direction::In)
        ),
        Err(EditError::Cycle)
    );
    assert_eq!(graph.connections().count(), 2);
    assert_eq!(graph.downstream(a), vec![b, c]);
    assert_eq!(EditError::Cycle.to_string(), "That connection would loop.");
    assert_eq!(
        EditError::InputTaken.to_string(),
        "That input is already connected."
    );
}

#[test]
fn downstream_walks_each_function_once_in_wire_order() {
    let mut graph = Graph::new();
    let a = graph.create(FunctionKind::Action, "A", "", 0.0, 0.0);
    let b = graph.create(FunctionKind::Action, "B", "", 0.0, 0.0);
    let c = graph.create(FunctionKind::Action, "C", "", 0.0, 0.0);
    let d = graph.create(FunctionKind::Action, "D", "", 0.0, 0.0);
    let a_also = graph.add_port(a, Direction::Out, "also").unwrap();
    graph
        .connect(
            a,
            port(&graph, a, Direction::Out),
            b,
            port(&graph, b, Direction::In),
        )
        .unwrap();
    graph
        .connect(a, a_also, c, port(&graph, c, Direction::In))
        .unwrap();
    graph
        .connect(
            b,
            port(&graph, b, Direction::Out),
            d,
            port(&graph, d, Direction::In),
        )
        .unwrap();
    let d_also = graph.add_port(d, Direction::In, "else").unwrap();
    graph
        .connect(c, port(&graph, c, Direction::Out), d, d_also)
        .unwrap();
    assert_eq!(graph.downstream(a), vec![b, c, d]);
    assert!(graph.downstream(d).is_empty());
}

#[test]
fn places_group_where_functions_work() {
    let mut graph = Graph::new();
    let loose = graph.create(FunctionKind::Trigger, "", "", 0.0, 0.0);
    let home = graph.create(FunctionKind::Trigger, "", "Home", 0.0, 0.0);
    let settings = graph.create(FunctionKind::Trigger, "", "Settings", 0.0, 0.0);
    let home_again = graph.create(FunctionKind::Action, "", "Home", 0.0, 0.0);
    assert_eq!(
        graph.places(),
        vec!["Home".to_string(), "Settings".to_string()]
    );
    assert_eq!(graph.at_place("Home"), vec![home, home_again]);
    assert_eq!(graph.at_place("Settings"), vec![settings]);
    assert_eq!(graph.at_place(""), vec![loose]);
    assert!(graph.at_place("Missing").is_empty());
}

#[test]
fn geometry_hits_ports_before_the_body_and_later_nodes_first() {
    let mut graph = Graph::new();
    let trigger = graph.create(FunctionKind::Trigger, "", "", 10.0, 20.0);
    assert_eq!(
        graph.node_rect(trigger),
        Some(uii_function::Rect {
            x: 10.0,
            y: 20.0,
            width: NODE_WIDTH,
            height: height(1),
        })
    );
    assert_eq!(
        graph.port_center(trigger, PortId(1)),
        Some(out_at(10.0, 20.0, 0))
    );
    let (ox, oy) = out_at(10.0, 20.0, 0);
    assert_eq!(
        graph.hit_test(ox, oy),
        Some(Hit::Port {
            function: trigger,
            port: PortId(1),
        })
    );
    let slop = PORT_RADIUS + PORT_SLOP;
    assert_eq!(
        graph.hit_test(ox - slop, oy),
        Some(Hit::Port {
            function: trigger,
            port: PortId(1),
        })
    );
    assert_eq!(
        graph.hit_test(ox - (slop + 1.0), oy),
        Some(Hit::Body { function: trigger })
    );
    assert_eq!(
        graph.hit_test(10.0, 20.0),
        Some(Hit::Body { function: trigger })
    );
    assert_eq!(graph.hit_test(9.0, 20.0), None);

    let action = graph.create(FunctionKind::Action, "", "", 0.0, 0.0);
    let (ix, iy) = in_at(0.0, 0.0, 0);
    assert_eq!(graph.port_center(action, PortId(1)), Some((ix, iy)));
    assert_eq!(
        graph.hit_test(ix, iy),
        Some(Hit::Port {
            function: action,
            port: PortId(1),
        })
    );
    graph.add_port(action, Direction::In, "more").unwrap();
    assert_eq!(graph.node_rect(action).unwrap().height, height(2));

    let covered = graph.create(FunctionKind::Trigger, "", "", 0.0, 0.0);
    let over = graph.create(FunctionKind::Trigger, "", "", 0.0, 0.0);
    assert_eq!(
        graph.hit_test(20.0, 30.0),
        Some(Hit::Body { function: over })
    );
    let (cx, cy) = out_at(0.0, 0.0, 0);
    let _ = covered;
    let right = graph.create(FunctionKind::Trigger, "", "", NODE_WIDTH, 0.0);
    assert_eq!(graph.hit_test(cx, cy), Some(Hit::Body { function: right }));
    graph.delete(trigger).unwrap();
    assert_eq!(graph.node_rect(trigger), None);
    assert_eq!(graph.port_center(trigger, PortId(1)), None);
}

#[test]
fn camera_pans_and_zooms_around_the_cursor() {
    let mut camera = Camera::new();
    assert_eq!(camera.zoom(), 1.0);
    assert_eq!(camera.pan(), (0.0, 0.0));
    assert_eq!(camera.world_to_canvas(10.0, 10.0), (10.0, 10.0));
    camera.set_view(5.0, -3.0, 2.0);
    assert_eq!(camera.world_to_canvas(10.0, 10.0), (10.0, 26.0));
    assert_eq!(camera.canvas_to_world(10.0, 26.0), (10.0, 10.0));
    camera.pan_by(10.0, 0.0);
    assert_eq!(camera.pan(), (0.0, -3.0));

    camera.set_view(0.0, 0.0, 1.0);
    camera.zoom_at(100.0, 50.0, 1.0);
    assert_eq!(camera.zoom(), 1.0 * 1.1);
    let (wx, wy) = camera.canvas_to_world(100.0, 50.0);
    assert!((wx - 100.0).abs() < 1e-4, "{wx}");
    assert!((wy - 50.0).abs() < 1e-4, "{wy}");

    camera.set_view(0.0, 0.0, 4.0);
    camera.zoom_at(100.0, 50.0, 1.0);
    assert_eq!(camera.zoom(), 4.0);
    assert_eq!(camera.pan(), (0.0, 0.0));
    camera.set_view(0.0, 0.0, 0.25);
    camera.zoom_at(100.0, 50.0, -1.0);
    assert_eq!(camera.zoom(), 0.25);
    assert_eq!(camera.pan(), (0.0, 0.0));
    camera.set_view(1.0, 2.0, 100.0);
    assert_eq!(camera.zoom(), 4.0);
    assert_eq!(camera.pan(), (1.0, 2.0));
    camera.set_view(0.0, 0.0, 0.01);
    assert_eq!(camera.zoom(), 0.25);
}

#[test]
fn editor_moves_selects_pans_and_wires_from_the_canvas() {
    let mut editor = Editor::new();
    let trigger = editor.add_function_at(0.0, 0.0, FunctionKind::Trigger);
    assert_eq!(editor.selected(), Some(trigger));
    assert_eq!(editor.graph().get(trigger).unwrap().position(), (0.0, 0.0));
    assert_eq!(editor.graph().get(trigger).unwrap().name(), "");
    editor.camera_mut().set_view(0.0, 0.0, 2.0);
    let value = editor.add_function_at(40.0, 50.0, FunctionKind::Value);
    assert_eq!(editor.graph().get(value).unwrap().position(), (20.0, 25.0));
    editor.graph_mut().move_to(value, 0.0, 500.0).unwrap();
    editor.camera_mut().set_view(0.0, 0.0, 1.0);

    editor.pointer_down(20.0, 20.0, Button::Primary, false);
    editor.pointer_move(50.0, 40.0);
    assert_eq!(
        editor.graph().get(trigger).unwrap().position(),
        (30.0, 20.0)
    );
    editor.pointer_up(50.0, 40.0, Button::Primary);
    assert_eq!(editor.selected(), Some(trigger));

    editor.pointer_down(-10.0, -10.0, Button::Primary, false);
    assert_eq!(editor.selected(), None);
    editor.pointer_down(20.0, 20.0, Button::Secondary, false);
    assert_eq!(editor.selected(), None);

    let action = editor
        .graph_mut()
        .create(FunctionKind::Action, "", "", 0.0, 0.0);
    let (ix, iy) = editor.graph().port_center(action, PortId(1)).unwrap();
    editor.pointer_down(ix, iy, Button::Primary, false);
    editor.pointer_move(ix + 40.0, iy + 10.0);
    assert_eq!(editor.graph().get(action).unwrap().position(), (0.0, 0.0));
    assert!(editor.linking().is_none());
    assert_eq!(editor.selected(), Some(action));
    editor.pointer_up(ix + 40.0, iy + 10.0, Button::Primary);

    editor.graph_mut().move_to(trigger, 0.0, 0.0).unwrap();
    let (ox, oy) = editor.graph().port_center(trigger, PortId(1)).unwrap();
    editor.graph_mut().move_to(action, 300.0, 0.0).unwrap();
    let (ix, iy) = editor.graph().port_center(action, PortId(1)).unwrap();
    editor.pointer_down(ox, oy, Button::Primary, false);
    assert_eq!(
        editor.linking().map(|link| (link.from, link.from_port)),
        Some((trigger, PortId(1)))
    );
    editor.pointer_move(ix, iy);
    assert_eq!(editor.graph().get(trigger).unwrap().position(), (0.0, 0.0));
    editor.pointer_up(ix, iy, Button::Primary);
    assert_eq!(editor.graph().connections().count(), 1);
    assert!(editor.linking().is_none());
    assert!(editor.last_error().is_none());

    editor.pointer_down(10.0, 10.0, Button::Middle, false);
    editor.pointer_move(30.0, 15.0);
    assert_eq!(editor.camera().pan(), (-20.0, -5.0));
    assert_eq!(editor.graph().get(trigger).unwrap().position(), (0.0, 0.0));
    assert_eq!(editor.selected(), Some(trigger));
    editor.pointer_up(30.0, 15.0, Button::Middle);

    editor.camera_mut().set_view(0.0, 0.0, 1.0);
    editor.pointer_down(ox, oy, Button::Primary, true);
    editor.pointer_move(ox + 12.0, oy);
    assert!(editor.linking().is_none());
    assert_eq!(editor.camera().pan(), (-12.0, 0.0));
    editor.pointer_up(ox + 12.0, oy, Button::Primary);

    editor.scroll(100.0, 50.0, 1.0);
    assert_eq!(editor.camera().zoom(), 1.0 * 1.1);
    assert_eq!(editor.graph().get(trigger).unwrap().position(), (0.0, 0.0));

    editor.select(Some(value));
    editor.delete_selection();
    assert!(editor.graph().get(value).is_none());
    assert_eq!(editor.selected(), None);
    editor.delete_selection();
    editor.clear_selection();

    let stale = editor.add_function_at(8.0, 8.0, FunctionKind::Trigger);
    editor.graph_mut().delete(stale).unwrap();
    assert_eq!(editor.selected(), Some(stale));
    editor.retain_selection();
    assert_eq!(editor.selected(), None);
}

#[test]
fn editor_reports_a_wire_that_would_loop() {
    let mut editor = Editor::new();
    let a = editor
        .graph_mut()
        .create(FunctionKind::Action, "A", "", 0.0, 0.0);
    let b = editor
        .graph_mut()
        .create(FunctionKind::Action, "B", "", 300.0, 0.0);
    let c = editor
        .graph_mut()
        .create(FunctionKind::Action, "C", "", 600.0, 0.0);
    let a_out = editor.graph().port_center(a, PortId(2)).unwrap();
    let b_in = editor.graph().port_center(b, PortId(1)).unwrap();
    let b_out = editor.graph().port_center(b, PortId(2)).unwrap();
    let c_in = editor.graph().port_center(c, PortId(1)).unwrap();
    let c_out = editor.graph().port_center(c, PortId(2)).unwrap();
    let a_in = editor.graph().port_center(a, PortId(1)).unwrap();

    editor.pointer_down(a_out.0, a_out.1, Button::Primary, false);
    editor.pointer_up(b_in.0, b_in.1, Button::Primary);
    editor.pointer_down(b_out.0, b_out.1, Button::Primary, false);
    editor.pointer_up(c_in.0, c_in.1, Button::Primary);
    editor.pointer_down(c_out.0, c_out.1, Button::Primary, false);
    editor.pointer_up(a_in.0, a_in.1, Button::Primary);
    assert_eq!(editor.last_error(), Some(EditError::Cycle));
    assert_eq!(editor.graph().connections().count(), 2);

    editor.pointer_down(-40.0, -40.0, Button::Primary, false);
    assert_eq!(editor.last_error(), None);
}

#[test]
fn graph_file_round_trips_and_keeps_id_gaps() {
    let mut graph = Graph::new();
    let save = graph.create(FunctionKind::Trigger, "Save", "Home", 10.5, 20.0);
    graph.set_detail(save, "click").unwrap();
    let open = graph.create(FunctionKind::Action, "Open", "Settings", 240.0, 20.0);
    graph.set_detail(open, "open Settings").unwrap();
    graph
        .connect(
            save,
            port(&graph, save, Direction::Out),
            open,
            port(&graph, open, Direction::In),
        )
        .unwrap();
    let text = to_text(&graph);
    assert_eq!(
        text,
        "\
uii-function 1
fn 1 trigger 10.5 20 \"Home\" \"Save\" \"click\"
port 1 1 out \"when\"
fn 2 action 240 20 \"Settings\" \"Open\" \"open Settings\"
port 2 1 in \"run\"
port 2 2 out \"then\"
wire 1 1 1 2 1
"
    );
    let loaded = from_text(&text).unwrap();
    assert_eq!(to_text(&loaded), text);
    assert_eq!(loaded.get(save).unwrap().detail(), "click");

    let mut spaced = text.replace('\n', "\r\n");
    spaced.insert(spaced.find("fn 2").unwrap(), '\n');
    assert_eq!(to_text(&from_text(&spaced).unwrap()), text);

    graph.set_name(save, "say \"hi\"").unwrap();
    graph.set_detail(save, "line\n2").unwrap();
    let quoted = from_text(&to_text(&graph)).unwrap();
    assert_eq!(quoted.get(save).unwrap().name(), "say \"hi\"");
    assert_eq!(quoted.get(save).unwrap().detail(), "line\n2");

    graph.delete(save).unwrap();
    let kept = from_text(&to_text(&graph)).unwrap();
    assert!(kept.get(save).is_none());
    assert!(kept.get(open).is_some());
    assert_eq!(kept.connections().count(), 0);
    let mut kept = kept;
    assert_eq!(
        kept.create(FunctionKind::Value, "", "", 0.0, 0.0),
        FunctionId(3)
    );

    let mut dropped_tail = Graph::new();
    let first = dropped_tail.create(FunctionKind::Trigger, "", "", 0.0, 0.0);
    let second = dropped_tail.create(FunctionKind::Trigger, "", "", 0.0, 0.0);
    dropped_tail.delete(second).unwrap();
    let mut reloaded = from_text(&to_text(&dropped_tail)).unwrap();
    assert!(reloaded.get(first).is_some());
    assert_eq!(
        reloaded.create(FunctionKind::Trigger, "", "", 0.0, 0.0),
        FunctionId(2)
    );

    let gap = from_text(
        "\
uii-function 1
fn 2 trigger 0 0 \"\" \"\" \"\"
port 2 1 out \"when\"
",
    )
    .unwrap();
    assert!(gap.get(FunctionId(1)).is_none());
    assert!(gap.get(FunctionId(2)).is_some());
    let mut gap = gap;
    assert_eq!(
        gap.create(FunctionKind::Trigger, "", "", 0.0, 0.0),
        FunctionId(3)
    );

    let mut ports = from_text(
        "\
uii-function 1
fn 1 trigger 0 0 \"\" \"\" \"\"
port 1 3 out \"when\"
",
    )
    .unwrap();
    assert_eq!(
        ports
            .get(FunctionId(1))
            .unwrap()
            .ports()
            .map(|port| port.id)
            .collect::<Vec<_>>(),
        vec![PortId(3)]
    );
    assert_eq!(
        ports
            .add_port(FunctionId(1), Direction::Out, "next")
            .unwrap(),
        PortId(4)
    );

    let empty = from_text("uii-function 1\n").unwrap();
    assert_eq!(empty.len(), 0);
    assert_eq!(
        from_text("nope").unwrap_err().to_string(),
        "expected \"uii-function 1\""
    );
    assert!(from_text("uii-function 1\nbogus\n")
        .unwrap_err()
        .to_string()
        .contains("line 2"));

    let err = from_text(
        "\
uii-function 1
fn 1 action 0 0 \"\" \"A\" \"\"
port 1 1 in \"run\"
port 1 2 out \"then\"
fn 2 action 10 0 \"\" \"B\" \"\"
port 2 1 in \"run\"
port 2 2 out \"then\"
wire 1 1 2 2 1
wire 2 2 2 1 1
",
    )
    .unwrap_err();
    let message = err.to_string();
    assert!(message.contains("loop"), "{message}");
    assert!(message.contains('9'), "{message}");
}

#[test]
fn default_graph_path_uses_the_data_home_then_home() {
    assert_eq!(
        default_graph_path(Some(Path::new("/tmp/data")), None),
        Path::new("/tmp/data/UII/function.graph")
    );
    assert_eq!(
        default_graph_path(None, Some(Path::new("/home/owl"))),
        Path::new("/home/owl/.local/share/UII/function.graph")
    );
    assert_eq!(default_graph_path(None, None), Path::new("function.graph"));
}
