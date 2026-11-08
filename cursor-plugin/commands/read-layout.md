---
name: read-layout
description: Read the current UII layout, styles, region purposes, and Owl's UI intent.
---

# Read the current layout

UII is a native Rust desktop app, not a web app. HTML and CSS on the intent are Owl's description of the layout.

1. Call `read_layout` on the `uii-layout` MCP server.
2. Call `read_intent`.
3. Report, in this order:
   - Owl's intent (summary, then the HTML, then the CSS, unchanged).
   - The region tree. For each region: id, name, purpose, and style declarations.
4. If a tool returns `no layout published`, stop and say the desktop app has not published a layout. Do not invent one.
