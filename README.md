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

The top of the window has three tabs:

- **Design** is the layout canvas. Layers stay on the left and properties stay on the right: position, size, fill, and type. Text uses its font size. Rectangles and frames have a fill and a corner radius. Frames clip their children and can nest.
- **Animation** is the same canvas with a playhead. Motion is position and size, with linear, ease, and hold curves. Playing it does not write the sampled pose back over the design.
- **Function** is the behavior graph. Save writes `function.graph` under the UII data directory.

The tools on Design and Animation are Select, Frame, Text, Rectangle, Ellipse, and Inspect. Inspect is a tool. When an object is selected it shows that object's type, size, and color. It is not a tab. The properties column shows the same selection while you are designing, including position and fill.

Shift-click adds to the selection. Dragging on empty canvas selects everything the rectangle covers. Moving snaps edges and centers to other objects. Undo is Ctrl+Z. The full shortcut list is in the properties column: undo, redo, copy, paste, duplicate, delete, select all, group, zoom in, zoom out, and fit.

**Assets** and **Components** are in the Content drawer at the bottom. It starts collapsed. Ctrl+Space toggles it. Pin keeps it open; otherwise a click outside closes it. Add an asset by path and place it, or save the selection as a component and insert it later.

**HTML / CSS** opens the markup for the current canvas. Apply writes it back onto the canvas. It starts closed. Leave it closed while you draw.

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

Raven is [oscar-soirey/Raven](https://github.com/oscar-soirey/Raven), commit `7f4411c376cec8fad2d071d63696b8444ec2640d`. It is a C GUI library, not a Dear ImGui fork and not a crate inside Banter. The editor window stays on Banter's Dear ImGui (`dear-imgui-rs` 0.22 and `dear-imgui-glow` 0.22). The Raven export is a separate file of that same imgui, with functions added for export and import: `imgui_dock_area`, `imgui_header_bar`, `imgui_island`, and `imgui_vbox`, plus label, button, image, and table. Those functions call Dear ImGui's dockspace, windows, child windows, and tables (`include/raven/raven_shell.h`, `include/raven/raven_controls.h`). The window does not compile that file and does not link `libraven`.

Where the toolkit has no dock widget, the export still carries the layout and names the closest equivalent:

- egui has no dockspace. The note uses `egui_dock` (`DockState`, `split_above`).
- Slint has no dock widget. The file is a `Window` with `StandardTableView` and rectangles.
- SwiftUI uses `NavigationSplitView` and `Table`.
- GTK 4 uses `gtk_paned_new` and `gtk_grid_attach`.
- Compose uses `Scaffold`.
- Dear ImGui C++ uses `DockBuilderSplitNode`, `DockSpace`, `BeginChild`, and `BeginTable`.
- WinUI uses a `DockPanel`.

HTML regions become docks, HTML tables become tables, and nested boxes become child windows. Text, buttons, images, and frames are included, and so are rectangles and ellipses drawn on the canvas.

Banter import reads the Dear ImGui calls that export emits. Raven import reads the functions added to the exported imgui (`imgui_dock_area`, `imgui_vbox`, `imgui_label`, `imgui_button`, `imgui_table`, `imgui_image`). Slint and WinUI imports read their own files. The other native imports read the `uii_dock`, `uii_frame`, `uii_child`, `uii_text`, `uii_button`, `uii_image`, `uii_table`, `uii_rectangle`, and `uii_ellipse` calls that UII writes next to the toolkit notes. They do not parse arbitrary egui, Qt, Swift, GTK, or Compose source.

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

The Cursor plugin in `cursor-plugin/` is separate. The window publishes the open layout on loopback and writes `layout.json` for `uii-cursor`. The tools there are `read_layout`, `read_intent`, and `read_region`. HTML and CSS in that snapshot are the description of the layout. The plugin does not serve a page.

## What is missing

- CSS is a layout subset: size, position, color, font size, border radius, and simple selectors. No flex, grid, media queries, or generated content.
- HTML import still estimates text width with a fixed factor. The canvas draws text at the node's font size.
- Images are paths. The canvas does not decode the file.
- The design itself has no save file. The function graph does. There is no collaboration.
- Animation keys are x, y, width, and height.
- Imports of egui, Qt Widgets, SwiftUI, GTK, Compose, and Dear ImGui C++ accept UII's emitted calls, not a handwritten app.
- Raven's Vulkan renderer is not linked. DockArea, the header bar, the island, and VBox are functions in the exported imgui file. The editor window does not call them. Rectangles and ellipses stay canvas shapes.
