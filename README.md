# UII

UII is a native layout tool. This branch adds the Animation tab only. Design, Function, the assets dock, Inspect, import/export, and the Cursor plugin are not in this tree.

Animation is a top-level tab for motion on the layout. The shell is not on `main` yet, so the tab is the crate `uii-animation` and the binary `uii-animation` (`crates/uii-animation-host`). When the shell lands, depend on `uii-animation`, register tab id `animation` with label `Animation`, and call `draw_animation_tab` inside that tab. `AnimationTab::posed_layout` is the rest targets with the active clip sampled at the playhead. `NodeId` is a `u32` newtype so it can match the design document's node id.

## Run

Rust 1.95 or newer (Dear ImGui 0.22). A C++ compiler, CMake, OpenGL, and X11 headers are required. `glfw-sys` builds GLFW 3.4 from source when the system copy is older.

```
cargo test -p uii-animation
cargo run -p uii-animation-host
```

The window title is `UII — Animation`. The document is the user's JSON:

- `UII_ANIMATION_PATH`, if set
- otherwise `$XDG_DATA_HOME/uii/animation.json`
- otherwise `~/.local/share/uii/animation.json`

An empty tab does not write a file. A document that fails to parse is left in place and copied to `*.json.bad`. `UII_ANIMATION_FRAMES` closes the window after that many frames.
