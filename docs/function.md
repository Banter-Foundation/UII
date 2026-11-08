# Function tab

The Function tab is the behavior graph. A function says how something works. Its place string says where it works. A connection runs from an output port to an input port.

This crate does not draw Design, Animation, the assets dock, Inspect, import/export, or the Cursor plugin. The place string is the join to a layout node: it is the node's `data-uii-name`. An empty place means the function is not placed.

## Crate

Workspace member `crates/uii-function`, package `uii-function`.

- `cargo test -p uii-function` builds the graph, the editor, and the file format. It does not link GLFW.
- The Function tab is drawn inside the UII window (`cargo run -p uii`). This crate does not ship a second `uii` binary.

Public types: `Graph`, `Function`, `FunctionId`, `FunctionKind`, `Port`, `PortId`, `Direction`, `Connection`, `ConnectionId`, `EditError`, `Editor`, `Camera`, `Hit`, `Rect`. File helpers: `to_text`, `from_text`, `default_graph_path`, `StoreError`.

With the `desktop` feature the crate also exports `Session`, `draw_function_panel`, `draw_function_tab`, and `run`. The shell calls `draw_function_panel`.

## Graph

`FunctionKind` is `Trigger`, `Action`, or `Value`. The file words are `trigger`, `action`, and `value`.

A new function gets default ports, which `set_kind` does not change:

| Kind | Ports |
| --- | --- |
| Trigger | output `when` |
| Action | input `run`, output `then` |
| Value | output `value` |

Ids start at 1. Deleting a function or a wire leaves a tombstone, and creating another appends, so an id is not reused while the graph is open. Loading sizes the slots to the highest id in the file. A deleted highest id is reused after save and load, because it is absent from the file. A gap below a later id is kept.

`connect` rejects, in order: a missing function, a missing port, a port used in the wrong direction (the source must be an output and the destination an input), a wire whose ends are the same function, an input that already has a wire, and a wire that would loop. One output may feed many inputs. `disconnect` of a missing wire is `MissingConnection`. Deleting a function or a port drops the wires that touch it.

`places` is the first-seen non-empty place strings, in function id order. `at_place("")` is the functions with an empty place. Names, places, and details are stored as typed, including surrounding spaces. An empty name is shown as "Untitled" in the window only.

## File

Header `uii-function 1`. Then each live function in id order, immediately followed by its live ports in id order, then the live wires in id order. Numbers are printed with trailing zeros removed (`10.5`, `20`, `240`). Strings are always quoted, with `\\`, `\"`, `\n`, `\r`, and `\t` escaped. The writer uses `\n` and no blank lines. The reader accepts blank lines and CRLF.

```
uii-function 1
fn 1 trigger 10.5 20 "Home" "Save" "click"
port 1 1 out "when"
fn 2 action 240 20 "Settings" "Open" "open Settings"
port 2 1 in "run"
port 2 2 out "then"
wire 1 1 1 2 1
```

`fn` fields are id, kind, x, y, place, name, detail. `wire` fields are id, from function, from port, to function, to port. A bad header displays as `expected "uii-function 1"`. Other errors name the line. A loop error contains `loop`.

`default_graph_path` is `data_home/UII/function.graph`, else `home/.local/share/UII/function.graph`, else `function.graph`.

## Editor

Canvas coordinates are pixels inside the canvas child. The camera pan is the world point at the canvas origin. Zoom is canvas pixels per world pixel, clamped to 0.25..=4. Zoom keeps the world point under the cursor. If the zoom does not change because of the clamp, the pan is left alone.

Hit testing walks live functions from the highest id to the lowest. On a node, ports are tested before the body. A port hit is a circle of radius `PORT_RADIUS + PORT_SLOP` (5 + 2). The body is half-open: `px >= x && py >= y && px < x + w && py < y + h`. Node width is 200. Height is the header (28) plus `max(inputs, outputs, 1)` rows of 22, plus 8 pixels of padding. An input sits on the left edge. An output sits on the right edge.

Primary on an output starts a wire and selects that function. Primary on an input selects and does not drag. Primary on the body selects and drags. Primary on empty canvas clears the selection. Releasing a wire on an input connects it. A miss cancels the wire and does not set an error. A rejected wire sets `last_error`. Middle-drag, or primary with Alt, pans and does not change the selection. The secondary button is ignored.

## Window

`run` opens one 1440×900 window titled UII, vsync on, docking enabled. `draw_function_tab` fills the viewport with one Dear ImGui window and a tab bar (`uii-top`) that contains only Function. It returns true when the user presses Save or Ctrl+S. The host then calls `Session::save`. On exit, `save_if_changed` writes only when the text differs from the baseline and the file was readable. A parse or IO failure opens an empty editor, shows the error, and will not overwrite that file until Save or Ctrl+S.

The tab is three panes: Where (the place list), the canvas, and How (name, place, detail, kind, ports, and the wires in and out). Clicking a row in Where selects that function and moves the view so the node sits near the canvas origin, keeping the zoom. Dock layout is stored in `uii-dock.ini` next to the graph file.
