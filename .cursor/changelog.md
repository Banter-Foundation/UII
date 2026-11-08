## 2026-11-07 18:26:59 +0000 [Owl] — Inspect tool

Inspect is a tool, not a tab. It shows the selected item's size, color, and type. The window starts with nothing selected.

- New: Inspect tool on GLFW, OpenGL 3.3, and Dear ImGui 0.22, the same stack as Banter.
- Size is width × height. Color is `#RRGGBB`, or `#RRGGBBAA` when alpha is not opaque. Type is the item's type name.
## 2026-11-07 18:32:10 +00:00 [Cursor Agent] — Cursor layout plugin

Cursor can plug into the native UII app and read the current layout: region structure, style declarations, what each region is for, and Owl's UI intent (summary plus the HTML and CSS used to describe it). The plug is loopback-only. Editor tabs, the dock, and exporters are not in this change.
## 2026-11-07 12:58:17 -0600 [Cursor Agent] — Native UII editor

First version of the native editor. The window uses Banter's Dear ImGui stack. Design and Animation are the top tabs, Inspect is a tool, and Assets and Components sit in the Content drawer. HTML and CSS are the layout model and stay off the main canvas. Import and export cover the fifteen requested formats. `uii get`, `uii apply`, and `uii mcp` talk to the open window on 127.0.0.1.

## 2026-11-07 13:14:02 -0600 [Cursor Agent] — Raven is oscar-soirey/Raven

The custom imgui export now follows oscar-soirey/Raven at commit 7f4411c376cec8fad2d071d63696b8444ec2640d: `window_from_xml` with DockArea, VBox, Table, Button, Label, and Image. The Dear ImGui half of that file stays on dear-imgui-rs 0.22 and dear-imgui-glow 0.22, the crates Banter uses. libraven is not linked into the designer.

## 2026-11-07 14:02:01 -0600 [Cursor Agent] — Raven features on Dear ImGui

The custom export is Banter's Dear ImGui 0.22 with Raven's shell added onto `Ui`: `dock_area`, `header_bar`, `island`, and `vbox`, plus label, button, image, and table. Those methods are implemented with Dear ImGui's dockspace, windows, and tables. libraven is not linked.

## 2026-11-07 14:07:09 -0600 [Cursor Agent] — Raven stays in export and import

Raven's DockArea, header bar, island, and VBox are functions in the exported Dear ImGui file. Import reads those functions back onto the canvas. The editor window does not compile or call them.

## 2026-11-08 02:41:01 +0000 [Cursor Agent] — Layout canvas, shortcuts, and folded tabs

Design keeps layers and properties on screen. Moving snaps and aligns, selection can be more than one object, and undo and redo cover those edits. Frames clip nested children. Text draws at its font size, and shapes keep fill and corner radius. Shortcuts for undo, redo, copy, paste, duplicate, delete, select all, group, zoom, and fit are listed in the properties column. HTML and CSS stay behind a closed panel. The Function tab and the Cursor layout plugin are in this tree. The content drawer toggles with Ctrl+Space and can be pinned.
