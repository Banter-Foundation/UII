use std::collections::{HashSet, VecDeque};

pub const NODE_WIDTH: f32 = 200.0;
pub const HEADER_HEIGHT: f32 = 28.0;
pub const PORT_ROW: f32 = 22.0;
pub const PORT_PAD: f32 = 8.0;
pub const PORT_RADIUS: f32 = 5.0;
pub const PORT_SLOP: f32 = 2.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct FunctionId(pub u32);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PortId(pub u32);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ConnectionId(pub u32);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FunctionKind {
    Trigger,
    Action,
    Value,
}

impl FunctionKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Trigger => "trigger",
            Self::Action => "action",
            Self::Value => "value",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Trigger => "Trigger",
            Self::Action => "Action",
            Self::Value => "Value",
        }
    }

    pub fn parse(text: &str) -> Option<Self> {
        match text {
            "trigger" => Some(Self::Trigger),
            "action" => Some(Self::Action),
            "value" => Some(Self::Value),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    In,
    Out,
}

impl Direction {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::In => "in",
            Self::Out => "out",
        }
    }

    pub(crate) fn parse(text: &str) -> Option<Self> {
        match text {
            "in" => Some(Self::In),
            "out" => Some(Self::Out),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Port {
    pub id: PortId,
    pub name: String,
    pub direction: Direction,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Rect {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Hit {
    Port { function: FunctionId, port: PortId },
    Body { function: FunctionId },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EditError {
    MissingFunction,
    MissingPort,
    MissingConnection,
    WrongDirection,
    SameFunction,
    InputTaken,
    Cycle,
    EmptyPortName,
    DuplicatePortName,
}

impl std::fmt::Display for EditError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::MissingFunction => "That function is not in the graph.",
            Self::MissingPort => "That port is not on the function.",
            Self::MissingConnection => "That connection is not in the graph.",
            Self::WrongDirection => "Connect an output to an input.",
            Self::SameFunction => "A function cannot connect to itself.",
            Self::InputTaken => "That input is already connected.",
            Self::Cycle => "That connection would loop.",
            Self::EmptyPortName => "A port needs a name.",
            Self::DuplicatePortName => "That port name is already used.",
        })
    }
}

#[derive(Debug, Clone)]
pub struct Connection {
    pub id: ConnectionId,
    pub from_fn: FunctionId,
    pub from_port: PortId,
    pub to_fn: FunctionId,
    pub to_port: PortId,
    pub(crate) alive: bool,
}

#[derive(Debug, Clone)]
struct PortSlot {
    alive: bool,
    port: Port,
}

#[derive(Debug, Clone)]
pub struct Function {
    id: FunctionId,
    alive: bool,
    name: String,
    place: String,
    detail: String,
    kind: FunctionKind,
    x: f32,
    y: f32,
    ports: Vec<PortSlot>,
}

impl Function {
    pub fn id(&self) -> FunctionId {
        self.id
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn place(&self) -> &str {
        &self.place
    }

    pub fn detail(&self) -> &str {
        &self.detail
    }

    pub fn kind(&self) -> FunctionKind {
        self.kind
    }

    pub fn position(&self) -> (f32, f32) {
        (self.x, self.y)
    }

    pub fn ports(&self) -> impl Iterator<Item = &Port> {
        self.ports
            .iter()
            .filter(|slot| slot.alive)
            .map(|slot| &slot.port)
    }

    pub fn inputs(&self) -> impl Iterator<Item = &Port> {
        self.ports().filter(|port| port.direction == Direction::In)
    }

    pub fn outputs(&self) -> impl Iterator<Item = &Port> {
        self.ports().filter(|port| port.direction == Direction::Out)
    }
}

#[derive(Debug, Clone)]
pub struct Graph {
    functions: Vec<Function>,
    connections: Vec<Connection>,
}

impl Default for Graph {
    fn default() -> Self {
        Self::new()
    }
}

impl Graph {
    pub fn new() -> Self {
        Self {
            functions: vec![dead_function(FunctionId(0))],
            connections: vec![dead_connection(ConnectionId(0))],
        }
    }

    pub fn len(&self) -> usize {
        self.functions
            .iter()
            .filter(|function| function.alive)
            .count()
    }

    pub(crate) fn slot_len(&self) -> u32 {
        self.functions.len() as u32
    }

    pub fn get(&self, id: FunctionId) -> Option<&Function> {
        self.functions
            .get(id.0 as usize)
            .filter(|function| function.alive)
    }

    pub fn connections(&self) -> impl Iterator<Item = &Connection> {
        self.connections.iter().filter(|wire| wire.alive)
    }

    pub fn create(
        &mut self,
        kind: FunctionKind,
        name: &str,
        place: &str,
        x: f32,
        y: f32,
    ) -> FunctionId {
        let id = FunctionId(self.functions.len() as u32);
        self.functions.push(Function {
            id,
            alive: true,
            name: name.to_string(),
            place: place.to_string(),
            detail: String::new(),
            kind,
            x,
            y,
            ports: vec![dead_port(PortId(0))],
        });
        match kind {
            FunctionKind::Trigger => {
                let _ = self.insert_port(id, Direction::Out, "when");
            }
            FunctionKind::Action => {
                let _ = self.insert_port(id, Direction::In, "run");
                let _ = self.insert_port(id, Direction::Out, "then");
            }
            FunctionKind::Value => {
                let _ = self.insert_port(id, Direction::Out, "value");
            }
        }
        id
    }

    pub fn set_name(&mut self, id: FunctionId, name: &str) -> Result<(), EditError> {
        self.function_mut(id)?.name = name.to_string();
        Ok(())
    }

    pub fn set_place(&mut self, id: FunctionId, place: &str) -> Result<(), EditError> {
        self.function_mut(id)?.place = place.to_string();
        Ok(())
    }

    pub fn set_detail(&mut self, id: FunctionId, detail: &str) -> Result<(), EditError> {
        self.function_mut(id)?.detail = detail.to_string();
        Ok(())
    }

    pub fn set_kind(&mut self, id: FunctionId, kind: FunctionKind) -> Result<(), EditError> {
        self.function_mut(id)?.kind = kind;
        Ok(())
    }

    pub fn move_to(&mut self, id: FunctionId, x: f32, y: f32) -> Result<(), EditError> {
        let function = self.function_mut(id)?;
        function.x = x;
        function.y = y;
        Ok(())
    }

    pub fn add_port(
        &mut self,
        id: FunctionId,
        direction: Direction,
        name: &str,
    ) -> Result<PortId, EditError> {
        let name = clean_port_name(name)?;
        self.ensure_port_name(id, None, &name)?;
        self.insert_port(id, direction, &name)
    }

    pub fn rename_port(
        &mut self,
        id: FunctionId,
        port: PortId,
        name: &str,
    ) -> Result<(), EditError> {
        let name = clean_port_name(name)?;
        self.ensure_port_name(id, Some(port), &name)?;
        let slot = self
            .function_mut(id)?
            .ports
            .get_mut(port.0 as usize)
            .filter(|slot| slot.alive)
            .ok_or(EditError::MissingPort)?;
        slot.port.name = name;
        Ok(())
    }

    pub fn remove_port(&mut self, id: FunctionId, port: PortId) -> Result<(), EditError> {
        {
            let function = self.function_mut(id)?;
            let slot = function
                .ports
                .get_mut(port.0 as usize)
                .filter(|slot| slot.alive)
                .ok_or(EditError::MissingPort)?;
            slot.alive = false;
        }
        for wire in &mut self.connections {
            if wire.alive
                && ((wire.from_fn == id && wire.from_port == port)
                    || (wire.to_fn == id && wire.to_port == port))
            {
                wire.alive = false;
            }
        }
        Ok(())
    }

    pub fn connect(
        &mut self,
        from: FunctionId,
        from_port: PortId,
        to: FunctionId,
        to_port: PortId,
    ) -> Result<ConnectionId, EditError> {
        self.check_link(from, from_port, to, to_port)?;
        let id = ConnectionId(self.connections.len() as u32);
        self.connections.push(Connection {
            id,
            from_fn: from,
            from_port,
            to_fn: to,
            to_port,
            alive: true,
        });
        Ok(id)
    }

    pub fn disconnect(&mut self, id: ConnectionId) -> Result<(), EditError> {
        let wire = self
            .connections
            .get_mut(id.0 as usize)
            .filter(|wire| wire.alive)
            .ok_or(EditError::MissingConnection)?;
        wire.alive = false;
        Ok(())
    }

    pub fn delete(&mut self, id: FunctionId) -> Result<(), EditError> {
        {
            let function = self.function_mut(id)?;
            function.alive = false;
        }
        for wire in &mut self.connections {
            if wire.alive && (wire.from_fn == id || wire.to_fn == id) {
                wire.alive = false;
            }
        }
        Ok(())
    }

    pub fn wires_from(&self, id: FunctionId) -> Vec<&Connection> {
        self.connections()
            .filter(|wire| wire.from_fn == id)
            .collect()
    }

    pub fn wires_to(&self, id: FunctionId) -> Vec<&Connection> {
        self.connections().filter(|wire| wire.to_fn == id).collect()
    }

    pub fn places(&self) -> Vec<String> {
        let mut places = Vec::new();
        for function in self.functions.iter().filter(|function| function.alive) {
            if function.place.is_empty() {
                continue;
            }
            if !places.iter().any(|place| place == &function.place) {
                places.push(function.place.clone());
            }
        }
        places
    }

    pub fn at_place(&self, place: &str) -> Vec<FunctionId> {
        self.functions
            .iter()
            .filter(|function| function.alive && function.place == place)
            .map(|function| function.id)
            .collect()
    }

    pub fn downstream(&self, id: FunctionId) -> Vec<FunctionId> {
        let mut out = Vec::new();
        let mut seen = HashSet::new();
        seen.insert(id);
        let mut queue = VecDeque::new();
        queue.push_back(id);
        while let Some(current) = queue.pop_front() {
            for wire in self.wires_from(current) {
                if seen.insert(wire.to_fn) {
                    queue.push_back(wire.to_fn);
                    out.push(wire.to_fn);
                }
            }
        }
        out
    }

    pub fn node_rect(&self, id: FunctionId) -> Option<Rect> {
        let function = self.get(id)?;
        Some(rect_of(function))
    }

    pub fn port_center(&self, id: FunctionId, port: PortId) -> Option<(f32, f32)> {
        let function = self.get(id)?;
        let port = function.ports().find(|item| item.id == port)?;
        Some(center_of(function, port))
    }

    pub fn hit_test(&self, x: f32, y: f32) -> Option<Hit> {
        let radius = PORT_RADIUS + PORT_SLOP;
        for function in self
            .functions
            .iter()
            .rev()
            .filter(|function| function.alive)
        {
            for port in function.ports() {
                let (px, py) = center_of(function, port);
                let dx = x - px;
                let dy = y - py;
                if dx * dx + dy * dy <= radius * radius {
                    return Some(Hit::Port {
                        function: function.id,
                        port: port.id,
                    });
                }
            }
            if contains(rect_of(function), x, y) {
                return Some(Hit::Body {
                    function: function.id,
                });
            }
        }
        None
    }

    pub(crate) fn check_link(
        &self,
        from: FunctionId,
        from_port: PortId,
        to: FunctionId,
        to_port: PortId,
    ) -> Result<(), EditError> {
        if self.get(from).is_none() || self.get(to).is_none() {
            return Err(EditError::MissingFunction);
        }
        let from_dir = self
            .get(from)
            .and_then(|function| function.ports().find(|port| port.id == from_port))
            .map(|port| port.direction)
            .ok_or(EditError::MissingPort)?;
        let to_dir = self
            .get(to)
            .and_then(|function| function.ports().find(|port| port.id == to_port))
            .map(|port| port.direction)
            .ok_or(EditError::MissingPort)?;
        if from_dir != Direction::Out || to_dir != Direction::In {
            return Err(EditError::WrongDirection);
        }
        if from == to {
            return Err(EditError::SameFunction);
        }
        if self
            .connections()
            .any(|wire| wire.to_fn == to && wire.to_port == to_port)
        {
            return Err(EditError::InputTaken);
        }
        if self.reaches(to, from) {
            return Err(EditError::Cycle);
        }
        Ok(())
    }

    pub(crate) fn insert_saved_function(&mut self, function: Function) {
        let index = function.id.0 as usize;
        if index >= self.functions.len() {
            return;
        }
        self.functions[index] = function;
    }

    pub(crate) fn grow_functions(&mut self, max_id: u32) {
        while self.functions.len() <= max_id as usize {
            let id = FunctionId(self.functions.len() as u32);
            self.functions.push(dead_function(id));
        }
    }

    pub(crate) fn grow_connections(&mut self, max_id: u32) {
        while self.connections.len() <= max_id as usize {
            let id = ConnectionId(self.connections.len() as u32);
            self.connections.push(dead_connection(id));
        }
    }

    pub(crate) fn insert_saved_connection(&mut self, connection: Connection) {
        let index = connection.id.0 as usize;
        if index < self.connections.len() {
            self.connections[index] = connection;
        }
    }

    fn reaches(&self, start: FunctionId, goal: FunctionId) -> bool {
        let mut stack = vec![start];
        let mut seen = HashSet::new();
        while let Some(id) = stack.pop() {
            if !seen.insert(id) {
                continue;
            }
            for wire in self.wires_from(id) {
                if wire.to_fn == goal {
                    return true;
                }
                stack.push(wire.to_fn);
            }
        }
        false
    }

    fn function_mut(&mut self, id: FunctionId) -> Result<&mut Function, EditError> {
        self.functions
            .get_mut(id.0 as usize)
            .filter(|function| function.alive)
            .ok_or(EditError::MissingFunction)
    }

    fn insert_port(
        &mut self,
        id: FunctionId,
        direction: Direction,
        name: &str,
    ) -> Result<PortId, EditError> {
        let function = self.function_mut(id)?;
        let port_id = PortId(function.ports.len() as u32);
        function.ports.push(PortSlot {
            alive: true,
            port: Port {
                id: port_id,
                name: name.to_string(),
                direction,
            },
        });
        Ok(port_id)
    }

    fn ensure_port_name(
        &self,
        id: FunctionId,
        ignore: Option<PortId>,
        name: &str,
    ) -> Result<(), EditError> {
        let function = self.get(id).ok_or(EditError::MissingFunction)?;
        if function
            .ports()
            .any(|port| port.name == name && ignore.map(|id| id != port.id).unwrap_or(true))
        {
            return Err(EditError::DuplicatePortName);
        }
        Ok(())
    }
}

impl Function {
    pub(crate) fn saved(
        id: FunctionId,
        kind: FunctionKind,
        x: f32,
        y: f32,
        place: String,
        name: String,
        detail: String,
        ports: Vec<Port>,
    ) -> Self {
        let max_port = ports.iter().map(|port| port.id.0).max().unwrap_or(0);
        let mut slots = Vec::new();
        for index in 0..=max_port {
            slots.push(dead_port(PortId(index)));
        }
        for port in ports {
            let index = port.id.0 as usize;
            slots[index] = PortSlot { alive: true, port };
        }
        Self {
            id,
            alive: true,
            name,
            place,
            detail,
            kind,
            x,
            y,
            ports: slots,
        }
    }
}

fn dead_function(id: FunctionId) -> Function {
    Function {
        id,
        alive: false,
        name: String::new(),
        place: String::new(),
        detail: String::new(),
        kind: FunctionKind::Trigger,
        x: 0.0,
        y: 0.0,
        ports: Vec::new(),
    }
}

fn dead_port(id: PortId) -> PortSlot {
    PortSlot {
        alive: false,
        port: Port {
            id,
            name: String::new(),
            direction: Direction::In,
        },
    }
}

fn dead_connection(id: ConnectionId) -> Connection {
    Connection {
        id,
        from_fn: FunctionId(0),
        from_port: PortId(0),
        to_fn: FunctionId(0),
        to_port: PortId(0),
        alive: false,
    }
}

fn clean_port_name(name: &str) -> Result<String, EditError> {
    let trimmed = name.trim();
    if trimmed.is_empty() {
        Err(EditError::EmptyPortName)
    } else {
        Ok(trimmed.to_string())
    }
}

fn rect_of(function: &Function) -> Rect {
    let rows = function
        .inputs()
        .count()
        .max(function.outputs().count())
        .max(1) as f32;
    Rect {
        x: function.x,
        y: function.y,
        width: NODE_WIDTH,
        height: HEADER_HEIGHT + rows * PORT_ROW + PORT_PAD,
    }
}

fn center_of(function: &Function, port: &Port) -> (f32, f32) {
    let index = function
        .ports()
        .filter(|item| item.direction == port.direction)
        .position(|item| item.id == port.id)
        .unwrap_or(0) as f32;
    let y = function.y + HEADER_HEIGHT + PORT_ROW * (index + 0.5);
    let x = match port.direction {
        Direction::In => function.x,
        Direction::Out => function.x + NODE_WIDTH,
    };
    (x, y)
}

fn contains(rect: Rect, x: f32, y: f32) -> bool {
    x >= rect.x && y >= rect.y && x < rect.x + rect.width && y < rect.y + rect.height
}
