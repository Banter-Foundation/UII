//! Layout-tool behavior the Design canvas has to keep: multi-select, snap,
//! undo, clipping frames, real type size, corner radius, and the shortcut list.

use uii_core::{
    command_for, format_color, shortcuts, snap_rect, Bind, BindKey, Command, Easing, Editor,
    Motion, NodeKind, Rect, Rgba, Tool, WorkspaceTab, MARKUP_STARTS_CLOSED, SNAP_PX,
};

#[test]
fn design_tabs_include_function_and_markup_starts_closed() {
    let labels: Vec<_> = Editor::tabs().iter().map(|tab| tab.label()).collect();
    assert_eq!(labels, vec!["Design", "Animation", "Function"]);
    assert!(MARKUP_STARTS_CLOSED);
    assert_eq!(Editor::new().tab(), WorkspaceTab::Design);
}

#[test]
fn properties_stay_available_on_the_select_tool() {
    let mut editor = Editor::new();
    editor.set_tool(Tool::Select);
    let id = editor.add_shape(
        NodeKind::Rectangle,
        "Shade",
        Rect::new(12.0, 18.0, 40.0, 50.0),
    );
    editor.set_fill(id, Rgba::rgba(0x11, 0x22, 0x33, 0x80));
    editor.finish_edit();
    let props = editor.properties().expect("selection");
    assert_eq!(props.name, "Shade");
    assert_eq!(props.kind, "rectangle");
    assert_eq!(props.x, 12.0);
    assert_eq!(props.y, 18.0);
    assert_eq!(props.width, 40.0);
    assert_eq!(props.height, 50.0);
    assert_eq!(props.fill, "#11223380");
    assert_eq!(format_color(Rgba::rgb(0x11, 0x22, 0x33)), "#112233");
}

#[test]
fn multi_select_moves_together_without_double_moving_a_child() {
    let mut editor = Editor::new();
    let frame = editor.add_shape(NodeKind::Frame, "Card", Rect::new(0.0, 0.0, 80.0, 40.0));
    editor.select(frame);
    let child = editor.add_shape(NodeKind::Rectangle, "Dot", Rect::new(10.0, 8.0, 12.0, 12.0));
    assert_eq!(editor.document().node(child).parent, Some(frame));
    editor.select(frame);
    editor.add_to_selection(child);
    assert_eq!(editor.selection().len(), 2);
    editor.move_selection_by(5.0, 0.0);
    editor.finish_edit();
    assert_eq!(editor.document().node(frame).rect.x, 5.0);
    assert!((editor.document().absolute_rect(child).x - 15.0).abs() < 0.01);
}

#[test]
fn moving_snaps_an_edge_and_a_center_into_alignment() {
    let mut editor = Editor::new();
    let _keep = editor.add_shape(
        NodeKind::Rectangle,
        "Keep",
        Rect::new(0.0, 0.0, 100.0, 40.0),
    );
    let moved = editor.add_shape(
        NodeKind::Rectangle,
        "Moved",
        Rect::new(200.0, 0.0, 50.0, 20.0),
    );
    let (snapped, guides) = snap_rect(
        editor.document(),
        &[moved],
        Rect::new(104.0, 0.0, 50.0, 20.0),
        SNAP_PX,
    );
    assert_eq!(
        snapped.x, 100.0,
        "left edge aligns to the neighbor's right edge"
    );
    assert!(
        guides
            .iter()
            .any(|guide| guide.vertical && (guide.at - 100.0).abs() < 0.01),
        "{guides:?}"
    );
    let (centered, center_guides) = snap_rect(
        editor.document(),
        &[moved],
        Rect::new(33.0, 80.0, 40.0, 10.0),
        SNAP_PX,
    );
    assert!(
        (centered.x - 30.0).abs() < 0.01,
        "center aligns, x={}",
        centered.x
    );
    assert!(
        center_guides
            .iter()
            .any(|guide| guide.vertical && (guide.at - 50.0).abs() < 0.01),
        "{center_guides:?}"
    );
}

#[test]
fn undo_and_redo_restore_a_move_and_a_delete() {
    let mut editor = Editor::new();
    let id = editor.add_shape(NodeKind::Frame, "Box", Rect::new(0.0, 0.0, 10.0, 10.0));
    editor.move_node(id, 40.0, 6.0);
    editor.finish_edit();
    assert!(editor.undo());
    assert_eq!(editor.document().node(id).rect.x, 0.0);
    assert!(editor.redo());
    assert_eq!(editor.document().node(id).rect.x, 40.0);
    editor.select(id);
    editor.delete_selected();
    assert!(editor.document().get(id).is_none());
    assert!(editor.undo());
    assert_eq!(editor.document().node(id).rect.x, 40.0);
}

#[test]
fn frames_clip_children_and_can_nest() {
    let mut editor = Editor::new();
    let outer = editor.add_shape(NodeKind::Frame, "Outer", Rect::new(0.0, 0.0, 100.0, 100.0));
    editor.select(outer);
    let inner = editor.add_shape(NodeKind::Frame, "Inner", Rect::new(10.0, 10.0, 40.0, 40.0));
    assert_eq!(editor.document().node(inner).parent, Some(outer));
    assert_eq!(editor.document().node(inner).rect.x, 10.0);
    editor.select(inner);
    let deep = editor.add_shape(
        NodeKind::Rectangle,
        "Deep",
        Rect::new(20.0, 20.0, 10.0, 10.0),
    );
    assert_eq!(editor.document().node(deep).parent, Some(inner));
    assert_eq!(editor.document().hit_test(25.0, 25.0), Some(deep));
    editor.move_node(deep, 200.0, 200.0);
    editor.finish_edit();
    assert!(
        editor.document().hit_test(215.0, 215.0).is_none(),
        "a child drawn outside its frame is clipped"
    );
    editor.clear_selection();
    let loose = editor.add_shape(
        NodeKind::Rectangle,
        "Loose",
        Rect::new(300.0, 300.0, 20.0, 20.0),
    );
    editor.select(loose);
    editor.move_node(loose, 60.0, 60.0);
    editor.finish_edit();
    editor.drop_selection();
    assert_eq!(editor.document().node(loose).parent, Some(outer));
    assert!((editor.document().absolute_rect(loose).x - 60.0).abs() < 0.01);
}

#[test]
fn text_uses_its_font_size_and_shapes_keep_fill_and_corner_radius() {
    let mut editor = Editor::new();
    let text = editor.add_shape(NodeKind::Text, "Hello", Rect::new(0.0, 0.0, 80.0, 28.0));
    editor.set_font_size(text, 32.0);
    editor.finish_edit();
    let shape = editor.add_shape(
        NodeKind::Rectangle,
        "Plate",
        Rect::new(0.0, 40.0, 80.0, 40.0),
    );
    editor.set_fill(shape, Rgba::rgb(0xcc, 0x33, 0x44));
    editor.set_corner_radius(shape, 8.0);
    editor.finish_edit();
    assert_eq!(editor.document().node(text).font_size, 32.0);
    assert_eq!(editor.properties_for(text).unwrap().font_size, Some(32.0));
    assert_eq!(editor.document().node(shape).corner_radius, 8.0);
    assert_eq!(
        editor.document().node(shape).fill,
        Rgba::rgb(0xcc, 0x33, 0x44)
    );
    let (html, css) = {
        let (html, css) = editor.markup();
        (html.to_string(), css.to_string())
    };
    assert!(css.contains("font-size: 32px"), "{css}");
    assert!(css.contains("border-radius: 8px"), "{css}");
    assert!(html.contains("background-color:#cc3344"), "{html}");
    editor.apply_markup(&html, &css);
    let again = editor.document().find_named("Hello").expect("text");
    assert_eq!(again.font_size, 32.0);
    let plate = editor.document().find_named("Plate").expect("shape");
    assert_eq!(plate.corner_radius, 8.0);
    assert_eq!(plate.fill, Rgba::rgb(0xcc, 0x33, 0x44));
}

#[test]
fn group_copy_paste_duplicate_select_all_and_delete() {
    let mut editor = Editor::new();
    let a = editor.add_shape(NodeKind::Rectangle, "A", Rect::new(0.0, 0.0, 20.0, 10.0));
    let b = editor.add_shape(NodeKind::Rectangle, "B", Rect::new(30.0, 0.0, 20.0, 10.0));
    editor.select(a);
    editor.add_to_selection(b);
    let group = editor.group().expect("group");
    assert_eq!(editor.document().node(group).kind, NodeKind::Frame);
    assert_eq!(editor.document().node(a).parent, Some(group));
    assert_eq!(editor.document().node(b).parent, Some(group));
    editor.select_all();
    assert!(editor.selection().len() >= 3);
    editor.select(a);
    editor.copy();
    let before = editor.document().nodes().count();
    editor.paste();
    assert!(editor.document().nodes().count() > before);
    editor.select(b);
    let count = editor.document().nodes().count();
    editor.duplicate();
    assert_eq!(editor.document().nodes().count(), count + 1);
    let copied = editor.selected().expect("duplicate");
    assert_ne!(copied, b);
    editor.delete_selected();
    assert!(editor.document().get(copied).is_none());
    assert!(editor.document().get(b).is_some());
}

#[test]
fn shortcuts_cover_the_usual_layout_commands() {
    let want = [
        (Command::Undo, "Undo", "Ctrl+Z"),
        (Command::Redo, "Redo", "Ctrl+Shift+Z"),
        (Command::Copy, "Copy", "Ctrl+C"),
        (Command::Paste, "Paste", "Ctrl+V"),
        (Command::Duplicate, "Duplicate", "Ctrl+D"),
        (Command::Delete, "Delete", "Delete"),
        (Command::SelectAll, "Select all", "Ctrl+A"),
        (Command::Group, "Group", "Ctrl+G"),
        (Command::ZoomIn, "Zoom in", "Ctrl+="),
        (Command::ZoomOut, "Zoom out", "Ctrl+-"),
        (Command::Fit, "Fit", "Shift+1"),
    ];
    for (command, label, keys) in want {
        assert!(
            shortcuts().iter().any(|shortcut| {
                shortcut.command == command && shortcut.label == label && shortcut.keys == keys
            }),
            "{label}"
        );
    }
    assert_eq!(
        command_for(Bind {
            ctrl: true,
            shift: false,
            key: BindKey::Z
        }),
        Some(Command::Undo)
    );
    assert_eq!(
        command_for(Bind {
            ctrl: true,
            shift: true,
            key: BindKey::Z
        }),
        Some(Command::Redo)
    );
    assert_eq!(
        command_for(Bind {
            ctrl: true,
            shift: false,
            key: BindKey::Y
        }),
        Some(Command::Redo)
    );
    assert_eq!(
        command_for(Bind {
            ctrl: true,
            shift: false,
            key: BindKey::C
        }),
        Some(Command::Copy)
    );
    assert_eq!(
        command_for(Bind {
            ctrl: true,
            shift: false,
            key: BindKey::V
        }),
        Some(Command::Paste)
    );
    assert_eq!(
        command_for(Bind {
            ctrl: true,
            shift: false,
            key: BindKey::D
        }),
        Some(Command::Duplicate)
    );
    assert_eq!(
        command_for(Bind {
            ctrl: false,
            shift: false,
            key: BindKey::Delete
        }),
        Some(Command::Delete)
    );
    assert_eq!(
        command_for(Bind {
            ctrl: false,
            shift: false,
            key: BindKey::Backspace
        }),
        Some(Command::Delete)
    );
    assert_eq!(
        command_for(Bind {
            ctrl: true,
            shift: false,
            key: BindKey::A
        }),
        Some(Command::SelectAll)
    );
    assert_eq!(
        command_for(Bind {
            ctrl: true,
            shift: false,
            key: BindKey::G
        }),
        Some(Command::Group)
    );
    assert_eq!(
        command_for(Bind {
            ctrl: true,
            shift: false,
            key: BindKey::Equal
        }),
        Some(Command::ZoomIn)
    );
    assert_eq!(
        command_for(Bind {
            ctrl: true,
            shift: true,
            key: BindKey::Equal
        }),
        Some(Command::ZoomIn)
    );
    assert_eq!(
        command_for(Bind {
            ctrl: true,
            shift: false,
            key: BindKey::Minus
        }),
        Some(Command::ZoomOut)
    );
    assert_eq!(
        command_for(Bind {
            ctrl: false,
            shift: true,
            key: BindKey::Digit1
        }),
        Some(Command::Fit)
    );
}

#[test]
fn zoom_and_fit_frame_the_document() {
    let mut editor = Editor::new();
    assert_eq!(editor.zoom(), 1.0);
    editor.zoom_in();
    assert!(editor.zoom() > 1.0);
    editor.zoom_out();
    editor.zoom_out();
    assert!(editor.zoom() < 1.0);
    editor.add_shape(NodeKind::Frame, "Stage", Rect::new(100.0, 80.0, 50.0, 40.0));
    editor.fit(200.0, 200.0);
    assert!(editor.zoom() > 0.1 && editor.zoom() <= 8.0);
    let pan = editor.pan();
    assert!(pan[0].is_finite() && pan[1].is_finite());
}

#[test]
fn animation_easing_can_hold_a_key() {
    let mut editor = Editor::new();
    let id = editor.add_shape(NodeKind::Frame, "Box", Rect::new(0.0, 0.0, 10.0, 10.0));
    editor.add_key(id, Motion::X, 0.0, 0.0);
    editor.add_key(id, Motion::X, 1.0, 100.0);
    editor.set_key_easing(id, Motion::X, 0.0, Easing::Hold);
    editor.set_time(0.5);
    assert!(editor.pose(id).x.abs() < 0.01, "{}", editor.pose(id).x);
    editor.set_time(1.0);
    assert!((editor.pose(id).x - 100.0).abs() < 0.01);
}

#[test]
fn content_drawer_toggles_on_ctrl_space_and_pin_keeps_it_open() {
    let mut editor = Editor::new();
    assert!(editor.drawer_collapsed());
    assert!(!editor.drawer_pinned());
    editor.poll_drawer_chord(true, false);
    assert!(!editor.drawer_collapsed());
    editor.poll_drawer_chord(true, false);
    assert!(!editor.drawer_collapsed());
    editor.poll_drawer_chord(false, false);
    editor.set_drawer_pinned(true);
    editor.dismiss_unpinned_drawer();
    assert!(!editor.drawer_collapsed());
    editor.set_drawer_pinned(false);
    editor.dismiss_unpinned_drawer();
    assert!(editor.drawer_collapsed());
    editor.poll_drawer_chord(true, true);
    assert!(editor.drawer_collapsed());
}

#[test]
fn commands_drive_the_same_actions_as_the_shortcut_list() {
    let mut editor = Editor::new();
    let id = editor.add_shape(NodeKind::Rectangle, "A", Rect::new(0.0, 0.0, 10.0, 10.0));
    editor.apply_command(Command::SelectAll, [200.0, 200.0]);
    assert_eq!(editor.selected(), Some(id));
    editor.apply_command(Command::Duplicate, [200.0, 200.0]);
    assert!(editor.document().nodes().count() >= 2);
    editor.apply_command(Command::Undo, [200.0, 200.0]);
    assert_eq!(editor.document().nodes().count(), 1);
    editor.apply_command(Command::ZoomIn, [200.0, 200.0]);
    assert!(editor.zoom() > 1.0);
    editor.apply_command(Command::Fit, [200.0, 200.0]);
    assert!(editor.zoom() > 0.0);
}
