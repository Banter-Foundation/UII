# UII

Native Rust app. The window is GLFW and OpenGL 3.3. The interface is Dear ImGui from the docking branch (`dear-imgui-rs` 0.22), the same stack as [Banter](https://github.com/Banter-Foundation/Banter).

This tree contains the Inspect tool. Inspect is a tool, not a tab. It shows the selected item's size, color, and type. Nothing is selected until the rest of UII sets a selection through `InspectState::set_selection`. This tree does not include other tabs, the assets dock, import/export, or the Cursor plugin.

## Build

Rust 1.95 or newer. Banter is built with 1.99. A display is required to open the window.

```
cargo run -p uii
```

## Inspect

`InspectTool` is the tool. `InspectState` holds the current selection.

- Size is width × height.
- Color is `#RRGGBB`, or `#RRGGBBAA` when alpha is not opaque.
- Type is the selected item's type name.

With no selection the tool shows "Nothing selected".
