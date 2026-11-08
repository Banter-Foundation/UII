//! Keyboard commands for the design canvas. The window shows [`shortcuts`].

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Command {
    Undo,
    Redo,
    Copy,
    Paste,
    Duplicate,
    Delete,
    SelectAll,
    Group,
    ZoomIn,
    ZoomOut,
    Fit,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Shortcut {
    pub command: Command,
    pub label: &'static str,
    pub keys: &'static str,
}

const SHORTCUTS: [Shortcut; 11] = [
    Shortcut {
        command: Command::Undo,
        label: "Undo",
        keys: "Ctrl+Z",
    },
    Shortcut {
        command: Command::Redo,
        label: "Redo",
        keys: "Ctrl+Shift+Z",
    },
    Shortcut {
        command: Command::Copy,
        label: "Copy",
        keys: "Ctrl+C",
    },
    Shortcut {
        command: Command::Paste,
        label: "Paste",
        keys: "Ctrl+V",
    },
    Shortcut {
        command: Command::Duplicate,
        label: "Duplicate",
        keys: "Ctrl+D",
    },
    Shortcut {
        command: Command::Delete,
        label: "Delete",
        keys: "Delete",
    },
    Shortcut {
        command: Command::SelectAll,
        label: "Select all",
        keys: "Ctrl+A",
    },
    Shortcut {
        command: Command::Group,
        label: "Group",
        keys: "Ctrl+G",
    },
    Shortcut {
        command: Command::ZoomIn,
        label: "Zoom in",
        keys: "Ctrl+=",
    },
    Shortcut {
        command: Command::ZoomOut,
        label: "Zoom out",
        keys: "Ctrl+-",
    },
    Shortcut {
        command: Command::Fit,
        label: "Fit",
        keys: "Shift+1",
    },
];

pub fn shortcuts() -> &'static [Shortcut] {
    &SHORTCUTS
}

/// HTML and CSS stay behind a closed panel until the designer opens them.
pub const MARKUP_STARTS_CLOSED: bool = true;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BindKey {
    Z,
    Y,
    C,
    V,
    D,
    A,
    G,
    Equal,
    Minus,
    Delete,
    Backspace,
    Digit1,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Bind {
    pub ctrl: bool,
    pub shift: bool,
    pub key: BindKey,
}

pub fn command_for(bind: Bind) -> Option<Command> {
    use BindKey::*;
    Some(match (bind.ctrl, bind.shift, bind.key) {
        (true, false, Z) => Command::Undo,
        (true, true, Z) | (true, false, Y) => Command::Redo,
        (true, false, C) => Command::Copy,
        (true, false, V) => Command::Paste,
        (true, false, D) => Command::Duplicate,
        (false, _, Delete) | (false, _, Backspace) => Command::Delete,
        (true, false, A) => Command::SelectAll,
        (true, false, G) => Command::Group,
        (true, _, Equal) => Command::ZoomIn,
        (true, false, Minus) => Command::ZoomOut,
        (false, true, Digit1) => Command::Fit,
        _ => return None,
    })
}
