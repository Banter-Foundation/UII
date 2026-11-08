//! Bottom content drawer. Assets and Components share this one dock.
//! They are not top-level tabs.

/// Height of the always-visible drawer bar.
pub const HANDLE_H: f32 = 32.0;
/// Body height used until the user drags the top edge.
pub const DEFAULT_BODY_H: f32 = 280.0;
/// Smallest body height stored from a drag or a saved session.
pub const MIN_BODY_H: f32 = 160.0;
/// Largest body height stored from a drag or a saved session.
pub const MAX_BODY_H: f32 = 1200.0;
/// Open body cannot take more than this share of the window.
pub const MAX_BODY_FRACTION: f32 = 0.75;
/// Left pane (Assets) cannot shrink below this share of the body.
pub const MIN_ASSETS_FRACTION: f32 = 0.25;
/// Left pane (Assets) cannot grow past this share of the body.
pub const MAX_ASSETS_FRACTION: f32 = 0.75;
/// Gap between Assets and Components, and between the asset tree and its contents.
pub const SPLITTER_W: f32 = 4.0;
/// Pin control width on the right of the open handle.
pub const PIN_W: f32 = 72.0;
/// Preferred width of the asset folder tree.
pub const TREE_W: f32 = 220.0;
/// Preferred minimum width of the asset contents column.
pub const CONTENTS_MIN_W: f32 = 120.0;
/// Pointer distance from the body top that starts a resize drag.
pub const RESIZE_HIT: f32 = 6.0;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Rect {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

impl Rect {
    pub const ZERO: Self = Self {
        x: 0.0,
        y: 0.0,
        w: 0.0,
        h: 0.0,
    };

    pub fn right(self) -> f32 {
        self.x + self.w
    }

    pub fn bottom(self) -> f32 {
        self.y + self.h
    }

    pub fn contains(self, x: f32, y: f32) -> bool {
        x >= self.x && y >= self.y && x < self.right() && y < self.bottom()
    }

    pub fn overlaps(self, other: Self) -> bool {
        self.w > 0.0
            && self.h > 0.0
            && other.w > 0.0
            && other.h > 0.0
            && self.x < other.right()
            && other.x < self.right()
            && self.y < other.bottom()
            && other.y < self.bottom()
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DrawerState {
    pub open: bool,
    pub pinned: bool,
    pub body_height: f32,
    pub assets_fraction: f32,
}

impl Default for DrawerState {
    fn default() -> Self {
        Self {
            open: false,
            pinned: false,
            body_height: DEFAULT_BODY_H,
            assets_fraction: 0.5,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum DrawerCommand {
    Toggle,
    SetOpen(bool),
    TogglePin,
    SetPinned(bool),
    SetBodyHeight(f32),
    SetAssetsFraction(f32),
    /// A click in the stage. Closes the drawer only when it is open and unpinned.
    PointerLeftStage,
}

impl DrawerState {
    pub fn apply(&mut self, command: DrawerCommand) {
        match command {
            DrawerCommand::Toggle => self.open = !self.open,
            DrawerCommand::SetOpen(open) => self.open = open,
            DrawerCommand::TogglePin => self.pinned = !self.pinned,
            DrawerCommand::SetPinned(pinned) => self.pinned = pinned,
            DrawerCommand::SetBodyHeight(height) => {
                if height.is_finite() {
                    self.body_height = height.clamp(MIN_BODY_H, MAX_BODY_H);
                }
            }
            DrawerCommand::SetAssetsFraction(fraction) => {
                if fraction.is_finite() {
                    self.assets_fraction = fraction.clamp(MIN_ASSETS_FRACTION, MAX_ASSETS_FRACTION);
                }
            }
            DrawerCommand::PointerLeftStage => {
                if self.open && !self.pinned {
                    self.open = false;
                }
            }
        }
    }

    pub fn resolved_body_height(self, viewport_h: f32) -> f32 {
        if !self.open {
            return 0.0;
        }
        let viewport_h = viewport_h.max(0.0);
        let available = (viewport_h - HANDLE_H).max(0.0);
        if available <= 0.0 {
            return 0.0;
        }
        let max_h = (viewport_h * MAX_BODY_FRACTION).min(available);
        let min_h = MIN_BODY_H.min(max_h);
        self.body_height.clamp(min_h, max_h)
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DrawerLayout {
    pub stage: Rect,
    pub handle: Rect,
    pub body: Option<Rect>,
    pub assets: Option<Rect>,
    pub components: Option<Rect>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PaneSplit {
    pub tree: Rect,
    pub contents: Rect,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct HandleHits {
    pub toggle: Rect,
    pub pin: Option<Rect>,
}

pub fn layout_drawer(viewport_w: f32, viewport_h: f32, state: &DrawerState) -> DrawerLayout {
    let viewport_w = viewport_w.max(0.0);
    let viewport_h = viewport_h.max(0.0);
    if viewport_w == 0.0 || viewport_h == 0.0 {
        return DrawerLayout {
            stage: Rect::ZERO,
            handle: Rect::ZERO,
            body: None,
            assets: None,
            components: None,
        };
    }

    let handle_h = HANDLE_H.min(viewport_h);
    let handle = Rect {
        x: 0.0,
        y: viewport_h - handle_h,
        w: viewport_w,
        h: handle_h,
    };
    let body_h = state.resolved_body_height(viewport_h);
    let body = if state.open && body_h > 0.0 {
        Some(Rect {
            x: 0.0,
            y: handle.y - body_h,
            w: viewport_w,
            h: body_h,
        })
    } else {
        None
    };
    let stage_h = if state.open && state.pinned {
        body.map(|body| body.y).unwrap_or(handle.y)
    } else {
        handle.y
    };
    let stage = Rect {
        x: 0.0,
        y: 0.0,
        w: viewport_w,
        h: stage_h.max(0.0),
    };
    let (assets, components) = match body {
        Some(body) => {
            let (assets, components) = split_panes(body, state.assets_fraction);
            (Some(assets), Some(components))
        }
        None => (None, None),
    };
    DrawerLayout {
        stage,
        handle,
        body,
        assets,
        components,
    }
}

pub fn split_panes(body: Rect, fraction: f32) -> (Rect, Rect) {
    let fraction = fraction.clamp(MIN_ASSETS_FRACTION, MAX_ASSETS_FRACTION);
    let gap = if body.w > SPLITTER_W { SPLITTER_W } else { 0.0 };
    let inner = (body.w - gap).max(0.0);
    let assets_w = inner * fraction;
    let components_w = inner - assets_w;
    (
        Rect {
            x: body.x,
            y: body.y,
            w: assets_w,
            h: body.h,
        },
        Rect {
            x: body.x + assets_w + gap,
            y: body.y,
            w: components_w,
            h: body.h,
        },
    )
}

pub fn layout_assets_pane(pane: Rect) -> PaneSplit {
    let pane = Rect {
        x: pane.x,
        y: pane.y,
        w: pane.w.max(0.0),
        h: pane.h.max(0.0),
    };
    let gap = if pane.w > SPLITTER_W { SPLITTER_W } else { 0.0 };
    let inner = (pane.w - gap).max(0.0);
    let tree_w = if inner <= CONTENTS_MIN_W {
        inner * 0.4
    } else {
        TREE_W.min((inner - CONTENTS_MIN_W).max(0.0))
    };
    let tree_w = tree_w.clamp(0.0, inner);
    let contents_w = inner - tree_w;
    PaneSplit {
        tree: Rect {
            x: pane.x,
            y: pane.y,
            w: tree_w,
            h: pane.h,
        },
        contents: Rect {
            x: pane.x + tree_w + gap,
            y: pane.y,
            w: contents_w,
            h: pane.h,
        },
    }
}

pub fn handle_hits(handle: Rect, open: bool) -> HandleHits {
    if open && handle.w > PIN_W {
        let pin = Rect {
            x: handle.right() - PIN_W,
            y: handle.y,
            w: PIN_W,
            h: handle.h,
        };
        HandleHits {
            toggle: Rect {
                x: handle.x,
                y: handle.y,
                w: handle.w - PIN_W,
                h: handle.h,
            },
            pin: Some(pin),
        }
    } else {
        HandleHits {
            toggle: handle,
            pin: None,
        }
    }
}

pub fn pointer_on_resize_edge(body: Rect, pointer_y: f32) -> bool {
    (pointer_y - body.y).abs() <= RESIZE_HIT
}

pub fn body_height_from_pointer(viewport_h: f32, pointer_y: f32) -> f32 {
    (viewport_h - HANDLE_H) - pointer_y
}

pub fn pointer_in_dock(layout: &DrawerLayout, x: f32, y: f32) -> bool {
    layout.handle.contains(x, y) || layout.body.is_some_and(|body| body.contains(x, y))
}

pub fn should_dismiss_unpinned(state: &DrawerState, clicked: bool, pointer_in_dock: bool) -> bool {
    state.open && !state.pinned && clicked && !pointer_in_dock
}

/// Edge detector for Ctrl+Space. Holding the chord does not repeat.
#[derive(Clone, Debug, Default)]
pub struct ShortcutEdge {
    chord_down: bool,
}

impl ShortcutEdge {
    pub fn ctrl_space(&mut self, ctrl: bool, space: bool) -> bool {
        let chord = ctrl && space;
        let fire = chord && !self.chord_down;
        self.chord_down = chord;
        fire
    }
}

pub fn shortcut_fires(edge: bool, text_input_active: bool) -> bool {
    edge && !text_input_active
}
