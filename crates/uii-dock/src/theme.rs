//! Drawer colors taken from Banter's Dear ImGui theme tokens.
//!
//! Banter documents these in `crates/banter/src/theme.rs`. UII uses the same
//! dark imgui skin so the content drawer matches the foundation client.

/// `0xRRGGBB` as normalized RGBA.
pub const fn hex(rgb: u32) -> [f32; 4] {
    [
        ((rgb >> 16) & 0xFF) as f32 / 255.0,
        ((rgb >> 8) & 0xFF) as f32 / 255.0,
        (rgb & 0xFF) as f32 / 255.0,
        1.0,
    ]
}

pub const STAGE_BG: [f32; 4] = hex(0x313338);
pub const PANEL_BG: [f32; 4] = hex(0x232428);
pub const SIDEBAR_BG: [f32; 4] = hex(0x2B2D31);
pub const INPUT_BG: [f32; 4] = hex(0x1E1F22);
pub const HANDLE_BG: [f32; 4] = hex(0x1A1B1E);
pub const TEXT: [f32; 4] = hex(0xF2F3F5);
pub const MUTED: [f32; 4] = hex(0x949BA4);
pub const BLURPLE: [f32; 4] = hex(0x5865F2);
pub const ROW_HOVER: [f32; 4] = hex(0x35373C);
pub const ROW_SELECTED: [f32; 4] = hex(0x404249);
pub const DIVIDER: [f32; 4] = hex(0x3F4147);
pub const SCROLL_THUMB: [f32; 4] = hex(0x1A1B1E);

pub fn apply(style: &mut dear_imgui_rs::Style) {
    style.set_color(dear_imgui_rs::StyleColor::Text, TEXT);
    style.set_color(dear_imgui_rs::StyleColor::TextDisabled, MUTED);
    style.set_color(dear_imgui_rs::StyleColor::WindowBg, PANEL_BG);
    style.set_color(dear_imgui_rs::StyleColor::ChildBg, SIDEBAR_BG);
    style.set_color(dear_imgui_rs::StyleColor::PopupBg, PANEL_BG);
    style.set_color(dear_imgui_rs::StyleColor::Border, DIVIDER);
    style.set_color(dear_imgui_rs::StyleColor::FrameBg, INPUT_BG);
    style.set_color(dear_imgui_rs::StyleColor::FrameBgHovered, ROW_HOVER);
    style.set_color(dear_imgui_rs::StyleColor::FrameBgActive, ROW_SELECTED);
    style.set_color(dear_imgui_rs::StyleColor::TitleBg, HANDLE_BG);
    style.set_color(dear_imgui_rs::StyleColor::TitleBgActive, HANDLE_BG);
    style.set_color(dear_imgui_rs::StyleColor::TitleBgCollapsed, HANDLE_BG);
    style.set_color(dear_imgui_rs::StyleColor::ScrollbarBg, SIDEBAR_BG);
    style.set_color(dear_imgui_rs::StyleColor::ScrollbarGrab, SCROLL_THUMB);
    style.set_color(dear_imgui_rs::StyleColor::CheckMark, BLURPLE);
    style.set_color(dear_imgui_rs::StyleColor::Button, SIDEBAR_BG);
    style.set_color(dear_imgui_rs::StyleColor::ButtonHovered, ROW_HOVER);
    style.set_color(dear_imgui_rs::StyleColor::ButtonActive, ROW_SELECTED);
    style.set_color(dear_imgui_rs::StyleColor::Header, ROW_SELECTED);
    style.set_color(dear_imgui_rs::StyleColor::HeaderHovered, ROW_HOVER);
    style.set_color(dear_imgui_rs::StyleColor::HeaderActive, BLURPLE);
    style.set_color(dear_imgui_rs::StyleColor::Separator, DIVIDER);
    style.set_color(dear_imgui_rs::StyleColor::DockingPreview, BLURPLE);
    style.set_color(dear_imgui_rs::StyleColor::DockingEmptyBg, STAGE_BG);
    style.set_window_rounding(0.0);
    style.set_frame_rounding(4.0);
    style.set_window_padding([8.0, 8.0]);
    style.set_frame_padding([8.0, 4.0]);
    style.set_item_spacing([8.0, 6.0]);
    style.set_window_border_size(0.0);
}
