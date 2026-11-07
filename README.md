# UII

Import and export a UI document. The host application stays native Rust on Banter imgui (`dear-imgui-rs` 0.22, GLFW, OpenGL, docking enabled before the first frame). This repository is the call point. It does not open a window and it does not draw an editor.

```rust
let source = uii::export(uii::Format::DearImgui, &document)?;
let document = uii::import(uii::Format::HtmlCss, html)?;
uii::call(uii::Call::Export { format, document })?;
```

The `uii` binary is the same call on stdin and stdout:

```
uii import html-css < page.html
uii export dear-imgui < document.json
```

An unknown format exits 2. A conversion error exits 1.

## Formats

Both directions, and a round trip compares equal:

| Format | Name |
| --- | --- |
| HTML/CSS | `html-css` (`html`, `html/css`) |
| React | `react` |
| Vue | `vue` |
| Svelte | `svelte` |
| Angular | `angular` |
| Banter imgui | `banter-imgui` (`banter`) |
| Raven imgui | `custom-imgui` (`raven`, `raven-imgui`) |
| Dear ImGui C++ | `dear-imgui` (`imgui`, `dear-imgui-c++`) |
| egui | `egui` |
| Slint | `slint` |
| Qt | `qt` |
| WinUI | `winui` |
| SwiftUI | `swiftui` |
| GTK | `gtk` |
| Jetpack Compose | `jetpack-compose` (`compose`, `jetpack`) |

HTML `header`, `nav`, `main` / `section` / `article`, `aside`, and `footer` become dock regions (top, left, center, right, bottom). A `table` stays a table. Nested boxes become child windows where the target has them: Banter imgui, the Raven imgui file, and Dear ImGui C++. The other targets keep their own panels, stacks, and layouts and do not emit `BeginChild` or `ui.child_window`.

Code formats carry one `// uii` or `<!-- uii -->` record per node plus the toolkit's own calls. Import rebuilds the tree from the records, then rejects the file when a required call is missing (for example `ImGui::DockSpace`).

## Raven

Banter's repository has no Raven symbol. The custom imgui format follows the public C API in [oscar-soirey/Raven](https://github.com/oscar-soirey/Raven): `Rdock_area_create`, `Rdock_area_add_panel`, `R_DOCK_LEFT` / `RIGHT` / `TOP` / `BOTTOM` / `TAB` (there is no center side; the center panel is the first tab), `R_WIDGET_TABLE` with `Rtable_add_column`, `Rtable_add_row`, and `Rtable_set_cell`, plus `R_WIDGET_VBOX`, `R_WIDGET_HBOX`, `Rheader_bar_create`, and `R_WIDGET_ISLAND`. That file also emits Dear ImGui docking, tables, and child windows. The records are written once, in the Dear ImGui half.

Banter imgui text uses the `dear-imgui-rs` 0.22 calls Banter already links (`ui.dockspace`, `DockLayout`, `child_window`, `ui.table`) and `layout::Region` for the shell slugs. This crate does not depend on `dear-imgui-rs`.
