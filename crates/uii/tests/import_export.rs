//! Import/export contract. These checks were written before the converters.
//! They define the public call point and the mapping rules; do not weaken them
//! to match a thinner implementation.

use std::collections::BTreeMap;
use std::process::Command;

use uii::{
    call, export, import, Axis, Call, CallOutput, DockSide, Document, Error, Format, Node, NodeKind,
};

fn node(id: &str, kind: NodeKind, name: &str, text: &str) -> Node {
    Node {
        id: id.to_string(),
        kind,
        name: name.to_string(),
        text: text.to_string(),
        columns: Vec::new(),
        rows: Vec::new(),
        dock: None,
        axis: None,
        props: BTreeMap::new(),
        children: Vec::new(),
    }
}

fn sample() -> Document {
    let mut row = node("row-box", NodeKind::Box, "row-box", "");
    row.axis = Some(Axis::Horizontal);
    let mut draft = node("draft", NodeKind::TextInput, "draft", "hi");
    draft.props.insert("placeholder".into(), "Message".into());
    row.children = vec![node("send", NodeKind::Button, "send", "Send"), draft];

    let mut stage_box = node("stage-box", NodeKind::Box, "stage-box", "");
    stage_box.axis = Some(Axis::Vertical);
    stage_box.props.insert("gap".into(), "8".into());
    stage_box.children = vec![
        row,
        node("title", NodeKind::Label, "title", "Say \"hi\" & <ok>"),
    ];

    let mut table = node("people", NodeKind::Table, "people", "");
    table.columns = vec!["Name".into(), "Status".into()];
    table.rows = vec![
        vec!["Ada".into(), "online".into()],
        vec!["a|b".into(), "idle".into()],
    ];

    let mut rail = node("rail", NodeKind::Region, "rail", "");
    rail.dock = Some(DockSide::Left);
    rail.children = vec![node("servers", NodeKind::Label, "servers", "Servers")];

    let mut stage = node("stage", NodeKind::Region, "stage", "");
    stage.dock = Some(DockSide::Center);
    stage.children = vec![stage_box, table];

    let mut mute = node("mute", NodeKind::Checkbox, "mute", "Mute");
    mute.props.insert("checked".into(), "true".into());
    let mut volume = node("volume", NodeKind::Slider, "volume", "Volume");
    volume.props.insert("value".into(), "0.4".into());
    let mut members = node("members", NodeKind::Region, "members", "");
    members.dock = Some(DockSide::Right);
    members.children = vec![mute, volume];

    let mut dock = node("dock", NodeKind::Dock, "shell", "");
    dock.children = vec![rail, stage, members];

    let mut island = node("now", NodeKind::Island, "now", "Now playing");
    island.children = vec![node("track", NodeKind::Label, "track", "Clair")];

    let mut root = node("root", NodeKind::Window, "Shell", "");
    root.children = vec![
        dock,
        node("caption", NodeKind::HeaderBar, "caption", "Shell"),
        island,
        node("rule", NodeKind::Separator, "rule", ""),
        node("avatar", NodeKind::Image, "avatar", "avatar.png"),
    ];
    Document {
        name: "Shell".into(),
        root,
    }
}

fn round_trip(format: Format, document: &Document) -> Result<Document, Error> {
    let source = export(format, document)?;
    import(format, &source)
}

#[test]
fn format_names_cover_the_requested_set() {
    assert_eq!(Format::ALL.len(), 15);
    assert_eq!(Format::from_name("html/css").unwrap(), Format::HtmlCss);
    assert_eq!(Format::from_name("html-css").unwrap(), Format::HtmlCss);
    assert_eq!(
        Format::from_name("banter-imgui").unwrap(),
        Format::BanterImgui
    );
    assert_eq!(
        Format::from_name("custom-imgui").unwrap(),
        Format::RavenImgui
    );
    assert_eq!(
        Format::from_name("raven-imgui").unwrap(),
        Format::RavenImgui
    );
    assert_eq!(Format::from_name("dear-imgui").unwrap(), Format::DearImgui);
    assert_eq!(
        Format::from_name("jetpack-compose").unwrap(),
        Format::JetpackCompose
    );
    assert!(Format::from_name("electron").is_err());
}

#[test]
fn round_trip_every_format() {
    let document = sample();
    for format in Format::ALL {
        let back = round_trip(format, &document).unwrap_or_else(|err| {
            panic!("{format:?} round trip failed: {err}");
        });
        assert_eq!(back, document, "{format:?}");
    }
}

const FIXTURE: &str = r#"<!DOCTYPE html>
<html>
<head>
<meta charset="utf-8">
<title>Banter</title>
<style>
#compose { display: flex; flex-direction: column; gap: 8px; }
</style>
</head>
<body>
<header id="top">Banter</header>
<nav id="rail"><span id="servers">Servers</span></nav>
<main id="stage">
  <div id="compose">
    <button id="send">Send</button>
    <input id="draft" value="hi" placeholder="Message">
  </div>
  <table id="people">
    <thead><tr><th>Name</th><th>Status</th></tr></thead>
    <tbody>
      <tr><td>Ada</td><td>online</td></tr>
    </tbody>
  </table>
  <div id="outer"><div id="inner">Hi</div></div>
</main>
<aside id="members">Members</aside>
</body>
</html>
"#;

fn find<'a>(node: &'a Node, id: &str) -> &'a Node {
    if node.id == id {
        return node;
    }
    for child in &node.children {
        if let Some(found) = find_opt(child, id) {
            return found;
        }
    }
    panic!("missing node {id}");
}

fn find_opt<'a>(node: &'a Node, id: &str) -> Option<&'a Node> {
    if node.id == id {
        return Some(node);
    }
    for child in &node.children {
        if let Some(found) = find_opt(child, id) {
            return Some(found);
        }
    }
    None
}

fn imported_fixture() -> Document {
    import(Format::HtmlCss, FIXTURE).expect("fixture html")
}

#[test]
fn html_regions_tables_and_nested_boxes() {
    let doc = imported_fixture();
    assert_eq!(doc.name, "Banter");
    assert_eq!(doc.root.kind, NodeKind::Window);
    assert_eq!(doc.root.id, "window");
    assert_eq!(doc.root.children.len(), 1);
    let dock = &doc.root.children[0];
    assert_eq!(dock.kind, NodeKind::Dock);
    assert_eq!(dock.id, "dock-window");
    let sides: Vec<_> = dock
        .children
        .iter()
        .map(|child| (child.id.as_str(), child.kind, child.dock))
        .collect();
    assert_eq!(
        sides,
        vec![
            ("top", NodeKind::Region, Some(DockSide::Top)),
            ("rail", NodeKind::Region, Some(DockSide::Left)),
            ("stage", NodeKind::Region, Some(DockSide::Center)),
            ("members", NodeKind::Region, Some(DockSide::Right)),
        ]
    );
    assert_eq!(find(&doc.root, "top").text, "Banter");
    assert_eq!(find(&doc.root, "servers").kind, NodeKind::Label);
    assert_eq!(find(&doc.root, "servers").text, "Servers");
    assert_eq!(find(&doc.root, "members").text, "Members");

    let compose = find(&doc.root, "compose");
    assert_eq!(compose.kind, NodeKind::Box);
    assert_eq!(compose.axis, Some(Axis::Vertical));
    assert_eq!(compose.props.get("gap").map(String::as_str), Some("8px"));
    assert_eq!(compose.children[0].kind, NodeKind::Button);
    assert_eq!(compose.children[0].text, "Send");
    assert_eq!(compose.children[1].kind, NodeKind::TextInput);
    assert_eq!(compose.children[1].text, "hi");
    assert_eq!(
        compose.children[1]
            .props
            .get("placeholder")
            .map(String::as_str),
        Some("Message")
    );

    let people = find(&doc.root, "people");
    assert_eq!(people.kind, NodeKind::Table);
    assert_eq!(people.columns, vec!["Name", "Status"]);
    assert_eq!(people.rows, vec![vec!["Ada", "online"]]);

    let outer = find(&doc.root, "outer");
    let inner = find(&doc.root, "inner");
    assert_eq!(outer.kind, NodeKind::Box);
    assert_eq!(inner.kind, NodeKind::Box);
    assert_eq!(outer.axis, Some(Axis::Vertical));
    assert_eq!(inner.axis, Some(Axis::Vertical));
    assert_eq!(inner.text, "Hi");
    assert!(std::ptr::eq(inner, &outer.children[0]));
}

#[test]
fn html_fixture_round_trips_every_format() {
    let doc = imported_fixture();
    for format in Format::ALL {
        let back = round_trip(format, &doc).unwrap_or_else(|err| {
            panic!("{format:?} fixture round trip failed: {err}");
        });
        assert_eq!(back, doc, "{format:?}");
    }
}

#[test]
fn html_maps_onto_docking_tables_and_child_windows() {
    let doc = imported_fixture();

    let dear = export(Format::DearImgui, &doc).unwrap();
    assert!(dear.contains("ImGui::DockSpace"), "{dear}");
    assert!(dear.contains("ImGui::BeginTable"), "{dear}");
    assert!(
        dear.matches("ImGui::BeginChild").count() >= 3,
        "nested boxes become child windows:\n{dear}"
    );
    assert!(
        dear.contains("ImGui::DockBuilderDockWindow(\"rail\""),
        "{dear}"
    );
    assert!(dear.contains("ImGuiDir_Left"), "{dear}");
    assert!(dear.contains("ImGuiDir_Right"), "{dear}");
    assert!(dear.contains("ImGuiDir_Up"), "{dear}");
    let dear_back = import(Format::DearImgui, &dear).unwrap();
    assert_eq!(dear_back, doc);

    let banter = export(Format::BanterImgui, &doc).unwrap();
    assert!(
        banter.contains("ConfigFlags::DOCKING_ENABLE"),
        "Banter turns docking on before the first frame:\n{banter}"
    );
    assert!(banter.contains("ui.dockspace()"), "{banter}");
    assert!(banter.contains("DockLayout::tabs([\"rail\"])"), "{banter}");
    assert!(banter.contains("DockLayout::tabs([\"stage\"])"), "{banter}");
    assert!(
        banter.contains("DockLayout::tabs([\"members\"])"),
        "{banter}"
    );
    assert!(banter.contains("DockLayout::tabs([\"top\"])"), "{banter}");
    assert!(banter.contains("DockSplit::Left"), "{banter}");
    assert!(banter.contains("DockSplit::Right"), "{banter}");
    assert!(banter.contains("DockSplit::Up"), "{banter}");
    assert!(banter.contains("ui.table("), "{banter}");
    assert!(
        banter.matches("child_window(").count() >= 3,
        "nested boxes become child windows:\n{banter}"
    );
    assert!(banter.contains("layout::Region::Rail"), "{banter}");
    assert!(banter.contains("layout::Region::Stage"), "{banter}");
    assert!(banter.contains("layout::Region::Members"), "{banter}");
    assert!(banter.contains("dear_imgui_rs"), "{banter}");
    assert_eq!(import(Format::BanterImgui, &banter).unwrap(), doc);

    let raven = export(Format::RavenImgui, &doc).unwrap();
    assert!(raven.contains("#include <imgui.h>"), "{raven}");
    assert!(raven.contains("#include <Raven.h>"), "{raven}");
    assert!(raven.contains("ImGui::DockSpace"), "{raven}");
    assert!(raven.contains("ImGui::BeginTable"), "{raven}");
    assert!(raven.matches("ImGui::BeginChild").count() >= 3, "{raven}");
    assert!(raven.contains("Rdock_area_create"), "{raven}");
    assert!(raven.contains("Rdock_area_add_panel"), "{raven}");
    assert!(raven.contains("R_DOCK_LEFT"), "{raven}");
    assert!(raven.contains("R_DOCK_RIGHT"), "{raven}");
    assert!(raven.contains("R_DOCK_TOP"), "{raven}");
    assert!(raven.contains("R_WIDGET_TABLE"), "{raven}");
    assert!(raven.contains("Rtable_add_column"), "{raven}");
    assert!(raven.contains("Rtable_add_row"), "{raven}");
    assert!(raven.contains("R_WIDGET_VBOX"), "{raven}");
    assert!(!raven.contains("RavenBeginChild"), "{raven}");
    assert_eq!(import(Format::RavenImgui, &raven).unwrap(), doc);

    let qt = export(Format::Qt, &doc).unwrap();
    assert!(qt.contains("QDockWidget"), "{qt}");
    assert!(qt.contains("QTableWidget"), "{qt}");
    assert!(qt.contains("QVBoxLayout"), "{qt}");
    assert!(qt.contains("Qt::LeftDockWidgetArea"), "{qt}");
    assert!(qt.contains("Qt::RightDockWidgetArea"), "{qt}");
    assert!(qt.contains("Qt::TopDockWidgetArea"), "{qt}");
    assert!(!qt.contains("BeginChild"), "{qt}");
    assert_eq!(import(Format::Qt, &qt).unwrap(), doc);

    let egui = export(Format::Egui, &doc).unwrap();
    assert!(egui.contains("SidePanel::left"), "{egui}");
    assert!(egui.contains("SidePanel::right"), "{egui}");
    assert!(egui.contains("TopBottomPanel::top"), "{egui}");
    assert!(egui.contains("CentralPanel"), "{egui}");
    assert!(egui.contains("TableBuilder"), "{egui}");
    assert!(egui.contains("ui.vertical"), "{egui}");
    assert!(!egui.contains("BeginChild"), "{egui}");
    assert!(!egui.contains("child_window"), "{egui}");
    assert_eq!(import(Format::Egui, &egui).unwrap(), doc);
}

#[test]
fn targets_without_child_windows_do_not_emit_them() {
    let doc = imported_fixture();
    let plain = [
        Format::HtmlCss,
        Format::React,
        Format::Vue,
        Format::Svelte,
        Format::Angular,
        Format::Egui,
        Format::Slint,
        Format::WinUi,
        Format::SwiftUi,
        Format::Gtk,
        Format::JetpackCompose,
        Format::Qt,
    ];
    for format in plain {
        let source = export(format, &doc).unwrap();
        assert!(
            !source.contains("BeginChild") && !source.contains("child_window"),
            "{format:?} invented an ImGui child window:\n{source}"
        );
        assert!(
            !source.contains("ImGui::DockSpace") && !source.contains("ui.dockspace()"),
            "{format:?} invented a Dear ImGui dockspace:\n{source}"
        );
    }

    let slint = export(Format::Slint, &doc).unwrap();
    assert!(slint.contains("VerticalBox"), "{slint}");
    assert!(!slint.contains("DockSpace"), "{slint}");

    let swift = export(Format::SwiftUi, &doc).unwrap();
    assert!(swift.contains("TableColumn"), "{swift}");
    assert!(swift.contains("VStack"), "{swift}");

    let gtk = export(Format::Gtk, &doc).unwrap();
    assert!(gtk.contains("gtk_column_view_new"), "{gtk}");
    assert!(gtk.contains("gtk_box_new"), "{gtk}");

    let compose = export(Format::JetpackCompose, &doc).unwrap();
    assert!(compose.contains("Column"), "{compose}");
    assert!(compose.contains("@Composable"), "{compose}");

    let winui = export(Format::WinUi, &doc).unwrap();
    assert!(winui.contains("StackPanel"), "{winui}");
    assert!(winui.contains("<Window"), "{winui}");
}

#[test]
fn sample_raven_keeps_header_bar_and_island() {
    let raven = export(Format::RavenImgui, &sample()).unwrap();
    assert!(raven.contains("Rheader_bar_create"), "{raven}");
    assert!(raven.contains("R_WIDGET_ISLAND"), "{raven}");
    assert!(raven.contains("R_WIDGET_HBOX"), "{raven}");
    assert!(raven.contains("Rbutton_create"), "{raven}");
    assert!(raven.contains("Rcheckbox_create"), "{raven}");
    assert!(raven.contains("Rtext_input_set_placeholder"), "{raven}");
    assert!(raven.contains("Rtable_set_cell"), "{raven}");
    assert_eq!(import(Format::RavenImgui, &raven).unwrap(), sample());
}

#[test]
fn dear_imgui_import_rejects_a_dock_without_dockspace() {
    let source = "\
// uii-document name=Shell
// uii id=root kind=window name=Shell
// uii id=dock kind=dock name=shell parent=root
";
    let err = import(Format::DearImgui, source).expect_err("dock without DockSpace");
    assert!(
        err.to_string().contains("DockSpace"),
        "missing docking call should be reported: {err}"
    );
}

#[test]
fn call_point_imports_and_exports_without_a_window() {
    let document = sample();
    let exported = call(Call::Export {
        format: Format::DearImgui,
        document: document.clone(),
    })
    .unwrap();
    let CallOutput::Source(source) = exported else {
        panic!("export returns source text");
    };
    assert!(source.contains("ImGui::DockSpace"));
    let imported = call(Call::Import {
        format: Format::DearImgui,
        source,
    })
    .unwrap();
    let CallOutput::Document(back) = imported else {
        panic!("import returns a document");
    };
    assert_eq!(back, document);
}

fn run_cli(args: &[&str], stdin: &str) -> std::process::Output {
    use std::io::Write;
    use std::process::Stdio;
    let mut child = Command::new(env!("CARGO_BIN_EXE_uii"))
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn uii");
    child
        .stdin
        .take()
        .unwrap()
        .write_all(stdin.as_bytes())
        .unwrap();
    child.wait_with_output().expect("wait uii")
}

#[test]
fn cli_is_the_host_call_point() {
    let json = r#"{"name":"Shell","root":{"id":"root","kind":"window","name":"Shell"}}"#;
    let exported = run_cli(&["export", "dear-imgui"], json);
    assert!(
        exported.status.success(),
        "export failed: {}",
        String::from_utf8_lossy(&exported.stderr)
    );
    let source = String::from_utf8(exported.stdout).unwrap();
    assert!(source.contains("ImGui::Begin"), "{source}");
    let imported = run_cli(&["import", "dear-imgui"], &source);
    assert!(
        imported.status.success(),
        "import failed: {}",
        String::from_utf8_lossy(&imported.stderr)
    );
    let back = String::from_utf8(imported.stdout).unwrap();
    assert!(back.contains("\"kind\": \"window\""), "{back}");
    assert!(back.contains("\"id\": \"root\""), "{back}");

    let bad = run_cli(&["import", "no-such-format"], "");
    assert!(!bad.status.success());
}
