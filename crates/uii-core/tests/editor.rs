use uii_core::{Editor, NodeKind, Rect, Tool, WorkspaceTab};

#[test]
fn top_tabs_are_design_animation_and_function() {
    let labels: Vec<_> = Editor::tabs().iter().map(|tab| tab.label()).collect();
    assert_eq!(labels, vec!["Design", "Animation", "Function"]);
    assert!(!labels.contains(&"Prototype"));
    assert!(!labels.contains(&"Inspect"));
    assert!(!labels.contains(&"Assets"));
    assert!(!labels.contains(&"Components"));
    assert_eq!(Editor::new().tab(), WorkspaceTab::Design);
    assert!(!WorkspaceTab::Design.is_placeholder());
    assert!(WorkspaceTab::Animation.is_placeholder());
    assert!(WorkspaceTab::Function.is_placeholder());
}

#[test]
fn design_tools_are_the_canvas_tools() {
    let labels: Vec<_> = Editor::tools().iter().map(|tool| tool.label()).collect();
    assert_eq!(
        labels,
        vec!["Select", "Frame", "Text", "Rectangle", "Ellipse"]
    );
    assert!(!labels.iter().any(|label| *label == "Inspect"));
    assert_eq!(Tool::Frame.kind(), Some(NodeKind::Frame));
    assert_eq!(Tool::Text.kind(), Some(NodeKind::Text));
    assert_eq!(Tool::Rectangle.kind(), Some(NodeKind::Rectangle));
    assert_eq!(Tool::Ellipse.kind(), Some(NodeKind::Ellipse));
    assert_eq!(Tool::Select.kind(), None);
}

#[test]
fn moving_on_the_design_canvas_rewrites_html() {
    let mut editor = Editor::new();
    editor.set_tab(WorkspaceTab::Design);
    let id = editor.add_shape(NodeKind::Frame, "Box", Rect::new(1.0, 2.0, 30.0, 40.0));
    editor.move_node(id, 5.0, 6.0);
    let (html, css) = editor.markup();
    assert!(html.contains("data-uii-name=\"Box\""), "{html}");
    assert!(html.contains("left:5px"), "{html}");
    assert!(html.contains("top:6px"), "{html}");
    assert!(css.contains("left: 5px"), "{css}");
}

#[test]
fn applying_markup_replaces_the_canvas() {
    let mut editor = Editor::new();
    editor.apply_markup(
        "<header class=\"bar\">Title</header>",
        "header.bar { height: 40px; }",
    );
    let (html, css) = editor.markup();
    assert!(html.contains("Title"), "{html}");
    assert!(css.contains("height: 40px") || html.contains("height:40px"), "{html}\n{css}");
    assert!(editor.document().find_named("bar").is_some() || editor.document().find_named("Title").is_some());
}

#[test]
fn a_new_design_is_empty() {
    let editor = Editor::new();
    assert!(editor.document().roots().is_empty());
    assert_eq!(editor.markup(), ("", ""));
    assert!(editor.selected().is_none());
}
