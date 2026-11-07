//! Draw the stage dockspace and the bottom Assets/Components drawer.

use dear_imgui_rs::{Condition, DockNodeFlags, MouseButton, StyleColor, Ui, WindowFlags};

use crate::drawer::{
    body_height_from_pointer, layout_assets_pane, layout_drawer, pointer_in_dock,
    pointer_on_resize_edge, shortcut_fires, should_dismiss_unpinned, DrawerCommand, DrawerLayout,
    Rect,
};
use crate::library::{AssetId, AssetKind, ComponentId, LibraryError};
use crate::session::{
    asset_rows, component_rows, expand_for_new_folder, folder_contents, insertion_folder,
    prune_browser, DockHost,
};
use crate::stage::draw_stage_guest;
use crate::theme;

const FIXED: WindowFlags = WindowFlags::NO_DECORATION
    .union(WindowFlags::NO_MOVE)
    .union(WindowFlags::NO_DOCKING)
    .union(WindowFlags::NO_BRING_TO_FRONT_ON_FOCUS)
    .union(WindowFlags::NO_FOCUS_ON_APPEARING)
    .union(WindowFlags::NO_SAVED_SETTINGS)
    .union(WindowFlags::NO_NAV_FOCUS);

pub(crate) struct FrameInput {
    pub width: f32,
    pub height: f32,
    pub ctrl: bool,
    pub space: bool,
}

pub(crate) fn draw_frame(ui: &Ui, host: &mut DockHost, frame: FrameInput) {
    if !(frame.width.is_finite() && frame.height.is_finite())
        || frame.width <= 0.0
        || frame.height <= 0.0
    {
        return;
    }
    let edge = host.shortcut_mut().ctrl_space(frame.ctrl, frame.space);
    if shortcut_fires(edge, ui.io().want_text_input()) {
        host.drawer.apply(DrawerCommand::Toggle);
        host.mark_dirty();
    }
    apply_pointer(ui, host, frame.height);
    let layout = layout_drawer(frame.width, frame.height, &host.drawer);
    draw_stage(ui, layout.stage);
    draw_stage_guest(ui);
    draw_handle(ui, host, &layout);
    if let Some(body) = layout.body {
        draw_body(ui, host, body);
    }
    host.persist();
}

fn apply_pointer(ui: &Ui, host: &mut DockHost, viewport_h: f32) {
    let pointer = ui.mouse_pos();
    let clicked = ui.is_mouse_clicked(MouseButton::Left);
    let down = ui.is_mouse_down(MouseButton::Left);
    if host.resizing() {
        if down {
            let height = body_height_from_pointer(viewport_h, pointer[1]);
            host.drawer.apply(DrawerCommand::SetBodyHeight(height));
            host.mark_dirty();
        } else {
            host.set_resizing(false);
        }
        return;
    }
    if !clicked {
        return;
    }
    let layout = layout_drawer(ui.io().display_size()[0], viewport_h, &host.drawer);
    if layout
        .body
        .is_some_and(|body| pointer_on_resize_edge(body, pointer[1]))
    {
        host.set_resizing(true);
        let height = body_height_from_pointer(viewport_h, pointer[1]);
        host.drawer.apply(DrawerCommand::SetBodyHeight(height));
        host.mark_dirty();
        return;
    }
    if should_dismiss_unpinned(
        &host.drawer,
        true,
        pointer_in_dock(&layout, pointer[0], pointer[1]),
    ) {
        host.drawer.apply(DrawerCommand::PointerLeftStage);
        host.mark_dirty();
    }
}

fn draw_stage(ui: &Ui, stage: Rect) {
    if stage.h <= 0.0 || stage.w <= 0.0 {
        return;
    }
    let _bg = ui.push_style_color(StyleColor::WindowBg, theme::STAGE_BG);
    ui.window("Stage")
        .flags(FIXED)
        .position([stage.x, stage.y], Condition::Always)
        .size([stage.w, stage.h], Condition::Always)
        .build(|| {
            let id = ui.get_id("stage-dock");
            ui.dock_space_raw(
                id,
                ui.content_region_avail(),
                DockNodeFlags::PASSTHRU_CENTRAL_NODE,
                None,
            );
        });
}

fn draw_handle(ui: &Ui, host: &mut DockHost, layout: &DrawerLayout) {
    let handle = layout.handle;
    if handle.w <= 0.0 || handle.h <= 0.0 {
        return;
    }
    let _bg = ui.push_style_color(StyleColor::WindowBg, theme::HANDLE_BG);
    ui.window("AssetsComponentsHandle")
        .flags(FIXED)
        .position([handle.x, handle.y], Condition::Always)
        .size([handle.w, handle.h], Condition::Always)
        .build(|| {
            let chevron = if host.drawer.open { "-" } else { "+" };
            let title = format!("{chevron}  Assets    Components");
            let avail = ui.content_region_avail_width();
            let pin_w = if host.drawer.open {
                crate::drawer::PIN_W.min(avail)
            } else {
                0.0
            };
            let gap = if host.drawer.open && pin_w > 0.0 {
                8.0
            } else {
                0.0
            };
            let toggle_w = (avail - pin_w - gap).max(1.0);
            let button_h = ui.content_region_avail()[1].max(1.0);
            if ui.button_with_size(title, [toggle_w, button_h]) {
                host.drawer.apply(DrawerCommand::Toggle);
                host.mark_dirty();
            }
            if host.drawer.open {
                ui.same_line();
                let pin = if host.drawer.pinned { "Pinned" } else { "Pin" };
                if ui.button_with_size(pin, [pin_w.max(1.0), button_h]) {
                    host.drawer.apply(DrawerCommand::TogglePin);
                    host.mark_dirty();
                }
            }
        });
}

fn draw_body(ui: &Ui, host: &mut DockHost, body: Rect) {
    ui.window("AssetsComponentsDrawer")
        .flags(FIXED)
        .position([body.x, body.y], Condition::Always)
        .size([body.w, body.h], Condition::Always)
        .build(|| {
            ui.separator();
            if !host.status.is_empty() {
                ui.text(host.status.as_str());
            }
            let avail = ui.content_region_avail();
            let (assets_w, components_w) = pane_widths(avail[0], host.drawer.assets_fraction);
            let height = avail[1].max(1.0);
            ui.child_window("AssetsPane")
                .size([assets_w, height])
                .flags(WindowFlags::NO_DOCKING)
                .build(ui, || draw_assets(ui, host));
            ui.same_line_with_spacing(0.0, crate::drawer::SPLITTER_W);
            ui.child_window("ComponentsPane")
                .size([components_w.max(1.0), height])
                .flags(WindowFlags::NO_DOCKING)
                .build(ui, || draw_components(ui, host));
        });
}

fn pane_widths(total: f32, fraction: f32) -> (f32, f32) {
    let gap = if total > crate::drawer::SPLITTER_W {
        crate::drawer::SPLITTER_W
    } else {
        0.0
    };
    let inner = (total - gap).max(0.0);
    let fraction = fraction.clamp(
        crate::drawer::MIN_ASSETS_FRACTION,
        crate::drawer::MAX_ASSETS_FRACTION,
    );
    let assets = inner * fraction;
    (assets, (inner - assets).max(1.0))
}

fn draw_assets(ui: &Ui, host: &mut DockHost) {
    ui.text("Assets");
    if ui
        .input_text("##asset-search", &mut host.browser.asset_query)
        .hint("Search assets")
        .build()
    {
        host.mark_dirty();
    }
    ui.input_text("##asset-name", host.asset_name_mut())
        .hint("Name")
        .build();
    if ui.button("New folder") {
        create_folder(host);
    }
    ui.same_line();
    if ui.button("New asset") {
        create_asset(host);
    }
    ui.same_line();
    if ui.button("Rename") {
        rename_asset(host);
    }
    ui.same_line();
    if ui.button("Delete") {
        delete_asset(host);
    }

    let avail = ui.content_region_avail();
    let split = layout_assets_pane(Rect {
        x: 0.0,
        y: 0.0,
        w: avail[0],
        h: avail[1].max(1.0),
    });
    let query = host.browser.asset_query.clone();
    let expanded = host.browser.expanded.clone();
    let rows = asset_rows(&host.library, &query, &expanded);
    let contents = folder_contents(&host.library, host.browser.selected_folder, &query);
    ui.child_window("AssetTree")
        .size([split.tree.w, split.tree.h])
        .flags(WindowFlags::NO_DOCKING)
        .build(ui, || {
            if rows.is_empty() {
                ui.text("No assets");
                return;
            }
            let mut chosen: Option<(AssetId, AssetKind)> = None;
            let mut toggled: Option<AssetId> = None;
            for row in &rows {
                let Some(record) = host.library.asset(row.id) else {
                    continue;
                };
                if row.kind == AssetKind::Folder {
                    let open = !query.trim().is_empty() || expanded.contains(&row.id);
                    let marker = if open { "-" } else { "+" };
                    if ui.button(format!("{marker}##fold-{}", row.id.0)) {
                        toggled = Some(row.id);
                    }
                    ui.same_line();
                } else if row.depth > 0 {
                    ui.text(" ");
                    ui.same_line();
                }
                let selected = host.browser.selected_asset == Some(row.id)
                    || (host.browser.selected_folder == Some(row.id)
                        && host.browser.selected_asset.is_none());
                let label = if row.kind == AssetKind::Folder {
                    format!("{}/##tree-{}", record.name, row.id.0)
                } else {
                    format!("{}##tree-{}", record.name, row.id.0)
                };
                if ui.selectable_config(label).selected(selected).build() {
                    chosen = Some((row.id, row.kind));
                }
            }
            if let Some(id) = toggled {
                if host.browser.expanded.contains(&id) {
                    host.browser.expanded.remove(&id);
                } else {
                    host.browser.expanded.insert(id);
                }
                host.mark_dirty();
            }
            if let Some((id, kind)) = chosen {
                select_asset(host, id, kind);
            }
        });
    ui.same_line_with_spacing(0.0, crate::drawer::SPLITTER_W);
    ui.child_window("AssetContents")
        .size([split.contents.w.max(1.0), split.contents.h])
        .flags(WindowFlags::NO_DOCKING)
        .build(ui, || {
            if contents.is_empty() {
                ui.text(if query.trim().is_empty() {
                    "No assets"
                } else {
                    "No matches"
                });
                return;
            }
            let mut chosen: Option<(AssetId, AssetKind)> = None;
            for id in &contents {
                let Some(record) = host.library.asset(*id) else {
                    continue;
                };
                let selected = host.browser.selected_asset == Some(*id)
                    || (host.browser.selected_folder == Some(*id)
                        && host.browser.selected_asset.is_none());
                let label = if record.kind == AssetKind::Folder {
                    format!("{}/##content-{}", record.name, id.0)
                } else {
                    format!("{}##content-{}", record.name, id.0)
                };
                if ui.selectable_config(label).selected(selected).build() {
                    chosen = Some((*id, record.kind));
                }
            }
            if let Some((id, kind)) = chosen {
                select_asset(host, id, kind);
            }
        });
}

fn draw_components(ui: &Ui, host: &mut DockHost) {
    ui.text("Components");
    if ui
        .input_text("##component-search", &mut host.browser.component_query)
        .hint("Search components")
        .build()
    {
        host.mark_dirty();
    }
    ui.input_text("##component-name", host.component_name_mut())
        .hint("Name")
        .build();
    if ui.button("New component") {
        create_component(host);
    }
    ui.same_line();
    if ui.button("Rename##component") {
        rename_component(host);
    }
    ui.same_line();
    if ui.button("Delete##component") {
        delete_component(host);
    }
    let rows = component_rows(&host.library, &host.browser.component_query);
    let query_empty = host.browser.component_query.trim().is_empty();
    if rows.is_empty() {
        ui.text(if query_empty {
            "No components"
        } else {
            "No matches"
        });
        return;
    }
    let mut chosen: Option<ComponentId> = None;
    for id in rows {
        let Some(record) = host.library.component(id) else {
            continue;
        };
        let selected = host.browser.selected_component == Some(id);
        if ui
            .selectable_config(format!("{}##component-{}", record.name, id.0))
            .selected(selected)
            .build()
        {
            chosen = Some(id);
        }
    }
    if let Some(id) = chosen {
        host.browser.selected_component = Some(id);
        host.mark_dirty();
    }
}

fn select_asset(host: &mut DockHost, id: AssetId, kind: AssetKind) {
    match kind {
        AssetKind::Folder => {
            host.browser.selected_folder = Some(id);
            host.browser.selected_asset = None;
        }
        AssetKind::Asset => {
            host.browser.selected_asset = Some(id);
            host.browser.selected_folder = host.library.asset(id).and_then(|asset| asset.parent);
        }
    }
    host.mark_dirty();
}

fn create_folder(host: &mut DockHost) {
    let parent = insertion_folder(&host.library, &host.browser);
    let name = host.asset_name().to_string();
    match host.library.create_folder(parent, &name) {
        Ok(id) => {
            expand_for_new_folder(&mut host.browser.expanded, parent, id);
            host.browser.selected_folder = Some(id);
            host.browser.selected_asset = None;
            host.clear_asset_name();
            host.status.clear();
            host.mark_dirty();
        }
        Err(err) => note_error(host, err),
    }
}

fn create_asset(host: &mut DockHost) {
    let parent = insertion_folder(&host.library, &host.browser);
    let name = host.asset_name().to_string();
    match host.library.create_asset(parent, &name) {
        Ok(id) => {
            if let Some(parent) = parent {
                host.browser.expanded.insert(parent);
            }
            host.browser.selected_asset = Some(id);
            host.browser.selected_folder = parent;
            host.clear_asset_name();
            host.status.clear();
            host.mark_dirty();
        }
        Err(err) => note_error(host, err),
    }
}

fn rename_asset(host: &mut DockHost) {
    let Some(id) = host.browser.selected_asset.or(host.browser.selected_folder) else {
        host.status = "Select an asset first.".to_string();
        return;
    };
    let name = host.asset_name().to_string();
    match host.library.rename_asset(id, &name) {
        Ok(()) => {
            host.clear_asset_name();
            host.status.clear();
            host.mark_dirty();
        }
        Err(err) => note_error(host, err),
    }
}

fn delete_asset(host: &mut DockHost) {
    let Some(id) = host.browser.selected_asset.or(host.browser.selected_folder) else {
        host.status = "Select an asset first.".to_string();
        return;
    };
    match host.library.delete_asset(id) {
        Ok(()) => {
            prune_browser(&mut host.browser, &host.library);
            host.status.clear();
            host.mark_dirty();
        }
        Err(err) => note_error(host, err),
    }
}

fn create_component(host: &mut DockHost) {
    let name = host.component_name().to_string();
    match host.library.create_component(&name) {
        Ok(id) => {
            host.browser.selected_component = Some(id);
            host.clear_component_name();
            host.status.clear();
            host.mark_dirty();
        }
        Err(err) => note_error(host, err),
    }
}

fn rename_component(host: &mut DockHost) {
    let Some(id) = host.browser.selected_component else {
        host.status = "Select a component first.".to_string();
        return;
    };
    let name = host.component_name().to_string();
    match host.library.rename_component(id, &name) {
        Ok(()) => {
            host.clear_component_name();
            host.status.clear();
            host.mark_dirty();
        }
        Err(err) => note_error(host, err),
    }
}

fn delete_component(host: &mut DockHost) {
    let Some(id) = host.browser.selected_component else {
        host.status = "Select a component first.".to_string();
        return;
    };
    match host.library.delete_component(id) {
        Ok(()) => {
            prune_browser(&mut host.browser, &host.library);
            host.status.clear();
            host.mark_dirty();
        }
        Err(err) => note_error(host, err),
    }
}

fn note_error(host: &mut DockHost, err: LibraryError) {
    host.status = err.to_string();
}
