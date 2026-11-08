use std::path::{Path, PathBuf};

use crate::{
    Connection, ConnectionId, Direction, EditError, Function, FunctionId, FunctionKind, Graph,
    Port, PortId,
};

#[derive(Debug, PartialEq, Eq)]
pub enum StoreError {
    Header,
    Syntax { line: usize },
    Invalid { line: usize, message: &'static str },
}

impl std::fmt::Display for StoreError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Header => f.write_str("expected \"uii-function 1\""),
            Self::Syntax { line } => write!(f, "line {line}: could not read the function graph"),
            Self::Invalid { line, message } => write!(f, "line {line}: {message}"),
        }
    }
}

pub fn default_graph_path(data_home: Option<&Path>, home: Option<&Path>) -> PathBuf {
    if let Some(dir) = data_home {
        return dir.join("UII").join("function.graph");
    }
    if let Some(home) = home {
        return home
            .join(".local")
            .join("share")
            .join("UII")
            .join("function.graph");
    }
    PathBuf::from("function.graph")
}

pub fn to_text(graph: &Graph) -> String {
    let mut out = String::from("uii-function 1\n");
    for index in 1..graph.slot_len() {
        let Some(function) = graph.get(FunctionId(index)) else {
            continue;
        };
        out.push_str(&format!(
            "fn {} {} {} {} {} {} {}\n",
            function.id().0,
            function.kind().as_str(),
            fmt_num(function.position().0),
            fmt_num(function.position().1),
            quote(function.place()),
            quote(function.name()),
            quote(function.detail()),
        ));
        for port in function.ports() {
            out.push_str(&format!(
                "port {} {} {} {}\n",
                function.id().0,
                port.id.0,
                port.direction.as_str(),
                quote(&port.name),
            ));
        }
    }
    for wire in graph.connections() {
        out.push_str(&format!(
            "wire {} {} {} {} {}\n",
            wire.id.0, wire.from_fn.0, wire.from_port.0, wire.to_fn.0, wire.to_port.0,
        ));
    }
    out
}

pub fn from_text(text: &str) -> Result<Graph, StoreError> {
    let mut lines = text.split('\n').enumerate();
    let header = loop {
        let Some((index, line)) = lines.next() else {
            return Err(StoreError::Header);
        };
        let line = line.trim_end_matches('\r').trim();
        if line.is_empty() {
            continue;
        }
        break (index + 1, line);
    };
    if header.1 != "uii-function 1" {
        return Err(StoreError::Header);
    }

    struct DraftFn {
        line: usize,
        id: FunctionId,
        kind: FunctionKind,
        x: f32,
        y: f32,
        place: String,
        name: String,
        detail: String,
        ports: Vec<(usize, Port)>,
    }
    struct DraftWire {
        line: usize,
        id: ConnectionId,
        from_fn: FunctionId,
        from_port: PortId,
        to_fn: FunctionId,
        to_port: PortId,
    }

    let mut functions: Vec<DraftFn> = Vec::new();
    let mut wires: Vec<DraftWire> = Vec::new();

    for (index, raw) in lines {
        let line_no = index + 1;
        let line = raw.trim_end_matches('\r').trim();
        if line.is_empty() {
            continue;
        }
        let tokens = split_line(line).map_err(|_| StoreError::Syntax { line: line_no })?;
        match tokens.first().map(String::as_str) {
            Some("fn") if tokens.len() == 8 => {
                let id = parse_id(&tokens[1]).map_err(|_| StoreError::Syntax { line: line_no })?;
                let kind = FunctionKind::parse(&tokens[2]).ok_or(StoreError::Invalid {
                    line: line_no,
                    message: "unknown kind",
                })?;
                let x = parse_f32(&tokens[3]).map_err(|_| StoreError::Syntax { line: line_no })?;
                let y = parse_f32(&tokens[4]).map_err(|_| StoreError::Syntax { line: line_no })?;
                if functions.iter().any(|item| item.id.0 == id) {
                    return Err(StoreError::Invalid {
                        line: line_no,
                        message: "duplicate id",
                    });
                }
                functions.push(DraftFn {
                    line: line_no,
                    id: FunctionId(id),
                    kind,
                    x,
                    y,
                    place: tokens[5].clone(),
                    name: tokens[6].clone(),
                    detail: tokens[7].clone(),
                    ports: Vec::new(),
                });
            }
            Some("port") if tokens.len() == 5 => {
                let fn_id =
                    parse_id(&tokens[1]).map_err(|_| StoreError::Syntax { line: line_no })?;
                let port_id =
                    parse_id(&tokens[2]).map_err(|_| StoreError::Syntax { line: line_no })?;
                let direction = Direction::parse(&tokens[3]).ok_or(StoreError::Invalid {
                    line: line_no,
                    message: "unknown direction",
                })?;
                let function = functions.iter_mut().find(|item| item.id.0 == fn_id).ok_or(
                    StoreError::Invalid {
                        line: line_no,
                        message: "missing function",
                    },
                )?;
                if function.ports.iter().any(|(_, port)| port.id.0 == port_id) {
                    return Err(StoreError::Invalid {
                        line: line_no,
                        message: "duplicate id",
                    });
                }
                function.ports.push((
                    line_no,
                    Port {
                        id: PortId(port_id),
                        name: tokens[4].clone(),
                        direction,
                    },
                ));
            }
            Some("wire") if tokens.len() == 6 => {
                let id = parse_id(&tokens[1]).map_err(|_| StoreError::Syntax { line: line_no })?;
                let from_fn =
                    parse_id(&tokens[2]).map_err(|_| StoreError::Syntax { line: line_no })?;
                let from_port =
                    parse_id(&tokens[3]).map_err(|_| StoreError::Syntax { line: line_no })?;
                let to_fn =
                    parse_id(&tokens[4]).map_err(|_| StoreError::Syntax { line: line_no })?;
                let to_port =
                    parse_id(&tokens[5]).map_err(|_| StoreError::Syntax { line: line_no })?;
                if wires.iter().any(|wire| wire.id.0 == id) {
                    return Err(StoreError::Invalid {
                        line: line_no,
                        message: "duplicate id",
                    });
                }
                wires.push(DraftWire {
                    line: line_no,
                    id: ConnectionId(id),
                    from_fn: FunctionId(from_fn),
                    from_port: PortId(from_port),
                    to_fn: FunctionId(to_fn),
                    to_port: PortId(to_port),
                });
            }
            _ => return Err(StoreError::Syntax { line: line_no }),
        }
    }

    let max_fn = functions.iter().map(|item| item.id.0).max().unwrap_or(0);
    let max_wire = wires.iter().map(|wire| wire.id.0).max().unwrap_or(0);
    let mut graph = Graph::new();
    graph.grow_functions(max_fn);
    graph.grow_connections(max_wire);
    for draft in functions {
        let _ = draft.line;
        let ports: Vec<Port> = draft.ports.into_iter().map(|(_, port)| port).collect();
        graph.insert_saved_function(Function::saved(
            draft.id,
            draft.kind,
            draft.x,
            draft.y,
            draft.place,
            draft.name,
            draft.detail,
            ports,
        ));
    }
    for wire in wires {
        if let Err(err) = graph.check_link(wire.from_fn, wire.from_port, wire.to_fn, wire.to_port) {
            return Err(StoreError::Invalid {
                line: wire.line,
                message: link_message(err),
            });
        }
        graph.insert_saved_connection(Connection {
            id: wire.id,
            from_fn: wire.from_fn,
            from_port: wire.from_port,
            to_fn: wire.to_fn,
            to_port: wire.to_port,
            alive: true,
        });
    }
    Ok(graph)
}

fn link_message(err: EditError) -> &'static str {
    match err {
        EditError::Cycle => "that connection would loop",
        EditError::MissingFunction => "missing function",
        EditError::MissingPort => "missing port",
        EditError::WrongDirection => "connect an output to an input",
        EditError::SameFunction => "a function cannot connect to itself",
        EditError::InputTaken => "that input is already connected",
        EditError::MissingConnection | EditError::EmptyPortName | EditError::DuplicatePortName => {
            "invalid connection"
        }
    }
}

fn parse_id(text: &str) -> Result<u32, ()> {
    let id: u32 = text.parse().map_err(|_| ())?;
    if id == 0 {
        Err(())
    } else {
        Ok(id)
    }
}

fn parse_f32(text: &str) -> Result<f32, ()> {
    let value: f32 = text.parse().map_err(|_| ())?;
    if value.is_finite() {
        Ok(value)
    } else {
        Err(())
    }
}

fn fmt_num(n: f32) -> String {
    let mut text = format!("{n:.8}");
    if text.contains('.') {
        while text.ends_with('0') {
            text.pop();
        }
        if text.ends_with('.') {
            text.pop();
        }
    }
    if text == "-0" || text.is_empty() {
        "0".to_string()
    } else {
        text
    }
}

fn quote(text: &str) -> String {
    let mut out = String::from("\"");
    for ch in text.chars() {
        match ch {
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            other => out.push(other),
        }
    }
    out.push('"');
    out
}

fn split_line(line: &str) -> Result<Vec<String>, ()> {
    let chars: Vec<char> = line.chars().collect();
    let mut out = Vec::new();
    let mut index = 0;
    while index < chars.len() {
        while index < chars.len() && chars[index].is_whitespace() {
            index += 1;
        }
        if index >= chars.len() {
            break;
        }
        if chars[index] == '"' {
            index += 1;
            let mut text = String::new();
            loop {
                if index >= chars.len() {
                    return Err(());
                }
                let ch = chars[index];
                index += 1;
                if ch == '\\' {
                    if index >= chars.len() {
                        return Err(());
                    }
                    let escaped = chars[index];
                    index += 1;
                    match escaped {
                        '\\' => text.push('\\'),
                        '"' => text.push('"'),
                        'n' => text.push('\n'),
                        'r' => text.push('\r'),
                        't' => text.push('\t'),
                        _ => return Err(()),
                    }
                } else if ch == '"' {
                    break;
                } else {
                    text.push(ch);
                }
            }
            out.push(text);
        } else {
            let start = index;
            while index < chars.len() && !chars[index].is_whitespace() {
                index += 1;
            }
            out.push(chars[start..index].iter().collect());
        }
    }
    Ok(out)
}
