//! Chrome for the native editor.
//!
//! The arrangement follows Figma: a slim tool bar, layers on the left, the
//! canvas in the middle, properties on the right, and a collapsible dock
//! along the bottom. Surfaces are neutral dark gray. The accent is a dark
//! navy, used for the active tool, the selected tab, and the selection.
//! Type is 14px, inset is 8–12px, and corners are 4px.

use dear_imgui_rs::{FontAtlas, FontSource, StbTrueTypeFontData, Style, StyleColor};

pub const TAB_H: f32 = 36.0;
pub const TOOLBAR_H: f32 = 40.0;
pub const LAYERS_W: f32 = 240.0;
pub const PROPS_W: f32 = 260.0;
pub const DOCK_BAR: f32 = 36.0;
pub const DOCK_BODY: f32 = 204.0;
pub const TIMELINE_H: f32 = 280.0;
pub const ROW_H: f32 = 26.0;
pub const INSET: f32 = 12.0;

const fn rgb(hex: u32) -> [f32; 4] {
    [
        ((hex >> 16) & 0xff) as f32 / 255.0,
        ((hex >> 8) & 0xff) as f32 / 255.0,
        (hex & 0xff) as f32 / 255.0,
        1.0,
    ]
}

/// Toolbar, tab strip, and dock bar.
pub const CHROME: [f32; 4] = rgb(0x1C1C_1C);
/// Layers, properties, and dock body.
pub const PANEL: [f32; 4] = rgb(0x2A2A_2A);
/// Design surface. Darker than the panels so frames read as objects.
pub const CANVAS: [f32; 4] = rgb(0x1414_14);
pub const INPUT: [f32; 4] = rgb(0x1C1C_1C);
pub const HOVER: [f32; 4] = rgb(0x3333_33);
/// Selected layer row. A navy wash on the gray panel.
pub const ROW_SELECTED: [f32; 4] = rgb(0x1A2C_44);
pub const DIVIDER: [f32; 4] = rgb(0x3A3A_3A);
pub const TEXT: [f32; 4] = rgb(0xE6E6_E6);
pub const MUTED: [f32; 4] = rgb(0x8E8E_8E);
/// Dark navy. Active tool, selected tab, and the canvas selection.
pub const ACCENT: [f32; 4] = rgb(0x1B3F_73);
pub const ACCENT_HOVER: [f32; 4] = rgb(0x2452_8F);
pub const WHITE: [f32; 4] = [1.0, 1.0, 1.0, 1.0];
pub const TOOL: [f32; 4] = rgb(0x2A2A_2A);

pub fn clear_color() -> [f32; 3] {
    [CANVAS[0], CANVAS[1], CANVAS[2]]
}

pub fn apply(style: &mut Style) {
    style.set_window_padding([0.0, 0.0]);
    style.set_window_rounding(0.0);
    style.set_window_border_size(0.0);
    style.set_child_rounding(0.0);
    style.set_child_border_size(0.0);
    style.set_popup_rounding(6.0);
    style.set_frame_padding([8.0, 3.0]);
    style.set_frame_rounding(4.0);
    style.set_frame_border_size(0.0);
    style.set_item_spacing([8.0, 6.0]);
    style.set_item_inner_spacing([6.0, 4.0]);
    style.set_indent_spacing(16.0);
    style.set_scrollbar_size(10.0);
    style.set_scrollbar_rounding(4.0);
    style.set_grab_rounding(4.0);
    style.set_tab_rounding(4.0);
    style.set_selectable_rounding(4.0);
    style.set_selectable_text_align([0.0, 0.5]);
    style.set_button_text_align([0.5, 0.5]);

    style.set_color(StyleColor::Text, TEXT);
    style.set_color(StyleColor::TextDisabled, MUTED);
    style.set_color(StyleColor::WindowBg, PANEL);
    style.set_color(StyleColor::ChildBg, PANEL);
    style.set_color(StyleColor::PopupBg, CANVAS);
    style.set_color(StyleColor::Border, DIVIDER);
    style.set_color(StyleColor::BorderShadow, [0.0, 0.0, 0.0, 0.0]);
    style.set_color(StyleColor::FrameBg, INPUT);
    style.set_color(StyleColor::FrameBgHovered, HOVER);
    style.set_color(StyleColor::FrameBgActive, HOVER);
    style.set_color(StyleColor::TitleBg, CHROME);
    style.set_color(StyleColor::TitleBgActive, CHROME);
    style.set_color(StyleColor::TitleBgCollapsed, CHROME);
    style.set_color(StyleColor::MenuBarBg, CHROME);
    style.set_color(StyleColor::ScrollbarBg, PANEL);
    style.set_color(StyleColor::ScrollbarGrab, DIVIDER);
    style.set_color(StyleColor::ScrollbarGrabHovered, HOVER);
    style.set_color(StyleColor::ScrollbarGrabActive, MUTED);
    style.set_color(StyleColor::CheckMark, ACCENT);
    style.set_color(StyleColor::SliderGrab, ACCENT);
    style.set_color(StyleColor::SliderGrabActive, ACCENT_HOVER);
    style.set_color(StyleColor::Button, TOOL);
    style.set_color(StyleColor::ButtonHovered, HOVER);
    style.set_color(StyleColor::ButtonActive, ACCENT);
    style.set_color(StyleColor::Header, ROW_SELECTED);
    style.set_color(StyleColor::HeaderHovered, HOVER);
    style.set_color(StyleColor::HeaderActive, ROW_SELECTED);
    style.set_color(StyleColor::Separator, DIVIDER);
    style.set_color(StyleColor::SeparatorHovered, ACCENT);
    style.set_color(StyleColor::SeparatorActive, ACCENT);
    style.set_color(StyleColor::ResizeGrip, DIVIDER);
    style.set_color(StyleColor::ResizeGripHovered, ACCENT);
    style.set_color(StyleColor::ResizeGripActive, ACCENT_HOVER);
    style.set_color(StyleColor::Tab, PANEL);
    style.set_color(StyleColor::TabHovered, HOVER);
    style.set_color(StyleColor::TabSelected, ACCENT);
    style.set_color(
        StyleColor::TextSelectedBg,
        [ACCENT[0], ACCENT[1], ACCENT[2], 0.35],
    );
    style.set_color(StyleColor::NavCursor, ACCENT);
    style.set_color(
        StyleColor::DockingPreview,
        [ACCENT[0], ACCENT[1], ACCENT[2], 0.4],
    );
    style.set_color(StyleColor::DockingEmptyBg, CANVAS);
}

/// One UI face for the whole window. Inter when the machine has it, otherwise
/// Dear ImGui's scalable default, both at 14px.
pub fn load_font(atlas: &FontAtlas) {
    const CANDIDATES: &[&str] = &[
        "/usr/share/fonts/truetype/macos/Inter-Regular.ttf",
        "/usr/share/fonts/truetype/inter/Inter-Regular.ttf",
        "/usr/share/fonts/truetype/liberation/LiberationSans-Regular.ttf",
        "/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf",
    ];
    atlas.clear_fonts();
    for path in CANDIDATES {
        if let Ok(data) = StbTrueTypeFontData::from_file(path) {
            atlas.add_font(&[FontSource::stb_truetype_with_size(data, 14.0)]);
            return;
        }
    }
    atlas.add_font(&[FontSource::default_vector_with_size(14.0)]);
}
