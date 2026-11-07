## 2026-11-07 12:58:17 -0600 [Cursor Agent] — Native UII editor

First version of the native editor. The window uses Banter's Dear ImGui stack. Design and Animation are the top tabs, Inspect is a tool, and Assets and Components sit in the Content drawer. HTML and CSS are the layout model and stay off the main canvas. Import and export cover the fifteen requested formats. `uii get`, `uii apply`, and `uii mcp` talk to the open window on 127.0.0.1.

## 2026-11-07 13:14:02 -0600 [Cursor Agent] — Raven is oscar-soirey/Raven

The custom imgui export now follows oscar-soirey/Raven at commit 7f4411c376cec8fad2d071d63696b8444ec2640d: `window_from_xml` with DockArea, VBox, Table, Button, Label, and Image. The Dear ImGui half of that file stays on dear-imgui-rs 0.22 and dear-imgui-glow 0.22, the crates Banter uses. libraven is not linked into the designer.

## 2026-11-07 14:02:01 -0600 [Cursor Agent] — Raven features on Dear ImGui

The custom export is Banter's Dear ImGui 0.22 with Raven's shell added onto `Ui`: `dock_area`, `header_bar`, `island`, and `vbox`, plus label, button, image, and table. Those methods are implemented with Dear ImGui's dockspace, windows, and tables. libraven is not linked.

## 2026-11-07 14:07:09 -0600 [Cursor Agent] — Raven stays in export and import

Raven's DockArea, header bar, island, and VBox are functions in the exported Dear ImGui file. Import reads those functions back onto the canvas. The editor window does not compile or call them.
