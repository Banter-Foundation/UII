//! Dear ImGui for the Animation tab. The caller supplies the window or the
//! shell's tab content; this fills the available region.

use dear_imgui_rs::{Key, MouseButton, Ui};

use crate::easing::Easing;
use crate::model::{ClipId, KeyframeId, MotionProperty, NodeId};
use crate::tab::{AnimationTab, Drag};
use crate::timeline::{active_key_refs, fit_preview, hit_keyframe, rotated_corners, timeline_rows};

const TARGET_COLUMN: f32 = 248.0;
const INSPECTOR_COLUMN: f32 = 268.0;
const ROW_HEIGHT: f32 = 22.0;
const RULER_HEIGHT: f32 = 22.0;
const LABEL_WIDTH: f32 = 108.0;
const PREVIEW_MIN: f32 = 140.0;

pub fn draw_animation_tab(ui: &Ui, tab: &mut AnimationTab) {
    shortcuts(ui, tab);
    draw_transport(ui, tab);
    if !tab.status().is_empty() {
        ui.text(tab.status());
    }
    let avail = ui.content_region_avail();
    let preview_h = (avail[1] * 0.34)
        .clamp(PREVIEW_MIN, 280.0)
        .min(avail[1] * 0.5);
    let body_h = (avail[1] - preview_h - 6.0).max(64.0);
    ui.child_window("animation-body")
        .size([avail[0].max(1.0), body_h])
        .build(ui, || {
            let width = ui.content_region_avail_width().max(1.0);
            let left = TARGET_COLUMN.min(width * 0.3);
            let right = INSPECTOR_COLUMN.min(width * 0.3);
            let mid = (width - left - right - 12.0).max(80.0);
            ui.child_window("animation-targets")
                .size([left, 0.0])
                .border(true)
                .build(ui, || draw_targets(ui, tab));
            ui.same_line();
            ui.child_window("animation-timeline")
                .size([mid, 0.0])
                .border(true)
                .build(ui, || draw_timeline(ui, tab));
            ui.same_line();
            ui.child_window("animation-inspector")
                .size([right, 0.0])
                .border(true)
                .build(ui, || draw_inspector(ui, tab));
        });
    ui.child_window("animation-preview")
        .size([0.0, preview_h.max(1.0)])
        .border(true)
        .build(ui, || draw_preview(ui, tab));
    if !ui.is_mouse_down(MouseButton::Left) {
        tab.commit_drag();
    }
}

fn shortcuts(ui: &Ui, tab: &mut AnimationTab) {
    if ui.io().want_text_input() {
        return;
    }
    let (duration, looping) = tab.active_timing();
    if ui.is_key_pressed(Key::Space) && !ui.io().key_ctrl() {
        if tab.playback.playing {
            tab.playback.pause();
        } else {
            tab.playback.play(duration, looping);
        }
    }
    if ui.is_key_pressed(Key::Delete) || ui.is_key_pressed(Key::Backspace) {
        if let (Some(clip), Some(key)) = (tab.project.active_clip(), tab.selected_keyframe) {
            tab.edit(|project| {
                let _ = project.delete_keyframe(clip, key);
            });
            tab.selected_keyframe = None;
        }
    }
    if ui.io().key_ctrl() && ui.is_key_pressed(Key::Z) {
        if ui.io().key_shift() {
            tab.redo();
        } else {
            tab.undo();
        }
    } else if ui.io().key_ctrl() && ui.is_key_pressed(Key::Y) {
        tab.redo();
    }
}

fn draw_transport(ui: &Ui, tab: &mut AnimationTab) {
    ui.text("Animation");
    ui.same_line();
    let clips: Vec<(ClipId, String)> = tab
        .project
        .clips()
        .iter()
        .map(|clip| (clip.id(), clip.name().to_string()))
        .collect();
    if clips.is_empty() {
        ui.text("No clip");
    } else {
        let mut index = tab
            .project
            .active_clip()
            .and_then(|id| clips.iter().position(|(clip, _)| *clip == id))
            .unwrap_or(0);
        let labels: Vec<&str> = clips.iter().map(|(_, name)| name.as_str()).collect();
        ui.set_next_item_width(160.0);
        if ui.combo_simple_string("Clip", &mut index, &labels) {
            let id = clips[index].0;
            tab.edit(|project| {
                let _ = project.set_active_clip(Some(id));
            });
        }
    }
    ui.same_line();
    if ui.button("Add clip") {
        tab.edit(|project| {
            project.add_clip("");
        });
    }
    ui.same_line();
    if ui.button("Duplicate") {
        if let Some(id) = tab.project.active_clip() {
            tab.edit(|project| {
                let _ = project.duplicate_clip(id);
            });
        }
    }
    ui.same_line();
    if ui.button("Delete clip") {
        if let Some(id) = tab.project.active_clip() {
            tab.edit(|project| {
                let _ = project.delete_clip(id);
            });
        }
    }
    if let Some(id) = tab.project.active_clip() {
        if tab.clip_name_for != Some(id) {
            tab.clip_name_for = Some(id);
            tab.clip_name_buf = tab
                .project
                .clip(id)
                .map(|clip| clip.name().to_string())
                .unwrap_or_default();
        }
        ui.same_line();
        ui.set_next_item_width(140.0);
        if ui.input_text("Name", &mut tab.clip_name_buf).build() {
            let name = tab.clip_name_buf.clone();
            tab.edit(|project| {
                let _ = project.rename_clip(id, &name);
            });
        }
        let mut duration = tab
            .project
            .clip(id)
            .map(|clip| clip.duration())
            .unwrap_or(1.0);
        ui.same_line();
        ui.set_next_item_width(90.0);
        if ui.input_float("Duration", &mut duration) {
            tab.edit(|project| {
                let _ = project.set_clip_duration(id, duration);
            });
        }
        let mut looping = tab.project.clip(id).is_some_and(|clip| clip.looping());
        ui.same_line();
        if ui.checkbox("Loop", &mut looping) {
            tab.edit(|project| {
                let _ = project.set_clip_loop(id, looping);
            });
        }
    }
    let (duration, looping) = tab.active_timing();
    if tab.playback.playing {
        if ui.button("Pause") {
            tab.playback.pause();
        }
    } else if ui.button("Play") {
        tab.playback.play(duration, looping);
    }
    ui.same_line();
    if ui.button("Stop") {
        tab.playback.stop();
    }
    ui.same_line();
    ui.text(format!("{:.3} s", tab.playback.time));
    ui.same_line();
    if ui.button("Undo") {
        tab.undo();
    }
    ui.same_line();
    if ui.button("Redo") {
        tab.redo();
    }
}

fn draw_targets(ui: &Ui, tab: &mut AnimationTab) {
    ui.text("Layout targets");
    if ui.button("Add target") {
        let id = tab.edit_with(|project| project.add_target(""));
        tab.selected_target = Some(id);
        tab.name_for = None;
    }
    let rows: Vec<(NodeId, String)> = tab
        .project
        .targets()
        .iter()
        .map(|target| (target.id, target.name.clone()))
        .collect();
    if rows.is_empty() {
        ui.text("Add a target to put motion on the layout.");
        return;
    }
    for (id, name) in &rows {
        let _id = ui.push_id(id.0 as usize);
        if ui
            .selectable_config(name.as_str())
            .selected(tab.selected_target == Some(*id))
            .build()
        {
            tab.selected_target = Some(*id);
        }
    }
    let Some(id) = tab.selected_target else {
        return;
    };
    if tab.project.target(id).is_none() {
        tab.selected_target = None;
        return;
    }
    ui.separator();
    if tab.name_for != Some(id) {
        tab.name_for = Some(id);
        tab.name_buf = tab
            .project
            .target(id)
            .map(|target| target.name.clone())
            .unwrap_or_default();
    }
    if ui.input_text("Target", &mut tab.name_buf).build() {
        let name = tab.name_buf.clone();
        tab.edit(|project| {
            let _ = project.rename_target(id, &name);
        });
    }
    let Some(node) = tab.project.target(id).cloned() else {
        return;
    };
    let mut x = node.x;
    let mut y = node.y;
    let mut width = node.width;
    let mut height = node.height;
    let mut rotation = node.rotation_deg;
    let mut opacity = node.opacity;
    let mut fill = node.fill;
    let mut changed = false;
    changed |= ui.input_float("X", &mut x);
    changed |= ui.input_float("Y", &mut y);
    changed |= ui.input_float("Width", &mut width);
    changed |= ui.input_float("Height", &mut height);
    changed |= ui.input_float("Rotation", &mut rotation);
    changed |= ui.input_float("Opacity", &mut opacity);
    changed |= ui.color_edit4("Fill", &mut fill);
    if changed {
        let mut rest = node;
        rest.x = x;
        rest.y = y;
        rest.width = width;
        rest.height = height;
        rest.rotation_deg = rotation;
        rest.opacity = opacity;
        rest.fill = fill;
        tab.edit(|project| {
            let _ = project.set_rest(id, rest);
        });
    }
    if ui.button("Remove target") {
        tab.edit(|project| {
            let _ = project.remove_target(id);
        });
        if tab.selected_target == Some(id) {
            tab.selected_target = None;
        }
    }
}

fn draw_timeline(ui: &Ui, tab: &mut AnimationTab) {
    let width = ui.content_region_avail_width().max(1.0);
    let rows = timeline_rows(&tab.project, tab.selected_target, ROW_HEIGHT);
    let keys = active_key_refs(&tab.project);
    let (duration, _) = tab.active_timing();
    let ruler_pos = ui.cursor_screen_pos();
    ui.invisible_button("ruler", [width, RULER_HEIGHT]);
    let ruler_active = ui.is_item_active();
    let timeline_hovered = ui.is_window_hovered();
    let track_left = ruler_pos[0] + LABEL_WIDTH;
    if ruler_active && ui.is_mouse_down(MouseButton::Left) {
        let time = tab.view.x_to_time(ui.mouse_pos()[0], track_left).max(0.0);
        tab.playback.seek(time, duration);
        tab.drag = Drag::Playhead;
    }
    if timeline_hovered {
        let wheel = ui.io().mouse_wheel();
        if wheel.abs() > 0.0 {
            let anchor = tab.view.x_to_time(ui.mouse_pos()[0], track_left);
            let factor = if wheel > 0.0 { 1.12 } else { 1.0 / 1.12 };
            tab.view
                .zoom_at(factor, anchor, track_left, (width - LABEL_WIDTH).max(1.0));
        }
        if ui.is_mouse_dragging(MouseButton::Middle) {
            tab.view.pan(ui.io().mouse_delta()[0]);
        }
    }
    paint_ruler(ui, ruler_pos, width, tab);
    let content_origin = ui.cursor_screen_pos();
    if rows.is_empty() {
        ui.text("Add a clip and a target. Select the target to key its layout channels.");
        return;
    }
    for row in &rows {
        let _id = ui.push_id((row.node.0 as usize) * 16 + property_index(row.property));
        let pos = ui.cursor_screen_pos();
        ui.invisible_button("track", [width, row.height]);
        let hovered = ui.is_item_hovered();
        let clicked = ui.is_item_clicked();
        let double = hovered && ui.is_mouse_double_clicked(MouseButton::Left);
        paint_row(
            ui,
            pos,
            width,
            row.height,
            hovered,
            tab,
            row,
            &keys,
            content_origin,
        );
        if clicked {
            tab.selected_target = Some(row.node);
            tab.selected_property = Some(row.property);
            let pointer_y = ui.mouse_pos()[1] - content_origin[1];
            if let Some(hit) = hit_keyframe(
                ui.mouse_pos()[0],
                pointer_y,
                &rows,
                &keys,
                &tab.view,
                track_left,
                8.0,
            ) {
                let origin_time = keys
                    .iter()
                    .find(|key| key.id == hit.keyframe)
                    .map(|key| key.time)
                    .unwrap_or(tab.playback.time);
                tab.selected_keyframe = Some(hit.keyframe);
                tab.selected_property = Some(hit.property);
                tab.selected_target = Some(hit.node);
                tab.begin_drag();
                tab.drag = Drag::Keyframe {
                    id: hit.keyframe,
                    origin_time,
                };
            }
        }
        if double {
            capture_at_playhead(tab, row.node, row.property);
        }
    }
    if let Drag::Keyframe { id, origin_time } = tab.drag {
        if ui.is_mouse_down(MouseButton::Left) {
            let dx = ui.mouse_drag_delta(MouseButton::Left)[0];
            let time = origin_time + dx / tab.view.pixels_per_second.max(1.0);
            if let Some(clip) = tab.project.active_clip() {
                let _ = tab.project.move_keyframe(clip, id, time);
            }
        }
    }
    let bottom = content_origin[1] + rows.len() as f32 * ROW_HEIGHT;
    let playhead = tab.view.time_to_x(tab.playback.time, track_left);
    if playhead.is_finite() {
        let draw = ui.get_window_draw_list();
        draw.add_line(
            [playhead, ruler_pos[1]],
            [playhead, bottom.max(ruler_pos[1] + RULER_HEIGHT)],
            [0.95, 0.62, 0.28, 1.0],
        )
        .thickness(2.0)
        .build();
    }
}

fn paint_ruler(ui: &Ui, origin: [f32; 2], width: f32, tab: &AnimationTab) {
    let draw = ui.get_window_draw_list();
    draw.add_rect(
        origin,
        [origin[0] + width, origin[1] + RULER_HEIGHT],
        [0.14, 0.14, 0.18, 1.0],
    )
    .filled(true)
    .build();
    let track_left = origin[0] + LABEL_WIDTH;
    let mut time = tab.view.origin.floor().max(0.0);
    let end = tab.view.x_to_time(origin[0] + width, track_left) + 0.5;
    while time < end && time < 10_000.0 {
        let x = tab.view.time_to_x(time, track_left);
        if x >= origin[0] && x <= origin[0] + width && x.is_finite() {
            draw.add_line(
                [x, origin[1] + 10.0],
                [x, origin[1] + RULER_HEIGHT],
                [0.55, 0.58, 0.66, 1.0],
            )
            .thickness(1.0)
            .build();
            if (time - time.round()).abs() < 0.01 {
                draw.add_text(
                    [x + 3.0, origin[1] + 2.0],
                    [0.75, 0.78, 0.84, 1.0],
                    format!("{time:.0}s"),
                );
            }
        }
        time += 0.5;
    }
}

fn paint_row(
    ui: &Ui,
    pos: [f32; 2],
    width: f32,
    height: f32,
    hovered: bool,
    tab: &AnimationTab,
    row: &crate::timeline::TimelineRow,
    keys: &[crate::timeline::KeyRef],
    content_origin: [f32; 2],
) {
    let draw = ui.get_window_draw_list();
    let selected =
        tab.selected_target == Some(row.node) && tab.selected_property == Some(row.property);
    let color = if selected {
        [0.22, 0.30, 0.42, 1.0]
    } else if hovered {
        [0.18, 0.18, 0.22, 1.0]
    } else if ((row.y / ROW_HEIGHT) as i32) % 2 == 0 {
        [0.12, 0.12, 0.15, 1.0]
    } else {
        [0.10, 0.10, 0.13, 1.0]
    };
    draw.add_rect(pos, [pos[0] + width, pos[1] + height], color)
        .filled(true)
        .build();
    let target = tab
        .project
        .target(row.node)
        .map(|target| target.name.as_str())
        .unwrap_or("Target");
    draw.add_text(
        [pos[0] + 4.0, pos[1] + 3.0],
        [0.86, 0.88, 0.92, 1.0],
        format!("{target}  {}", row.property.label()),
    );
    let track_left = content_origin[0] + LABEL_WIDTH;
    for key in keys
        .iter()
        .filter(|key| key.node == row.node && key.property == row.property)
    {
        let x = tab.view.time_to_x(key.time, track_left);
        let y = pos[1] + height * 0.5;
        if !x.is_finite() || !y.is_finite() {
            continue;
        }
        let color = if tab.selected_keyframe == Some(key.id) {
            [0.98, 0.78, 0.35, 1.0]
        } else {
            [0.55, 0.78, 0.95, 1.0]
        };
        draw.add_quad_filled(
            [x, y - 6.0],
            [x + 6.0, y],
            [x, y + 6.0],
            [x - 6.0, y],
            color,
        );
    }
}

fn capture_at_playhead(tab: &mut AnimationTab, node: NodeId, property: MotionProperty) {
    let Some(clip) = tab.project.active_clip() else {
        return;
    };
    let time = tab.playback.time;
    let created =
        tab.edit_with(|project| project.capture_keyframe(clip, node, property, time).ok());
    if let Some(id) = created {
        tab.selected_keyframe = Some(id);
        tab.selected_property = Some(property);
        tab.selected_target = Some(node);
    }
}

fn draw_inspector(ui: &Ui, tab: &mut AnimationTab) {
    ui.text("Keyframe");
    let Some(clip) = tab.project.active_clip() else {
        ui.text("Add a clip first.");
        return;
    };
    if ui.button("Add key at playhead") {
        if let (Some(node), Some(property)) = (tab.selected_target, tab.selected_property) {
            capture_at_playhead(tab, node, property);
        }
    }
    if tab.selected_property.is_none() {
        ui.text("Select a track row, then add a key.");
    }
    let Some(key_id) = tab.selected_keyframe else {
        return;
    };
    let Some(found) = find_key(&tab.project, clip, key_id) else {
        tab.selected_keyframe = None;
        return;
    };
    ui.text(format!(
        "{}  {}",
        target_name(&tab.project, found.node),
        found.property.label()
    ));
    let mut time = found.time;
    let mut value = found.value;
    ui.set_next_item_width(120.0);
    if ui.input_float("Time", &mut time) {
        tab.edit(|project| {
            let _ = project.move_keyframe(clip, key_id, time);
        });
    }
    ui.set_next_item_width(120.0);
    if ui.input_float("Value", &mut value) {
        tab.edit(|project| {
            let _ = project.set_keyframe_value(clip, key_id, value);
        });
    }
    let labels = [
        "Linear",
        "Ease In",
        "Ease Out",
        "Ease In Out",
        "Hold",
        "Cubic Bezier",
    ];
    let mut index = easing_index(found.easing);
    if ui.combo_simple_string("Easing", &mut index, &labels) {
        let easing = easing_from_index(index, found.easing);
        tab.edit(|project| {
            let _ = project.set_keyframe_easing(clip, key_id, easing);
        });
    }
    if let Easing::CubicBezier { x1, y1, x2, y2 } = found.easing {
        let mut controls = [x1, y1, x2, y2];
        let edited = ui.input_float("Bezier X1", &mut controls[0])
            | ui.input_float("Bezier Y1", &mut controls[1])
            | ui.input_float("Bezier X2", &mut controls[2])
            | ui.input_float("Bezier Y2", &mut controls[3]);
        if edited {
            let easing = Easing::CubicBezier {
                x1: controls[0],
                y1: controls[1],
                x2: controls[2],
                y2: controls[3],
            };
            tab.edit(|project| {
                let _ = project.set_keyframe_easing(clip, key_id, easing);
            });
        }
    }
    if ui.button("Delete keyframe") {
        tab.edit(|project| {
            let _ = project.delete_keyframe(clip, key_id);
        });
        tab.selected_keyframe = None;
    }
}

fn draw_preview(ui: &Ui, tab: &AnimationTab) {
    ui.text("Motion preview");
    let posed = tab.posed_layout();
    if posed.is_empty() {
        ui.text("Add a target to see layout motion.");
        return;
    }
    let origin = ui.cursor_screen_pos();
    let avail = ui.content_region_avail();
    let width = avail[0].max(1.0);
    let height = avail[1].max(1.0);
    ui.invisible_button("preview-canvas", [width, height]);
    let draw = ui.get_window_draw_list();
    draw.add_rect(
        origin,
        [origin[0] + width, origin[1] + height],
        [0.07, 0.07, 0.09, 1.0],
    )
    .filled(true)
    .build();
    // Fit the rest pose and the playhead together so the frame stays put while
    // a target moves, instead of recentering on the moving box every frame.
    let mut fit_nodes = tab.project.targets().to_vec();
    fit_nodes.extend(posed.iter().cloned());
    let Some(fit) = fit_preview(&fit_nodes, width, height, 16.0) else {
        return;
    };
    for node in tab.project.targets() {
        let corners = rotated_corners(node.x, node.y, node.width, node.height, node.rotation_deg);
        let Some(screen) = map_corners(origin, fit, corners) else {
            continue;
        };
        draw.add_quad(
            screen[0],
            screen[1],
            screen[2],
            screen[3],
            [0.45, 0.48, 0.55, 0.9],
            1.0,
            dear_imgui_rs::DrawFlags::NONE,
        );
    }
    for node in &posed {
        let corners = rotated_corners(node.x, node.y, node.width, node.height, node.rotation_deg);
        let Some(screen) = map_corners(origin, fit, corners) else {
            continue;
        };
        let mut color = node.fill;
        color[3] *= node.opacity.clamp(0.0, 1.0);
        draw.add_quad_filled(screen[0], screen[1], screen[2], screen[3], color);
        draw.add_quad(
            screen[0],
            screen[1],
            screen[2],
            screen[3],
            [0.9, 0.92, 0.95, color[3].max(0.35)],
            1.0,
            dear_imgui_rs::DrawFlags::NONE,
        );
        draw.add_text(
            screen[0],
            [1.0, 1.0, 1.0, color[3].max(0.75)],
            node.name.as_str(),
        );
    }
}

struct FoundKey {
    node: NodeId,
    property: MotionProperty,
    time: f32,
    value: f32,
    easing: Easing,
}

fn map_corners(
    origin: [f32; 2],
    fit: crate::timeline::PreviewFit,
    corners: [[f32; 2]; 4],
) -> Option<[[f32; 2]; 4]> {
    let screen = corners.map(|point| {
        [
            origin[0] + fit.origin_x + point[0] * fit.scale,
            origin[1] + fit.origin_y + point[1] * fit.scale,
        ]
    });
    if screen
        .iter()
        .any(|point| !point[0].is_finite() || !point[1].is_finite())
    {
        None
    } else {
        Some(screen)
    }
}

fn find_key(
    project: &crate::model::AnimationProject,
    clip: ClipId,
    id: KeyframeId,
) -> Option<FoundKey> {
    let clip = project.clip(clip)?;
    for track in clip.tracks() {
        if let Some(key) = track.keyframes().iter().find(|key| key.id() == id) {
            return Some(FoundKey {
                node: track.node(),
                property: track.property(),
                time: key.time(),
                value: key.value(),
                easing: key.easing(),
            });
        }
    }
    None
}

fn target_name(project: &crate::model::AnimationProject, id: NodeId) -> &str {
    project
        .target(id)
        .map(|target| target.name.as_str())
        .unwrap_or("Target")
}

fn property_index(property: MotionProperty) -> usize {
    MotionProperty::all()
        .iter()
        .position(|item| *item == property)
        .unwrap_or(0)
}

fn easing_index(easing: Easing) -> usize {
    match easing {
        Easing::Linear => 0,
        Easing::EaseIn => 1,
        Easing::EaseOut => 2,
        Easing::EaseInOut => 3,
        Easing::Hold => 4,
        Easing::CubicBezier { .. } => 5,
    }
}

fn easing_from_index(index: usize, current: Easing) -> Easing {
    match index {
        0 => Easing::Linear,
        1 => Easing::EaseIn,
        2 => Easing::EaseOut,
        3 => Easing::EaseInOut,
        4 => Easing::Hold,
        _ => match current {
            Easing::CubicBezier { .. } => current,
            _ => Easing::CubicBezier {
                x1: 0.42,
                y1: 0.0,
                x2: 0.58,
                y2: 1.0,
            },
        },
    }
}
