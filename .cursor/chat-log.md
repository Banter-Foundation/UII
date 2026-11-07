Build only the Animation tab in https://github.com/Banter-Foundation/UII. Native Rust desktop app on Banter’s imgui (https://github.com/Banter-Foundation/Banter). Animation is a top-level tab for motion on the layout. Do not implement Design, Function, the assets dock, Inspect, import/export, or the Cursor plugin. If the shell is not on the default branch yet, add the Animation tab in your own module so it can be merged. Branch: animation-tab. Open a PR.

## Summary

The shell is not on `main`. Animation is `crates/uii-animation` plus the host `crates/uii-animation-host`. Tests in `crates/uii-animation/tests/motion.rs` define the motion API. The host was exercised on a real window: add a clip, add a target, capture keys, edit a value, scrub, and play. The preview keeps the rest outline and moves the posed box.
