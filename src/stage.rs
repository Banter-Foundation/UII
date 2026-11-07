//! The region above the Assets/Components drawer.
//!
//! [`draw_stage_guest`] is where other editor surfaces add their Dear ImGui
//! windows. Those windows dock into the stage dockspace. This module does not
//! draw Design, Animation, Function, or Inspect.

use dear_imgui_rs::Ui;

pub fn draw_stage_guest(_ui: &Ui) {}
