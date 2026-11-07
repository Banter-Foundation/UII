Build only the Function tab in https://github.com/Banter-Foundation/UII. Native Rust desktop app on Banter’s imgui (https://github.com/Banter-Foundation/Banter). Function is a top-level tab for how and where things work and connect. Do not implement Design, Animation, the assets dock, Inspect, import/export, or the Cursor plugin. Branch: function-tab. Open a PR.

## Summary

Function is a behavior graph in `crates/uii-function`. A function has a kind (trigger, action, or value), a place string for where it works, a name, and a detail for how it works. Wires run from an output port to an input port. The window follows Banter: GLFW, OpenGL 3.3 core, glow, dear-imgui-rs 0.22 with docking enabled before the first frame. The graph starts empty and is stored in `function.graph`. Design, Animation, the assets dock, Inspect, import/export, and the Cursor plugin are left for the other branches.
