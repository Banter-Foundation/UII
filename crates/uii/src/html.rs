//! HTML/CSS import and export.
//!
//! Regions (`header`, `nav`, `main`, `aside`, `footer`, `section`, `article`)
//! become dock panels. `table` becomes a table. Nested `div` boxes stay boxes
//! so a later export can turn them into child windows. A `<style>` block's
//! `#id` rules set the box axis (`flex-direction`) and other properties.

use std::collections::BTreeMap;

use crate::codec::{self, xml_attr, xml_text};
use crate::error::Error;
use crate::model::{Axis, DockSide, Document, Node, NodeKind};

pub(crate) fn import(source: &str) -> Result<Document, Error> {
    if source.trim().is_empty() {
        return Err(Error::new("empty HTML document"));
    }
    let dom = parse_fragment(source)?;
    let mut title = None;
    let mut css = String::new();
    let mut roots = Vec::new();
    for node in dom {
        collect(node, &mut roots, &mut title, &mut css);
    }
    let mut counter = 1usize;
    let mut nodes = Vec::new();
    for root in roots {
        if let Some(node) = convert(root, &mut counter)? {
            nodes.push(node);
        }
    }
    if nodes.is_empty() {
        return Err(Error::new("HTML document has no elements"));
    }
    let mut document = if nodes.len() == 1 && nodes[0].kind == NodeKind::Window {
        let root = nodes.pop().unwrap();
        let name = root
            .props
            .get("uii-document")
            .cloned()
            .or(title)
            .unwrap_or_else(|| root.name.clone());
        let mut root = root;
        root.props.remove("uii-document");
        Document { name, root }
    } else {
        let name = title.unwrap_or_else(|| "document".to_string());
        let root = Node {
            id: fresh("window", &nodes, &mut counter),
            kind: NodeKind::Window,
            name: name.clone(),
            text: String::new(),
            columns: Vec::new(),
            rows: Vec::new(),
            dock: None,
            axis: None,
            props: BTreeMap::new(),
            children: nodes,
        };
        Document { name, root }
    };
    lift_all(&mut document.root);
    apply_css(&mut document.root, &parse_css(&css));
    ensure_unique(&document.root, &mut BTreeMap::new())?;
    Ok(document)
}

pub(crate) fn export(document: &Document) -> Result<String, Error> {
    let mut css = String::new();
    write_css(&document.root, &mut css);
    let mut body = String::new();
    write_node(&document.root, &document.name, true, 1, &mut body);
    let mut out = String::new();
    out.push_str("<!DOCTYPE html>\n<html>\n<head>\n<meta charset=\"utf-8\">\n<title>");
    out.push_str(&xml_text(&document.name));
    out.push_str("</title>\n");
    if !css.is_empty() {
        out.push_str("<style>\n");
        out.push_str(&css);
        out.push_str("</style>\n");
    }
    out.push_str("</head>\n<body>\n");
    out.push_str(&body);
    out.push_str("</body>\n</html>\n");
    Ok(out)
}

fn fresh(preferred: &str, existing: &[Node], counter: &mut usize) -> String {
    if !contains_id(existing, preferred) {
        return preferred.to_string();
    }
    loop {
        let id = format!("n{counter}");
        *counter += 1;
        if !contains_id(existing, &id) {
            return id;
        }
    }
}

fn contains_id(nodes: &[Node], id: &str) -> bool {
    nodes
        .iter()
        .any(|node| node.id == id || contains_id(&node.children, id))
}

fn collect(dom: Dom, roots: &mut Vec<Dom>, title: &mut Option<String>, css: &mut String) {
    match dom {
        Dom::Text(_) => {}
        Dom::El { tag, children, .. } if matches!(tag.as_str(), "html" | "head" | "body") => {
            for child in children {
                collect(child, roots, title, css);
            }
        }
        Dom::El { tag, children, .. } if tag == "title" => {
            *title = Some(text_of(&children));
        }
        Dom::El { tag, children, .. } if tag == "style" => {
            css.push_str(&text_of(&children));
            css.push('\n');
        }
        Dom::El { tag, .. } if matches!(tag.as_str(), "script" | "meta" | "link") => {}
        other => roots.push(other),
    }
}

fn text_of(children: &[Dom]) -> String {
    let mut out = String::new();
    for child in children {
        if let Dom::Text(text) = child {
            out.push_str(text);
        }
    }
    out.trim().to_string()
}

fn convert(dom: Dom, counter: &mut usize) -> Result<Option<Node>, Error> {
    let Dom::El {
        tag,
        attrs,
        children,
    } = dom
    else {
        return Ok(None);
    };
    if matches!(tag.as_str(), "script" | "meta" | "link" | "title" | "style") {
        return Ok(None);
    }
    let explicit = attr(&attrs, "data-uii-kind");
    let kind = if let Some(kind) = explicit {
        NodeKind::parse(kind)?
    } else {
        infer_kind(&tag, &attrs)
    };
    let id = attr(&attrs, "data-uii-id")
        .or_else(|| attr(&attrs, "id"))
        .map(|s| s.to_string())
        .unwrap_or_else(|| {
            let id = format!("n{counter}");
            *counter += 1;
            id
        });
    let name = attr(&attrs, "data-uii-name")
        .map(|s| s.to_string())
        .unwrap_or_else(|| id.clone());
    let mut text = attr(&attrs, "data-uii-text")
        .map(|s| s.to_string())
        .unwrap_or_default();
    let mut columns = match attr(&attrs, "data-uii-columns") {
        Some(value) => codec::decode_list(value)?,
        None => Vec::new(),
    };
    let mut rows = match attr(&attrs, "data-uii-rows") {
        Some(value) => codec::decode_rows(value)?,
        None => Vec::new(),
    };
    let dock = if let Some(value) = attr(&attrs, "data-uii-dock") {
        Some(DockSide::parse(value)?)
    } else if explicit.is_none() && kind == NodeKind::Region {
        Some(infer_side(&tag))
    } else {
        None
    };
    let axis = if let Some(value) = attr(&attrs, "data-uii-axis") {
        Some(Axis::parse(value)?)
    } else if explicit.is_none() && kind == NodeKind::Box {
        Some(Axis::Vertical)
    } else {
        None
    };
    let mut props = match attr(&attrs, "data-uii-props") {
        Some(value) => codec::decode_props(value)?,
        None => BTreeMap::new(),
    };
    if let Some(doc_name) = attr(&attrs, "data-uii-document") {
        props.insert("uii-document".into(), doc_name.to_string());
    }
    if props.is_empty() {
        if let Some(placeholder) = attr(&attrs, "placeholder") {
            props.insert("placeholder".into(), placeholder.to_string());
        }
        if kind == NodeKind::Slider {
            if let Some(value) = attr(&attrs, "value") {
                props.insert("value".into(), value.to_string());
            }
        }
        if kind == NodeKind::Checkbox && attrs.iter().any(|(key, _)| key == "checked") {
            props.insert("checked".into(), "true".into());
        }
    }
    if text.is_empty() && kind == NodeKind::TextInput {
        if let Some(value) = attr(&attrs, "value") {
            text = value.to_string();
        }
    }
    if text.is_empty() && kind == NodeKind::Image {
        if let Some(src) = attr(&attrs, "src") {
            text = src.to_string();
        }
    }

    let structural_table = kind == NodeKind::Table && columns.is_empty() && rows.is_empty();
    if structural_table {
        columns = harvest_headers(&children);
        rows = harvest_rows(&children);
    }

    let mut child_nodes = Vec::new();
    let mut loose = String::new();
    for child in children {
        match child {
            Dom::Text(value) => loose.push_str(&value),
            Dom::El { tag, .. }
                if kind == NodeKind::Table
                    && matches!(
                        tag.as_str(),
                        "thead" | "tbody" | "tfoot" | "tr" | "colgroup" | "col"
                    ) => {}
            other => {
                if let Some(node) = convert(other, counter)? {
                    child_nodes.push(node);
                }
            }
        }
    }
    let loose = loose.trim();
    if text.is_empty() && attr(&attrs, "data-uii-text").is_none() {
        if child_nodes.is_empty() {
            text = loose.to_string();
        } else if !loose.is_empty() {
            let label_id = format!("n{counter}");
            *counter += 1;
            child_nodes.insert(
                0,
                Node {
                    id: label_id.clone(),
                    kind: NodeKind::Label,
                    name: label_id,
                    text: loose.to_string(),
                    columns: Vec::new(),
                    rows: Vec::new(),
                    dock: None,
                    axis: None,
                    props: BTreeMap::new(),
                    children: Vec::new(),
                },
            );
        }
    }

    Ok(Some(Node {
        id,
        kind,
        name,
        text,
        columns,
        rows,
        dock,
        axis,
        props,
        children: child_nodes,
    }))
}

fn infer_kind(tag: &str, attrs: &[(String, String)]) -> NodeKind {
    match tag {
        "header" | "nav" | "main" | "aside" | "footer" | "section" | "article" => NodeKind::Region,
        "table" => NodeKind::Table,
        "button" => NodeKind::Button,
        "textarea" => NodeKind::TextInput,
        "img" => NodeKind::Image,
        "hr" => NodeKind::Separator,
        "input" => match attr(attrs, "type").unwrap_or("text") {
            "checkbox" => NodeKind::Checkbox,
            "range" => NodeKind::Slider,
            _ => NodeKind::TextInput,
        },
        "div" | "ul" | "ol" | "form" => NodeKind::Box,
        _ => NodeKind::Label,
    }
}

fn infer_side(tag: &str) -> DockSide {
    match tag {
        "header" => DockSide::Top,
        "nav" => DockSide::Left,
        "aside" => DockSide::Right,
        "footer" => DockSide::Bottom,
        _ => DockSide::Center,
    }
}

fn harvest_headers(children: &[Dom]) -> Vec<String> {
    let mut headers = Vec::new();
    walk_cells(children, true, &mut headers);
    headers
}

fn harvest_rows(children: &[Dom]) -> Vec<Vec<String>> {
    let mut rows = Vec::new();
    for child in children {
        match child {
            Dom::El { tag, children, .. } if tag == "tr" && !row_is_header(children) => {
                let mut cells = Vec::new();
                walk_cells(std::slice::from_ref(child), false, &mut cells);
                if !cells.is_empty() {
                    rows.push(cells);
                }
            }
            Dom::El { tag, children, .. } if matches!(tag.as_str(), "tbody" | "tfoot") => {
                rows.extend(harvest_rows(children));
            }
            _ => {}
        }
    }
    rows
}

fn row_is_header(children: &[Dom]) -> bool {
    children
        .iter()
        .any(|child| matches!(child, Dom::El { tag, .. } if tag == "th"))
        && children.iter().all(|child| match child {
            Dom::Text(text) => text.trim().is_empty(),
            Dom::El { tag, .. } => tag == "th",
        })
}

fn walk_cells(nodes: &[Dom], header: bool, into: &mut Vec<String>) {
    for node in nodes {
        match node {
            Dom::El { tag, children, .. } if tag == "tr" || tag == "thead" || tag == "tbody" => {
                walk_cells(children, header, into);
            }
            Dom::El { tag, children, .. }
                if (header && tag == "th") || (!header && tag == "td") =>
            {
                into.push(text_deep(children));
            }
            _ => {}
        }
    }
}

fn text_deep(children: &[Dom]) -> String {
    let mut out = String::new();
    for child in children {
        match child {
            Dom::Text(text) => out.push_str(text),
            Dom::El { children, .. } => out.push_str(&text_deep(children)),
        }
    }
    out.trim().to_string()
}

fn lift_all(node: &mut Node) {
    for child in &mut node.children {
        lift_all(child);
    }
    if node.kind != NodeKind::Dock {
        node.children = lift_regions(std::mem::take(&mut node.children), &node.id);
    }
}

fn lift_regions(children: Vec<Node>, parent_id: &str) -> Vec<Node> {
    if !children.iter().any(|child| child.kind == NodeKind::Region) {
        return children;
    }
    let mut regions = Vec::new();
    let mut others = Vec::new();
    let mut dock_at = None;
    for child in children {
        if child.kind == NodeKind::Region {
            if dock_at.is_none() {
                dock_at = Some(others.len());
            }
            regions.push(child);
        } else {
            others.push(child);
        }
    }
    let mut id = format!("dock-{parent_id}");
    let mut n = 2;
    while others.iter().any(|node| node.id == id) || regions.iter().any(|node| node.id == id) {
        id = format!("dock-{parent_id}-{n}");
        n += 1;
    }
    let dock = Node {
        id,
        kind: NodeKind::Dock,
        name: "dock".into(),
        text: String::new(),
        columns: Vec::new(),
        rows: Vec::new(),
        dock: None,
        axis: None,
        props: BTreeMap::new(),
        children: regions,
    };
    others.insert(dock_at.unwrap_or(0), dock);
    others
}

struct CssRule {
    id: String,
    decls: Vec<(String, String)>,
}

fn parse_css(css: &str) -> Vec<CssRule> {
    let mut rules = Vec::new();
    for chunk in css.split('}') {
        let Some((selector, body)) = chunk.split_once('{') else {
            continue;
        };
        for sel in selector.split(',') {
            let Some(id) = selector_id(sel) else {
                continue;
            };
            let mut decls = Vec::new();
            for decl in body.split(';') {
                let Some((prop, value)) = decl.split_once(':') else {
                    continue;
                };
                let prop = prop.trim().to_ascii_lowercase();
                let value = value.trim().to_string();
                if prop.is_empty() || value.is_empty() {
                    continue;
                }
                decls.push((prop, value));
            }
            if !decls.is_empty() {
                rules.push(CssRule { id, decls });
            }
        }
    }
    rules
}

fn selector_id(selector: &str) -> Option<String> {
    let selector = selector.trim();
    let rest = selector.strip_prefix('#')?;
    let id: String = rest
        .chars()
        .take_while(|ch| ch.is_ascii_alphanumeric() || *ch == '-' || *ch == '_')
        .collect();
    if id.is_empty() {
        None
    } else {
        Some(id)
    }
}

fn apply_css(node: &mut Node, rules: &[CssRule]) {
    for rule in rules.iter().filter(|rule| rule.id == node.id) {
        for (prop, value) in &rule.decls {
            match prop.as_str() {
                "display" => {}
                "flex-direction" => {
                    node.axis = Some(match value.as_str() {
                        "row" | "row-reverse" => Axis::Horizontal,
                        _ => Axis::Vertical,
                    });
                }
                _ => {
                    node.props.insert(prop.clone(), value.clone());
                }
            }
        }
    }
    for child in &mut node.children {
        apply_css(child, rules);
    }
}

fn ensure_unique(node: &Node, seen: &mut BTreeMap<String, ()>) -> Result<(), Error> {
    if seen.insert(node.id.clone(), ()).is_some() {
        return Err(Error::new(format!("duplicate id `{}`", node.id)));
    }
    for child in &node.children {
        ensure_unique(child, seen)?;
    }
    Ok(())
}

fn write_css(node: &Node, out: &mut String) {
    if node.axis.is_some() || !node.props.is_empty() {
        out.push('#');
        out.push_str(&node.id);
        out.push_str(" {\n");
        if let Some(axis) = node.axis {
            out.push_str("  display: flex;\n  flex-direction: ");
            out.push_str(match axis {
                Axis::Vertical => "column",
                Axis::Horizontal => "row",
            });
            out.push_str(";\n");
        }
        for (key, value) in &node.props {
            out.push_str("  ");
            out.push_str(key);
            out.push_str(": ");
            out.push_str(value);
            out.push_str(";\n");
        }
        out.push_str("}\n");
    }
    for child in &node.children {
        write_css(child, out);
    }
}

fn write_node(node: &Node, document_name: &str, is_root: bool, depth: usize, out: &mut String) {
    let pad = "  ".repeat(depth);
    let tag = match node.kind {
        NodeKind::Region => match node.dock.unwrap_or(DockSide::Center) {
            DockSide::Top => "header",
            DockSide::Left => "nav",
            DockSide::Right => "aside",
            DockSide::Bottom => "footer",
            DockSide::Center | DockSide::Tab => "main",
        },
        NodeKind::HeaderBar => "header",
        NodeKind::Table => "table",
        NodeKind::Button => "button",
        NodeKind::TextInput | NodeKind::Checkbox | NodeKind::Slider => "input",
        NodeKind::Image => "img",
        NodeKind::Separator => "hr",
        NodeKind::Label => "span",
        _ => "div",
    };
    out.push_str(&pad);
    out.push('<');
    out.push_str(tag);
    write_attrs(node, document_name, is_root, tag, out);
    if matches!(tag, "img" | "input" | "hr") {
        out.push_str(">\n");
        return;
    }
    out.push('>');
    if node.kind == NodeKind::Table {
        out.push('\n');
        write_table(node, depth + 1, out);
        write_children_only(node, document_name, depth, out);
        out.push_str(&pad);
        out.push_str("</table>\n");
        return;
    }
    if node.children.is_empty() {
        out.push_str(&xml_text(&node.text));
        out.push_str("</");
        out.push_str(tag);
        out.push_str(">\n");
        return;
    }
    if !node.text.is_empty() {
        out.push_str(&xml_text(&node.text));
    }
    out.push('\n');
    for child in &node.children {
        write_node(child, document_name, false, depth + 1, out);
    }
    out.push_str(&pad);
    out.push_str("</");
    out.push_str(tag);
    out.push_str(">\n");
}

fn write_children_only(node: &Node, document_name: &str, depth: usize, out: &mut String) {
    for child in &node.children {
        write_node(child, document_name, false, depth + 1, out);
    }
}

fn write_table(node: &Node, depth: usize, out: &mut String) {
    let pad = "  ".repeat(depth);
    if !node.columns.is_empty() {
        out.push_str(&pad);
        out.push_str("<thead><tr>");
        for column in &node.columns {
            out.push_str("<th>");
            out.push_str(&xml_text(column));
            out.push_str("</th>");
        }
        out.push_str("</tr></thead>\n");
    }
    if !node.rows.is_empty() {
        out.push_str(&pad);
        out.push_str("<tbody>\n");
        for row in &node.rows {
            out.push_str(&pad);
            out.push_str("  <tr>");
            for cell in row {
                out.push_str("<td>");
                out.push_str(&xml_text(cell));
                out.push_str("</td>");
            }
            out.push_str("</tr>\n");
        }
        out.push_str(&pad);
        out.push_str("</tbody>\n");
    }
}

fn write_attrs(node: &Node, document_name: &str, is_root: bool, tag: &str, out: &mut String) {
    out.push_str(" data-uii-id=\"");
    out.push_str(&xml_attr(&node.id));
    out.push_str("\" data-uii-kind=\"");
    out.push_str(node.kind.as_str());
    out.push_str("\" data-uii-name=\"");
    out.push_str(&xml_attr(&node.name));
    out.push('"');
    if is_root {
        out.push_str(" data-uii-document=\"");
        out.push_str(&xml_attr(document_name));
        out.push('"');
    }
    if !node.text.is_empty() {
        out.push_str(" data-uii-text=\"");
        out.push_str(&xml_attr(&node.text));
        out.push('"');
    }
    if let Some(dock) = node.dock {
        out.push_str(" data-uii-dock=\"");
        out.push_str(dock.as_str());
        out.push('"');
    }
    if let Some(axis) = node.axis {
        out.push_str(" data-uii-axis=\"");
        out.push_str(axis.as_str());
        out.push('"');
    }
    if !node.columns.is_empty() {
        out.push_str(" data-uii-columns=\"");
        out.push_str(&xml_attr(&codec::encode_list(&node.columns)));
        out.push('"');
    }
    if !node.rows.is_empty() {
        out.push_str(" data-uii-rows=\"");
        out.push_str(&xml_attr(&codec::encode_rows(&node.rows)));
        out.push('"');
    }
    if !node.props.is_empty() {
        out.push_str(" data-uii-props=\"");
        out.push_str(&xml_attr(&codec::encode_props(&node.props)));
        out.push('"');
    }
    match node.kind {
        NodeKind::TextInput => {
            out.push_str(" type=\"text\" value=\"");
            out.push_str(&xml_attr(&node.text));
            out.push('"');
            if let Some(placeholder) = node.props.get("placeholder") {
                out.push_str(" placeholder=\"");
                out.push_str(&xml_attr(placeholder));
                out.push('"');
            }
        }
        NodeKind::Checkbox => {
            out.push_str(" type=\"checkbox\"");
            if node.props.get("checked").map(String::as_str) == Some("true") {
                out.push_str(" checked");
            }
        }
        NodeKind::Slider => {
            out.push_str(" type=\"range\"");
            if let Some(value) = node.props.get("value") {
                out.push_str(" value=\"");
                out.push_str(&xml_attr(value));
                out.push('"');
            }
        }
        NodeKind::Image => {
            out.push_str(" src=\"");
            out.push_str(&xml_attr(&node.text));
            out.push_str("\" alt=\"\"");
        }
        _ => {}
    }
    let _ = tag;
}

fn attr<'a>(attrs: &'a [(String, String)], name: &str) -> Option<&'a str> {
    attrs
        .iter()
        .find(|(key, _)| key == name)
        .map(|(_, value)| value.as_str())
}

enum Dom {
    El {
        tag: String,
        attrs: Vec<(String, String)>,
        children: Vec<Dom>,
    },
    Text(String),
}

fn parse_fragment(source: &str) -> Result<Vec<Dom>, Error> {
    let mut parser = Parser { s: source, i: 0 };
    parser.parse_nodes(None)
}

struct Parser<'a> {
    s: &'a str,
    i: usize,
}

impl<'a> Parser<'a> {
    fn parse_nodes(&mut self, until: Option<&str>) -> Result<Vec<Dom>, Error> {
        let mut nodes = Vec::new();
        while self.i < self.s.len() {
            if self.s[self.i..].starts_with("</") {
                let close = self.read_close()?;
                if until == Some(close.as_str()) {
                    return Ok(nodes);
                }
                return Err(Error::new(format!("unexpected closing tag `</{close}>`")));
            }
            if self.s[self.i..].starts_with("<!--") {
                self.skip_comment()?;
                continue;
            }
            if self.s[self.i..].starts_with("<!") {
                self.skip_until('>')?;
                continue;
            }
            if self.s[self.i..].starts_with('<') {
                let el = self.parse_element()?;
                nodes.push(el);
                continue;
            }
            let text = self.read_text();
            if !text.is_empty() {
                nodes.push(Dom::Text(text));
            }
        }
        if let Some(tag) = until {
            return Err(Error::new(format!("unclosed `<{tag}>`")));
        }
        Ok(nodes)
    }

    fn parse_element(&mut self) -> Result<Dom, Error> {
        self.i += 1;
        let tag = self.read_name()?.to_ascii_lowercase();
        let attrs = self.read_attrs()?;
        self.skip_ws();
        let self_close = self.s[self.i..].starts_with("/>");
        if self_close {
            self.i += 2;
        } else if self.s[self.i..].starts_with('>') {
            self.i += 1;
        } else {
            return Err(Error::new(format!("malformed `<{tag}>`")));
        }
        if self_close || is_void(&tag) {
            return Ok(Dom::El {
                tag,
                attrs,
                children: Vec::new(),
            });
        }
        let children = if tag == "script" || tag == "style" {
            let raw = self.read_raw(&tag)?;
            if raw.is_empty() {
                Vec::new()
            } else {
                vec![Dom::Text(raw)]
            }
        } else {
            self.parse_nodes(Some(&tag))?
        };
        Ok(Dom::El {
            tag,
            attrs,
            children,
        })
    }

    fn read_attrs(&mut self) -> Result<Vec<(String, String)>, Error> {
        let mut attrs = Vec::new();
        loop {
            self.skip_ws();
            if self.i >= self.s.len()
                || self.s[self.i..].starts_with('>')
                || self.s[self.i..].starts_with("/>")
            {
                break;
            }
            let name = self.read_name()?.to_ascii_lowercase();
            self.skip_ws();
            if self.s[self.i..].starts_with('=') {
                self.i += 1;
                self.skip_ws();
                let value = self.read_value()?;
                attrs.push((name, decode_entities(&value)));
            } else {
                attrs.push((name, "true".into()));
            }
        }
        Ok(attrs)
    }

    fn read_value(&mut self) -> Result<String, Error> {
        if self.i >= self.s.len() {
            return Err(Error::new("unterminated attribute"));
        }
        let bytes = self.s.as_bytes();
        let quote = bytes[self.i];
        if quote == b'"' || quote == b'\'' {
            self.i += 1;
            let start = self.i;
            while self.i < self.s.len() && self.s.as_bytes()[self.i] != quote {
                self.i += 1;
            }
            if self.i >= self.s.len() {
                return Err(Error::new("unterminated attribute"));
            }
            let value = self.s[start..self.i].to_string();
            self.i += 1;
            Ok(value)
        } else {
            let start = self.i;
            while self.i < self.s.len() {
                let ch = self.s[self.i..].chars().next().unwrap();
                if ch.is_whitespace() || ch == '>' || ch == '/' {
                    break;
                }
                self.i += ch.len_utf8();
            }
            Ok(self.s[start..self.i].to_string())
        }
    }

    fn read_name(&mut self) -> Result<String, Error> {
        let start = self.i;
        while self.i < self.s.len() {
            let ch = self.s[self.i..].chars().next().unwrap();
            if ch.is_ascii_alphanumeric() || ch == '-' || ch == ':' || ch == '_' {
                self.i += ch.len_utf8();
            } else {
                break;
            }
        }
        if start == self.i {
            return Err(Error::new("expected a tag or attribute name"));
        }
        Ok(self.s[start..self.i].to_string())
    }

    fn read_text(&mut self) -> String {
        let start = self.i;
        if let Some(rel) = self.s[self.i..].find('<') {
            self.i += rel;
        } else {
            self.i = self.s.len();
        }
        decode_entities(&self.s[start..self.i])
    }

    fn read_raw(&mut self, tag: &str) -> Result<String, Error> {
        let close = format!("</{tag}");
        let start = self.i;
        if let Some(rel) = self.s[self.i..].to_ascii_lowercase().find(&close) {
            let raw = self.s[start..self.i + rel].to_string();
            self.i += rel;
            let _ = self.read_close()?;
            Ok(raw)
        } else {
            Err(Error::new(format!("unclosed `<{tag}>`")))
        }
    }

    fn read_close(&mut self) -> Result<String, Error> {
        if !self.s[self.i..].starts_with("</") {
            return Err(Error::new("expected a closing tag"));
        }
        self.i += 2;
        let name = self.read_name()?.to_ascii_lowercase();
        self.skip_ws();
        if !self.s[self.i..].starts_with('>') {
            return Err(Error::new(format!("malformed `</{name}>`")));
        }
        self.i += 1;
        Ok(name)
    }

    fn skip_comment(&mut self) -> Result<(), Error> {
        if let Some(rel) = self.s[self.i..].find("-->") {
            self.i += rel + 3;
            Ok(())
        } else {
            Err(Error::new("unterminated comment"))
        }
    }

    fn skip_until(&mut self, ch: char) -> Result<(), Error> {
        if let Some(rel) = self.s[self.i..].find(ch) {
            self.i += rel + ch.len_utf8();
            Ok(())
        } else {
            Err(Error::new("unterminated markup declaration"))
        }
    }

    fn skip_ws(&mut self) {
        while self.i < self.s.len() {
            let ch = self.s[self.i..].chars().next().unwrap();
            if ch.is_whitespace() {
                self.i += ch.len_utf8();
            } else {
                break;
            }
        }
    }
}

fn is_void(tag: &str) -> bool {
    matches!(
        tag,
        "area"
            | "base"
            | "br"
            | "col"
            | "embed"
            | "hr"
            | "img"
            | "input"
            | "link"
            | "meta"
            | "source"
            | "track"
            | "wbr"
    )
}

fn decode_entities(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    let mut rest = value;
    while let Some(start) = rest.find('&') {
        out.push_str(&rest[..start]);
        rest = &rest[start..];
        if let Some(end) = rest.find(';') {
            let entity = &rest[1..end];
            if let Some(ch) = named_entity(entity).or_else(|| numeric_entity(entity)) {
                out.push(ch);
                rest = &rest[end + 1..];
                continue;
            }
        }
        out.push('&');
        rest = &rest[1..];
    }
    out.push_str(rest);
    out
}

fn named_entity(entity: &str) -> Option<char> {
    match entity {
        "amp" => Some('&'),
        "lt" => Some('<'),
        "gt" => Some('>'),
        "quot" => Some('"'),
        "apos" => Some('\''),
        "nbsp" => Some('\u{00a0}'),
        _ => None,
    }
}

fn numeric_entity(entity: &str) -> Option<char> {
    let hex = entity.strip_prefix('#')?;
    let code = if let Some(hex) = hex.strip_prefix(['x', 'X']) {
        u32::from_str_radix(hex, 16).ok()?
    } else {
        hex.parse().ok()?
    };
    char::from_u32(code)
}
