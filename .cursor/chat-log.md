## 2026-11-07 18:26:59 +0000

Build only the Inspect tool in https://github.com/Banter-Foundation/UII. Native Rust app on Banter’s imgui (https://github.com/Banter-Foundation/Banter). Inspect is a tool, not a tab. It shows the selected item’s size, color, and type. Do not implement the other tabs, the assets dock, import/export, or the Cursor plugin. Branch: inspect-tool. Open a PR.

### Summary

UII's repo was only a README. Banter's desktop stack is GLFW 0.63, OpenGL 3.3, glow 0.18, and dear-imgui-rs 0.22 with docking enabled before the first frame. Inspect is implemented as that tool: a label-sized tool control, then size, color, and type of `InspectState`'s selection. No tab bar, assets dock, import/export, or Cursor plugin. The host does not create items, so the window opens on "Nothing selected" until another part of UII calls `set_selection`.
