## 2026-11-07 18:32:10 +00:00

Build only the Cursor plugin for https://github.com/Banter-Foundation/UII. Cursor must be able to plug in, read the current layout (structure, styles, and what each region is for), and understand Owl’s UI intent. The app is a native Rust desktop app, not a web app. Do not build the editor tabs, dock, or exporters. Branch: cursor-plugin. Open a PR.

### Summary

The plugin is `cursor-plugin/`. The native app publishes a `LayoutSnapshot` on a loopback socket and in `layout.json`. Cursor's MCP server `uii-layout` reads that snapshot with `read_layout`, `read_intent`, and `read_region`. HTML and CSS stay on the intent as Owl's description. They are not a web UI. The running app is still Banter's imgui stack. Tabs, the dock, and exporters were left for the other slices.
