//! HTML and CSS are the layout Owl edits. Import turns markup into nodes.
//! Export writes the same nodes back to HTML and CSS.

use crate::document::{DockSlot, Document, NodeId, NodeKind, Rect, Rgba};

const TEXT_WIDTH: f32 = 0.56;
const LINE_HEIGHT: f32 = 1.4;

pub fn import_html_css(html: &str, css: &str) -> Document {
    let mut doc = Document::new("Imported");
    let (roots, embedded) = parse_fragment(html);
    let rules = parse_css(&format!("{embedded}\n{css}"));
    let mut cursor_y = 0.0;
    for el in roots {
        if is_skipped_tag(&el.tag) {
            continue;
        }
        let advance = place(&mut doc, &el, None, &[], &rules, 0.0, cursor_y, 800.0, None);
        cursor_y += advance;
    }
    doc
}

pub(crate) fn export_html_document(doc: &Document) -> String {
    let (markup, css) = html_and_css_parts(doc);
    format!("<style>\n{css}</style>\n{markup}")
}

pub(crate) fn html_and_css_parts(doc: &Document) -> (String, String) {
    let mut css = String::new();
    let mut html = String::new();
    for id in doc.roots() {
        write_node(doc, *id, &mut html, &mut css, 0);
    }
    (html, css)
}

fn write_node(doc: &Document, id: NodeId, html: &mut String, css: &mut String, depth: usize) {
    let node = doc.node(id);
    let class = format!("n{}", id.0);
    let pad = "  ".repeat(depth);
    css.push_str(&format!(
        ".{class} {{\n  position: absolute;\n  left: {left}px;\n  top: {top}px;\n  width: {width}px;\n  height: {height}px;\n",
        left = css_num(node.rect.x),
        top = css_num(node.rect.y),
        width = css_num(node.rect.width),
        height = css_num(node.rect.height),
    ));
    if node.fill.a > 0 {
        css.push_str(&format!("  background-color: {};\n", rgba_css(node.fill)));
    }
    if node.kind == NodeKind::Text || node.kind == NodeKind::Button {
        css.push_str(&format!(
            "  color: {};\n  font-size: {}px;\n",
            rgba_css(node.color),
            css_num(node.font_size)
        ));
    }
    if node.kind == NodeKind::Ellipse {
        css.push_str("  border-radius: 50%;\n");
    } else if node.corner_radius > 0.0 {
        css.push_str(&format!("  border-radius: {}px;\n", css_num(node.corner_radius)));
    }
    css.push_str("}\n");

    let style = format!(
        "position:absolute;left:{left}px;top:{top}px;width:{width}px;height:{height}px;{extra}",
        left = css_num(node.rect.x),
        top = css_num(node.rect.y),
        width = css_num(node.rect.width),
        height = css_num(node.rect.height),
        extra = inline_extra(node)
    );
    let kind = kind_attr(node.kind);
    let slot = node
        .dock_slot
        .map(|slot| format!(" data-slot=\"{}\"", slot.as_str()))
        .unwrap_or_default();
    let name = escape_attr(&node.name);
    match node.kind {
        NodeKind::Text => {
            html.push_str(&format!(
                "{pad}<p class=\"{class}\" data-uii=\"text\" data-uii-name=\"{name}\" style=\"{style}\">{}</p>\n",
                escape_text(&node.text)
            ));
        }
        NodeKind::Button => {
            html.push_str(&format!(
                "{pad}<button class=\"{class}\" data-uii=\"button\" data-uii-name=\"{name}\" style=\"{style}\">{}</button>\n",
                escape_text(&node.text)
            ));
        }
        NodeKind::Image => {
            html.push_str(&format!(
                "{pad}<img class=\"{class}\" data-uii=\"image\" data-uii-name=\"{name}\" alt=\"{name}\" src=\"{}\" width=\"{}\" height=\"{}\" style=\"{style}\">\n",
                escape_attr(&node.image_src),
                css_num(node.rect.width),
                css_num(node.rect.height)
            ));
        }
        NodeKind::Table => {
            html.push_str(&format!(
                "{pad}<table class=\"{class}\" data-uii=\"table\" data-uii-name=\"{name}\" style=\"{style}\">\n"
            ));
            html.push_str(&format!("{pad}  <tr>"));
            for column in &node.columns {
                html.push_str(&format!("<th>{}</th>", escape_text(column)));
            }
            html.push_str("</tr>\n");
            for row in &node.rows {
                html.push_str(&format!("{pad}  <tr>"));
                for cell in row {
                    html.push_str(&format!("<td>{}</td>", escape_text(cell)));
                }
                html.push_str("</tr>\n");
            }
            html.push_str(&format!("{pad}</table>\n"));
        }
        NodeKind::Dock => {
            let tag = dock_tag(node.dock_slot.unwrap_or(DockSlot::Center));
            html.push_str(&format!(
                "{pad}<{tag} class=\"{class}\" data-uii=\"dock\" data-uii-name=\"{name}\"{slot} style=\"{style}\">\n"
            ));
            for child in &node.children {
                write_node(doc, *child, html, css, depth + 1);
            }
            html.push_str(&format!("{pad}</{tag}>\n"));
        }
        _ => {
            html.push_str(&format!(
                "{pad}<div class=\"{class}\" data-uii=\"{kind}\" data-uii-name=\"{name}\"{slot} style=\"{style}\">\n"
            ));
            for child in &node.children {
                write_node(doc, *child, html, css, depth + 1);
            }
            html.push_str(&format!("{pad}</div>\n"));
        }
    }
}

fn inline_extra(node: &crate::document::Node) -> String {
    let mut extra = String::new();
    if node.fill.a > 0 {
        extra.push_str(&format!("background-color:{};", rgba_css(node.fill)));
    }
    if node.kind == NodeKind::Text || node.kind == NodeKind::Button {
        extra.push_str(&format!(
            "color:{};font-size:{}px;",
            rgba_css(node.color),
            css_num(node.font_size)
        ));
    }
    if node.kind == NodeKind::Ellipse {
        extra.push_str("border-radius:50%;");
    } else if node.corner_radius > 0.0 {
        extra.push_str(&format!("border-radius:{}px;", css_num(node.corner_radius)));
    }
    extra
}

fn kind_attr(kind: NodeKind) -> &'static str {
    match kind {
        NodeKind::Frame => "frame",
        NodeKind::Text => "text",
        NodeKind::Rectangle => "rectangle",
        NodeKind::Ellipse => "ellipse",
        NodeKind::Button => "button",
        NodeKind::Image => "image",
        NodeKind::Table => "table",
        NodeKind::Dock => "dock",
        NodeKind::Child => "child",
    }
}

fn dock_tag(slot: DockSlot) -> &'static str {
    match slot {
        DockSlot::Top => "header",
        DockSlot::Bottom => "footer",
        DockSlot::Left => "nav",
        DockSlot::Right => "aside",
        DockSlot::Center => "main",
    }
}

fn css_num(n: f32) -> String {
    let rounded = (n * 100.0).round() / 100.0;
    if (rounded - rounded.round()).abs() < 0.001 {
        format!("{}", rounded.round() as i32)
    } else {
        format!("{rounded}")
    }
}

fn rgba_css(color: Rgba) -> String {
    format!("#{:02x}{:02x}{:02x}", color.r, color.g, color.b)
}

fn escape_text(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

fn escape_attr(text: &str) -> String {
    escape_text(text).replace('"', "&quot;")
}

struct El {
    tag: String,
    attrs: Vec<(String, String)>,
    children: Vec<Child>,
}

enum Child {
    El(El),
    Text(String),
}

fn parse_fragment(input: &str) -> (Vec<El>, String) {
    let mut css = String::new();
    let mut roots = Vec::new();
    let chars: Vec<char> = input.chars().collect();
    let mut i = 0;
    parse_nodes(&chars, &mut i, None, &mut roots, &mut css);
    if roots.len() == 1 && roots[0].tag == "html" {
        let inner = std::mem::take(&mut roots[0].children);
        roots.clear();
        hoist(inner, &mut roots);
    }
    if let Some(body_at) = roots.iter().position(|el| el.tag == "body") {
        let body = roots.remove(body_at);
        let mut inner = Vec::new();
        hoist(body.children, &mut inner);
        roots.splice(body_at..body_at, inner);
    }
    (roots, css)
}

fn hoist(children: Vec<Child>, out: &mut Vec<El>) {
    for child in children {
        if let Child::El(el) = child {
            if el.tag == "head" {
                continue;
            }
            if el.tag == "body" {
                hoist(el.children, out);
            } else {
                out.push(el);
            }
        }
    }
}

fn parse_nodes(
    chars: &[char],
    i: &mut usize,
    until: Option<&str>,
    out: &mut Vec<El>,
    css: &mut String,
) {
    while *i < chars.len() {
        if chars[*i].is_whitespace() {
            *i += 1;
            continue;
        }
        if until.is_some() && starts_with_ci(chars, *i, &format!("</{}", until.unwrap())) {
            return;
        }
        if starts_with_ci(chars, *i, "<!--") {
            *i += 4;
            while *i + 2 < chars.len() && !(chars[*i] == '-' && chars[*i + 1] == '-' && chars[*i + 2] == '>') {
                *i += 1;
            }
            *i = (*i + 3).min(chars.len());
            continue;
        }
        if chars[*i] != '<' {
            let start = *i;
            while *i < chars.len() && chars[*i] != '<' {
                *i += 1;
            }
            let text: String = chars[start..*i].iter().collect();
            let collapsed = collapse_ws(&text);
            if !collapsed.is_empty() {
                // Text between elements is attached later by the caller when inside an element.
                // At the root, ignore stray text.
                let _ = collapsed;
            }
            continue;
        }
        if until.is_some() && starts_with_ci(chars, *i, "</") {
            return;
        }
        if starts_with_ci(chars, *i, "</") {
            return;
        }
        *i += 1;
        if *i < chars.len() && chars[*i] == '/' {
            return;
        }
        let tag = read_name(chars, i);
        let attrs = read_attrs(chars, i);
        let self_close = attrs.0;
        let attrs = attrs.1;
        if *i < chars.len() && chars[*i] == '>' {
            *i += 1;
        }
        let tag_l = tag.to_lowercase();
        if tag_l == "style" {
            css.push_str(&read_until_close(chars, i, "style"));
            css.push('\n');
            continue;
        }
        if tag_l == "script" {
            let _ = read_until_close(chars, i, "script");
            continue;
        }
        let mut el = El {
            tag: tag_l.clone(),
            attrs,
            children: Vec::new(),
        };
        if !self_close && !is_void(&tag_l) {
            parse_children(chars, i, &tag_l, &mut el.children, css);
        }
        out.push(el);
    }
}

fn parse_children(chars: &[char], i: &mut usize, until: &str, out: &mut Vec<Child>, css: &mut String) {
    while *i < chars.len() {
        if starts_with_ci(chars, *i, &format!("</{until}")) {
            *i += 2 + until.len();
            while *i < chars.len() && chars[*i] != '>' {
                *i += 1;
            }
            if *i < chars.len() {
                *i += 1;
            }
            return;
        }
        if chars[*i].is_whitespace() {
            let start = *i;
            while *i < chars.len() && chars[*i].is_whitespace() {
                *i += 1;
            }
            let _ = start;
            continue;
        }
        if starts_with_ci(chars, *i, "<!--") {
            *i += 4;
            while *i + 2 < chars.len() && !(chars[*i] == '-' && chars[*i + 1] == '-' && chars[*i + 2] == '>') {
                *i += 1;
            }
            *i = (*i + 3).min(chars.len());
            continue;
        }
        if chars[*i] != '<' {
            let start = *i;
            while *i < chars.len() && chars[*i] != '<' {
                *i += 1;
            }
            let text: String = chars[start..*i].iter().collect();
            let collapsed = collapse_ws(&text);
            if !collapsed.is_empty() {
                out.push(Child::Text(collapsed));
            }
            continue;
        }
        let save = *i;
        *i += 1;
        if *i < chars.len() && chars[*i] == '/' {
            *i = save;
            if starts_with_ci(chars, *i, &format!("</{until}")) {
                *i += 2 + until.len();
                while *i < chars.len() && chars[*i] != '>' {
                    *i += 1;
                }
                if *i < chars.len() {
                    *i += 1;
                }
            }
            return;
        }
        let tag = read_name(chars, i);
        let (self_close, attrs) = read_attrs(chars, i);
        if *i < chars.len() && chars[*i] == '>' {
            *i += 1;
        }
        let tag_l = tag.to_lowercase();
        if tag_l == "style" {
            css.push_str(&read_until_close(chars, i, "style"));
            css.push('\n');
            continue;
        }
        if tag_l == "script" {
            let _ = read_until_close(chars, i, "script");
            continue;
        }
        let mut el = El {
            tag: tag_l.clone(),
            attrs,
            children: Vec::new(),
        };
        if !self_close && !is_void(&tag_l) {
            parse_children(chars, i, &tag_l, &mut el.children, css);
        }
        out.push(Child::El(el));
    }
}

fn read_until_close(chars: &[char], i: &mut usize, tag: &str) -> String {
    let start = *i;
    let end = format!("</{tag}");
    while *i < chars.len() && !starts_with_ci(chars, *i, &end) {
        *i += 1;
    }
    let text: String = chars[start..*i].iter().collect();
    if starts_with_ci(chars, *i, &end) {
        *i += end.len();
        while *i < chars.len() && chars[*i] != '>' {
            *i += 1;
        }
        if *i < chars.len() {
            *i += 1;
        }
    }
    text
}

fn read_name(chars: &[char], i: &mut usize) -> String {
    let start = *i;
    while *i < chars.len() && (chars[*i].is_ascii_alphanumeric() || chars[*i] == '-' || chars[*i] == ':' || chars[*i] == '.') {
        *i += 1;
    }
    chars[start..*i].iter().collect()
}

fn read_attrs(chars: &[char], i: &mut usize) -> (bool, Vec<(String, String)>) {
    let mut attrs = Vec::new();
    loop {
        while *i < chars.len() && chars[*i].is_whitespace() {
            *i += 1;
        }
        if *i >= chars.len() {
            break;
        }
        if chars[*i] == '>' {
            break;
        }
        if chars[*i] == '/' {
            *i += 1;
            while *i < chars.len() && chars[*i].is_whitespace() {
                *i += 1;
            }
            return (true, attrs);
        }
        let name = read_name(chars, i).to_lowercase();
        if name.is_empty() {
            *i += 1;
            continue;
        }
        while *i < chars.len() && chars[*i].is_whitespace() {
            *i += 1;
        }
        let mut value = String::new();
        if *i < chars.len() && chars[*i] == '=' {
            *i += 1;
            while *i < chars.len() && chars[*i].is_whitespace() {
                *i += 1;
            }
            if *i < chars.len() && (chars[*i] == '"' || chars[*i] == '\'') {
                let quote = chars[*i];
                *i += 1;
                while *i < chars.len() && chars[*i] != quote {
                    value.push(chars[*i]);
                    *i += 1;
                }
                if *i < chars.len() {
                    *i += 1;
                }
            } else {
                while *i < chars.len() && !chars[*i].is_whitespace() && chars[*i] != '>' && chars[*i] != '/' {
                    value.push(chars[*i]);
                    *i += 1;
                }
            }
        }
        attrs.push((name, decode_basic(&value)));
    }
    (false, attrs)
}

fn decode_basic(value: &str) -> String {
    value
        .replace("&quot;", "\"")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&amp;", "&")
}

fn collapse_ws(text: &str) -> String {
    let mut out = String::new();
    let mut space = false;
    for ch in text.split_whitespace() {
        if space {
            out.push(' ');
        }
        out.push_str(ch);
        space = true;
    }
    out
}

fn starts_with_ci(chars: &[char], i: usize, prefix: &str) -> bool {
    let p: Vec<char> = prefix.chars().collect();
    if i + p.len() > chars.len() {
        return false;
    }
    chars[i..i + p.len()]
        .iter()
        .zip(p.iter())
        .all(|(a, b)| a.eq_ignore_ascii_case(b))
}

fn is_void(tag: &str) -> bool {
    matches!(tag, "img" | "br" | "hr" | "meta" | "link" | "input" | "source" | "area" | "col" | "wbr")
}

fn is_skipped_tag(tag: &str) -> bool {
    matches!(tag, "style" | "script" | "meta" | "link" | "head" | "title")
}

#[derive(Clone)]
struct Rule {
    selector: Selector,
    decls: Vec<(String, String)>,
    order: usize,
    spec: (u32, u32, u32),
}

#[derive(Clone)]
struct Selector {
    parts: Vec<Compound>,
}

#[derive(Clone)]
struct Compound {
    tag: Option<String>,
    id: Option<String>,
    classes: Vec<String>,
}

fn parse_css(css: &str) -> Vec<Rule> {
    let mut rules = Vec::new();
    let chars: Vec<char> = strip_css_comments(css).chars().collect();
    let mut i = 0;
    let mut order = 0;
    while i < chars.len() {
        while i < chars.len() && chars[i].is_whitespace() {
            i += 1;
        }
        if i >= chars.len() {
            break;
        }
        let start = i;
        while i < chars.len() && chars[i] != '{' {
            i += 1;
        }
        if i >= chars.len() {
            break;
        }
        let prelude: String = chars[start..i].iter().collect();
        i += 1;
        let body_start = i;
        while i < chars.len() && chars[i] != '}' {
            i += 1;
        }
        let body: String = chars[body_start..i].iter().collect();
        if i < chars.len() {
            i += 1;
        }
        let decls = parse_decls(&body);
        for sel_src in prelude.split(',') {
            let sel_src = sel_src.trim();
            if sel_src.is_empty() || sel_src.starts_with('@') {
                continue;
            }
            if let Some(selector) = parse_selector(sel_src) {
                let spec = specificity(&selector);
                rules.push(Rule {
                    selector,
                    decls: decls.clone(),
                    order,
                    spec,
                });
                order += 1;
            }
        }
    }
    rules
}

fn strip_css_comments(css: &str) -> String {
    let mut out = String::new();
    let chars: Vec<char> = css.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        if chars[i] == '/' && i + 1 < chars.len() && chars[i + 1] == '*' {
            i += 2;
            while i + 1 < chars.len() && !(chars[i] == '*' && chars[i + 1] == '/') {
                i += 1;
            }
            i = (i + 2).min(chars.len());
            continue;
        }
        out.push(chars[i]);
        i += 1;
    }
    out
}

fn parse_decls(body: &str) -> Vec<(String, String)> {
    body.split(';')
        .filter_map(|part| {
            let part = part.trim().trim_end_matches("!important").trim();
            let (name, value) = part.split_once(':')?;
            let name = name.trim().to_lowercase();
            let value = value.trim().to_string();
            if name.is_empty() {
                None
            } else {
                Some((name, value))
            }
        })
        .collect()
}

fn parse_selector(src: &str) -> Option<Selector> {
    let mut parts = Vec::new();
    for token in src.split_whitespace() {
        let token = token.trim();
        if token.is_empty() || token == ">" || token == "+" || token == "~" {
            continue;
        }
        let mut compound = Compound {
            tag: None,
            id: None,
            classes: Vec::new(),
        };
        let chars: Vec<char> = token.chars().collect();
        let mut i = 0;
        if i < chars.len() && chars[i] != '.' && chars[i] != '#' {
            let start = i;
            while i < chars.len() && chars[i] != '.' && chars[i] != '#' {
                i += 1;
            }
            let tag: String = chars[start..i].iter().collect();
            if tag != "*" && !tag.is_empty() {
                compound.tag = Some(tag.to_lowercase());
            }
        }
        while i < chars.len() {
            if chars[i] == '.' {
                i += 1;
                let start = i;
                while i < chars.len() && chars[i] != '.' && chars[i] != '#' {
                    i += 1;
                }
                compound.classes.push(chars[start..i].iter().collect());
            } else if chars[i] == '#' {
                i += 1;
                let start = i;
                while i < chars.len() && chars[i] != '.' && chars[i] != '#' {
                    i += 1;
                }
                compound.id = Some(chars[start..i].iter().collect());
            } else {
                i += 1;
            }
        }
        parts.push(compound);
    }
    if parts.is_empty() {
        None
    } else {
        Some(Selector { parts })
    }
}

fn specificity(sel: &Selector) -> (u32, u32, u32) {
    let mut ids = 0;
    let mut classes = 0;
    let mut tags = 0;
    for part in &sel.parts {
        if part.id.is_some() {
            ids += 1;
        }
        classes += part.classes.len() as u32;
        if part.tag.is_some() {
            tags += 1;
        }
    }
    (ids, classes, tags)
}

#[derive(Clone)]
struct Style {
    width: Option<Len>,
    height: Option<Len>,
    left: Option<Len>,
    top: Option<Len>,
    margin: Box4,
    padding: Box4,
    position: Position,
    background: Option<Rgba>,
    color: Option<Rgba>,
    font_size: Option<f32>,
    radius: Option<Len>,
}

#[derive(Clone, Copy)]
enum Len {
    Px(f32),
    Pct(f32),
    Auto,
}

#[derive(Clone, Copy)]
struct Box4 {
    top: f32,
    right: f32,
    bottom: f32,
    left: f32,
}

impl Default for Box4 {
    fn default() -> Self {
        Self {
            top: 0.0,
            right: 0.0,
            bottom: 0.0,
            left: 0.0,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Position {
    Static,
    Absolute,
}

impl Default for Style {
    fn default() -> Self {
        Self {
            width: None,
            height: None,
            left: None,
            top: None,
            margin: Box4::default(),
            padding: Box4::default(),
            position: Position::Static,
            background: None,
            color: None,
            font_size: None,
            radius: None,
        }
    }
}

fn resolve_style(el: &El, ancestors: &[&El], rules: &[Rule]) -> Style {
    let mut matched: Vec<&Rule> = rules
        .iter()
        .filter(|rule| selector_matches(&rule.selector, el, ancestors))
        .collect();
    matched.sort_by(|a, b| a.spec.cmp(&b.spec).then(a.order.cmp(&b.order)));
    let mut style = Style::default();
    for rule in matched {
        apply_decls(&mut style, &rule.decls);
    }
    if let Some(inline) = attr(el, "style") {
        apply_decls(&mut style, &parse_decls(inline));
    }
    style
}

fn selector_matches(sel: &Selector, el: &El, ancestors: &[&El]) -> bool {
    let Some(subject) = sel.parts.last() else {
        return false;
    };
    if !compound_matches(subject, el) {
        return false;
    }
    let mut idx = ancestors.len();
    for part in sel.parts.iter().rev().skip(1) {
        let mut found = false;
        while idx > 0 {
            idx -= 1;
            if compound_matches(part, ancestors[idx]) {
                found = true;
                break;
            }
        }
        if !found {
            return false;
        }
    }
    true
}

fn compound_matches(part: &Compound, el: &El) -> bool {
    if let Some(tag) = &part.tag {
        if el.tag != *tag {
            return false;
        }
    }
    if let Some(id) = &part.id {
        if attr(el, "id") != Some(id.as_str()) {
            return false;
        }
    }
    let classes = class_list(el);
    for class in &part.classes {
        if !classes.iter().any(|item| item == class) {
            return false;
        }
    }
    true
}

fn apply_decls(style: &mut Style, decls: &[(String, String)]) {
    for (name, value) in decls {
        match name.as_str() {
            "width" => style.width = parse_len(value),
            "height" => style.height = parse_len(value),
            "left" => style.left = parse_len(value),
            "top" => style.top = parse_len(value),
            "margin" => style.margin = parse_box4(value),
            "margin-top" => style.margin.top = parse_px(value).unwrap_or(0.0),
            "margin-right" => style.margin.right = parse_px(value).unwrap_or(0.0),
            "margin-bottom" => style.margin.bottom = parse_px(value).unwrap_or(0.0),
            "margin-left" => style.margin.left = parse_px(value).unwrap_or(0.0),
            "padding" => style.padding = parse_box4(value),
            "padding-top" => style.padding.top = parse_px(value).unwrap_or(0.0),
            "padding-right" => style.padding.right = parse_px(value).unwrap_or(0.0),
            "padding-bottom" => style.padding.bottom = parse_px(value).unwrap_or(0.0),
            "padding-left" => style.padding.left = parse_px(value).unwrap_or(0.0),
            "position" => {
                style.position = if value.trim() == "absolute" || value.trim() == "fixed" {
                    Position::Absolute
                } else {
                    Position::Static
                };
            }
            "background" | "background-color" => {
                if let Some(color) = parse_color(value) {
                    style.background = Some(color);
                }
            }
            "color" => style.color = parse_color(value),
            "font-size" => style.font_size = parse_px(value),
            "border-radius" => style.radius = parse_len(value),
            _ => {}
        }
    }
}

fn parse_len(value: &str) -> Option<Len> {
    let value = value.trim();
    if value == "auto" {
        return Some(Len::Auto);
    }
    if let Some(pct) = value.strip_suffix('%') {
        return pct.trim().parse().ok().map(Len::Pct);
    }
    parse_px(value).map(Len::Px)
}

fn parse_px(value: &str) -> Option<f32> {
    let value = value.trim().trim_end_matches("px").trim();
    value.parse().ok()
}

fn parse_box4(value: &str) -> Box4 {
    let parts: Vec<f32> = value
        .split_whitespace()
        .filter_map(|part| parse_px(part))
        .collect();
    match parts.as_slice() {
        [all] => Box4 {
            top: *all,
            right: *all,
            bottom: *all,
            left: *all,
        },
        [tb, lr] => Box4 {
            top: *tb,
            right: *lr,
            bottom: *tb,
            left: *lr,
        },
        [top, lr, bottom] => Box4 {
            top: *top,
            right: *lr,
            bottom: *bottom,
            left: *lr,
        },
        [top, right, bottom, left, ..] => Box4 {
            top: *top,
            right: *right,
            bottom: *bottom,
            left: *left,
        },
        _ => Box4::default(),
    }
}

fn parse_color(value: &str) -> Option<Rgba> {
    let value = value.trim().trim_matches(|ch| ch == ';' || ch == ' ');
    let lower = value.to_lowercase();
    if let Some(hex) = lower.strip_prefix('#') {
        return parse_hex(hex);
    }
    if let Some(inner) = lower.strip_prefix("rgba(").and_then(|v| v.strip_suffix(')')) {
        let parts: Vec<&str> = inner.split(',').map(|p| p.trim()).collect();
        if parts.len() >= 4 {
            let r = parts[0].parse().ok()?;
            let g = parts[1].parse().ok()?;
            let b = parts[2].parse().ok()?;
            let a: f32 = parts[3].parse().ok()?;
            return Some(Rgba::rgba(r, g, b, (a * 255.0).round() as u8));
        }
    }
    if let Some(inner) = lower.strip_prefix("rgb(").and_then(|v| v.strip_suffix(')')) {
        let parts: Vec<&str> = inner.split(',').map(|p| p.trim()).collect();
        if parts.len() >= 3 {
            let r = parts[0].parse().ok()?;
            let g = parts[1].parse().ok()?;
            let b = parts[2].parse().ok()?;
            return Some(Rgba::rgb(r, g, b));
        }
    }
    Some(match lower.as_str() {
        "black" => Rgba::rgb(0, 0, 0),
        "white" => Rgba::rgb(255, 255, 255),
        "red" => Rgba::rgb(255, 0, 0),
        "green" => Rgba::rgb(0, 128, 0),
        "blue" => Rgba::rgb(0, 0, 255),
        "transparent" => Rgba::rgba(0, 0, 0, 0),
        _ => return None,
    })
}

fn parse_hex(hex: &str) -> Option<Rgba> {
    let hex = hex.trim();
    let byte = |s: &str| u8::from_str_radix(s, 16).ok();
    match hex.len() {
        3 | 4 => {
            let nib = |i: usize| hex[i..i + 1].chars().next()?.to_digit(16).map(|n| (n as u8) * 17);
            Some(Rgba::rgba(
                nib(0)?,
                nib(1)?,
                nib(2)?,
                if hex.len() == 4 { nib(3)? } else { 255 },
            ))
        }
        6 | 8 => Some(Rgba::rgba(
            byte(&hex[0..2])?,
            byte(&hex[2..4])?,
            byte(&hex[4..6])?,
            if hex.len() == 8 { byte(&hex[6..8])? } else { 255 },
        )),
        _ => None,
    }
}

fn attr<'a>(el: &'a El, name: &str) -> Option<&'a str> {
    el.attrs
        .iter()
        .find(|(key, _)| key == name)
        .map(|(_, value)| value.as_str())
}

fn class_list(el: &El) -> Vec<String> {
    attr(el, "class")
        .unwrap_or("")
        .split_whitespace()
        .map(|s| s.to_string())
        .collect()
}

fn place(
    doc: &mut Document,
    el: &El,
    parent: Option<NodeId>,
    ancestors: &[&El],
    rules: &[Rule],
    x0: f32,
    y0: f32,
    containing_w: f32,
    inherited: Option<Rgba>,
) -> f32 {
    if el.tag == "template" {
        let mut y = y0;
        let mut advance = 0.0;
        for child in &el.children {
            if let Child::El(child_el) = child {
                let used = place(doc, child_el, parent, ancestors, rules, x0, y, containing_w, inherited);
                y += used;
                advance += used;
            }
        }
        return advance;
    }
    if el.tag == "table" {
        return place_table(doc, el, parent, ancestors, rules, x0, y0, containing_w);
    }
    let style = resolve_style(el, ancestors, rules);
    let font = style.font_size.unwrap_or(match el.tag.as_str() {
        "h1" => 32.0,
        "h2" => 24.0,
        "h3" => 20.0,
        _ => 16.0,
    });
    let color = style.color.or(inherited).unwrap_or(Rgba::rgb(0x1a, 0x1a, 0x1a));
    let text = direct_text(el);
    let element_children: Vec<&El> = el
        .children
        .iter()
        .filter_map(|child| match child {
            Child::El(el) => Some(el),
            Child::Text(_) => None,
        })
        .collect();
    let kind = classify(el, &style, parent.is_some(), element_children.is_empty(), containing_w);
    let name = node_name(el, &text, &kind);
    let content_w = match style.width {
        Some(Len::Px(px)) => px - style.padding.left - style.padding.right,
        Some(Len::Pct(pct)) => containing_w * pct / 100.0 - style.padding.left - style.padding.right,
        _ => {
            if matches!(kind, NodeKind::Text | NodeKind::Button) {
                measure(&text, font).0
            } else if parent.is_none() {
                800.0
            } else {
                containing_w
            }
        }
    };
    let (measured_w, measured_h) = measure(&text, font);
    let width = match style.width {
        Some(Len::Px(px)) => px,
        Some(Len::Pct(pct)) => containing_w * pct / 100.0,
        _ => match kind {
            NodeKind::Text => measured_w,
            NodeKind::Button => (measured_w + 16.0).max(64.0),
            NodeKind::Image => attr(el, "width").and_then(parse_px).unwrap_or(32.0),
            NodeKind::Dock => {
                if matches!(el.tag.as_str(), "nav" | "aside") {
                    220.0
                } else {
                    content_w.max(0.0)
                }
            }
            _ => content_w.max(0.0),
        },
    };
    let mut height = match style.height {
        Some(Len::Px(px)) => px,
        Some(Len::Pct(_)) => 40.0,
        _ => match kind {
            NodeKind::Text => measured_h,
            NodeKind::Button => 28.0,
            NodeKind::Image => attr(el, "height").and_then(parse_px).unwrap_or(32.0),
            NodeKind::Dock if matches!(el.tag.as_str(), "header" | "footer") => 48.0,
            _ => 0.0,
        },
    };
    let abs = style.position == Position::Absolute;
    let x = if abs {
        match style.left {
            Some(Len::Px(px)) => px,
            Some(Len::Pct(pct)) => containing_w * pct / 100.0,
            _ => 0.0,
        }
    } else {
        x0 + style.margin.left
    };
    let y = if abs {
        match style.top {
            Some(Len::Px(px)) => px,
            _ => 0.0,
        }
    } else {
        y0 + style.margin.top
    };
    let id = doc.create(parent, kind, name, Rect::new(x, y, width.max(1.0), height.max(1.0)));
    {
        let node = doc.node_mut(id);
        if let Some(fill) = style.background {
            node.fill = fill;
        }
        node.color = color;
        node.font_size = font;
        if let Some(Len::Px(px)) = style.radius {
            if kind != NodeKind::Ellipse {
                node.corner_radius = px.max(0.0);
            }
        }
        if kind == NodeKind::Text || kind == NodeKind::Button {
            node.text = text.clone();
        }
        if kind == NodeKind::Image {
            node.image_src = attr(el, "src").unwrap_or("").to_string();
        }
    }
    if let Some(slot) = dock_slot_of(el) {
        if kind == NodeKind::Dock {
            doc.set_dock_slot(id, slot);
        }
    }
    if kind == NodeKind::Text || kind == NodeKind::Button || kind == NodeKind::Image || kind == NodeKind::Ellipse || kind == NodeKind::Rectangle {
        let outer = if abs {
            height.max(1.0)
        } else {
            style.margin.top + height.max(1.0) + style.margin.bottom
        };
        if height < 1.0 {
            doc.resize_node(id, width.max(1.0), 1.0);
        }
        return outer;
    }
    let mut child_y = style.padding.top;
    let inner_w = (width - style.padding.left - style.padding.right).max(1.0);
    let mut ancestors_next = ancestors.to_vec();
    ancestors_next.push(el);
    if !text.is_empty() && kind != NodeKind::Text && kind != NodeKind::Button {
        let (tw, th) = measure(&text, font);
        let text_id = doc.create(
            Some(id),
            NodeKind::Text,
            text.clone(),
            Rect::new(style.padding.left, child_y, tw.max(1.0), th.max(1.0)),
        );
        let node = doc.node_mut(text_id);
        node.text = text.clone();
        node.color = color;
        node.font_size = font;
        child_y += th;
    }
    for child in &element_children {
        if child.tag == "table" {
            let used = place_table(doc, child, Some(id), &ancestors_next, rules, style.padding.left, child_y, inner_w);
            child_y += used;
            continue;
        }
        let used = place(
            doc,
            child,
            Some(id),
            &ancestors_next,
            rules,
            style.padding.left,
            child_y,
            inner_w,
            Some(color),
        );
        child_y += used;
    }
    if style.height.is_none() {
        height = (child_y + style.padding.bottom).max(if element_children.is_empty() && text.is_empty() { 40.0 } else { 1.0 });
        doc.resize_node(id, width.max(1.0), height);
    }
    if abs {
        height.max(1.0)
    } else {
        style.margin.top + doc.node(id).rect.height + style.margin.bottom
    }
}

fn place_table(
    doc: &mut Document,
    el: &El,
    parent: Option<NodeId>,
    ancestors: &[&El],
    rules: &[Rule],
    x0: f32,
    y0: f32,
    containing_w: f32,
) -> f32 {
    let style = resolve_style(el, ancestors, rules);
    let mut columns = Vec::new();
    let mut rows = Vec::new();
    let mut header = true;
    for child in &el.children {
        let Child::El(row) = child else { continue };
        if row.tag != "tr" {
            continue;
        }
        let mut cells = Vec::new();
        let mut all_th = true;
        let mut any = false;
        for cell in &row.children {
            let Child::El(cell) = cell else { continue };
            if cell.tag != "th" && cell.tag != "td" {
                continue;
            }
            any = true;
            if cell.tag != "th" {
                all_th = false;
            }
            cells.push(direct_text(cell));
        }
        if !any {
            continue;
        }
        if header && all_th {
            columns = cells;
            header = false;
        } else {
            header = false;
            rows.push(cells);
        }
    }
    if columns.is_empty() {
        let count = rows.first().map(|row| row.len()).unwrap_or(1).max(1);
        columns = (1..=count).map(|i| format!("Col{i}")).collect();
    }
    let width = match style.width {
        Some(Len::Px(px)) => px,
        Some(Len::Pct(pct)) => containing_w * pct / 100.0,
        _ => (columns.len().max(1) as f32) * 120.0,
    };
    let height = match style.height {
        Some(Len::Px(px)) => px,
        _ => ((rows.len() + 1) as f32) * 28.0,
    };
    let abs = style.position == Position::Absolute;
    let x = if abs {
        match style.left {
            Some(Len::Px(px)) => px,
            _ => 0.0,
        }
    } else {
        x0 + style.margin.left
    };
    let y = if abs {
        match style.top {
            Some(Len::Px(px)) => px,
            _ => 0.0,
        }
    } else {
        y0 + style.margin.top
    };
    let name = node_name(el, "", &NodeKind::Table);
    let id = doc.create(parent, NodeKind::Table, name, Rect::new(x, y, width.max(1.0), height.max(1.0)));
    if let Some(fill) = style.background {
        doc.node_mut(id).fill = fill;
    }
    doc.set_table(id, columns, rows);
    if abs {
        height
    } else {
        style.margin.top + height + style.margin.bottom
    }
}

fn classify(el: &El, style: &Style, nested: bool, leaf: bool, containing_w: f32) -> NodeKind {
    if let Some(kind) = attr(el, "data-uii") {
        return match kind {
            "frame" => NodeKind::Frame,
            "text" => NodeKind::Text,
            "rectangle" => NodeKind::Rectangle,
            "ellipse" => NodeKind::Ellipse,
            "button" => NodeKind::Button,
            "image" => NodeKind::Image,
            "table" => NodeKind::Table,
            "dock" => NodeKind::Dock,
            "child" => NodeKind::Child,
            "root" => NodeKind::Frame,
            _ => NodeKind::Frame,
        };
    }
    match el.tag.as_str() {
        "button" => return NodeKind::Button,
        "img" => return NodeKind::Image,
        "table" => return NodeKind::Table,
        "p" | "span" | "h1" | "h2" | "h3" | "h4" | "h5" | "h6" | "a" | "label" | "li" if leaf => {
            return NodeKind::Text;
        }
        "header" | "footer" | "nav" | "aside" | "main" | "section" | "article" => return NodeKind::Dock,
        _ => {}
    }
    if leaf && is_ellipse(el, style, containing_w) {
        return NodeKind::Ellipse;
    }
    if el.tag == "hr" {
        return NodeKind::Rectangle;
    }
    if nested {
        NodeKind::Child
    } else {
        NodeKind::Frame
    }
}

fn is_ellipse(el: &El, style: &Style, containing_w: f32) -> bool {
    if el.tag != "div" && el.tag != "span" {
        return false;
    }
    let width = match style.width {
        Some(Len::Px(px)) => px,
        Some(Len::Pct(pct)) => containing_w * pct / 100.0,
        _ => return false,
    };
    let height = match style.height {
        Some(Len::Px(px)) => px,
        _ => return false,
    };
    match style.radius {
        Some(Len::Pct(pct)) => pct >= 50.0,
        Some(Len::Px(px)) => px >= width.min(height) / 2.0 - 0.5,
        _ => false,
    }
}

fn dock_slot_of(el: &El) -> Option<DockSlot> {
    if let Some(slot) = attr(el, "data-slot").and_then(DockSlot::parse) {
        return Some(slot);
    }
    Some(match el.tag.as_str() {
        "header" => DockSlot::Top,
        "footer" => DockSlot::Bottom,
        "nav" => DockSlot::Left,
        "aside" => DockSlot::Right,
        "main" | "section" | "article" => DockSlot::Center,
        _ => return None,
    })
}

fn is_generated_class(class: &str) -> bool {
    let mut chars = class.chars();
    chars.next() == Some('n') && chars.all(|ch| ch.is_ascii_digit()) && class.len() > 1
}

fn node_name(el: &El, text: &str, kind: &NodeKind) -> String {
    if let Some(name) = attr(el, "data-uii-name") {
        if !name.is_empty() {
            return name.to_string();
        }
    }
    if let Some(id) = attr(el, "id") {
        if !id.is_empty() {
            return id.to_string();
        }
    }
    if let Some(class) = class_list(el).into_iter().find(|class| !is_generated_class(class)) {
        return class;
    }
    if matches!(kind, NodeKind::Text | NodeKind::Button) && !text.is_empty() {
        return text.to_string();
    }
    if kind == &NodeKind::Image {
        if let Some(alt) = attr(el, "alt") {
            if !alt.is_empty() {
                return alt.to_string();
            }
        }
        if let Some(src) = attr(el, "src") {
            return src.to_string();
        }
    }
    el.tag.clone()
}

fn direct_text(el: &El) -> String {
    let mut parts = Vec::new();
    for child in &el.children {
        match child {
            Child::Text(text) => parts.push(text.clone()),
            Child::El(child) if child.tag == "br" => parts.push("\n".into()),
            Child::El(_) => {}
        }
    }
    parts.join("").trim().to_string()
}

fn measure(text: &str, font: f32) -> (f32, f32) {
    let lines: Vec<&str> = if text.is_empty() { vec![""] } else { text.split('\n').collect() };
    let longest = lines.iter().map(|line| line.chars().count()).max().unwrap_or(0);
    let width = (longest as f32 * font * TEXT_WIDTH).ceil().max(1.0);
    let height = (lines.len() as f32 * font * LINE_HEIGHT).ceil().max(1.0);
    (width, height)
}

pub(crate) struct MarkupNode {
    pub tag: String,
    pub attrs: Vec<(String, String)>,
    pub text: String,
    pub children: Vec<MarkupNode>,
}

pub(crate) fn parse_markup(src: &str) -> Vec<MarkupNode> {
    let (roots, _) = parse_fragment(src);
    roots.into_iter().map(el_to_markup).collect()
}

fn el_to_markup(el: El) -> MarkupNode {
    let mut text = String::new();
    let mut children = Vec::new();
    for child in el.children {
        match child {
            Child::Text(value) => {
                let value = value.trim();
                if value.is_empty() {
                    continue;
                }
                if !text.is_empty() {
                    text.push(' ');
                }
                text.push_str(value);
            }
            Child::El(child) => children.push(el_to_markup(child)),
        }
    }
    MarkupNode {
        tag: el.tag,
        attrs: el.attrs,
        text,
        children,
    }
}
