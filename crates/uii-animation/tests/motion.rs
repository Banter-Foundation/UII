//! Public contract for the Animation tab. These tests are the API.
//! Layout motion is sampled from clips and applied onto layout nodes.
//! The shell is not required: `NodeId` is the join key when Design arrives.

use uii_animation::{
    active_key_refs, apply_pose, draw_animation_tab, fit_preview, hit_keyframe, interpolate,
    rotated_corners, timeline_rows, AnimationError, AnimationProject, AnimationTab, Easing, KeyRef,
    KeyframeHit, LayoutNode, MotionProperty, NodeId, Playback, TimelineRow, TimelineView, TAB_ID,
    TAB_LABEL,
};

fn near(actual: f32, expected: f32) {
    assert!((actual - expected).abs() < 1.0e-3, "{actual} != {expected}");
}

fn project_with_clip() -> (AnimationProject, uii_animation::ClipId, NodeId) {
    let mut project = AnimationProject::new();
    let clip = project.add_clip("Intro");
    let node = project.add_target("Header");
    (project, clip, node)
}

#[test]
fn tab_identity_is_the_animation_top_level_tab() {
    assert_eq!(TAB_ID, "animation");
    assert_eq!(TAB_LABEL, "Animation");
}

#[test]
fn motion_properties_cover_layout_channels_in_row_order() {
    let all = MotionProperty::all();
    assert_eq!(
        all,
        &[
            MotionProperty::X,
            MotionProperty::Y,
            MotionProperty::Width,
            MotionProperty::Height,
            MotionProperty::Rotation,
            MotionProperty::Opacity,
            MotionProperty::FillR,
            MotionProperty::FillG,
            MotionProperty::FillB,
            MotionProperty::FillA,
        ]
    );
    assert_eq!(MotionProperty::X.label(), "X");
    assert_eq!(MotionProperty::Opacity.label(), "Opacity");
    assert_eq!(MotionProperty::FillA.label(), "Fill A");
}

#[test]
fn new_target_starts_from_a_visible_rest_pose() {
    let node = LayoutNode::rest(NodeId(4), "Card");
    assert_eq!(node.id, NodeId(4));
    assert_eq!(node.name, "Card");
    assert_eq!(node.x, 32.0);
    assert_eq!(node.y, 32.0);
    assert_eq!(node.width, 160.0);
    assert_eq!(node.height, 64.0);
    assert_eq!(node.rotation_deg, 0.0);
    assert_eq!(node.opacity, 1.0);
    assert_eq!(node.fill, [0.35, 0.55, 0.95, 1.0]);
    assert_eq!(node.channel(MotionProperty::Width), 160.0);
}

#[test]
fn layout_channels_clamp_visual_ranges_and_leave_rotation() {
    let mut node = LayoutNode::rest(NodeId(1), "Card");
    node.set_channel(MotionProperty::Opacity, 2.0);
    node.set_channel(MotionProperty::Width, -4.0);
    node.set_channel(MotionProperty::FillR, 1.5);
    node.set_channel(MotionProperty::FillA, -1.0);
    node.set_channel(MotionProperty::Rotation, 400.0);
    node.set_channel(MotionProperty::X, -12.0);
    assert_eq!(node.opacity, 1.0);
    assert_eq!(node.width, 0.0);
    assert_eq!(node.fill[0], 1.0);
    assert_eq!(node.fill[3], 0.0);
    assert_eq!(node.rotation_deg, 400.0);
    assert_eq!(node.x, -12.0);
}

#[test]
fn easing_curves_match_their_definitions() {
    assert_eq!(Easing::Linear.apply(0.5), 0.5);
    assert_eq!(Easing::EaseIn.apply(0.5), 0.125);
    assert_eq!(Easing::EaseOut.apply(0.5), 0.875);
    assert_eq!(Easing::EaseInOut.apply(0.25), 0.0625);
    assert_eq!(Easing::EaseInOut.apply(0.5), 0.5);
    assert_eq!(Easing::Hold.apply(0.99), 0.0);
    assert_eq!(Easing::Hold.apply(1.0), 1.0);
    let linear = Easing::CubicBezier {
        x1: 0.0,
        y1: 0.0,
        x2: 1.0,
        y2: 1.0,
    };
    near(linear.apply(0.0), 0.0);
    near(linear.apply(1.0), 1.0);
    near(linear.apply(0.25), 0.25);
    let broken = Easing::CubicBezier {
        x1: f32::NAN,
        y1: 0.0,
        x2: 1.0,
        y2: 1.0,
    };
    near(broken.apply(0.5), 0.5);
    assert_eq!(Easing::Linear.label(), "Linear");
    assert_eq!(
        Easing::CubicBezier {
            x1: 0.1,
            y1: 0.2,
            x2: 0.3,
            y2: 0.4,
        }
        .label(),
        "Cubic Bezier"
    );
    assert_eq!(interpolate(0.0, 100.0, 0.5, Easing::EaseIn), 12.5);
    assert_eq!(interpolate(0.0, 100.0, 0.99, Easing::Hold), 0.0);
}

#[test]
fn clip_and_target_names_come_from_the_user() {
    let mut project = AnimationProject::new();
    let first = project.add_clip("   ");
    let second = project.add_clip("Intro");
    let third = project.add_clip("  Intro  ");
    let fourth = project.add_clip("");
    assert_eq!(project.clip(first).unwrap().name(), "Clip");
    assert_eq!(project.clip(second).unwrap().name(), "Intro");
    assert_eq!(project.clip(third).unwrap().name(), "Intro 2");
    assert_eq!(project.clip(fourth).unwrap().name(), "Clip 2");
    assert_eq!(project.clip(first).unwrap().duration(), 1.0);
    assert!(!project.clip(first).unwrap().looping());
    assert_eq!(project.active_clip(), Some(first));

    let header = project.add_target("  Header ");
    let blank = project.add_target("   ");
    let again = project.add_target("Header");
    assert_eq!(project.target(header).unwrap().name, "Header");
    assert_eq!(project.target(blank).unwrap().name, "Target");
    assert_eq!(project.target(again).unwrap().name, "Header 2");
    assert_eq!(header, NodeId(1));

    project.rename_clip(second, "   ").unwrap();
    assert_eq!(project.clip(second).unwrap().name(), "Intro");
    project.rename_clip(second, "Open").unwrap();
    assert_eq!(project.clip(second).unwrap().name(), "Open");
    project.rename_target(header, "   ").unwrap();
    assert_eq!(project.target(header).unwrap().name, "Header");
    project.rename_target(header, "Title").unwrap();
    assert_eq!(project.target(header).unwrap().name, "Title");
}

#[test]
fn duration_rejects_non_finite_and_clamps_keyframes() {
    let (mut project, clip, node) = project_with_clip();
    project
        .upsert_keyframe(clip, node, MotionProperty::X, 0.5, 1.0, Easing::Linear)
        .unwrap();
    project
        .upsert_keyframe(clip, node, MotionProperty::X, 1.5, 9.0, Easing::Linear)
        .unwrap();
    assert!(matches!(
        project.set_clip_duration(clip, -1.0),
        Err(AnimationError::InvalidDuration)
    ));
    assert!(matches!(
        project.set_clip_duration(clip, f32::NAN),
        Err(AnimationError::InvalidDuration)
    ));
    project.set_clip_duration(clip, 1.0).unwrap();
    let keys = project.clip(clip).unwrap().tracks()[0].keyframes();
    assert_eq!(keys.len(), 2);
    assert_eq!(keys[0].time(), 0.5);
    assert_eq!(keys[1].time(), 1.0);
    assert_eq!(keys[1].value(), 9.0);

    project
        .upsert_keyframe(clip, node, MotionProperty::Y, 1.0, 1.0, Easing::Linear)
        .unwrap();
    project
        .upsert_keyframe(clip, node, MotionProperty::Y, 2.0, 9.0, Easing::Linear)
        .unwrap();
    project.set_clip_duration(clip, 1.0).unwrap();
    let y = project
        .clip(clip)
        .unwrap()
        .tracks()
        .iter()
        .find(|track| track.property() == MotionProperty::Y)
        .unwrap();
    assert_eq!(y.keyframes().len(), 1);
    assert_eq!(y.keyframes()[0].time(), 1.0);
    assert_eq!(y.keyframes()[0].value(), 9.0);

    project.set_clip_duration(clip, 0.0).unwrap();
    let y = project
        .clip(clip)
        .unwrap()
        .tracks()
        .iter()
        .find(|track| track.property() == MotionProperty::Y)
        .unwrap();
    assert_eq!(y.keyframes().len(), 1);
    assert_eq!(y.keyframes()[0].time(), 0.0);
    assert_eq!(y.keyframes()[0].value(), 9.0);
}

#[test]
fn keyframes_interpolate_hold_and_stay_inside_the_clip() {
    let (mut project, clip, node) = project_with_clip();
    let first = project
        .upsert_keyframe(clip, node, MotionProperty::X, 0.0, 0.0, Easing::Linear)
        .unwrap();
    let again = project
        .upsert_keyframe(clip, node, MotionProperty::X, 0.0, 4.0, Easing::EaseIn)
        .unwrap();
    assert_eq!(first, again);
    assert_eq!(
        project.clip(clip).unwrap().tracks()[0].keyframes()[0].value(),
        4.0
    );
    project
        .upsert_keyframe(clip, node, MotionProperty::X, 1.0, 100.0, Easing::Linear)
        .unwrap();
    project.set_keyframe_value(clip, first, 0.0).unwrap();
    project
        .set_keyframe_easing(clip, first, Easing::Linear)
        .unwrap();

    let at = |project: &AnimationProject, time: f32| {
        project
            .sample(clip, time)
            .unwrap()
            .get(node, MotionProperty::X)
            .unwrap()
    };
    assert_eq!(at(&project, -1.0), 0.0);
    assert_eq!(at(&project, 0.5), 50.0);
    assert_eq!(at(&project, 5.0), 100.0);

    project
        .set_keyframe_easing(clip, first, Easing::EaseIn)
        .unwrap();
    assert_eq!(at(&project, 0.5), 12.5);
    project
        .set_keyframe_easing(clip, first, Easing::Hold)
        .unwrap();
    assert_eq!(at(&project, 0.99), 0.0);
    assert_eq!(at(&project, 1.0), 100.0);

    let outside = project
        .upsert_keyframe(clip, node, MotionProperty::Y, 5.0, 3.0, Easing::Linear)
        .unwrap();
    let stored = project
        .clip(clip)
        .unwrap()
        .tracks()
        .iter()
        .find(|track| track.property() == MotionProperty::Y)
        .unwrap()
        .keyframes()[0]
        .time();
    assert_eq!(stored, 1.0);
    project.move_keyframe(clip, outside, -2.0).unwrap();
    assert_eq!(
        project
            .clip(clip)
            .unwrap()
            .tracks()
            .iter()
            .find(|track| track.property() == MotionProperty::Y)
            .unwrap()
            .keyframes()[0]
            .time(),
        0.0
    );

    let left = project.clip(clip).unwrap().tracks()[0].keyframes()[0].id();
    let right = project.clip(clip).unwrap().tracks()[0].keyframes()[1].id();
    project.move_keyframe(clip, left, 1.0).unwrap();
    let xs = project.clip(clip).unwrap().tracks()[0].keyframes();
    assert_eq!(xs.len(), 1);
    assert_eq!(xs[0].id(), left);
    assert_eq!(xs[0].value(), 0.0);
    assert_ne!(xs[0].id(), right);

    project.delete_keyframe(clip, left).unwrap();
    assert!(project
        .clip(clip)
        .unwrap()
        .tracks()
        .iter()
        .all(|track| { track.property() != MotionProperty::X }));
    assert!(matches!(
        project.delete_keyframe(clip, left),
        Err(AnimationError::UnknownKeyframe)
    ));
    assert!(matches!(
        project.sample(uii_animation::ClipId(999), 0.0),
        Err(AnimationError::UnknownClip)
    ));
}

#[test]
fn capture_uses_rest_then_sampled_motion() {
    let (mut project, clip, node) = project_with_clip();
    let id = project
        .capture_keyframe(clip, node, MotionProperty::X, 0.0)
        .unwrap();
    assert_eq!(
        project.clip(clip).unwrap().tracks()[0].keyframes()[0].value(),
        32.0
    );
    project
        .upsert_keyframe(clip, node, MotionProperty::X, 1.0, 132.0, Easing::Linear)
        .unwrap();
    project
        .set_keyframe_easing(clip, id, Easing::Linear)
        .unwrap();
    let mid = project
        .capture_keyframe(clip, node, MotionProperty::X, 0.5)
        .unwrap();
    let value = project.clip(clip).unwrap().tracks()[0]
        .keyframes()
        .iter()
        .find(|key| key.id() == mid)
        .unwrap()
        .value();
    assert_eq!(value, 82.0);
}

#[test]
fn pose_overrides_channels_and_clamps_on_apply() {
    let (mut project, clip, node) = project_with_clip();
    project
        .upsert_keyframe(
            clip,
            node,
            MotionProperty::Opacity,
            0.0,
            2.0,
            Easing::Linear,
        )
        .unwrap();
    project
        .upsert_keyframe(
            clip,
            node,
            MotionProperty::Width,
            0.0,
            -10.0,
            Easing::Linear,
        )
        .unwrap();
    project
        .upsert_keyframe(
            clip,
            node,
            MotionProperty::Rotation,
            0.0,
            270.0,
            Easing::Linear,
        )
        .unwrap();
    project
        .upsert_keyframe(clip, node, MotionProperty::X, 0.0, 10.0, Easing::Linear)
        .unwrap();
    let pose = project.sample(clip, 0.0).unwrap();
    assert_eq!(pose.get(node, MotionProperty::Opacity), Some(2.0));
    assert!(pose.get(node, MotionProperty::Y).is_none());
    let applied = apply_pose(project.targets(), &pose);
    assert_eq!(applied.len(), 1);
    assert_eq!(applied[0].id, node);
    assert_eq!(applied[0].name, "Header");
    assert_eq!(applied[0].x, 10.0);
    assert_eq!(applied[0].y, 32.0);
    assert_eq!(applied[0].width, 0.0);
    assert_eq!(applied[0].opacity, 1.0);
    assert_eq!(applied[0].rotation_deg, 270.0);
    let empty = project.sample_active(0.0);
    assert!(!empty.is_empty());
    let mut bare = AnimationProject::new();
    assert!(bare.sample_active(0.0).is_empty());
    bare.add_target("Only");
    let same = apply_pose(bare.targets(), &bare.sample_active(0.0));
    assert_eq!(same[0].x, 32.0);
}

#[test]
fn deleting_a_target_drops_its_tracks_and_sync_keeps_ids() {
    let (mut project, clip, node) = project_with_clip();
    project
        .upsert_keyframe(clip, node, MotionProperty::X, 0.0, 1.0, Easing::Linear)
        .unwrap();
    let extra = LayoutNode::rest(NodeId(10), "From design");
    project.sync_targets(&[extra.clone()]);
    assert!(project.target(node).is_some());
    assert_eq!(project.target(NodeId(10)).unwrap().name, "From design");
    let mut moved = extra.clone();
    moved.x = 80.0;
    moved.name = "Synced".to_string();
    project.sync_targets(&[moved]);
    assert_eq!(project.target(NodeId(10)).unwrap().x, 80.0);
    assert_eq!(project.target(NodeId(10)).unwrap().name, "Synced");
    assert!(project.target(node).is_some());
    let created = project.add_target("After sync");
    assert_eq!(created, NodeId(11));

    project.remove_target(node).unwrap();
    assert!(project.target(node).is_none());
    assert!(project.clip(clip).unwrap().tracks().is_empty());
    assert!(matches!(
        project.remove_target(node),
        Err(AnimationError::UnknownTarget)
    ));

    let mut rest = project.target(NodeId(10)).unwrap().clone();
    rest.id = NodeId(1);
    rest.y = 7.0;
    project.set_rest(NodeId(10), rest).unwrap();
    assert_eq!(project.target(NodeId(10)).unwrap().id, NodeId(10));
    assert_eq!(project.target(NodeId(10)).unwrap().y, 7.0);
}

#[test]
fn active_clip_follows_delete_and_duplicate_copies_motion() {
    let (mut project, clip, node) = project_with_clip();
    project.set_clip_loop(clip, true).unwrap();
    project
        .upsert_keyframe(clip, node, MotionProperty::X, 0.25, 4.0, Easing::Hold)
        .unwrap();
    let copy = project.duplicate_clip(clip).unwrap();
    assert_eq!(project.active_clip(), Some(clip));
    assert_eq!(project.clip(copy).unwrap().name(), "Intro copy");
    assert!(project.clip(copy).unwrap().looping());
    assert_eq!(
        project.clip(copy).unwrap().tracks()[0].keyframes()[0].value(),
        4.0
    );
    assert_ne!(
        project.clip(copy).unwrap().tracks()[0].keyframes()[0].id(),
        project.clip(clip).unwrap().tracks()[0].keyframes()[0].id()
    );
    project.set_active_clip(Some(copy)).unwrap();
    project.delete_clip(copy).unwrap();
    assert_eq!(project.active_clip(), None);
    assert!(matches!(
        project.set_active_clip(Some(copy)),
        Err(AnimationError::UnknownClip)
    ));
    project.set_active_clip(Some(clip)).unwrap();
    project.delete_clip(clip).unwrap();
    assert_eq!(project.active_clip(), None);
    assert!(project.clips().is_empty());
}

#[test]
fn json_roundtrip_sorts_keys_and_rejects_a_bad_document() {
    let (mut project, clip, node) = project_with_clip();
    project
        .upsert_keyframe(clip, node, MotionProperty::X, 0.0, 1.0, Easing::Linear)
        .unwrap();
    project
        .upsert_keyframe(clip, node, MotionProperty::X, 1.0, 5.0, Easing::EaseOut)
        .unwrap();
    let text = project.to_json();
    let loaded = AnimationProject::from_json(&text).unwrap();
    assert_eq!(loaded.clip(clip).unwrap().name(), "Intro");
    assert_eq!(loaded.target(node).unwrap().name, "Header");
    let pose = loaded.sample(clip, 0.5).unwrap();
    // The segment uses the left key's easing. That key is linear, so the midpoint is 3.
    near(pose.get(node, MotionProperty::X).unwrap(), 3.0);

    let swapped = text
        .replacen("0.0", "1.0", 1)
        .replacen("\"time\": 1.0", "\"time\": 0.0", 1);
    let resorted = AnimationProject::from_json(&swapped).unwrap();
    let mid = resorted.sample(clip, 0.5).unwrap();
    assert!(mid.get(node, MotionProperty::X).is_some());

    assert!(matches!(
        AnimationProject::from_json("{"),
        Err(AnimationError::Json(_))
    ));
    let negative = text.replace("\"duration\": 1.0", "\"duration\": -2.0");
    assert!(matches!(
        AnimationProject::from_json(&negative),
        Err(AnimationError::InvalidDuration)
    ));
    let stale = text.replace("\"active\"", "\"active_ignored\"");
    let cleared = AnimationProject::from_json(&stale);
    if let Ok(cleared) = cleared {
        assert_eq!(cleared.active_clip(), None);
    }
}

#[test]
fn playback_plays_stops_loops_and_restarts_at_the_end() {
    let mut playback = Playback::new();
    assert!(!playback.playing);
    playback.play(1.0, false);
    playback.tick(0.25, 1.0, false);
    near(playback.time, 0.25);
    playback.pause();
    playback.tick(1.0, 1.0, false);
    near(playback.time, 0.25);
    playback.play(1.0, false);
    playback.tick(1.0, 1.0, false);
    near(playback.time, 1.0);
    assert!(!playback.playing);
    playback.play(1.0, false);
    near(playback.time, 0.0);
    assert!(playback.playing);

    playback.time = 0.8;
    playback.playing = true;
    playback.tick(0.5, 1.0, true);
    near(playback.time, 0.3);
    assert!(playback.playing);
    playback.time = 0.0;
    playback.tick(2.5, 1.0, true);
    near(playback.time, 0.5);
    playback.time = 0.0;
    playback.tick(1.0, 1.0, true);
    near(playback.time, 0.0);

    playback.seek(5.0, 1.0);
    near(playback.time, 1.0);
    assert!(playback.playing);
    playback.seek(-1.0, 1.0);
    near(playback.time, 0.0);
    playback.seek(0.4, 0.0);
    near(playback.time, 0.0);
    playback.time = 5.0;
    playback.playing = true;
    playback.tick(0.1, 0.0, false);
    near(playback.time, 0.0);
    assert!(playback.playing);
    playback.stop();
    near(playback.time, 0.0);
    assert!(!playback.playing);
    playback.play(1.0, false);
    playback.tick(-1.0, 1.0, false);
    near(playback.time, 0.0);
}

#[test]
fn timeline_maps_time_zooms_around_the_cursor_and_hits_keys() {
    let mut view = TimelineView::new();
    assert_eq!(view.time_to_x(1.0, 10.0), 10.0 + view.pixels_per_second);
    assert_eq!(view.x_to_time(view.time_to_x(0.4, 10.0), 10.0), 0.4);
    view.pixels_per_second = 100.0;
    view.zoom_at(2.0, 1.0, 0.0, 200.0);
    near(view.pixels_per_second, 200.0);
    near(view.origin, 0.5);
    near(view.time_to_x(1.0, 0.0), 100.0);
    view.origin = 0.0;
    view.pixels_per_second = 100.0;
    view.pan(50.0);
    near(view.origin, 0.0);
    view.pan(-50.0);
    near(view.origin, 0.5);
    view.zoom_at(0.0001, 0.0, 0.0, 100.0);
    assert!(view.pixels_per_second >= 20.0);
    view.zoom_at(1000.0, 0.0, 0.0, 100.0);
    assert!(view.pixels_per_second <= 2000.0);

    let view = TimelineView {
        origin: 0.0,
        pixels_per_second: 100.0,
    };
    let rows = [TimelineRow {
        node: NodeId(1),
        property: MotionProperty::X,
        y: 10.0,
        height: 20.0,
    }];
    let keys = [KeyRef {
        id: uii_animation::KeyframeId(7),
        node: NodeId(1),
        property: MotionProperty::X,
        time: 1.0,
    }];
    let hit = hit_keyframe(100.0, 20.0, &rows, &keys, &view, 0.0, 6.0).unwrap();
    assert_eq!(
        hit,
        KeyframeHit {
            keyframe: uii_animation::KeyframeId(7),
            node: NodeId(1),
            property: MotionProperty::X,
        }
    );
    assert!(hit_keyframe(100.0, 40.0, &rows, &keys, &view, 0.0, 6.0).is_none());
    assert!(hit_keyframe(130.0, 20.0, &rows, &keys, &view, 0.0, 6.0).is_none());
}

#[test]
fn timeline_rows_follow_the_active_clip_and_the_selection() {
    let mut project = AnimationProject::new();
    let node = project.add_target("Header");
    assert!(timeline_rows(&project, None, 18.0).is_empty());
    let rows = timeline_rows(&project, Some(node), 18.0);
    assert_eq!(rows.len(), MotionProperty::all().len());
    assert_eq!(rows[0].property, MotionProperty::X);
    assert_eq!(rows[0].y, 0.0);
    assert_eq!(rows[1].y, 18.0);

    let clip = project.add_clip("Intro");
    let other = project.add_target("Body");
    project
        .upsert_keyframe(
            clip,
            other,
            MotionProperty::Opacity,
            0.0,
            1.0,
            Easing::Linear,
        )
        .unwrap();
    let rows = timeline_rows(&project, Some(node), 18.0);
    assert_eq!(rows.len(), MotionProperty::all().len() + 1);
    assert_eq!(rows.last().unwrap().node, other);
    assert_eq!(rows.last().unwrap().property, MotionProperty::Opacity);
    let only = timeline_rows(&project, None, 18.0);
    assert_eq!(only.len(), 1);
    assert_eq!(only[0].node, other);
    let refs = active_key_refs(&project);
    assert_eq!(refs.len(), 1);
    assert_eq!(refs[0].node, other);
}

#[test]
fn preview_fit_and_clockwise_rotation() {
    let node = LayoutNode::rest(NodeId(1), "Card");
    let mut placed = node.clone();
    placed.x = 0.0;
    placed.y = 0.0;
    placed.width = 100.0;
    placed.height = 50.0;
    let fit = fit_preview(&[placed.clone()], 200.0, 100.0, 10.0).unwrap();
    near(fit.scale, 1.6);
    near(fit.origin_x, 20.0);
    near(fit.origin_y, 10.0);
    assert!(fit_preview(&[placed.clone()], 10.0, 10.0, 10.0).is_none());
    assert!(fit_preview(&[], 200.0, 100.0, 10.0).is_none());

    let corners = rotated_corners(0.0, 0.0, 100.0, 50.0, 0.0);
    near(corners[0][0], 0.0);
    near(corners[0][1], 0.0);
    near(corners[1][0], 100.0);
    near(corners[1][1], 0.0);
    near(corners[2][0], 100.0);
    near(corners[2][1], 50.0);
    near(corners[3][0], 0.0);
    near(corners[3][1], 50.0);
    let turned = rotated_corners(0.0, 0.0, 100.0, 50.0, 90.0);
    near(turned[0][0], 25.0);
    near(turned[0][1], 75.0);
    let _ = node;
}

#[test]
fn tab_undo_redo_ticks_the_active_clip_and_poses_the_layout() {
    let mut tab = AnimationTab::new();
    assert!(!tab.undo());
    tab.edit(|project| {
        project.add_clip("Intro");
    });
    assert!(tab.take_dirty());
    assert!(!tab.take_dirty());
    assert_eq!(tab.project.clips().len(), 1);
    assert!(tab.undo());
    assert!(tab.project.clips().is_empty());
    assert!(tab.redo());
    assert_eq!(tab.project.clips()[0].name(), "Intro");

    let clip = tab.project.clips()[0].id();
    let node = tab.edit_with(|project| project.add_target("Header"));
    tab.edit(|project| {
        project
            .upsert_keyframe(clip, node, MotionProperty::X, 0.0, 0.0, Easing::Linear)
            .unwrap();
        project
            .upsert_keyframe(clip, node, MotionProperty::X, 1.0, 100.0, Easing::Linear)
            .unwrap();
    });
    tab.playback.play(1.0, false);
    tab.tick(0.5);
    near(tab.playback.time, 0.5);
    let posed = tab.posed_layout();
    assert_eq!(posed[0].x, 50.0);
    assert_eq!(posed[0].y, 32.0);
    tab.edit(|project| {
        project.set_clip_loop(clip, true).unwrap();
    });
    tab.playback.time = 0.8;
    tab.playback.playing = true;
    tab.tick(0.5);
    near(tab.playback.time, 0.3);
}

#[test]
fn animation_tab_draws_a_frame_without_a_window() {
    let mut tab = AnimationTab::new();
    tab.edit(|project| {
        project.add_clip("Intro");
        project.add_target("Header");
    });
    let mut ctx = dear_imgui_rs::Context::create();
    ctx.io_mut().set_display_size([1280.0, 720.0]);
    ctx.io_mut().set_delta_time(1.0 / 60.0);
    ctx.font_atlas()
        .try_claim_legacy_renderer()
        .expect("legacy font atlas")
        .build();
    let ui = ctx.frame();
    ui.window("Animation")
        .size([1280.0, 720.0], dear_imgui_rs::Condition::Always)
        .position([0.0, 0.0], dear_imgui_rs::Condition::Always)
        .build(|| {
            draw_animation_tab(ui, &mut tab);
        });
    assert_eq!(tab.project.clips().len(), 1);
    assert_eq!(tab.project.targets().len(), 1);
}
