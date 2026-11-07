## 2026-11-07 18:44:57 +0000 [Cursor Agent] — Animation tab

Added the Animation tab as `uii-animation`, with the standalone host `uii-animation`, because the shell is not on `main`. Clips, property tracks, keyframes, easing, playback, the timeline, and the motion preview are in that crate. The host saves the user's JSON and does not seed a document.

## 2026-11-07 18:46:04 +0000 [Cursor Agent] — Stop imgui.ini writes

The headless Animation draw test no longer saves `imgui.ini` into the crate, and `imgui.ini` is ignored.
