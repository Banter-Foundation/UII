---
name: read-uii-layout
description: >
  Read the current UII layout from the native desktop app: structure, styles,
  what each region is for, and Owl's UI intent (HTML and CSS as the description,
  not a website). Use when editing or explaining a UII design, when Owl asks
  what a region is for, or before changing layout or styles.
---

# Read the UII layout

UII is a native Rust desktop app. It uses Banter's imgui stack: `dear-imgui-rs` 0.22, `dear-imgui-glow` 0.22, GLFW, and OpenGL 3.3 (`https://github.com/Banter-Foundation/Banter`). Do not treat the open design as a website and do not propose a browser UI for UII itself.

HTML and CSS are how Owl describes a layout. They travel on the intent. They are not a web app.

## Plug in

The `uii-layout` MCP server (this plugin) is the plug. Cursor may show the tools as `plugin-uii-cursor-uii-layout`.

1. Call `read_layout` for the whole document.
2. Call `read_intent` for Owl's summary, HTML, CSS, and each region's purpose.
3. Call `read_region` with an id when you need one region, its style, its children, and its ancestors.

If a tool returns `no layout published`, say so. The desktop app has not published the open document yet. Do not invent a layout.

## How to read the result

- **Structure** is the region tree (`root` and `children`), root first.
- **Styles** are `style.declarations` in source order (`property` / `value`). Do not invent tokens or computed web styles.
- **What a region is for** is the `purpose` string. An empty purpose means it was not set. Say that. Do not guess.
- **Owl's UI intent** is `intent.summary`, plus `intent.html` and `intent.css` unchanged. Quote them. Do not rewrite them into a different product.

The `reading` field is a plain walk of the same data. Use it to explain the layout, then use the JSON for exact styles and markup.

## Banter shell ids

When a region id is one of Banter's shell names and its `purpose` is empty, this is what that shell uses the region for. A non-empty `purpose` wins. Any other id has no glossary entry.

| id | What Banter's shell uses it for |
| --- | --- |
| `title-bar` | Window caption. Client-drawn on Windows; zero height on Linux so the system title bar is used. |
| `rail` | Server list. |
| `sidebar` | Friends or channels. |
| `stage` | Main content. Chat, and voice on the stage while in a voice channel. |
| `members` | Member list. Optional. |
| `header` | Header over the stage and the member list. |
| `voice-panel` | Voice controls at the bottom of the sidebar. |
| `user-panel` | Signed-in user, under the voice panel. |
| `composer` | Message composer at the bottom of the stage in a text channel. |

Those names come from Banter `crates/banter/src/layout.rs` and `docs/ui-theme-layout.md`. They are not a limit on what a UII document may contain.

## Out of scope

This plugin does not edit the canvas, draw editor tabs, build the dock, or export a layout.
