use uii_core::{resize_from_handle, Document, Handle, NodeKind, Rect};

#[test]
fn create_move_resize_delete_and_hit_test() {
    let mut doc = Document::new("Untitled");
    let frame = doc.create(
        None,
        NodeKind::Frame,
        "Frame",
        Rect::new(10.0, 20.0, 100.0, 80.0),
    );
    let text = doc.create(
        Some(frame),
        NodeKind::Text,
        "Label",
        Rect::new(5.0, 6.0, 40.0, 16.0),
    );
    doc.set_text(text, "Hi");
    doc.move_node(frame, 30.0, 40.0);
    assert_eq!(doc.node(frame).rect.x, 30.0);
    assert_eq!(doc.node(frame).rect.y, 40.0);
    doc.resize_node(text, 0.0, -4.0);
    assert_eq!(doc.node(text).rect.width, 1.0);
    assert_eq!(doc.node(text).rect.height, 1.0);

    let hit = doc.hit_test(30.0 + 5.0 + 0.4, 40.0 + 6.0 + 0.4);
    assert_eq!(hit, Some(text));
    assert_eq!(doc.hit_test(0.0, 0.0), None);

    let ell = doc.create(
        None,
        NodeKind::Ellipse,
        "Dot",
        Rect::new(200.0, 200.0, 100.0, 100.0),
    );
    assert_eq!(doc.hit_test(200.0, 200.0), None);
    assert_eq!(doc.hit_test(250.0, 250.0), Some(ell));

    doc.delete(frame);
    assert!(doc.get(frame).is_none());
    assert!(doc.get(text).is_none());
    assert!(doc.get(ell).is_some());
}

#[test]
fn west_handle_moves_the_origin_and_keeps_a_minimum_size() {
    let next = resize_from_handle(Rect::new(10.0, 10.0, 40.0, 20.0), Handle::West, 15.0, 0.0);
    assert_eq!(next.x, 25.0);
    assert_eq!(next.width, 25.0);
    assert_eq!(next.y, 10.0);
    assert_eq!(next.height, 20.0);

    let clamped = resize_from_handle(Rect::new(10.0, 10.0, 40.0, 20.0), Handle::East, -80.0, 0.0);
    assert_eq!(clamped.width, 1.0);
    assert_eq!(clamped.x, 10.0);
}
