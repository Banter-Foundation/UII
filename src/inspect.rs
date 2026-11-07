//! Inspect tool.
//!
//! Inspect is a tool, not a tab. Do not host it in a tab bar. The rest of UII
//! supplies the selection through [`InspectState::set_selection`]. This module
//! does not create items and does not edit size, color, or type. With no
//! selection it shows [`InspectTool::EMPTY_SELECTION`] and does not invent values.

use dear_imgui_rs::{Condition, Ui, WindowFlags};

/// Where a surface lives. Inspect is always a tool.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Surface {
    Tool,
    Tab,
}

/// The Inspect tool. One tool, not a document tab.
pub struct InspectTool;

impl InspectTool {
    pub const NAME: &'static str = "Inspect";
    pub const EMPTY_SELECTION: &'static str = "Nothing selected";
    pub const SIZE_LABEL: &'static str = "Size";
    pub const COLOR_LABEL: &'static str = "Color";
    pub const TYPE_LABEL: &'static str = "Type";

    pub fn surface() -> Surface {
        Surface::Tool
    }

    pub fn is_tool() -> bool {
        Self::surface() == Surface::Tool
    }

    pub fn is_tab() -> bool {
        Self::surface() == Surface::Tab
    }

    /// Paint the tool. `selection` is the item UII has selected, if any.
    pub fn draw(ui: &Ui, selection: Option<&SelectedItem>) {
        let display = ui.io().display_size();
        ui.window(Self::NAME)
            .flags(
                WindowFlags::NO_TITLE_BAR
                    | WindowFlags::NO_RESIZE
                    | WindowFlags::NO_MOVE
                    | WindowFlags::NO_COLLAPSE
                    | WindowFlags::NO_BRING_TO_FRONT_ON_FOCUS
                    | WindowFlags::NO_SAVED_SETTINGS,
            )
            .position([0.0, 0.0], Condition::Always)
            .size(display, Condition::Always)
            .build(|| {
                // Tool control sized to its label. A full-width selectable reads as a tab.
                let text = ui.calc_text_size(Self::NAME);
                let pad = ui.clone_style().frame_padding();
                let _clicked = ui
                    .selectable_config(Self::NAME)
                    .selected(true)
                    .size([text[0] + pad[0] * 2.0, 0.0])
                    .build();
                ui.separator();
                match selection.map(InspectReadout::from_item) {
                    Some(readout) => draw_readout(ui, &readout),
                    None => ui.text(Self::EMPTY_SELECTION),
                }
            });
    }
}

fn draw_readout(ui: &Ui, readout: &InspectReadout) {
    ui.text(InspectTool::SIZE_LABEL);
    ui.same_line();
    ui.text(readout.size_text());

    ui.text(InspectTool::COLOR_LABEL);
    ui.same_line();
    let _swatch = ui.color_button("##inspect-color", readout.color_imgui());
    ui.same_line();
    ui.text(readout.color_text());

    ui.text(InspectTool::TYPE_LABEL);
    ui.same_line();
    ui.text(readout.type_text());
}

/// Current selection for the Inspect tool.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct InspectState {
    selection: Option<SelectedItem>,
}

impl InspectState {
    pub fn new() -> Self {
        Self { selection: None }
    }

    pub fn set_selection(&mut self, item: Option<SelectedItem>) {
        self.selection = item;
    }

    pub fn selection(&self) -> Option<&SelectedItem> {
        self.selection.as_ref()
    }

    pub fn readout(&self) -> Option<InspectReadout> {
        self.selection.as_ref().map(InspectReadout::from_item)
    }

    pub fn draw(&self, ui: &Ui) {
        InspectTool::draw(ui, self.selection.as_ref());
    }
}

/// What Inspect shows for one selected item.
#[derive(Clone, Debug, PartialEq)]
pub struct InspectReadout {
    size: String,
    color: String,
    color_rgba: [u8; 4],
    color_imgui: [f32; 4],
    kind: String,
}

impl InspectReadout {
    pub fn from_item(item: &SelectedItem) -> Self {
        Self {
            size: item.size().text(),
            color: item.color().text(),
            color_rgba: item.color().channels(),
            color_imgui: item.color().imgui(),
            kind: item.kind().as_str().to_string(),
        }
    }

    pub fn size_text(&self) -> &str {
        &self.size
    }

    pub fn color_text(&self) -> &str {
        &self.color
    }

    pub fn type_text(&self) -> &str {
        &self.kind
    }

    pub fn color_rgba(&self) -> [u8; 4] {
        self.color_rgba
    }

    pub fn color_imgui(&self) -> [f32; 4] {
        self.color_imgui
    }
}

/// An item Inspect can describe. Other tools create these. Inspect only reads them.
#[derive(Clone, Debug, PartialEq)]
pub struct SelectedItem {
    size: ItemSize,
    color: Color,
    kind: ItemType,
}

impl SelectedItem {
    pub fn new(size: ItemSize, color: Color, kind: ItemType) -> Self {
        Self { size, color, kind }
    }

    pub fn size(&self) -> ItemSize {
        self.size
    }

    pub fn color(&self) -> Color {
        self.color
    }

    pub fn kind(&self) -> &ItemType {
        &self.kind
    }
}

/// Width and height of the selected item.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ItemSize {
    width: f32,
    height: f32,
}

impl ItemSize {
    pub fn try_new(width: f32, height: f32) -> Result<Self, ItemSizeError> {
        if !width.is_finite() || !height.is_finite() {
            return Err(ItemSizeError::NotFinite);
        }
        if width < 0.0 || height < 0.0 {
            return Err(ItemSizeError::Negative);
        }
        Ok(Self { width, height })
    }

    pub fn width(self) -> f32 {
        self.width
    }

    pub fn height(self) -> f32 {
        self.height
    }

    pub fn text(self) -> String {
        format!("{} × {}", format_px(self.width), format_px(self.height))
    }
}

/// Width or height was not a usable size.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ItemSizeError {
    NotFinite,
    Negative,
}

impl std::fmt::Display for ItemSizeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotFinite => write!(f, "size is not a finite number"),
            Self::Negative => write!(f, "size is negative"),
        }
    }
}

impl std::error::Error for ItemSizeError {}

fn format_px(value: f32) -> String {
    let scaled = (value * 100.0).round() as i64;
    if scaled % 100 == 0 {
        format!("{}", scaled / 100)
    } else if scaled % 10 == 0 {
        format!("{}.{}", scaled / 100, (scaled.abs() % 100) / 10)
    } else {
        format!("{}.{:02}", scaled / 100, scaled.abs() % 100)
    }
}

/// sRGB color of the selected item. Channels are 0–255.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Color {
    r: u8,
    g: u8,
    b: u8,
    a: u8,
}

impl Color {
    pub fn rgb(r: u8, g: u8, b: u8) -> Self {
        Self::rgba(r, g, b, 255)
    }

    pub fn rgba(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self { r, g, b, a }
    }

    pub fn channels(self) -> [u8; 4] {
        [self.r, self.g, self.b, self.a]
    }

    /// Dear ImGui color, channels in 0.0–1.0.
    pub fn imgui(self) -> [f32; 4] {
        [
            self.r as f32 / 255.0,
            self.g as f32 / 255.0,
            self.b as f32 / 255.0,
            self.a as f32 / 255.0,
        ]
    }

    /// `#RRGGBB`, or `#RRGGBBAA` when alpha is not opaque.
    pub fn text(self) -> String {
        if self.a == 255 {
            format!("#{:02X}{:02X}{:02X}", self.r, self.g, self.b)
        } else {
            format!("#{:02X}{:02X}{:02X}{:02X}", self.r, self.g, self.b, self.a)
        }
    }
}

/// Type name of the selected item. The name comes from the item, not from a list here.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ItemType {
    name: String,
}

impl ItemType {
    pub fn try_new(name: impl AsRef<str>) -> Result<Self, ItemTypeError> {
        let name = name.as_ref().trim();
        if name.is_empty() {
            return Err(ItemTypeError::Empty);
        }
        Ok(Self {
            name: name.to_string(),
        })
    }

    pub fn as_str(&self) -> &str {
        &self.name
    }
}

/// Type name was missing.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ItemTypeError {
    Empty,
}

impl std::fmt::Display for ItemTypeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Empty => write!(f, "item type is empty"),
        }
    }
}

impl std::error::Error for ItemTypeError {}
