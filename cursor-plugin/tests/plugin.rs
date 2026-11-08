//! Contract for the Cursor plug: read structure, styles, region purpose, and Owl's intent.
//!
//! These tests are the API. Implementation follows them.

use std::net::SocketAddr;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Mutex;
use std::time::Duration;

use serde_json::Value;
use uii_cursor::{
    decode_frames, discover, encode_frame, fetch_live, options_from_env, read_current,
    read_snapshot, respond_frames, snapshot_path_from, write_snapshot, Bounds, LayoutError,
    LayoutPlug, LayoutSnapshot, ReadOptions, Region, UiIntent, FORMAT_VERSION,
};

fn sample() -> LayoutSnapshot {
    let voice = Region::new("voice", "Voice", "")
        .unwrap()
        .with_declaration("color", "#fff");
    let account = Region::new("account", "Account", "shows the signed-in user")
        .unwrap()
        .with_child(voice);
    let shell = Region::new("shell", "Window", "holds the settings page")
        .unwrap()
        .with_declaration("display", "flex")
        .with_declaration("gap", "8px")
        .with_bounds(Bounds {
            x: 0.0,
            y: 0.0,
            width: 800.0,
            height: 600.0,
        })
        .with_child(account);
    let intent = UiIntent::new(
        "A quiet settings window for account and voice.",
        "<main id=\"shell\"><section id=\"account\">Owl</section></main>",
        "main { display: flex; }",
    );
    LayoutSnapshot::new("Settings", intent, shell).unwrap()
}

fn expected_reading() -> &'static str {
    "\
Document: Settings
Intent: A quiet settings window for account and voice.

- shell \"Window\" — holds the settings page
  style: display: flex; gap: 8px
  - account \"Account\" — shows the signed-in user
    style: (none)
    - voice \"Voice\" — purpose not set
      style: color: #fff
"
}

fn temp_dir() -> PathBuf {
    let dir = std::env::temp_dir().join(format!("uii-cursor-{}-{}", std::process::id(), unique()));
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn unique() -> u64 {
    use std::sync::atomic::AtomicU64;
    static N: AtomicU64 = AtomicU64::new(1);
    N.fetch_add(1, Ordering::Relaxed)
}

fn tool_text(response: &Value) -> (bool, String) {
    let result = response.get("result").expect("result");
    let is_error = result
        .get("isError")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    let text = result["content"][0]["text"].as_str().unwrap().to_string();
    (is_error, text)
}

fn call(
    name: &str,
    arguments: Value,
    load: impl Fn() -> Result<LayoutSnapshot, LayoutError>,
) -> Value {
    let body = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 7,
        "method": "tools/call",
        "params": { "name": name, "arguments": arguments }
    })
    .to_string();
    let (out, consumed) = respond_frames(&encode_frame(&body), load).unwrap();
    assert_eq!(consumed, encode_frame(&body).len());
    let decoded = decode_frames(&out).unwrap();
    assert_eq!(decoded.consumed, out.len());
    assert_eq!(decoded.messages.len(), 1);
    serde_json::from_str(&decoded.messages[0]).unwrap()
}

#[test]
fn format_version_is_one() {
    assert_eq!(FORMAT_VERSION, 1);
    assert_eq!(sample().format, 1);
}

#[test]
fn snapshot_round_trip_keeps_html_css_style_bounds_and_purpose() {
    let snap = sample();
    assert_eq!(
        snap.intent.html,
        "<main id=\"shell\"><section id=\"account\">Owl</section></main>"
    );
    assert_eq!(snap.intent.css, "main { display: flex; }");
    let json = serde_json::to_string(&snap).unwrap();
    let back: LayoutSnapshot = serde_json::from_str(&json).unwrap();
    assert_eq!(back, snap);
    assert_eq!(back.root.bounds.unwrap().width, 800.0);
    assert!(back.root.children[0].bounds.is_none());
}

#[test]
fn omitted_style_and_children_default_to_empty() {
    let raw = r#"{
        "format": 1,
        "document_name": "N",
        "intent": {"summary":"s","html":"<b></b>","css":""},
        "root": {"id":"a","name":"A","purpose":"p"}
    }"#;
    let snap: LayoutSnapshot = serde_json::from_str(raw).unwrap();
    snap.validate().unwrap();
    assert!(snap.root.children.is_empty());
    assert!(snap.root.style.declarations.is_empty());
    assert_eq!(snap.intent.html, "<b></b>");
}

#[test]
fn reading_lists_structure_styles_and_purpose() {
    assert_eq!(sample().reading(), expected_reading());
}

#[test]
fn empty_document_name_and_summary_use_placeholders_in_the_reading() {
    let mut snap = sample();
    snap.document_name.clear();
    snap.intent.summary.clear();
    let reading = snap.reading();
    assert!(reading.starts_with("Document: (untitled)\nIntent: (not set)\n"));
}

#[test]
fn purposes_walk_the_tree_in_order_and_keep_empty_purpose() {
    let purposes = sample().purposes();
    let ids: Vec<_> = purposes.iter().map(|p| p.id.as_str()).collect();
    assert_eq!(ids, vec!["shell", "account", "voice"]);
    assert_eq!(purposes[1].purpose, "shows the signed-in user");
    assert_eq!(purposes[2].purpose, "");
    assert_eq!(purposes[2].name, "Voice");
}

#[test]
fn region_detail_includes_style_purpose_children_and_ancestors() {
    let snap = sample();
    let account = snap.region_detail("account").unwrap();
    assert_eq!(account.purpose, "shows the signed-in user");
    assert!(account.style.declarations.is_empty());
    assert!(account.bounds.is_none());
    assert_eq!(account.child_ids, vec!["voice".to_string()]);
    assert_eq!(account.ancestor_ids, vec!["shell".to_string()]);

    let shell = snap.region_detail("shell").unwrap();
    assert_eq!(shell.bounds.unwrap().height, 600.0);
    assert_eq!(shell.child_ids, vec!["account".to_string()]);
    assert!(shell.ancestor_ids.is_empty());
    assert_eq!(shell.style.declarations[0].property, "display");
    assert_eq!(shell.style.declarations[0].value, "flex");
    assert_eq!(shell.style.declarations[1].property, "gap");

    let voice = snap.region_detail("voice").unwrap();
    assert_eq!(
        voice.ancestor_ids,
        vec!["shell".to_string(), "account".to_string()]
    );
    assert!(voice.child_ids.is_empty());
    assert_eq!(voice.style.declarations[0].value, "#fff");

    let missing = snap.region_detail("missing").unwrap_err();
    assert_eq!(missing.to_string(), "unknown region: missing");
}

#[test]
fn empty_id_and_duplicates_are_rejected() {
    let err = Region::new("", "Name", "purpose").unwrap_err();
    assert_eq!(err.to_string(), "region id is empty");

    let mut snap = sample();
    snap.root
        .children
        .push(Region::new("shell", "Again", "nope").unwrap());
    let err = snap.validate().unwrap_err();
    assert_eq!(err.to_string(), "duplicate region id: shell");

    let raw = r#"{
        "format": 1,
        "document_name": "N",
        "intent": {"summary":"","html":"","css":""},
        "root": {"id":"","name":"A","purpose":"p"}
    }"#;
    let parsed: LayoutSnapshot = serde_json::from_str(raw).unwrap();
    assert_eq!(
        parsed.validate().unwrap_err().to_string(),
        "region id is empty"
    );

    snap.format = 2;
    // The duplicate is still there; format is checked first.
    assert!(matches!(
        snap.validate(),
        Err(LayoutError::UnsupportedFormat { found: 2 })
    ));
}

#[test]
fn write_snapshot_rejects_duplicates_without_creating_a_file() {
    let dir = temp_dir();
    let path = dir.join("layout.json");
    let mut snap = sample();
    snap.root
        .children
        .push(Region::new("shell", "Again", "nope").unwrap());
    let err = write_snapshot(&path, &snap).unwrap_err();
    assert_eq!(err.to_string(), "duplicate region id: shell");
    assert!(!path.exists());
}

#[test]
fn snapshot_file_round_trip_and_bad_format() {
    let dir = temp_dir();
    let path = dir.join("nested").join("layout.json");
    write_snapshot(&path, &sample()).unwrap();
    let loaded = read_snapshot(&path).unwrap();
    assert_eq!(loaded, sample());
    let text = std::fs::read_to_string(&path).unwrap();
    assert!(text.contains("<main id=\\\"shell\\\">") || text.contains("<main id=\"shell\">"));
    assert!(text.contains("shows the signed-in user"));

    let bad = dir.join("bad.json");
    std::fs::write(
        &bad,
        r#"{"format":2,"document_name":"N","intent":{"summary":"","html":"","css":""},"root":{"id":"a","name":"A","purpose":""}}"#,
    )
    .unwrap();
    assert!(matches!(
        read_snapshot(&bad),
        Err(LayoutError::UnsupportedFormat { found: 2 })
    ));

    let garbage = dir.join("garbage.json");
    std::fs::write(&garbage, "not-json").unwrap();
    assert!(matches!(read_snapshot(&garbage), Err(LayoutError::Json(_))));
}

#[test]
fn data_dir_follows_banter_platform_roots() {
    let local = Path::new("C:/Users/Owl/AppData/Local");
    let home = Path::new("C:/Users/Owl");
    assert_eq!(
        snapshot_path_from("windows", home, Some(local), None),
        local.join("UII").join("layout.json")
    );
    assert_eq!(
        snapshot_path_from("windows", home, None, None),
        home.join("AppData/Local/UII/layout.json")
    );
    let mac = Path::new("/Users/owl");
    assert_eq!(
        snapshot_path_from("macos", mac, None, None),
        mac.join("Library/Application Support/UII/layout.json")
    );
    let xdg = Path::new("/xdg");
    assert_eq!(
        snapshot_path_from("linux", Path::new("/home/owl"), None, Some(xdg)),
        xdg.join("UII/layout.json")
    );
    let linux_home = Path::new("/home/owl");
    assert_eq!(
        snapshot_path_from("linux", linux_home, None, None),
        linux_home.join(".local/share/UII/layout.json")
    );
    assert_eq!(
        snapshot_path_from("freebsd", linux_home, None, None),
        linux_home.join(".local/share/UII/layout.json")
    );
}

#[test]
fn live_plug_serves_the_published_layout_and_stops_on_drop() {
    let addr = {
        let plug = LayoutPlug::bind().unwrap();
        assert!(plug.addr().ip().is_loopback());
        let err = fetch_live(plug.addr()).unwrap_err();
        assert_eq!(err.to_string(), "no layout published");

        plug.publish(sample()).unwrap();
        let got = fetch_live(plug.addr()).unwrap();
        assert_eq!(got.reading(), expected_reading());

        let mut next = sample();
        next.root.purpose = "updated purpose".to_string();
        plug.publish(next).unwrap();
        let got = fetch_live(plug.addr()).unwrap();
        assert_eq!(got.root.purpose, "updated purpose");

        let addr = plug.addr();
        std::thread::scope(|scope| {
            for _ in 0..4 {
                scope.spawn(move || {
                    let got = fetch_live(addr).unwrap();
                    assert_eq!(got.root.purpose, "updated purpose");
                });
            }
        });
        addr
    };
    assert!(matches!(
        fetch_live(addr),
        Err(LayoutError::PlugUnreachable)
    ));
}

#[test]
fn plug_rejects_non_loopback_without_connecting() {
    let addr: SocketAddr = "0.0.0.0:0".parse().unwrap();
    let err = LayoutPlug::bind_addr(addr).unwrap_err();
    assert_eq!(err.to_string(), "plug listens on loopback only");

    let addr: SocketAddr = "192.0.2.1:9".parse().unwrap();
    let err = fetch_live(addr).unwrap_err();
    assert_eq!(err.to_string(), "plug listens on loopback only");
    let started = std::time::Instant::now();
    let _ = fetch_live(addr);
    assert!(started.elapsed() < Duration::from_millis(200));
}

#[test]
fn read_current_prefers_a_reachable_plug_over_a_stale_or_broken_file() {
    let dir = temp_dir();
    let path = dir.join("layout.json");
    let mut stale = sample();
    stale.root.purpose = "stale purpose".to_string();
    write_snapshot(&path, &stale).unwrap();

    let plug = LayoutPlug::bind().unwrap();
    plug.publish(sample()).unwrap();
    std::fs::write(dir.join("plug.addr"), format!("{}\n", plug.addr())).unwrap();
    // Corrupt the file. A live plug is the current layout, so the file is not read.
    std::fs::write(&path, "not-json").unwrap();

    let opts = discover(&path);
    assert_eq!(opts.plug_addr, Some(plug.addr()));
    let got = read_current(&opts).unwrap();
    assert_eq!(got.root.purpose, "holds the settings page");
}

#[test]
fn reachable_plug_with_nothing_published_does_not_return_a_stale_file() {
    let dir = temp_dir();
    let path = dir.join("layout.json");
    write_snapshot(&path, &sample()).unwrap();
    let plug = LayoutPlug::bind().unwrap();
    std::fs::write(dir.join("plug.addr"), format!("{}\n", plug.addr())).unwrap();
    let err = read_current(&discover(&path)).unwrap_err();
    assert_eq!(err.to_string(), "no layout published");
}

#[test]
fn read_current_falls_back_to_the_file_when_the_plug_is_down() {
    let dir = temp_dir();
    let path = dir.join("layout.json");
    write_snapshot(&path, &sample()).unwrap();
    let (closed, _hold) = listen_without_accept();
    std::fs::write(dir.join("plug.addr"), format!("127.0.0.1:{closed}\n")).unwrap();
    let got = read_current(&discover(&path)).unwrap();
    assert_eq!(got.reading(), expected_reading());

    std::fs::write(dir.join("plug.addr"), "not-an-address\n").unwrap();
    let opts = discover(&path);
    assert!(opts.plug_addr.is_none());
    assert_eq!(read_current(&opts).unwrap().document_name, "Settings");
}

#[test]
fn missing_layout_is_a_clear_error() {
    let dir = temp_dir();
    let path = dir.join("missing.json");
    let err = read_current(&ReadOptions {
        plug_addr: None,
        snapshot_path: Some(path.clone()),
    })
    .unwrap_err();
    assert_eq!(err.to_string(), "no layout published");

    let (closed, _hold) = listen_without_accept();
    let err = read_current(&ReadOptions {
        plug_addr: Some(format!("127.0.0.1:{closed}").parse().unwrap()),
        snapshot_path: Some(path),
    })
    .unwrap_err();
    assert_eq!(err.to_string(), "no layout published");

    let garbage = dir.join("garbage.json");
    std::fs::write(&garbage, "{").unwrap();
    let err = read_current(&ReadOptions {
        plug_addr: None,
        snapshot_path: Some(garbage),
    })
    .unwrap_err();
    assert!(matches!(err, LayoutError::Json(_)));
}

#[test]
fn publish_to_writes_the_snapshot_and_the_plug_address() {
    let dir = temp_dir();
    let path = dir.join("UII").join("layout.json");
    let plug = LayoutPlug::bind().unwrap();
    plug.publish_to(sample(), &path).unwrap();
    assert_eq!(read_snapshot(&path).unwrap(), sample());
    let addr = std::fs::read_to_string(dir.join("UII").join("plug.addr")).unwrap();
    assert_eq!(addr.trim(), plug.addr().to_string());
    assert_eq!(fetch_live(plug.addr()).unwrap().document_name, "Settings");

    let mut replaced = sample();
    replaced.document_name = "Replaced".to_string();
    plug.publish_to(replaced, &path).unwrap();
    assert_eq!(read_snapshot(&path).unwrap().document_name, "Replaced");
}

/// Holds a listening socket and never answers, so the port cannot be reused by another test.
fn listen_without_accept() -> (u16, std::net::TcpListener) {
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    listener.set_nonblocking(true).unwrap();
    let port = listener.local_addr().unwrap().port();
    (port, listener)
}

static ENV_LOCK: Mutex<()> = Mutex::new(());

#[test]
fn env_options_honor_explicit_paths_and_reject_a_bad_address() {
    let _guard = ENV_LOCK.lock().unwrap();
    let dir = temp_dir();
    let path = dir.join("layout.json");
    std::env::set_var("UII_LAYOUT_SNAPSHOT", &path);
    std::env::remove_var("UII_PLUG_ADDR");
    let opts = options_from_env().unwrap();
    assert_eq!(opts.snapshot_path, Some(path.clone()));
    assert!(opts.plug_addr.is_none());

    std::env::set_var("UII_PLUG_ADDR", "127.0.0.1:9");
    let opts = options_from_env().unwrap();
    assert_eq!(
        opts.plug_addr,
        Some("127.0.0.1:9".parse::<SocketAddr>().unwrap())
    );

    std::env::set_var("UII_PLUG_ADDR", "not-a-socket");
    let err = options_from_env().unwrap_err();
    assert_eq!(err.to_string(), "invalid plug address: not-a-socket");

    std::env::remove_var("UII_PLUG_ADDR");
    std::env::remove_var("UII_LAYOUT_SNAPSHOT");
}

#[test]
fn framing_uses_content_length_in_bytes_and_keeps_partial_frames() {
    let body = "{\"a\":\"—\"}";
    let frame = encode_frame(body);
    let expected = format!("Content-Length: {}\r\n\r\n{body}", body.len());
    assert_eq!(frame, expected.as_bytes());

    let partial = &frame[..frame.len() - 1];
    let decoded = decode_frames(partial).unwrap();
    assert!(decoded.messages.is_empty());
    assert_eq!(decoded.consumed, 0);

    let mut both = frame.clone();
    both.extend_from_slice(&frame);
    let decoded = decode_frames(&both[..both.len() - 3]).unwrap();
    assert_eq!(decoded.messages.len(), 1);
    assert_eq!(decoded.messages[0], body);
    assert_eq!(decoded.consumed, frame.len());
}

#[test]
fn initialize_and_tool_list_do_not_need_a_layout() {
    let init = serde_json::json!({
        "jsonrpc": "2.0",
        "id": "abc",
        "method": "initialize",
        "params": {
            "protocolVersion": "2024-11-05",
            "capabilities": {},
            "clientInfo": {"name": "cursor", "version": "1"}
        }
    })
    .to_string();
    let list = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 3,
        "method": "tools/list"
    })
    .to_string();
    let note = r#"{"jsonrpc":"2.0","method":"notifications/initialized"}"#;
    let mut input = encode_frame(&init);
    input.extend(encode_frame(note));
    input.extend(encode_frame(&list));

    let calls = AtomicUsize::new(0);
    let (out, consumed) = respond_frames(&input, || {
        calls.fetch_add(1, Ordering::SeqCst);
        Err(LayoutError::NoLayout)
    })
    .unwrap();
    assert_eq!(consumed, input.len());
    assert_eq!(calls.load(Ordering::SeqCst), 0);

    let decoded = decode_frames(&out).unwrap();
    assert_eq!(decoded.messages.len(), 2);
    let init_resp: Value = serde_json::from_str(&decoded.messages[0]).unwrap();
    assert_eq!(init_resp["id"], "abc");
    assert_eq!(init_resp["result"]["protocolVersion"], "2024-11-05");
    assert_eq!(init_resp["result"]["serverInfo"]["name"], "uii-cursor");
    assert_eq!(init_resp["result"]["serverInfo"]["version"], "0.1.0");
    assert_eq!(
        init_resp["result"]["capabilities"]["tools"]["listChanged"],
        false
    );

    let list_resp: Value = serde_json::from_str(&decoded.messages[1]).unwrap();
    assert_eq!(list_resp["id"], 3);
    let tools = list_resp["result"]["tools"].as_array().unwrap();
    let by_name = |name: &str| {
        tools
            .iter()
            .find(|t| t["name"] == name)
            .unwrap_or_else(|| panic!("missing tool {name}"))
    };
    let layout = by_name("read_layout");
    let intent = by_name("read_intent");
    let region = by_name("read_region");
    for needle in ["structure", "style", "region", "intent"] {
        assert!(
            layout["description"].as_str().unwrap().contains(needle),
            "{needle} missing from read_layout description"
        );
    }
    let intent_desc = intent["description"].as_str().unwrap();
    assert!(intent_desc.contains("HTML"));
    assert!(intent_desc.contains("CSS"));
    assert!(region["description"].as_str().unwrap().contains("purpose"));
    assert_eq!(region["inputSchema"]["required"][0], "id");
}

#[test]
fn ping_unknown_method_and_parse_error() {
    let ping = r#"{"jsonrpc":"2.0","id":1,"method":"ping"}"#;
    let (out, _) = respond_frames(&encode_frame(ping), || panic!("load")).unwrap();
    let decoded = decode_frames(&out).unwrap();
    let resp: Value = serde_json::from_str(&decoded.messages[0]).unwrap();
    assert_eq!(resp["result"], serde_json::json!({}));
    assert_eq!(resp["id"], 1);

    let unknown = r#"{"jsonrpc":"2.0","id":3,"method":"foo"}"#;
    let (out, _) = respond_frames(&encode_frame(unknown), || panic!("load")).unwrap();
    let resp: Value = serde_json::from_str(&decode_frames(&out).unwrap().messages[0]).unwrap();
    assert_eq!(resp["error"]["code"], -32601);
    assert_eq!(resp["error"]["message"], "unknown method: foo");

    let (out, _) = respond_frames(&encode_frame("not json"), || panic!("load")).unwrap();
    let resp: Value = serde_json::from_str(&decode_frames(&out).unwrap().messages[0]).unwrap();
    assert_eq!(resp["id"], Value::Null);
    assert_eq!(resp["error"]["code"], -32700);
}

#[test]
fn mcp_reads_structure_styles_purpose_and_intent() {
    let load = || Ok(sample());
    let response = call("read_layout", serde_json::json!({}), &load);
    let (is_error, text) = tool_text(&response);
    assert!(!is_error);
    let body: Value = serde_json::from_str(&text).unwrap();
    assert_eq!(body["document_name"], "Settings");
    assert_eq!(body["format"], 1);
    assert_eq!(
        body["intent"]["html"],
        "<main id=\"shell\"><section id=\"account\">Owl</section></main>"
    );
    assert_eq!(body["intent"]["css"], "main { display: flex; }");
    assert_eq!(
        body["intent"]["summary"],
        "A quiet settings window for account and voice."
    );
    assert_eq!(body["root"]["id"], "shell");
    assert_eq!(body["root"]["purpose"], "holds the settings page");
    assert_eq!(
        body["root"]["style"]["declarations"][0]["property"],
        "display"
    );
    assert_eq!(body["root"]["style"]["declarations"][0]["value"], "flex");
    assert_eq!(
        body["root"]["children"][0]["purpose"],
        "shows the signed-in user"
    );
    assert_eq!(body["reading"], expected_reading());

    let response = call("read_intent", serde_json::json!({}), &load);
    let (is_error, text) = tool_text(&response);
    assert!(!is_error);
    let body: Value = serde_json::from_str(&text).unwrap();
    assert_eq!(
        body["summary"],
        "A quiet settings window for account and voice."
    );
    assert_eq!(
        body["html"],
        "<main id=\"shell\"><section id=\"account\">Owl</section></main>"
    );
    assert_eq!(body["css"], "main { display: flex; }");
    assert_eq!(body["regions"][0]["id"], "shell");
    assert_eq!(body["regions"][1]["purpose"], "shows the signed-in user");
    assert_eq!(body["regions"][2]["id"], "voice");
    assert_eq!(body["regions"][2]["purpose"], "");

    let response = call("read_region", serde_json::json!({"id": "account"}), &load);
    let (is_error, text) = tool_text(&response);
    assert!(!is_error);
    let body: Value = serde_json::from_str(&text).unwrap();
    assert_eq!(body["id"], "account");
    assert_eq!(body["purpose"], "shows the signed-in user");
    assert_eq!(body["child_ids"][0], "voice");
    assert_eq!(body["ancestor_ids"][0], "shell");
    assert!(body["bounds"].is_null());
    assert!(body["style"]["declarations"].as_array().unwrap().is_empty());
}

#[test]
fn mcp_errors_for_a_missing_layout_unknown_region_and_unknown_tool() {
    let response = call("read_layout", serde_json::json!({}), || {
        Err(LayoutError::NoLayout)
    });
    let (is_error, text) = tool_text(&response);
    assert!(is_error);
    assert_eq!(text, "no layout published");

    let response = call("read_region", serde_json::json!({}), || Ok(sample()));
    let (is_error, text) = tool_text(&response);
    assert!(is_error);
    assert_eq!(text, "region id is required");

    let response = call("read_region", serde_json::json!({"id": "nope"}), || {
        Ok(sample())
    });
    let (is_error, text) = tool_text(&response);
    assert!(is_error);
    assert_eq!(text, "unknown region: nope");

    let response = call("nope", serde_json::json!({}), || panic!("load"));
    let (is_error, text) = tool_text(&response);
    assert!(is_error);
    assert_eq!(text, "unknown tool: nope");
}

#[test]
fn two_tool_calls_load_the_layout_twice() {
    let calls = AtomicUsize::new(0);
    let one = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "tools/call",
        "params": {"name": "read_intent", "arguments": {}}
    })
    .to_string();
    let two = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 2,
        "method": "tools/call",
        "params": {"name": "read_layout", "arguments": {}}
    })
    .to_string();
    let mut input = encode_frame(&one);
    input.extend(encode_frame(&two));
    let (out, consumed) = respond_frames(&input, || {
        calls.fetch_add(1, Ordering::SeqCst);
        Ok(sample())
    })
    .unwrap();
    assert_eq!(consumed, input.len());
    assert_eq!(calls.load(Ordering::SeqCst), 2);
    let decoded = decode_frames(&out).unwrap();
    assert_eq!(decoded.messages.len(), 2);
    let first: Value = serde_json::from_str(&decoded.messages[0]).unwrap();
    let second: Value = serde_json::from_str(&decoded.messages[1]).unwrap();
    assert_eq!(first["id"], 1);
    assert_eq!(second["id"], 2);
}
