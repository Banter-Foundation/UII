# Session memory

- 2026-11-07: Assets and Components dock on branch `assets-dock`. Native GLFW + OpenGL 3.3 app using Dear ImGui docking (`dear-imgui-rs` 0.22, `dear-imgui-glow` 0.22), the same stack as Banter. Drawer geometry and the empty-library rules live in `src/drawer.rs`, `src/library.rs`, and `src/session.rs`. Tests are `tests/dock.rs`.
- Assets and Components are one bottom drawer (handle plus body). They are not top tabs. The stage above the handle is a dockspace (`Stage` window, id `stage-dock`). Other editor surfaces belong in `src/stage.rs` `draw_stage_guest`. This branch does not implement Design, Animation, Function, Inspect, import/export, or a Cursor plugin.
- A new library has no seeded assets or components. User-created records persist as `library.json` and `dock.json`. `UII_DATA` overrides the data directory.
- Drawer starts collapsed. Ctrl+Space toggles it unless a text field wants input. Pin keeps it in the layout. An unpinned drawer closes on a click outside it. Drag the top edge to resize.
- Asset names are unique across the whole asset tree. Component names are unique across components. The same text may exist once as an asset and once as a component.
