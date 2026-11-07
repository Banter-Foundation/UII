## 2026-11-07 18:53:40 +0000 [Cursor Agent] — Function tab

The Function tab is a behavior graph: a function says how something works, its place says where, and a wire runs from an output port to an input port. The window uses Banter's Dear ImGui stack (GLFW, OpenGL 3.3 core, glow, dear-imgui-rs 0.22 with docking enabled before the first frame). The graph starts empty and is stored in `function.graph`.

- New: `crates/uii-function` graph, editor, file format, and the Function window (`uii`).
- Design, Animation, the assets dock, Inspect, import/export, and the Cursor plugin are not in this branch.
