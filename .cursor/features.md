# Features

[*] Native window on Banter's Dear ImGui stack: GLFW, OpenGL 3.3, glow, dear-imgui-rs 0.22, dear-imgui-glow 0.22. Docking is enabled before the first frame.

[*] Empty document on launch.

[*] Design, Animation, and Function are the top tabs. Design is the layout canvas. Animation plays position and size on that canvas without rewriting the design rects. Function is the behavior graph.

[*] Layers and properties stay visible on Design. Properties edit position, size, fill, and type, plus font size and corner radius.

[*] Canvas tools: Select, Frame, Text, Rectangle, Ellipse. Select, move, resize, multi-select, and snap to edges and centers. Right-drag pans. Frames clip their children and can nest.

[*] Undo, redo, copy, paste, duplicate, delete, select all, group, zoom in, zoom out, and fit. The shortcut list is in the properties column.

[*] Inspect is a tool. With a selection it shows type, size, and color.

[*] Assets and Components stay in the collapsible Content drawer. The drawer starts collapsed. Ctrl+Space toggles it. Pin keeps it open. An unpinned drawer closes on a click outside it.

[*] HTML and CSS stay secondary, behind a closed panel. The canvas rewrites them, and Apply markup writes them back.

[*] Import and export: HTML/CSS, React, Vue, Svelte, Angular, Banter imgui, Raven imgui, Dear ImGui C++, egui, Slint, Qt Widgets, WinUI, SwiftUI, GTK, Jetpack Compose.

[*] Cursor socket on 127.0.0.1: `uii get`, `uii apply`, and `uii mcp` with `get_design` and `apply_layout`. The Cursor plugin `uii-cursor` reads the layout the window publishes.

[ ] Full CSS, decoded images, a design save file, and handwritten parsers for egui, Qt, SwiftUI, GTK, Compose, and Dear ImGui C++.
