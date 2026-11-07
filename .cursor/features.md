# Features

[*] Native window on Banter's Dear ImGui stack: GLFW, OpenGL 3.3, glow, dear-imgui-rs 0.22, dear-imgui-glow 0.22. Docking is enabled before the first frame.

[*] Empty document on launch.

[*] Top tabs are Design, Animation, and Function. Design is the layout canvas. Animation and Function are empty placeholders. The Function tab is not named Prototype.

[*] Design canvas: Select, Frame, Text, Rectangle, Ellipse. Select, move, and resize. Right-drag pans. Layers list the document.

[*] HTML and CSS are a secondary panel. The canvas rewrites them, and Apply markup writes them back.

[*] `content_dock.rs` and `inspect.rs` are empty hooks the shell calls. They are not a dock and not an Inspect tool.

[ ] Animation playback, Function connections, the Assets and Components dock, Inspect, multi-format import and export, and the Cursor plugin. Other agents own those.
