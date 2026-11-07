# UII

UII is a native layout editor. This branch is the window shell and the Design canvas. It is a Rust desktop app on the same Dear ImGui stack Banter uses (`dear-imgui-rs` 0.22, `dear-imgui-glow` 0.22, `glow` 0.18, GLFW 0.63, OpenGL 3.3). It is not a website.

The layout is HTML and CSS. The screen you work on is the canvas. HTML and CSS are a secondary panel.

## Install

Rust has to be new enough for `dear-imgui-rs` 0.22. That crate's build uses Cargo edition 2024. Rust 1.83 cannot fetch it. This tree was built with Rust 1.99.

On Linux, install a C++ compiler and the libraries GLFW needs:

```bash
sudo apt install build-essential cmake pkg-config \
  libgl1-mesa-dev libx11-dev libxrandr-dev libxinerama-dev \
  libxcursor-dev libxi-dev libxext-dev libwayland-dev libxkbcommon-dev
```

If `c++` is Clang and the Dear ImGui build stops on a missing `<climits>`, build with `CXX=g++`. If the link step cannot find `libstdc++`, add the GCC library directory to `LIBRARY_PATH`.

GLFW is built by the `glfw` crate. A system GLFW package is not required.

## Run

```bash
cargo run -p uii
```

The window opens on an empty document.

The top of the window has three tabs:

- **Design** is the layout canvas. Select, move, resize, and draw frames, text, rectangles, and ellipses. Right-drag pans. Delete removes the selection. Layers sit on the left.
- **Animation** is an empty placeholder. Another agent owns that tab.
- **Function** is an empty placeholder. It is not named Prototype. Another agent owns that tab.

**HTML / CSS** opens the markup for the current canvas. It starts closed. Apply writes it back onto the canvas.

`crates/uii/src/content_dock.rs` and `crates/uii/src/inspect.rs` are hooks the shell calls every frame. They draw nothing. The assets dock and Inspect belong to other agents. `crates/uii/src/animation.rs` and `crates/uii/src/function.rs` are the empty tab bodies.

## What this branch does not include

Import and export of the other formats, the Cursor socket, animation playback, Function connections, the Assets and Components dock, and the Inspect tool are not built here.
