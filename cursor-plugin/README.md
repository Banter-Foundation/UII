# UII Cursor plugin

Cursor plugs into the native UII desktop app and reads the layout that is open right now.

The reply has four parts:

- **Structure** — the region tree.
- **Styles** — each region's declarations, in source order.
- **Purpose** — what that region is for.
- **Intent** — Owl's summary, plus the HTML and CSS used to describe the layout.

HTML and CSS are the description Owl writes. They are not a web UI. UII itself stays a native Rust app on Banter's imgui stack (`dear-imgui-rs` 0.22, `dear-imgui-glow` 0.22, GLFW, OpenGL 3.3).

This plugin does not draw editor tabs, the dock, or exporters.

## Install

Add this repo as a Cursor plugin marketplace, or load `cursor-plugin/` as a local plugin. The manifest is `.cursor-plugin/plugin.json`. The first tool call compiles `uii-cursor` with Cargo, which needs to be on `PATH` (the same Rust toolchain Banter uses).

The MCP server is `uii-layout`. Its tools are `read_layout`, `read_intent`, and `read_region`.

## What the desktop app publishes

Call this from the native app when the open document changes. Loopback only.

```rust
let plug = uii_cursor::LayoutPlug::bind()?;
let root = uii_cursor::Region::new(id, name, purpose)?;
let intent = uii_cursor::UiIntent::new(summary, html, css);
let snapshot = uii_cursor::LayoutSnapshot::new(document_name, intent, root)?;
plug.publish_to(snapshot, &snapshot_path)?;
```

`publish_to` writes `layout.json` and `plug.addr` in that directory and serves the same snapshot on `127.0.0.1`. Cursor tries the live plug first. If the process is gone, it reads the file. If the process is up and has not published yet, Cursor reports `no layout published` instead of an older file.

Default file, same roots Banter uses for its data directory:

| Platform | Path |
| --- | --- |
| Windows | `%LOCALAPPDATA%\UII\layout.json` |
| macOS | `~/Library/Application Support/UII/layout.json` |
| Linux | `$XDG_DATA_HOME/UII/layout.json` or `~/.local/share/UII/layout.json` |

Override with `UII_LAYOUT_SNAPSHOT` (file) and `UII_PLUG_ADDR` (`127.0.0.1:port`).

## Wire

One JSON line each way, on the loopback socket:

```text
{"method":"layout"}
```

The response is the snapshot JSON, or `{"error":"no layout published"}`.

MCP frames on stdio use `Content-Length`, same as a language server.

## Tests

```text
cargo test --manifest-path cursor-plugin/Cargo.toml
```

License: MIT OR Apache-2.0, the same as the Banter client.
