# Raven reference

Source: https://github.com/oscar-soirey/Raven

Commit: `7f4411c376cec8fad2d071d63696b8444ec2640d` (2026-11-07, merge of PR 51)

Cloned and read for the custom imgui. The tree is not vendored.

Raven is a C11 GUI library. It opens its own windows and draws with Vulkan. The Rust binding is the `raven` crate (`bindings/rust`, published as `raven-gui`). A window is XML passed to `window_from_xml`.

Headers used:

- `include/raven/raven_shell.h` — `DockArea`, `Slot.Dock` (`Left`, `Right`, `Top`, `Bottom`, `Tab`), `Rdock_area_add_panel`
- `include/raven/raven_controls.h` — `Table` / `Column` / `Row`, `Button`, `Label`, `Image` (`Source`)
- `bindings/rust/src/lib.rs` — `window_from_xml`, `widget_find`, `action_bind`
- `bindings/rust/examples/hello.rs` — `<Window>`, `<VBox>`, `<Label>`, `<Button>`

Banter's window is not Raven. Banter's `crates/banter/Cargo.toml` depends on `dear-imgui-rs` 0.22 and `dear-imgui-glow` 0.22 from crates.io. That is the Dear ImGui docking branch. The designer and the Banter export stay on those crates. The Raven export writes Raven's XML, then the same layout in that Dear ImGui.
