# Raven reference

Source: https://github.com/oscar-soirey/Raven

Commit: `7f4411c376cec8fad2d071d63696b8444ec2640d` (2026-11-07, merge of PR 51)

Cloned and read for the custom imgui. The tree is not vendored.

Raven is a C11 GUI library. It opens its own windows and draws with Vulkan. That renderer stays out of UII. The editor window stays on Banter's Dear ImGui. The export file adds Raven's shell as functions on that same imgui, and import reads those functions.

Headers used:

- `include/raven/raven_shell.h` — `DockArea`, `Slot.Dock` (`Left`, `Right`, `Top`, `Bottom`, `Tab`), `Rdock_area_add_panel`
- `include/raven/raven_controls.h` — `Table` / `Column` / `Row`, `Button`, `Label`, `Image` (`Source`)
- `bindings/rust/src/lib.rs` — `window_from_xml`, `widget_find`, `action_bind`
- `bindings/rust/examples/hello.rs` — `<Window>`, `<VBox>`, `<Label>`, `<Button>`

Banter's `crates/banter/Cargo.toml` depends on `dear-imgui-rs` 0.22 and `dear-imgui-glow` 0.22 from crates.io. That is the Dear ImGui docking branch. The editor window and both imgui exports stay on those crates. The custom export adds `imgui_dock_area`, `imgui_header_bar`, `imgui_island`, and `imgui_vbox` in the exported file only. The application window does not call them.
