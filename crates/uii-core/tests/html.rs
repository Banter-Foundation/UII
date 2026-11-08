use uii_core::{import_html_css, DockSlot, NodeKind};

#[test]
fn html_and_css_become_dock_child_text_button_table_and_image() {
    let html = r#"
        <style>
          header.bar { width: 400px; height: 40px; background: #112233; color: #ffffff; }
          main.stage { width: 400px; height: 200px; background: #eeeeee; }
          div.card { width: 120px; height: 64px; background: #ffffff; margin: 8px; }
          p { font-size: 16px; color: #222222; }
        </style>
        <header class="bar">Title</header>
        <main class="stage">
          <div class="card">
            <p>Hello</p>
            <button>Save</button>
          </div>
          <img alt="Logo" src="logo.png" width="32" height="32">
          <table>
            <tr><th>Name</th><th>Role</th></tr>
            <tr><td>Ada</td><td>Admin</td></tr>
          </table>
        </main>
    "#;
    let doc = import_html_css(html, "");
    let header = doc.find_named("bar").expect("header");
    assert_eq!(header.kind, NodeKind::Dock);
    assert_eq!(header.dock_slot, Some(DockSlot::Top));
    assert_eq!(header.rect.width, 400.0);
    assert_eq!(header.rect.height, 40.0);
    assert_eq!(header.fill, uii_core::Rgba::rgb(0x11, 0x22, 0x33));
    let title = doc
        .children(header.id)
        .iter()
        .map(|id| doc.node(*id))
        .find(|node| node.kind == NodeKind::Text)
        .expect("title");
    assert_eq!(title.text, "Title");
    assert_eq!(title.color, uii_core::Rgba::rgb(0xff, 0xff, 0xff));

    let stage = doc.find_named("stage").expect("main");
    assert_eq!(stage.kind, NodeKind::Dock);
    assert_eq!(stage.dock_slot, Some(DockSlot::Center));
    assert_eq!(stage.rect, uii_core::Rect::new(0.0, 40.0, 400.0, 200.0));

    let card = doc.find_named("card").expect("card");
    assert_eq!(card.kind, NodeKind::Child);
    assert_eq!(card.parent, Some(stage.id));
    assert_eq!(card.rect, uii_core::Rect::new(8.0, 8.0, 120.0, 64.0));
    assert_eq!(card.fill, uii_core::Rgba::rgb(0xff, 0xff, 0xff));

    let hello = doc.find_named("Hello").expect("hello");
    assert_eq!(hello.kind, NodeKind::Text);
    assert_eq!(hello.parent, Some(card.id));
    assert_eq!(hello.text, "Hello");
    assert_eq!(hello.rect.x, 0.0);
    assert_eq!(hello.rect.y, 0.0);
    assert_eq!(hello.rect.width, 45.0);
    assert_eq!(hello.rect.height, 23.0);
    assert_eq!(hello.color, uii_core::Rgba::rgb(0x22, 0x22, 0x22));

    let save = doc.find_named("Save").expect("button");
    assert_eq!(save.kind, NodeKind::Button);
    assert_eq!(save.text, "Save");
    assert_eq!(save.parent, Some(card.id));
    assert_eq!(save.rect.y, 23.0);

    let logo = doc.find_named("Logo").expect("image");
    assert_eq!(logo.kind, NodeKind::Image);
    assert_eq!(logo.image_src, "logo.png");
    assert_eq!(logo.rect.width, 32.0);
    assert_eq!(logo.rect.height, 32.0);
    assert_eq!(logo.parent, Some(stage.id));

    let table = doc
        .nodes()
        .find(|node| node.kind == NodeKind::Table)
        .expect("table");
    assert_eq!(table.columns, vec!["Name".to_string(), "Role".to_string()]);
    assert_eq!(
        table.rows,
        vec![vec!["Ada".to_string(), "Admin".to_string()]]
    );
    assert_eq!(table.parent, Some(stage.id));
}

#[test]
fn inline_style_beats_the_stylesheet_and_a_circle_is_an_ellipse() {
    let html = r#"
        <div class="box" style="width: 40px; height: 40px; border-radius: 50%; background: #ff0000;"></div>
    "#;
    let css = ".box { width: 10px; height: 10px; background: #00ff00; }";
    let doc = import_html_css(html, css);
    let node = doc.nodes().next().expect("shape");
    assert_eq!(node.kind, NodeKind::Ellipse);
    assert_eq!(node.rect.width, 40.0);
    assert_eq!(node.rect.height, 40.0);
    assert_eq!(node.fill, uii_core::Rgba::rgb(0xff, 0, 0));
}

#[test]
fn scripts_are_not_nodes() {
    let doc = import_html_css("<script>alert(1)</script><p>Hi</p>", "");
    assert!(doc.nodes().all(|node| node.kind == NodeKind::Text));
    assert_eq!(doc.nodes().next().unwrap().text, "Hi");
}
