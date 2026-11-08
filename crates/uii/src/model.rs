use std::collections::BTreeMap;
use std::fmt;

use serde::{Deserialize, Serialize};

use crate::error::Error;

/// One UI document. Every format imports into this tree and exports from it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Document {
    pub name: String,
    pub root: Node,
}

/// A node in the document tree.
///
/// `Region` is an HTML region (header, nav, main, aside, footer, section) and
/// a docking panel where the target has docking. `Box` is a nested box and
/// becomes a child window where the target has child windows. `Table` is a
/// table where the target has tables.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Node {
    pub id: String,
    pub kind: NodeKind,
    pub name: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub text: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub columns: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub rows: Vec<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dock: Option<DockSide>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub axis: Option<Axis>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub props: BTreeMap<String, String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub children: Vec<Node>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum NodeKind {
    Window,
    Dock,
    Region,
    Table,
    Box,
    Label,
    Button,
    TextInput,
    Checkbox,
    Slider,
    Image,
    Separator,
    HeaderBar,
    Island,
}

impl NodeKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Window => "window",
            Self::Dock => "dock",
            Self::Region => "region",
            Self::Table => "table",
            Self::Box => "box",
            Self::Label => "label",
            Self::Button => "button",
            Self::TextInput => "text-input",
            Self::Checkbox => "checkbox",
            Self::Slider => "slider",
            Self::Image => "image",
            Self::Separator => "separator",
            Self::HeaderBar => "header-bar",
            Self::Island => "island",
        }
    }

    pub fn parse(value: &str) -> Result<Self, Error> {
        Ok(match value {
            "window" => Self::Window,
            "dock" => Self::Dock,
            "region" => Self::Region,
            "table" => Self::Table,
            "box" => Self::Box,
            "label" => Self::Label,
            "button" => Self::Button,
            "text-input" => Self::TextInput,
            "checkbox" => Self::Checkbox,
            "slider" => Self::Slider,
            "image" => Self::Image,
            "separator" => Self::Separator,
            "header-bar" => Self::HeaderBar,
            "island" => Self::Island,
            other => {
                return Err(Error::new(format!("unknown node kind `{other}`")));
            }
        })
    }
}

/// Where a region sits in a dock.
///
/// Dear ImGui maps these through `ImGuiDir_*` / `DockSplit`. Raven maps them
/// through `RDockSide` (`R_DOCK_LEFT` and the rest). There is no
/// `R_DOCK_CENTER`; the center panel is the first `R_DOCK_TAB`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum DockSide {
    Left,
    Right,
    Top,
    Bottom,
    Center,
    Tab,
}

impl DockSide {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Left => "left",
            Self::Right => "right",
            Self::Top => "top",
            Self::Bottom => "bottom",
            Self::Center => "center",
            Self::Tab => "tab",
        }
    }

    pub fn parse(value: &str) -> Result<Self, Error> {
        Ok(match value {
            "left" => Self::Left,
            "right" => Self::Right,
            "top" => Self::Top,
            "bottom" => Self::Bottom,
            "center" => Self::Center,
            "tab" => Self::Tab,
            other => return Err(Error::new(format!("unknown dock side `{other}`"))),
        })
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Axis {
    Vertical,
    Horizontal,
}

impl Axis {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Vertical => "vertical",
            Self::Horizontal => "horizontal",
        }
    }

    pub fn parse(value: &str) -> Result<Self, Error> {
        Ok(match value {
            "vertical" => Self::Vertical,
            "horizontal" => Self::Horizontal,
            other => return Err(Error::new(format!("unknown axis `{other}`"))),
        })
    }
}

/// A format the call point can import and export.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Format {
    HtmlCss,
    React,
    Vue,
    Svelte,
    Angular,
    BanterImgui,
    RavenImgui,
    DearImgui,
    Egui,
    Slint,
    Qt,
    WinUi,
    SwiftUi,
    Gtk,
    JetpackCompose,
}

impl Format {
    pub const ALL: [Format; 15] = [
        Format::HtmlCss,
        Format::React,
        Format::Vue,
        Format::Svelte,
        Format::Angular,
        Format::BanterImgui,
        Format::RavenImgui,
        Format::DearImgui,
        Format::Egui,
        Format::Slint,
        Format::Qt,
        Format::WinUi,
        Format::SwiftUi,
        Format::Gtk,
        Format::JetpackCompose,
    ];

    pub fn as_str(self) -> &'static str {
        match self {
            Self::HtmlCss => "html-css",
            Self::React => "react",
            Self::Vue => "vue",
            Self::Svelte => "svelte",
            Self::Angular => "angular",
            Self::BanterImgui => "banter-imgui",
            Self::RavenImgui => "raven-imgui",
            Self::DearImgui => "dear-imgui",
            Self::Egui => "egui",
            Self::Slint => "slint",
            Self::Qt => "qt",
            Self::WinUi => "winui",
            Self::SwiftUi => "swiftui",
            Self::Gtk => "gtk",
            Self::JetpackCompose => "jetpack-compose",
        }
    }

    pub fn from_name(name: &str) -> Result<Self, Error> {
        let key = name.trim().to_ascii_lowercase();
        Ok(match key.as_str() {
            "html" | "html-css" | "html/css" | "htmlcss" => Self::HtmlCss,
            "react" => Self::React,
            "vue" => Self::Vue,
            "svelte" => Self::Svelte,
            "angular" => Self::Angular,
            "banter" | "banter-imgui" | "banterimgui" => Self::BanterImgui,
            "raven" | "raven-imgui" | "custom-imgui" | "customimgui" => Self::RavenImgui,
            "dear-imgui" | "dearimgui" | "imgui" | "dear-imgui-c++" => Self::DearImgui,
            "egui" => Self::Egui,
            "slint" => Self::Slint,
            "qt" => Self::Qt,
            "winui" | "win-ui" => Self::WinUi,
            "swiftui" | "swift-ui" => Self::SwiftUi,
            "gtk" => Self::Gtk,
            "compose" | "jetpack" | "jetpack-compose" => Self::JetpackCompose,
            other => {
                return Err(Error::new(format!(
                    "unknown format `{other}` (expected one of html-css, react, vue, svelte, angular, banter-imgui, custom-imgui, dear-imgui, egui, slint, qt, winui, swiftui, gtk, jetpack-compose)"
                )));
            }
        })
    }
}

impl fmt::Display for Format {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Banter `layout::Region` slugs from `crates/banter/src/layout.rs`.
pub(crate) fn banter_region_variant(slug: &str) -> Option<&'static str> {
    match slug {
        "title-bar" => Some("TitleBar"),
        "rail" => Some("Rail"),
        "sidebar" => Some("Sidebar"),
        "stage" => Some("Stage"),
        "members" => Some("Members"),
        "header" => Some("Header"),
        "user-panel" => Some("UserPanel"),
        "voice-panel" => Some("VoicePanel"),
        "composer" => Some("Composer"),
        _ => None,
    }
}
