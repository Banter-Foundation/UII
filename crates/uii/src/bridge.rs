//! Local socket between the running window and Cursor.
//!
//! The window binds 127.0.0.1 and writes the port to a file. `uii get`,
//! `uii apply`, and `uii mcp` connect to that port. Each request is one JSON
//! object and a newline. This is not HTTP.

use std::env;
use std::fs;
use std::io::{BufRead, BufReader, Write};
use std::net::{TcpListener, TcpStream};
use std::path::PathBuf;
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread;
use std::time::Duration;

pub const DEFAULT_PORT: u16 = 47321;

pub struct Jobs {
    pub incoming: Receiver<Job>,
}

pub struct Job {
    pub request: String,
    pub reply: Sender<String>,
}

pub fn start() -> Jobs {
    let (tx, rx) = mpsc::channel();
    thread::spawn(move || listen(tx));
    Jobs { incoming: rx }
}

fn listen(tx: Sender<Job>) {
    let listener = match bind() {
        Ok(listener) => listener,
        Err(err) => {
            eprintln!("uii: cursor bridge did not start: {err}");
            return;
        }
    };
    let port = listener.local_addr().map(|addr| addr.port()).unwrap_or(DEFAULT_PORT);
    let path = port_file();
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    let _ = fs::write(&path, port.to_string());
    eprintln!("uii: cursor bridge on 127.0.0.1:{port}");
    for stream in listener.incoming() {
        let Ok(stream) = stream else { continue };
        let tx = tx.clone();
        thread::spawn(move || serve(stream, tx));
    }
}

fn bind() -> std::io::Result<TcpListener> {
    let mut last = None;
    for port in DEFAULT_PORT..DEFAULT_PORT + 8 {
        match TcpListener::bind(("127.0.0.1", port)) {
            Ok(listener) => return Ok(listener),
            Err(err) => last = Some(err),
        }
    }
    Err(last.unwrap_or_else(|| std::io::Error::other("no port")))
}

fn serve(stream: TcpStream, tx: Sender<Job>) {
    let _ = stream.set_read_timeout(Some(Duration::from_secs(30)));
    let mut reader = BufReader::new(stream.try_clone().expect("clone socket"));
    let mut line = String::new();
    if reader.read_line(&mut line).is_err() {
        return;
    }
    let (reply_tx, reply_rx) = mpsc::channel();
    if tx
        .send(Job {
            request: line,
            reply: reply_tx,
        })
        .is_err()
    {
        return;
    }
    let Ok(body) = reply_rx.recv_timeout(Duration::from_secs(10)) else {
        return;
    };
    let mut stream = stream;
    let _ = stream.write_all(body.as_bytes());
    if !body.ends_with('\n') {
        let _ = stream.write_all(b"\n");
    }
}

pub fn ask(request: &str) -> String {
    let port = read_port();
    let mut stream = TcpStream::connect(("127.0.0.1", port)).unwrap_or_else(|_| {
        eprintln!("uii: the editor window is not running on 127.0.0.1:{port}");
        std::process::exit(1);
    });
    let mut line = request.trim().to_string();
    line.push('\n');
    stream.write_all(line.as_bytes()).expect("write request");
    let mut reader = BufReader::new(stream);
    let mut response = String::new();
    reader.read_line(&mut response).expect("read response");
    response.trim().to_string()
}

fn read_port() -> u16 {
    fs::read_to_string(port_file())
        .ok()
        .and_then(|text| text.trim().parse().ok())
        .unwrap_or(DEFAULT_PORT)
}

fn port_file() -> PathBuf {
    if let Ok(dir) = env::var("XDG_RUNTIME_DIR") {
        return PathBuf::from(dir).join("uii.port");
    }
    env::temp_dir().join("uii.port")
}

pub fn mcp() {
    let stdin = std::io::stdin();
    let mut reader = BufReader::new(stdin.lock());
    let mut stdout = std::io::stdout().lock();
    loop {
        let Some(message) = read_frame(&mut reader) else {
            return;
        };
        if let Some(response) = dispatch(&message) {
            write_frame(&mut stdout, &response);
        }
    }
}

fn read_frame(reader: &mut impl BufRead) -> Option<String> {
    let mut length = None;
    loop {
        let mut line = String::new();
        if reader.read_line(&mut line).ok()? == 0 {
            return None;
        }
        let trimmed = line.trim();
        if trimmed.is_empty() {
            break;
        }
        if let Some(value) = trimmed
            .strip_prefix("Content-Length:")
            .or_else(|| trimmed.strip_prefix("content-length:"))
        {
            length = value.trim().parse::<usize>().ok();
        }
    }
    let length = length?;
    let mut buf = vec![0; length];
    reader.read_exact(&mut buf).ok()?;
    String::from_utf8(buf).ok()
}

fn write_frame(out: &mut impl Write, body: &str) {
    let _ = write!(out, "Content-Length: {}\r\n\r\n{}", body.len(), body);
    let _ = out.flush();
}

fn dispatch(message: &str) -> Option<String> {
    let method = json_string_field(message, "method").unwrap_or_default();
    let id = json_raw_field(message, "id");
    match method.as_str() {
        "initialize" => Some(rpc(
            id,
            r#"{"protocolVersion":"2024-11-05","capabilities":{"tools":{}},"serverInfo":{"name":"uii","version":"0.1.0"}}"#,
        )),
        "notifications/initialized" | "notifications/cancelled" => None,
        "ping" => Some(rpc(id, "{}")),
        "tools/list" => Some(rpc(id, TOOLS)),
        "tools/call" => Some(rpc(id, &call_tool(message))),
        _ => Some(rpc(
            id,
            r#"{"error":{"code":-32601,"message":"unknown method"}}"#,
        )),
    }
}

fn call_tool(message: &str) -> String {
    let name = json_string_field(message, "name").unwrap_or_default();
    let response = match name.as_str() {
        "get_design" => ask("{\"op\":\"get\"}"),
        "apply_layout" => {
            let html = json_string_field(message, "html").unwrap_or_default();
            let css = json_string_field(message, "css").unwrap_or_default();
            let request = format!(
                "{{\"op\":\"apply\",\"html\":{},\"css\":{}}}",
                json_encode(&html),
                json_encode(&css)
            );
            ask(&request)
        }
        _ => return r#"{"content":[{"type":"text","text":"unknown tool"}],"isError":true}"#.into(),
    };
    format!(
        "{{\"content\":[{{\"type\":\"text\",\"text\":{}}}]}}",
        json_encode(&response)
    )
}

fn rpc(id: Option<String>, result_or_error: &str) -> String {
    let id = id.unwrap_or_else(|| "null".into());
    if result_or_error.contains("\"error\"") && result_or_error.starts_with("{\"error\"") {
        return format!("{{\"jsonrpc\":\"2.0\",\"id\":{id},{}}}", &result_or_error[1..]);
    }
    format!("{{\"jsonrpc\":\"2.0\",\"id\":{id},\"result\":{result_or_error}}}")
}

const TOOLS: &str = r#"{"tools":[
  {"name":"get_design","description":"Read the open UII layout: HTML, CSS, and what each region is for.","inputSchema":{"type":"object","properties":{}}},
  {"name":"apply_layout","description":"Replace the open UII layout with HTML and CSS.","inputSchema":{"type":"object","properties":{"html":{"type":"string"},"css":{"type":"string"}},"required":["html","css"]}}
]}"#;

fn json_string_field(src: &str, key: &str) -> Option<String> {
    let needle = format!("\"{key}\"");
    let mut rest = src;
    while let Some(at) = rest.find(&needle) {
        let after = rest[at + needle.len()..].trim_start();
        if let Some(value) = after.strip_prefix(':') {
            let value = value.trim_start();
            if value.starts_with('"') {
                return decode_json_string(value);
            }
        }
        rest = &rest[at + needle.len()..];
    }
    None
}

fn json_raw_field(src: &str, key: &str) -> Option<String> {
    let needle = format!("\"{key}\"");
    let at = src.find(&needle)?;
    let after = src[at + needle.len()..].trim_start().strip_prefix(':')?.trim_start();
    if after.starts_with('"') {
        return decode_json_string(after);
    }
    let end = after
        .find([',', '}', ' ', '\n', '\r'])
        .unwrap_or(after.len());
    Some(after[..end].to_string())
}

fn decode_json_string(src: &str) -> Option<String> {
    let rest = src.trim_start().strip_prefix('"')?;
    let mut out = String::new();
    let mut chars = rest.chars();
    while let Some(ch) = chars.next() {
        if ch == '\\' {
            match chars.next() {
                Some('n') => out.push('\n'),
                Some('r') => out.push('\r'),
                Some('t') => out.push('\t'),
                Some('u') => {
                    let hex: String = chars.by_ref().take(4).collect();
                    let code = u32::from_str_radix(&hex, 16).ok()?;
                    out.push(char::from_u32(code).unwrap_or('\u{FFFD}'));
                }
                Some(other) => out.push(other),
                None => return None,
            }
        } else if ch == '"' {
            return Some(out);
        } else {
            out.push(ch);
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::{dispatch, json_encode, json_string_field};

    #[test]
    fn cursor_tools_are_get_design_and_apply_layout() {
        let response = dispatch(r#"{"jsonrpc":"2.0","id":1,"method":"tools/list"}"#).unwrap();
        assert!(response.contains("get_design"));
        assert!(response.contains("apply_layout"));
        assert!(response.contains("\"result\""));
    }

    #[test]
    fn apply_layout_fields_keep_markup() {
        let message = format!(
            "{{\"name\":\"apply_layout\",\"html\":{},\"css\":{}}}",
            json_encode("<header>Title</header>"),
            json_encode("header { height: 40px; }")
        );
        assert_eq!(
            json_string_field(&message, "html").as_deref(),
            Some("<header>Title</header>")
        );
        assert_eq!(
            json_string_field(&message, "css").as_deref(),
            Some("header { height: 40px; }")
        );
    }
}

fn json_encode(text: &str) -> String {
    let mut out = String::from("\"");
    for ch in text.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            ch if ch.is_control() => out.push_str(&format!("\\u{:04x}", ch as u32)),
            ch => out.push(ch),
        }
    }
    out.push('"');
    out
}
