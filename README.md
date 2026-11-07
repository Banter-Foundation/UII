# UII

UII is a native layout editor for Owl. The window is a Rust desktop app on the same Dear ImGui stack Banter uses (`dear-imgui-rs` 0.22, `dear-imgui-glow` 0.22, `glow` 0.18, GLFW 0.63, OpenGL 3.3). It is not a website and it does not serve HTTP.

The layout itself is HTML and CSS, because that is how a layout is described. The screen you work on is the canvas: select, move, resize, and draw. HTML and CSS are there when you want them, off to the side.

## Install

You need Rust new enough for `dear-imgui-rs` 0.22. That crate's build uses Cargo edition 2024. Rust 1.83 cannot fetch it. This tree was built with Rust 1.99.

On Linux, install a C++ compiler and the libraries GLFW needs:

```bash
sudo apt install build-essential cmake pkg-config \
  libgl1-mesa-dev libx11-dev libxrandr-dev libxinerama-dev \
  libxcursor-dev libxi-dev libxext-dev libwayland-dev libxkbcommon-dev
```

If `c++` is Clang and the Dear ImGui build stops on a missing `<climits>`, build with `CXX=g++`. If the link step cannot find `libstdc++`, add the GCC library directory to `LIBRARY_PATH` (on the machine used to build this tree, that was `/usr/lib/gcc/x86_64-linux-gnu/13`).

GLFW is built by the `glfw` crate. A system GLFW package is not required.

## Run

```bash
cargo run -p uii
```

The window opens on an empty document. Nothing is seeded.

The top of the window has two tabs:

- **Design** is the layout canvas.
- **Animation** is the same canvas with a playhead. Motion is position and size. Playing it does not write the sampled pose back over the design.

The tools on those tabs are Select, Frame, Text, Rectangle, Ellipse, and Inspect. Inspect is a tool. When an object is selected it shows that object's type, size, and color. It is not a tab.

**Assets** and **Components** are in the Content drawer at the bottom. It starts collapsed. Open it, then pick Assets or Components. Add an asset by path and place it, or save the selection as a component and insert it later.

**HTML / CSS** opens the markup for the current canvas. Apply writes it back onto the canvas. Leave it closed while you draw.

Drag with the right mouse button to pan the canvas. Delete removes the selection.

## Import and export

Import and Export are buttons on the tool row. Every format can be imported and exported:

| Id | What it is |
| --- | --- |
| `html-css` | HTML and CSS |
| `react` | React function component, JSX and CSS |
| `vue` | Vue single-file component |
| `svelte` | Svelte component |
| `angular` | Angular component |
| `banter-imgui` | Rust on `dear-imgui-rs` 0.22, the same crate the window uses |
| `raven-imgui` | Generated Rust in a `mod raven` that calls `dear-imgui-rs` |
| `imgui-cpp` | Dear ImGui C++ notes plus the layout calls |
| `egui` | egui notes plus the layout calls |
| `slint` | Slint |
| `qt-widgets` | Qt Widgets |
| `winui` | WinUI |
| `swiftui` | SwiftUI |
| `gtk` | GTK 4 |
| `compose` | Jetpack Compose |

Qt is **Qt Widgets**, not QML. The export says so. Docking is `QDockWidget` on a `QMainWindow`, tables are `QTableWidget`, and nested boxes are child `QWidget`s.

Raven is [oscar-soirey/Raven](https://github.com/oscar-soirey/Raven), commit `7f4411c376cec8fad2d071d63696b8444ec2640d`. It is a C GUI library (Vulkan, its own windows), not a crate inside Banter and not a Dear ImGui fork. The custom export follows that source: a Raven window loaded with `window_from_xml`, using `DockArea` and `Slot.Dock`, `VBox`, `Table`, `Button`, `Label`, and `Image` (`include/raven/raven_shell.h`, `include/raven/raven_controls.h`, `bindings/rust`). The same file then draws that layout with `dear-imgui-rs` 0.22 and `dear-imgui-glow` 0.22, the crates and docking branch Banter uses. The running editor links those crates only. It does not link `libraven`.

Where the toolkit has no dock widget, the export still carries the layout and names the closest equivalent:

- egui has no dockspace. The note uses `egui_dock` (`DockState`, `split_above`).
- Slint has no dock widget. The file is a `Window` with `StandardTableView` and rectangles.
- SwiftUI uses `NavigationSplitView` and `Table`.
- GTK 4 uses `gtk_paned_new` and `gtk_grid_attach`.
- Compose uses `Scaffold`.
- Dear ImGui C++ uses `DockBuilderSplitNode`, `DockSpace`, `BeginChild`, and `BeginTable`.
- WinUI uses a `DockPanel`.

HTML regions become docks, HTML tables become tables, and nested boxes become child windows. Text, buttons, images, and frames are included, and so are rectangles and ellipses drawn on the canvas.

Banter import reads the Dear ImGui calls that export emits. Raven import reads the Raven XML (`DockArea`, `VBox`, `Table`, `Button`, `Label`, `Image`). Slint and WinUI imports read their own files. The other native imports read the `uii_dock`, `uii_frame`, `uii_child`, `uii_text`, `uii_button`, `uii_image`, `uii_table`, `uii_rectangle`, and `uii_ellipse` calls that UII writes next to the toolkit notes. They do not parse arbitrary egui, Qt, Swift, GTK, or Compose source.

## Cursor

The running window listens on `127.0.0.1`. The port is `47321`, or the next free port up to `47328`. It writes the port to `$XDG_RUNTIME_DIR/uii.port`, or `/tmp/uii.port` when that directory is unset. This is a line of JSON in and a line of JSON out. It is not a web server.

With the window open:

```bash
uii get
uii apply <<'EOF'
{"op":"apply","html":"<header class=\"bar\">Title</header>","css":"header.bar { height: 40px; }"}
EOF
```

`get` returns the HTML, the CSS, and a `regions` list. Each region has a purpose, such as `Top dock for bar` or `Text Title`. `apply` replaces the open design.

`uii mcp` speaks MCP on stdin and stdout (Content-Length frames) and forwards to that socket. The tools are `get_design` and `apply_layout`. `.cursor/mcp.json` starts it with `cargo run -p uii --quiet -- -- mcp`. Start the window first. `get` and `apply` do nothing useful until it is open.

## What is missing

- CSS is a layout subset: size, position, color, font size, border radius, and simple selectors. No flex, grid, media queries, or generated content.
- Text measurement is a fixed width factor, not a real font.
- Images are paths. The canvas does not decode the file.
- There is no undo, no save file, and no collaboration.
- Animation keys are only x, y, width, and height.
- Imports of egui, Qt Widgets, SwiftUI, GTK, Compose, and Dear ImGui C++ accept UII's emitted calls, not a handwritten app.
- Raven's own renderer is not linked. The export writes Raven's XML and a Dear ImGui 0.22 drawing of the same layout. Rectangles and ellipses are canvas shapes; Raven's headers do not name those two widgets.
