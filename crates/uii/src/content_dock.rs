//! Assets and Components dock. Another agent owns this file.
//! The shell calls `draw` once per frame. This hook draws nothing.

use dear_imgui_rs::Ui;
use uii_core::Editor;

pub fn draw(_ui: &Ui, _editor: &mut Editor) {}
