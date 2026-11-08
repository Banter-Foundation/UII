//! Minimal MCP stdio server. Cursor launches it and calls the layout tools.
//!
//! Frames are `Content-Length` messages, the same framing as the Language Server Protocol.

use std::io::{self, Read, Write};

use serde_json::{json, Value};

use crate::model::{LayoutError, LayoutSnapshot};
use crate::plug::{options_from_env, read_current};
use crate::{SERVER_NAME, SERVER_VERSION};

const MAX_FRAME: usize = 16 * 1024 * 1024;
const PROTOCOL_DEFAULT: &str = "2024-11-05";

#[derive(Debug)]
pub struct DecodedFrames {
    pub messages: Vec<String>,
    pub consumed: usize,
}

pub fn encode_frame(body: &str) -> Vec<u8> {
    let header = format!("Content-Length: {}\r\n\r\n", body.len());
    let mut out = header.into_bytes();
    out.extend_from_slice(body.as_bytes());
    out
}

pub fn decode_frames(input: &[u8]) -> Result<DecodedFrames, LayoutError> {
    let mut messages = Vec::new();
    let mut offset = 0;
    while offset < input.len() {
        let rest = &input[offset..];
        if rest.len() > MAX_FRAME {
            return Err(LayoutError::Json("layout frame is too large".into()));
        }
        let Some(header_end) = find_subsequence(rest, b"\r\n\r\n") else {
            if rest.len() > 64 * 1024 {
                return Err(LayoutError::Json("layout frame header is too large".into()));
            }
            break;
        };
        let header = std::str::from_utf8(&rest[..header_end])
            .map_err(|err| LayoutError::Json(err.to_string()))?;
        let len = content_length(header)?;
        if len > MAX_FRAME {
            return Err(LayoutError::Json("layout frame is too large".into()));
        }
        let body_start = header_end + 4;
        if rest.len() < body_start + len {
            break;
        }
        let body = &rest[body_start..body_start + len];
        let text = std::str::from_utf8(body).map_err(|err| LayoutError::Json(err.to_string()))?;
        messages.push(text.to_string());
        offset += body_start + len;
    }
    Ok(DecodedFrames {
        messages,
        consumed: offset,
    })
}

pub fn respond_frames(
    input: &[u8],
    load: impl Fn() -> Result<LayoutSnapshot, LayoutError>,
) -> Result<(Vec<u8>, usize), LayoutError> {
    let decoded = decode_frames(input)?;
    let mut out = Vec::new();
    for message in &decoded.messages {
        if let Some(response) = dispatch_message(message, &load)? {
            out.extend(encode_frame(&response));
        }
    }
    Ok((out, decoded.consumed))
}

pub fn run_stdio() -> Result<(), LayoutError> {
    let stdin = io::stdin();
    let mut reader = stdin.lock();
    let stdout = io::stdout();
    let mut writer = stdout.lock();
    let mut buf = Vec::new();
    let mut tmp = [0u8; 8192];
    loop {
        let n = reader.read(&mut tmp)?;
        if n == 0 {
            return Ok(());
        }
        buf.extend_from_slice(&tmp[..n]);
        let (out, consumed) = respond_frames(&buf, || {
            let opts = options_from_env()?;
            read_current(&opts)
        })?;
        if !out.is_empty() {
            writer.write_all(&out)?;
            writer.flush()?;
        }
        if consumed > buf.len() {
            return Err(LayoutError::Json("frame consumed past the buffer".into()));
        }
        buf.drain(..consumed);
    }
}

fn dispatch_message(
    message: &str,
    load: &impl Fn() -> Result<LayoutSnapshot, LayoutError>,
) -> Result<Option<String>, LayoutError> {
    let value: Value = match serde_json::from_str(message) {
        Ok(value) => value,
        Err(_) => return Ok(Some(error_response(Value::Null, -32700, "parse error"))),
    };
    let method = value.get("method").and_then(|m| m.as_str()).unwrap_or("");
    if value.get("id").is_none() || method.starts_with("notifications/") {
        return Ok(None);
    }
    let id = value.get("id").cloned().unwrap_or(Value::Null);
    let response = match method {
        "initialize" => initialize_response(&value, id),
        "ping" => result_response(id, json!({})),
        "tools/list" => result_response(id, json!({ "tools": tool_definitions() })),
        "tools/call" => tool_call(id, value.get("params"), load),
        other => error_response(id, -32601, &format!("unknown method: {other}")),
    };
    Ok(Some(response))
}

fn initialize_response(value: &Value, id: Value) -> String {
    let version = value
        .get("params")
        .and_then(|params| params.get("protocolVersion"))
        .and_then(|version| version.as_str())
        .unwrap_or(PROTOCOL_DEFAULT);
    result_response(
        id,
        json!({
            "protocolVersion": version,
            "capabilities": { "tools": { "listChanged": false } },
            "serverInfo": { "name": SERVER_NAME, "version": SERVER_VERSION },
            "instructions": "Read the current UII layout with read_layout. The region tree is the structure, declarations are the styles, purpose is what each region is for, and intent.html plus intent.css are Owl's description of the layout. UII is a native Rust desktop app, not a web app."
        }),
    )
}

fn tool_definitions() -> Vec<Value> {
    vec![
        json!({
            "name": "read_layout",
            "description": "Read the current UII layout: the region tree (structure), each region's style declarations, what each region is for, and Owl's UI intent. HTML and CSS on the intent are how Owl described the layout, not a web app.",
            "inputSchema": {
                "type": "object",
                "properties": {},
                "additionalProperties": false
            }
        }),
        json!({
            "name": "read_intent",
            "description": "Read Owl's UI intent for the current layout: the summary, the HTML and CSS used to describe it, and what each region is for.",
            "inputSchema": {
                "type": "object",
                "properties": {},
                "additionalProperties": false
            }
        }),
        json!({
            "name": "read_region",
            "description": "Read one region by id: its purpose, style declarations, bounds, child ids, and ancestor ids.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "id": { "type": "string", "description": "Region id" }
                },
                "required": ["id"],
                "additionalProperties": false
            }
        }),
    ]
}

fn tool_call(
    id: Value,
    params: Option<&Value>,
    load: &impl Fn() -> Result<LayoutSnapshot, LayoutError>,
) -> String {
    let name = params
        .and_then(|params| params.get("name"))
        .and_then(|name| name.as_str())
        .unwrap_or("");
    let arguments = params
        .and_then(|params| params.get("arguments"))
        .cloned()
        .unwrap_or_else(|| json!({}));
    match name {
        "read_layout" => match load() {
            Ok(snapshot) => match layout_body(&snapshot) {
                Ok(text) => tool_result(id, text, false),
                Err(err) => tool_result(id, err.to_string(), true),
            },
            Err(err) => tool_result(id, err.to_string(), true),
        },
        "read_intent" => match load() {
            Ok(snapshot) => match serde_json::to_string_pretty(&json!({
                "document_name": snapshot.document_name,
                "summary": snapshot.intent.summary,
                "html": snapshot.intent.html,
                "css": snapshot.intent.css,
                "regions": snapshot.purposes(),
            })) {
                Ok(text) => tool_result(id, text, false),
                Err(err) => tool_result(id, err.to_string(), true),
            },
            Err(err) => tool_result(id, err.to_string(), true),
        },
        "read_region" => {
            let Some(region_id) = arguments
                .get("id")
                .and_then(|id| id.as_str())
                .filter(|id| !id.is_empty())
            else {
                return tool_result(id, "region id is required".into(), true);
            };
            match load() {
                Ok(snapshot) => match snapshot.region_detail(region_id) {
                    Ok(detail) => match serde_json::to_string_pretty(&detail) {
                        Ok(text) => tool_result(id, text, false),
                        Err(err) => tool_result(id, err.to_string(), true),
                    },
                    Err(err) => tool_result(id, err.to_string(), true),
                },
                Err(err) => tool_result(id, err.to_string(), true),
            }
        }
        other => tool_result(id, format!("unknown tool: {other}"), true),
    }
}

fn layout_body(snapshot: &LayoutSnapshot) -> Result<String, LayoutError> {
    let mut body = serde_json::to_value(snapshot)?;
    body["reading"] = json!(snapshot.reading());
    Ok(serde_json::to_string_pretty(&body)?)
}

fn tool_result(id: Value, text: String, is_error: bool) -> String {
    result_response(
        id,
        json!({
            "content": [{ "type": "text", "text": text }],
            "isError": is_error
        }),
    )
}

fn result_response(id: Value, result: Value) -> String {
    json!({
        "jsonrpc": "2.0",
        "id": id,
        "result": result
    })
    .to_string()
}

fn error_response(id: Value, code: i32, message: &str) -> String {
    json!({
        "jsonrpc": "2.0",
        "id": id,
        "error": { "code": code, "message": message }
    })
    .to_string()
}

fn content_length(header: &str) -> Result<usize, LayoutError> {
    for line in header.split("\r\n") {
        let Some((name, value)) = line.split_once(':') else {
            continue;
        };
        if name.eq_ignore_ascii_case("Content-Length") {
            return value
                .trim()
                .parse::<usize>()
                .map_err(|_| LayoutError::Json("invalid content length".into()));
        }
    }
    Err(LayoutError::Json("missing content length".into()))
}

fn find_subsequence(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack
        .windows(needle.len())
        .position(|window| window == needle)
}
