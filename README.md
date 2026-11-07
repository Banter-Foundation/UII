# UII

Native Rust desktop shell. The window is GLFW, OpenGL 3.3, and the Dear ImGui docking build Banter uses (`dear-imgui-rs` 0.22, `dear-imgui-glow` 0.22).

Assets and Components share one collapsible drawer along the bottom of the window, in the same place as Unreal's content drawer. The bar stays visible. Click it, or press Ctrl+Space while a text field is not active, to open and close the drawer. Pin keeps it in the layout. Without a pin, a click in the stage above the drawer closes it. Drag the top edge of the open drawer to change its height.

The library starts empty. Folders, assets, and components are created in the drawer and saved as `library.json` and `dock.json`.

| OS | Data directory |
| --- | --- |
| Windows | `%LOCALAPPDATA%\UII` |
| macOS | `~/Library/Application Support/UII` |
| Linux | `$XDG_DATA_HOME/UII`, or `~/.local/share/UII` |

`UII_DATA` overrides that directory when it is non-empty.

```
cargo run
```

```
cargo test
```
