//! Lossless record lines shared by the code formats.
//!
//! Markup formats (HTML/CSS and the component wrappers) carry the same fields
//! as attributes. Code formats carry one `// uii` line per node plus the
//! native widget call. Import rebuilds the tree from the lines, then checks
//! the native calls are actually there.

use std::collections::BTreeMap;

use crate::error::Error;
use crate::model::{Axis, DockSide, Document, Node, NodeKind};

pub(crate) fn percent_encode(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    for byte in value.as_bytes() {
        let unreserved = byte.is_ascii_alphanumeric() || matches!(*byte, b'-' | b'.' | b'_' | b'~');
        if unreserved {
            out.push(*byte as char);
        } else {
            out.push('%');
            out.push(nibble(byte >> 4));
            out.push(nibble(byte & 0xf));
        }
    }
    out
}

fn nibble(value: u8) -> char {
    char::from(b"0123456789ABCDEF"[value as usize])
}

pub(crate) fn percent_decode(value: &str) -> Result<String, Error> {
    let bytes = value.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' {
            if i + 2 >= bytes.len() {
                return Err(Error::new(format!("bad percent-encoding in `{value}`")));
            }
            let hi = hex_val(bytes[i + 1])?;
            let lo = hex_val(bytes[i + 2])?;
            out.push((hi << 4) | lo);
            i += 3;
        } else {
            out.push(bytes[i]);
            i += 1;
        }
    }
    String::from_utf8(out)
        .map_err(|_| Error::new(format!("percent-encoding is not utf-8 in `{value}`")))
}

fn hex_val(byte: u8) -> Result<u8, Error> {
    match byte {
        b'0'..=b'9' => Ok(byte - b'0'),
        b'a'..=b'f' => Ok(byte - b'a' + 10),
        b'A'..=b'F' => Ok(byte - b'A' + 10),
        _ => Err(Error::new("bad percent-encoding hex digit")),
    }
}

pub(crate) fn encode_list(cells: &[String]) -> String {
    cells
        .iter()
        .map(|cell| percent_encode(cell))
        .collect::<Vec<_>>()
        .join("|")
}

pub(crate) fn decode_list(value: &str) -> Result<Vec<String>, Error> {
    if value.is_empty() {
        return Ok(Vec::new());
    }
    value
        .split('|')
        .map(percent_decode)
        .collect::<Result<Vec<_>, _>>()
}

pub(crate) fn encode_rows(rows: &[Vec<String>]) -> String {
    rows.iter()
        .map(|row| encode_list(row))
        .collect::<Vec<_>>()
        .join(";")
}

pub(crate) fn decode_rows(value: &str) -> Result<Vec<Vec<String>>, Error> {
    if value.is_empty() {
        return Ok(Vec::new());
    }
    value
        .split(';')
        .map(decode_list)
        .collect::<Result<Vec<_>, _>>()
}

pub(crate) fn encode_props(props: &BTreeMap<String, String>) -> String {
    props
        .iter()
        .map(|(key, value)| format!("{}={}", percent_encode(key), percent_encode(value)))
        .collect::<Vec<_>>()
        .join("&")
}

pub(crate) fn decode_props(value: &str) -> Result<BTreeMap<String, String>, Error> {
    let mut props = BTreeMap::new();
    if value.is_empty() {
        return Ok(props);
    }
    for pair in value.split('&') {
        let (key, val) = pair
            .split_once('=')
            .ok_or_else(|| Error::new(format!("bad property `{pair}`")))?;
        props.insert(percent_decode(key)?, percent_decode(val)?);
    }
    Ok(props)
}

pub(crate) fn record_body(node: &Node, parent: Option<&str>) -> String {
    let mut parts = Vec::new();
    parts.push(format!("id={}", percent_encode(&node.id)));
    parts.push(format!("kind={}", node.kind.as_str()));
    parts.push(format!("name={}", percent_encode(&node.name)));
    if !node.text.is_empty() {
        parts.push(format!("text={}", percent_encode(&node.text)));
    }
    if !node.columns.is_empty() {
        parts.push(format!("columns={}", encode_list(&node.columns)));
    }
    if !node.rows.is_empty() {
        parts.push(format!("rows={}", encode_rows(&node.rows)));
    }
    if let Some(dock) = node.dock {
        parts.push(format!("dock={}", dock.as_str()));
    }
    if let Some(axis) = node.axis {
        parts.push(format!("axis={}", axis.as_str()));
    }
    if !node.props.is_empty() {
        parts.push(format!("props={}", encode_props(&node.props)));
    }
    if let Some(parent) = parent {
        parts.push(format!("parent={}", percent_encode(parent)));
    }
    parts.join(" ")
}

pub(crate) fn line_comment(body: &str) -> String {
    format!("// uii {body}")
}

pub(crate) fn xml_comment(body: &str) -> String {
    format!("<!-- uii {body} -->")
}

pub(crate) fn document_line(name: &str) -> String {
    format!("// uii-document name={}", percent_encode(name))
}

pub(crate) fn document_xml(name: &str) -> String {
    format!("<!-- uii-document name={} -->", percent_encode(name))
}

struct Rec {
    node: Node,
    parent: Option<String>,
}

pub(crate) fn document_from_records(source: &str) -> Result<Document, Error> {
    let (name, records) = scan(source)?;
    if records.is_empty() {
        return Err(Error::new("no UII records"));
    }
    let mut by_id: BTreeMap<String, Node> = BTreeMap::new();
    let mut order: Vec<String> = Vec::new();
    let mut parent_of: BTreeMap<String, Option<String>> = BTreeMap::new();
    for rec in records {
        if by_id.contains_key(&rec.node.id) {
            return Err(Error::new(format!("duplicate id `{}`", rec.node.id)));
        }
        order.push(rec.node.id.clone());
        parent_of.insert(rec.node.id.clone(), rec.parent);
        by_id.insert(rec.node.id.clone(), rec.node);
    }
    for (id, parent) in &parent_of {
        if let Some(parent) = parent {
            if !by_id.contains_key(parent) {
                return Err(Error::new(format!("`{id}` parent `{parent}` is missing")));
            }
        }
    }
    let mut child_ids: BTreeMap<String, Vec<String>> = BTreeMap::new();
    let mut roots = Vec::new();
    for id in &order {
        match parent_of.get(id).and_then(|p| p.clone()) {
            Some(parent) => child_ids.entry(parent).or_default().push(id.clone()),
            None => roots.push(id.clone()),
        }
    }
    if roots.len() != 1 {
        return Err(Error::new(format!(
            "expected one root record, found {}",
            roots.len()
        )));
    }
    fn take(
        id: &str,
        by_id: &mut BTreeMap<String, Node>,
        child_ids: &BTreeMap<String, Vec<String>>,
    ) -> Node {
        let mut node = by_id.remove(id).expect("id present");
        if let Some(kids) = child_ids.get(id) {
            node.children = kids.iter().map(|kid| take(kid, by_id, child_ids)).collect();
        }
        node
    }
    let root = take(&roots[0], &mut by_id, &child_ids);
    let name = name.unwrap_or_else(|| root.name.clone());
    Ok(Document { name, root })
}

fn scan(source: &str) -> Result<(Option<String>, Vec<Rec>), Error> {
    let mut name = None;
    let mut records = Vec::new();
    for raw in source.lines() {
        let line = raw.trim();
        if let Some(body) = comment_body(line, "uii-document ") {
            let mut doc_name = None;
            for (key, value) in tokens(body)? {
                if key == "name" {
                    doc_name = Some(percent_decode(&value)?);
                }
            }
            name = doc_name.or(name);
            continue;
        }
        if let Some(body) = comment_body(line, "uii ") {
            records.push(parse_record(body)?);
        }
    }
    Ok((name, records))
}

fn comment_body<'a>(line: &'a str, prefix: &str) -> Option<&'a str> {
    let rest = if let Some(rest) = line.strip_prefix("// ") {
        rest
    } else if let Some(rest) = line.strip_prefix("<!-- ") {
        rest.trim_end().trim_end_matches("-->").trim()
    } else {
        return None;
    };
    rest.strip_prefix(prefix)
}

fn parse_record(body: &str) -> Result<Rec, Error> {
    let mut id = None;
    let mut kind = None;
    let mut name = None;
    let mut text = String::new();
    let mut columns = Vec::new();
    let mut rows = Vec::new();
    let mut dock = None;
    let mut axis = None;
    let mut props = BTreeMap::new();
    let mut parent = None;
    for (key, value) in tokens(body)? {
        match key.as_str() {
            "id" => id = Some(percent_decode(&value)?),
            "kind" => kind = Some(NodeKind::parse(&value)?),
            "name" => name = Some(percent_decode(&value)?),
            "text" => text = percent_decode(&value)?,
            "columns" => columns = decode_list(&value)?,
            "rows" => rows = decode_rows(&value)?,
            "dock" => dock = Some(DockSide::parse(&value)?),
            "axis" => axis = Some(Axis::parse(&value)?),
            "props" => props = decode_props(&value)?,
            "parent" => parent = Some(percent_decode(&value)?),
            other => return Err(Error::new(format!("unknown record field `{other}`"))),
        }
    }
    let id = id.ok_or_else(|| Error::new("UII record is missing id"))?;
    let kind = kind.ok_or_else(|| Error::new(format!("UII record `{id}` is missing kind")))?;
    let name = name.unwrap_or_else(|| id.clone());
    Ok(Rec {
        node: Node {
            id,
            kind,
            name,
            text,
            columns,
            rows,
            dock,
            axis,
            props,
            children: Vec::new(),
        },
        parent,
    })
}

fn tokens(body: &str) -> Result<Vec<(String, String)>, Error> {
    let mut out = Vec::new();
    for token in body.split_whitespace() {
        let (key, value) = token
            .split_once('=')
            .ok_or_else(|| Error::new(format!("bad UII token `{token}`")))?;
        out.push((key.to_string(), value.to_string()));
    }
    Ok(out)
}

pub(crate) fn rust_string(value: &str) -> String {
    format!("\"{}\"", escape_common(value))
}

pub(crate) fn c_string(value: &str) -> String {
    format!("\"{}\"", escape_common(value))
}

pub(crate) fn xml_text(value: &str) -> String {
    let mut out = String::new();
    for ch in value.chars() {
        match ch {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            _ => out.push(ch),
        }
    }
    out
}

pub(crate) fn xml_attr(value: &str) -> String {
    let mut out = String::new();
    for ch in value.chars() {
        match ch {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            _ => out.push(ch),
        }
    }
    out
}

fn escape_common(value: &str) -> String {
    let mut out = String::new();
    for ch in value.chars() {
        match ch {
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            _ => out.push(ch),
        }
    }
    out
}

pub(crate) fn c_ident(id: &str) -> String {
    let mut out = String::from("n_");
    if id
        .chars()
        .next()
        .map(|ch| ch.is_ascii_digit())
        .unwrap_or(true)
    {
        out.push('_');
    }
    for ch in id.chars() {
        if ch.is_ascii_alphanumeric() {
            out.push(ch);
        } else {
            out.push('_');
        }
    }
    out
}

pub(crate) fn type_name(name: &str) -> String {
    let mut out = String::new();
    let mut cap = true;
    for ch in name.chars() {
        if ch.is_ascii_alphanumeric() {
            if cap {
                out.extend(ch.to_uppercase());
                cap = false;
            } else {
                out.push(ch);
            }
        } else {
            cap = true;
        }
    }
    if out.is_empty() || out.chars().next().unwrap().is_ascii_digit() {
        out.insert_str(0, "Uii");
    }
    out
}
