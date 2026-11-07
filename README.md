# UII

UII is a native desktop app. This branch is the Function tab: how something works, where it works, and how those functions connect.

Design, Animation, the assets dock, Inspect, import/export, and the Cursor plugin are not in this branch.

## Build

Rust 1.99.1 is pinned in `rust-toolchain.toml`. The window uses Banter's Dear ImGui stack: GLFW 0.63, OpenGL 3.3 core, glow 0.18, and dear-imgui-rs 0.22 on the docking branch. Docking is turned on before the first frame.

You need a C++ compiler, the GLFW headers and libraries, and an OpenGL driver. On Linux, GLFW is built from source, so `cmake`, `pkg-config`, X11, Wayland, and `libxkbcommon` headers are required. If the linker is clang's `lld` and it reports `unable to find library -lstdc++`, link with g++:

```
CXX=g++ RUSTFLAGS="-C linker=g++" cargo run -p uii-function --features desktop --bin uii
```

The graph model does not need a display:

```
cargo test -p uii-function
```

## Run

```
cargo run -p uii-function --features desktop --bin uii
```

An optional path argument is the graph file. With no argument the file is `$XDG_DATA_HOME/UII/function.graph`, or `~/.local/share/UII/function.graph` when `XDG_DATA_HOME` is unset. The window is titled UII and opens on the Function tab. The graph starts empty. There is no sample graph.

New trigger, New action, and New value add a function. Double-click empty canvas to add a trigger. Drag from an output dot to an input dot. Save, or Ctrl+S, writes the file. Closing the window writes it when it has changed. A file that could not be read is left untouched until you save on purpose.

The file format and the editor API are described in `docs/function.md`.
