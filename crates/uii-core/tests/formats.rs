use uii_core::{
    export_source, formats, import_html_css, import_source, DockSlot, Document, NodeKind, Rect,
};

fn sample() -> Document {
    let mut doc = Document::new("Sample");
    let header = doc.create(None, NodeKind::Dock, "header", Rect::new(0.0, 0.0, 800.0, 48.0));
    doc.set_dock_slot(header, DockSlot::Top);
    let title = doc.create(
        Some(header),
        NodeKind::Text,
        "title",
        Rect::new(8.0, 8.0, 80.0, 24.0),
    );
    doc.set_text(title, "Title");
    let main = doc.create(
        None,
        NodeKind::Dock,
        "main",
        Rect::new(0.0, 48.0, 800.0, 400.0),
    );
    doc.set_dock_slot(main, DockSlot::Center);
    let card = doc.create(
        Some(main),
        NodeKind::Child,
        "card",
        Rect::new(16.0, 16.0, 200.0, 120.0),
    );
    let hello = doc.create(
        Some(card),
        NodeKind::Text,
        "hello",
        Rect::new(8.0, 8.0, 64.0, 20.0),
    );
    doc.set_text(hello, "Hello");
    let save = doc.create(
        Some(card),
        NodeKind::Button,
        "save",
        Rect::new(8.0, 36.0, 72.0, 28.0),
    );
    doc.set_text(save, "Save");
    let table = doc.create(
        Some(main),
        NodeKind::Table,
        "people",
        Rect::new(16.0, 150.0, 240.0, 80.0),
    );
    doc.set_table(
        table,
        vec!["Name".into(), "Role".into()],
        vec![vec!["Ada".into(), "Admin".into()]],
    );
    let image = doc.create(
        Some(main),
        NodeKind::Image,
        "logo",
        Rect::new(16.0, 240.0, 32.0, 32.0),
    );
    doc.set_image(image, "logo.png");
    doc.create(
        Some(main),
        NodeKind::Rectangle,
        "shade",
        Rect::new(260.0, 16.0, 40.0, 40.0),
    );
    doc.create(
        Some(main),
        NodeKind::Ellipse,
        "dot",
        Rect::new(310.0, 16.0, 24.0, 24.0),
    );
    doc.create(
        None,
        NodeKind::Frame,
        "palette",
        Rect::new(820.0, 0.0, 120.0, 80.0),
    );
    doc
}

fn assert_same_tree(left: &Document, right: &Document) {
    fn walk(doc: &Document, id: uii_core::NodeId, out: &mut Vec<String>) {
        let node = doc.node(id);
        out.push(format!(
            "{:?}|{}|{:?}|{}|{}|{}|{}|{}|{}|{}|{:?}",
            node.kind,
            node.name,
            node.dock_slot,
            node.text,
            node.image_src,
            node.rect.x,
            node.rect.y,
            node.rect.width,
            node.rect.height,
            node.columns.join(","),
            node.rows
        ));
        for child in doc.children(id) {
            walk(doc, *child, out);
        }
    }
    let mut a = Vec::new();
    let mut b = Vec::new();
    for id in left.roots() {
        walk(left, *id, &mut a);
    }
    for id in right.roots() {
        walk(right, *id, &mut b);
    }
    assert_eq!(a, b);
}

#[test]
fn registry_lists_html_and_every_toolkit() {
    let ids: Vec<_> = formats().iter().map(|format| format.id).collect();
    assert_eq!(
        ids,
        vec![
            "html-css",
            "react",
            "vue",
            "svelte",
            "angular",
            "banter-imgui",
            "raven-imgui",
            "imgui-cpp",
            "egui",
            "slint",
            "qt-widgets",
            "winui",
            "swiftui",
            "gtk",
            "compose",
        ]
    );
    for format in formats() {
        assert!(format.import && format.export, "{}", format.id);
    }
}

#[test]
fn html_css_export_contains_markup_and_a_stylesheet() {
    let mut doc = Document::new("X");
    doc.create(
        None,
        NodeKind::Frame,
        "Box",
        Rect::new(1.0, 2.0, 30.0, 40.0),
    );
    let source = export_source(&doc, "html-css").unwrap();
    assert!(source.contains("<style>"));
    assert!(source.contains("data-uii-name=\"Box\""));
    let back = import_html_css(&source, "");
    assert_eq!(
        back.find_named("Box").unwrap().rect,
        Rect::new(1.0, 2.0, 30.0, 40.0)
    );
}

#[test]
fn every_export_round_trips_docking_tables_children_and_shapes() {
    let original = sample();
    for format in formats() {
        if !format.export {
            continue;
        }
        let source = export_source(&original, format.id).unwrap_or_else(|err| {
            panic!("{} export failed: {}", format.id, err.message)
        });
        match format.id {
            "html-css" => {
                assert!(source.contains("<style>"));
                assert!(source.contains("data-uii"));
            }
            "react" => {
                assert!(source.contains("export function"));
                assert!(source.contains("className"));
                assert!(source.contains("Design.css"));
            }
            "vue" => {
                assert!(source.contains("<template>"));
                assert!(source.contains("<style"));
            }
            "svelte" => {
                assert!(source.contains("<script"));
                assert!(source.contains("<style"));
            }
            "angular" => {
                assert!(source.contains("@Component"));
                assert!(source.contains("template"));
            }
            "banter-imgui" => {
                assert!(source.contains("dear_imgui_rs"));
                assert!(source.contains("DockLayout"));
                assert!(source.contains("child_window"));
                assert!(source.contains("begin_table"));
                assert!(!source.contains("raven::"));
            }
            "raven-imgui" => {
                assert!(source.contains("dear-imgui-rs 0.22"));
                assert!(source.contains("oscar-soirey/Raven"));
                assert!(source.contains("window_from_xml"));
                assert!(source.contains("<DockArea"));
                assert!(source.contains("Slot.Dock=\"Top\""));
                assert!(source.contains("<Table"));
                assert!(source.contains("<VBox"));
                assert!(source.contains("dear_imgui_rs"));
                assert!(source.contains("DockLayout"));
                assert!(source.contains("begin_table"));
                assert!(!source.contains("R_BACKEND_VULKAN"));
            }
            "imgui-cpp" => {
                assert!(source.contains("DockBuilderSplitNode") || source.contains("DockSpace"));
                assert!(source.contains("BeginChild"));
                assert!(source.contains("BeginTable"));
            }
            "egui" => {
                assert!(source.contains("egui_dock"));
                assert!(source.contains("split_above") || source.contains("DockState"));
            }
            "slint" => {
                assert!(source.contains("inherits Window"));
                assert!(source.contains("StandardTableView"));
            }
            "qt-widgets" => {
                assert!(source.contains("QDockWidget"));
                assert!(source.contains("QTableWidget"));
                assert!(source.contains("Qt::TopDockWidgetArea"));
                assert!(source.contains("QWidget"));
            }
            "winui" => {
                assert!(source.contains("DockPanel"));
            }
            "swiftui" => {
                assert!(source.contains("NavigationSplitView"));
                assert!(source.contains("Table"));
            }
            "gtk" => {
                assert!(source.contains("gtk_paned_new"));
                assert!(source.contains("gtk_grid_attach") || source.contains("gtk_grid_new"));
            }
            "compose" => {
                assert!(source.contains("Scaffold"));
            }
            other => panic!("unexpected export {other}"),
        }
        let imported = import_source(format.id, &source).unwrap_or_else(|err| {
            panic!("{} import failed: {}\n{source}", format.id, err.message)
        });
        assert_same_tree(&original, &imported);
    }
}

#[test]
fn handwritten_banter_imgui_imports() {
    let src = r#"
        let header = WindowKey::new("header", "Header").unwrap();
        let layout = DockLayout::tabs([&header]);
        ui.dockspace().layout(&layout, DockLayoutApply::IfMissing).build().ok();
        ui.window(&header)
            .position([1.0, 2.0], Condition::Always)
            .size([300.0, 50.0], Condition::Always)
            .build(|| {
                ui.set_cursor_pos([4.0, 5.0]);
                ui.child_window("card").size([80.0, 30.0]).build(ui, || {
                    ui.text("Hello");
                });
                ui.set_cursor_pos([0.0, 40.0]);
                if ui.begin_table("people", 2).is_some() {
                    ui.table_setup_column("Name");
                    ui.table_setup_column("Role");
                    ui.table_next_row();
                    ui.table_next_column();
                    ui.text("Ada");
                    ui.table_next_column();
                    ui.text("Admin");
                }
                ui.set_cursor_pos([90.0, 6.0]);
                let _ = ui.button_with_size("Save", [60.0, 22.0]);
            });
    "#;
    let doc = import_source("banter-imgui", src).unwrap();
    let header = doc.find_named("Header").unwrap();
    assert_eq!(header.kind, NodeKind::Dock);
    assert_eq!(header.dock_slot, Some(DockSlot::Center));
    assert_eq!(header.rect, Rect::new(1.0, 2.0, 300.0, 50.0));
    let card = doc.find_named("card").unwrap();
    assert_eq!(card.kind, NodeKind::Child);
    assert_eq!(card.rect, Rect::new(4.0, 5.0, 80.0, 30.0));
    assert_eq!(doc.find_named("Hello").unwrap().text, "Hello");
    let table = doc.find_named("people").unwrap();
    assert_eq!(table.kind, NodeKind::Table);
    assert_eq!(table.columns, vec!["Name".to_string(), "Role".to_string()]);
    assert_eq!(table.rows, vec![vec!["Ada".to_string(), "Admin".to_string()]]);
    assert_eq!(table.rect.x, 0.0);
    assert_eq!(table.rect.y, 40.0);
    let save = doc.find_named("Save").unwrap();
    assert_eq!(save.kind, NodeKind::Button);
    assert_eq!(save.rect, Rect::new(90.0, 6.0, 60.0, 22.0));
}

#[test]
fn handwritten_raven_imgui_imports() {
    let src = r##"
        use raven::window_from_xml;
        let _window = unsafe { window_from_xml(r#"
            <Window Title="Design">
              <DockArea Name="Header" Slot.Dock="Top" X="0" Y="0" Width="400" Height="48">
                <Label Name="title" Text="Title" X="8" Y="8" Width="40" Height="16"/>
              </DockArea>
              <VBox Name="Card" UiiKind="child" X="10" Y="60" Width="100" Height="40">
                <Button Name="Save" Text="Save" X="4" Y="4" Width="50" Height="20"/>
              </VBox>
              <Table Name="People" X="0" Y="110" Width="180" Height="60">
                <Column Header="Name"/>
                <Column Header="Role"/>
                <Row>Ada|Admin</Row>
              </Table>
              <Image Name="Logo" Source="logo.png" X="0" Y="180" Width="16" Height="16"/>
            </Window>
        "#) };
    "##;
    let doc = import_source("raven-imgui", src).unwrap();
    let header = doc.find_named("Header").unwrap();
    assert_eq!(header.kind, NodeKind::Dock);
    assert_eq!(header.dock_slot, Some(DockSlot::Top));
    assert_eq!(doc.find_named("title").unwrap().text, "Title");
    assert_eq!(doc.find_named("Card").unwrap().kind, NodeKind::Child);
    assert_eq!(doc.find_named("Save").unwrap().kind, NodeKind::Button);
    let table = doc.find_named("People").unwrap();
    assert_eq!(table.columns, vec!["Name".to_string(), "Role".to_string()]);
    assert_eq!(table.rows, vec![vec!["Ada".to_string(), "Admin".to_string()]]);
    assert_eq!(doc.find_named("Logo").unwrap().image_src, "logo.png");
}

#[test]
fn html_import_then_banter_export_keeps_the_table_and_the_dock() {
    let doc = import_html_css(
        r#"<header>Top</header><main><table><tr><th>A</th></tr><tr><td>B</td></tr></table></main>"#,
        "header { width: 200px; height: 30px; } main { width: 200px; height: 80px; }",
    );
    let source = export_source(&doc, "banter-imgui").unwrap();
    let back = import_source("banter-imgui", &source).unwrap();
    assert!(back.nodes().any(|node| node.kind == NodeKind::Dock && node.dock_slot == Some(DockSlot::Top)));
    assert!(back.nodes().any(|node| node.kind == NodeKind::Table && node.columns == ["A"] && node.rows == [vec!["B".to_string()]]));
}
