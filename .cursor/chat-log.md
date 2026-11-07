Build only the Assets and Components dock in https://github.com/Banter-Foundation/UII. Native Rust app on Banter’s imgui (https://github.com/Banter-Foundation/Banter). Assets and Components are a collapsible dock like Unreal’s content drawer, not top tabs. Do not implement Design, Animation, Function, Inspect, import/export, or the Cursor plugin. Branch: assets-dock. Open a PR.

## Summary

UII was an empty repository. The dock is a bottom handle plus an optional body, with Assets and Components side by side inside that body. Layout, library rules, and persistence are tested without OpenGL. The stage above the handle is only a Dear ImGui dockspace so later editor surfaces can dock there. The library starts empty. `cargo test --test dock` passed (19). A three-frame Xvfb run wrote a stage dockspace in `uii-dock.ini` and exited cleanly.
