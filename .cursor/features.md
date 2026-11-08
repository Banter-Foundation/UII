# Features

[*] Native window on Banter's Dear ImGui stack: GLFW, OpenGL 3.3, glow, dear-imgui-rs 0.22, dear-imgui-glow 0.22. Docking is enabled before the first frame.

[*] Empty document on launch.

[*] Design and Animation are the only top tabs. Design is the layout canvas. Animation plays position and size on that canvas without rewriting the design rects.

[*] Canvas tools: Select, Frame, Text, Rectangle, Ellipse. Select, move, and resize. Right-drag pans.

[*] Inspect is a tool. With a selection it shows type, size, and color.

[*] Assets and Components stay in the collapsible Content drawer. The drawer starts collapsed.

[*] HTML and CSS stay secondary. The canvas rewrites them, and Apply markup writes them back.

[*] Import and export: HTML/CSS, React, Vue, Svelte, Angular, Banter imgui, Raven imgui, Dear ImGui C++, egui, Slint, Qt Widgets, WinUI, SwiftUI, GTK, Jetpack Compose.

[*] Cursor socket on 127.0.0.1: `uii get`, `uii apply`, and `uii mcp` with `get_design` and `apply_layout`.

[ ] Full CSS, real font metrics, decoded images, undo, persistence, and handwritten parsers for egui, Qt, SwiftUI, GTK, Compose, and Dear ImGui C++.
