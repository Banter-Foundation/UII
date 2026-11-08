//! Small scanners for the source formats UII imports. They understand the
//! subset each exporter writes, not a whole language.

#[derive(Clone, Debug)]
pub enum Arg {
    Str(String),
    Num(f32),
    Ident(String),
    List(Vec<Arg>),
    Call(String, Vec<Arg>),
    Body(String),
    Raw,
}

pub fn calls<'a>(src: &'a str, name: &str) -> Vec<Vec<Arg>> {
    let mut out = Vec::new();
    let bytes = src.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'"' || bytes[i] == b'\'' || bytes[i] == b'`' {
            i = skip_string(src, i);
            continue;
        }
        if bytes[i] == b'/' && i + 1 < bytes.len() && bytes[i + 1] == b'/' {
            i = src[i..].find('\n').map(|n| i + n + 1).unwrap_or(src.len());
            continue;
        }
        if let Some(start) = match_name_at(src, i, name) {
            let mut j = start;
            while j < src.len() && src.as_bytes()[j].is_ascii_whitespace() {
                j += 1;
            }
            if j < src.len() && src.as_bytes()[j] == b'(' {
                let (args, next) = parse_args(&src[j + 1..]);
                out.push(args);
                i = j + 1 + next;
                continue;
            }
        }
        i += 1;
    }
    out
}

fn match_name_at(src: &str, i: usize, name: &str) -> Option<usize> {
    if i > 0 {
        let prev = src.as_bytes()[i - 1];
        if prev.is_ascii_alphanumeric() || prev == b'_' {
            return None;
        }
    }
    let rest = src.get(i..)?;
    if !rest.starts_with(name) {
        return None;
    }
    let after = i + name.len();
    if after < src.len() {
        let ch = src.as_bytes()[after];
        if ch.is_ascii_alphanumeric() || ch == b'_' {
            return None;
        }
    }
    Some(after)
}

fn parse_args(src: &str) -> (Vec<Arg>, usize) {
    let bytes = src.as_bytes();
    let mut args = Vec::new();
    let mut i = 0;
    loop {
        i = skip_ws_comments(src, i);
        if i >= bytes.len() {
            return (args, i);
        }
        if bytes[i] == b')' {
            return (args, i + 1);
        }
        if bytes[i] == b',' {
            i += 1;
            continue;
        }
        let (arg, next) = parse_arg(src, i);
        i = next;
        args.push(arg);
    }
}

fn parse_arg(src: &str, mut i: usize) -> (Arg, usize) {
    let bytes = src.as_bytes();
    i = skip_ws_comments(src, i);
    if i >= bytes.len() {
        return (Arg::Raw, i);
    }
    if bytes[i] == b'"' || bytes[i] == b'\'' {
        let (text, next) = take_string(src, i);
        return (Arg::Str(text), next);
    }
    if bytes[i] == b'|' {
        if let Some(rel) = src[i..].find('{') {
            let brace = i + rel;
            let (body, next) = take_brace(&src[brace..]);
            return (Arg::Body(body), brace + next);
        }
    }
    if bytes[i] == b'&' || bytes[i] == b'[' {
        let start = if bytes[i] == b'&' { i + 1 } else { i };
        if start < bytes.len() && bytes[start] == b'[' {
            let (list, next) = take_list(&src[start..]);
            return (Arg::List(list), start + next);
        }
    }
    if bytes[i] == b'{' {
        let (body, next) = take_brace(&src[i..]);
        return (Arg::Body(body), i + next);
    }
    if bytes[i] == b'-' || bytes[i].is_ascii_digit() {
        let (num, next) = take_number(src, i);
        return (Arg::Num(num), next);
    }
    let start = i;
    while i < bytes.len() && (bytes[i].is_ascii_alphanumeric() || bytes[i] == b'_' || bytes[i] == b':' || bytes[i] == b'.' || bytes[i] == b'&') {
        i += 1;
    }
    if i == start {
        i += 1;
        return (Arg::Raw, i);
    }
    let ident = src[start..i].to_string();
    let look = skip_ws_comments(src, i);
    if look < src.len() && src.as_bytes()[look] == b'(' {
        let (args, next) = parse_args(&src[look + 1..]);
        return (Arg::Call(ident, args), look + 1 + next);
    }
    (Arg::Ident(ident), i)
}

fn take_list(src: &str) -> (Vec<Arg>, usize) {
    let bytes = src.as_bytes();
    let mut i = 1;
    let mut items = Vec::new();
    loop {
        i = skip_ws_comments(src, i);
        if i >= bytes.len() {
            return (items, i);
        }
        if bytes[i] == b']' {
            return (items, i + 1);
        }
        if bytes[i] == b',' {
            i += 1;
            continue;
        }
        let (arg, next) = parse_arg(src, i);
        i = next;
        items.push(arg);
    }
}

pub fn take_brace(src: &str) -> (String, usize) {
    let bytes = src.as_bytes();
    let mut depth = 0;
    let mut i = 0;
    let mut start = 1;
    while i < bytes.len() {
        if bytes[i] == b'"' || bytes[i] == b'\'' || bytes[i] == b'`' {
            i = skip_string(src, i);
            continue;
        }
        if bytes[i] == b'{' {
            if depth == 0 {
                start = i + 1;
            }
            depth += 1;
        } else if bytes[i] == b'}' {
            depth -= 1;
            if depth == 0 {
                return (src[start..i].to_string(), i + 1);
            }
        }
        i += 1;
    }
    (src[start.min(src.len())..].to_string(), src.len())
}

fn take_string(src: &str, i: usize) -> (String, usize) {
    let bytes = src.as_bytes();
    let quote = bytes[i];
    let mut out = String::new();
    let mut j = i + 1;
    while j < bytes.len() {
        if bytes[j] == b'\\' && j + 1 < bytes.len() {
            out.push(bytes[j + 1] as char);
            j += 2;
            continue;
        }
        if bytes[j] == quote {
            return (out, j + 1);
        }
        out.push(bytes[j] as char);
        j += 1;
    }
    (out, src.len())
}

fn take_number(src: &str, i: usize) -> (f32, usize) {
    let bytes = src.as_bytes();
    let mut j = i;
    if bytes[j] == b'-' {
        j += 1;
    }
    while j < bytes.len() && (bytes[j].is_ascii_digit() || bytes[j] == b'.' || bytes[j] == b'f' || bytes[j] == b'F') {
        j += 1;
    }
    let token = src[i..j].trim_end_matches(['f', 'F']);
    (token.parse().unwrap_or(0.0), j)
}

fn skip_string(src: &str, i: usize) -> usize {
    take_string(src, i).1
}

fn skip_ws_comments(src: &str, mut i: usize) -> usize {
    let bytes = src.as_bytes();
    loop {
        if i >= bytes.len() {
            return i;
        }
        if bytes[i].is_ascii_whitespace() {
            i += 1;
            continue;
        }
        if bytes[i] == b'/' && i + 1 < bytes.len() && bytes[i + 1] == b'/' {
            while i < bytes.len() && bytes[i] != b'\n' {
                i += 1;
            }
            continue;
        }
        return i;
    }
}

pub fn strings(args: &[Arg]) -> Vec<String> {
    args.iter()
        .filter_map(|arg| match arg {
            Arg::Str(text) => Some(text.clone()),
            _ => None,
        })
        .collect()
}

pub fn nums(args: &[Arg]) -> Vec<f32> {
    args.iter()
        .filter_map(|arg| match arg {
            Arg::Num(n) => Some(*n),
            _ => None,
        })
        .collect()
}

#[allow(dead_code)]
pub fn ident_ends(args: &[Arg], suffix: &str) -> bool {
    args.iter().any(|arg| matches!(arg, Arg::Ident(text) if text.ends_with(suffix)))
}

pub fn body_of(args: &[Arg]) -> Option<&str> {
    args.iter().rev().find_map(|arg| match arg {
        Arg::Body(text) => Some(text.as_str()),
        _ => None,
    })
}

pub fn read_call(src: &str, i: usize) -> Option<(String, Vec<Arg>, usize)> {
    let bytes = src.as_bytes();
    if i >= bytes.len() {
        return None;
    }
    let start = i;
    let mut j = i;
    while j < bytes.len() && (bytes[j].is_ascii_alphanumeric() || bytes[j] == b'_' || bytes[j] == b':' || bytes[j] == b'.') {
        j += 1;
    }
    if j == start {
        return None;
    }
    let name = src[start..j].to_string();
    while j < bytes.len() && bytes[j].is_ascii_whitespace() {
        j += 1;
    }
    if j >= bytes.len() || bytes[j] != b'(' {
        return None;
    }
    let (args, consumed) = parse_args(&src[j + 1..]);
    Some((name, args, j + 1 + consumed))
}

#[allow(dead_code)]
pub fn between(src: &str, start: &str, end: &str) -> Option<String> {
    let i = src.find(start)? + start.len();
    let j = src[i..].find(end)? + i;
    Some(src[i..j].to_string())
}

pub fn rust_string(text: &str) -> String {
    format!(
        "\"{}\"",
        text.replace('\\', "\\\\").replace('"', "\\\"").replace('\n', "\\n")
    )
}

pub fn num(n: f32) -> String {
    let rounded = (n * 1000.0).round() / 1000.0;
    if (rounded - rounded.round()).abs() < 0.0001 {
        format!("{:.1}", rounded.round())
    } else {
        format!("{rounded}")
    }
}
