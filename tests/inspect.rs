//! Inspect shows the selected item's size, color, and type.
//! The tool is not a tab. These tests are the contract for that readout.

use uii::{
    Color, InspectReadout, InspectState, InspectTool, ItemSize, ItemSizeError, ItemType,
    ItemTypeError, SelectedItem,
};

fn frame_item() -> SelectedItem {
    SelectedItem::new(
        ItemSize::try_new(128.0, 32.5).expect("size"),
        Color::rgb(0x3B, 0x82, 0xF6),
        ItemType::try_new("Frame").expect("type"),
    )
}

#[test]
fn inspect_is_a_tool_not_a_tab() {
    assert_eq!(InspectTool::NAME, "Inspect");
    assert!(InspectTool::is_tool());
    assert!(!InspectTool::is_tab());
    assert_eq!(InspectTool::EMPTY_SELECTION, "Nothing selected");
    assert_eq!(InspectTool::SIZE_LABEL, "Size");
    assert_eq!(InspectTool::COLOR_LABEL, "Color");
    assert_eq!(InspectTool::TYPE_LABEL, "Type");
}

#[test]
fn size_is_width_times_height() {
    let size = ItemSize::try_new(128.0, 32.5).expect("size");
    assert_eq!(size.width(), 128.0);
    assert_eq!(size.height(), 32.5);
    assert_eq!(size.text(), "128 × 32.5");

    assert_eq!(
        ItemSize::try_new(128.0, 32.0).expect("size").text(),
        "128 × 32"
    );
    assert_eq!(
        ItemSize::try_new(1.25, 0.5).expect("size").text(),
        "1.25 × 0.5"
    );
    assert_eq!(
        ItemSize::try_new(0.01, 0.1).expect("size").text(),
        "0.01 × 0.1"
    );
    assert_eq!(ItemSize::try_new(0.0, 0.0).expect("size").text(), "0 × 0");
}

#[test]
fn size_rejects_non_finite_and_negative_values() {
    assert_eq!(ItemSize::try_new(-1.0, 1.0), Err(ItemSizeError::Negative));
    assert_eq!(ItemSize::try_new(1.0, -0.001), Err(ItemSizeError::Negative));
    assert_eq!(
        ItemSize::try_new(f32::NAN, 1.0),
        Err(ItemSizeError::NotFinite)
    );
    assert_eq!(
        ItemSize::try_new(1.0, f32::INFINITY),
        Err(ItemSizeError::NotFinite)
    );
    assert_eq!(
        ItemSize::try_new(f32::NEG_INFINITY, f32::NAN),
        Err(ItemSizeError::NotFinite)
    );
}

#[test]
fn color_is_hex_and_keeps_alpha_when_translucent() {
    let opaque = Color::rgb(0x3B, 0x82, 0xF6);
    assert_eq!(opaque.text(), "#3B82F6");
    assert_eq!(opaque.channels(), [0x3B, 0x82, 0xF6, 255]);

    assert_eq!(Color::rgb(0x0A, 0x0B, 0x0C).text(), "#0A0B0C");
    assert_eq!(Color::rgba(0, 0, 0, 255).text(), "#000000");
    assert_eq!(Color::rgba(255, 0, 0, 128).text(), "#FF000080");
    assert_eq!(Color::rgba(0, 0, 0, 0).text(), "#00000000");

    let channels = Color::rgba(1, 2, 3, 4);
    assert_eq!(channels.channels(), [1, 2, 3, 4]);
    assert_eq!(
        channels.imgui(),
        [1.0 / 255.0, 2.0 / 255.0, 3.0 / 255.0, 4.0 / 255.0]
    );
}

#[test]
fn type_trims_and_rejects_empty() {
    assert_eq!(
        ItemType::try_new("  Frame  ").expect("type").as_str(),
        "Frame"
    );
    assert_eq!(ItemType::try_new(""), Err(ItemTypeError::Empty));
    assert_eq!(ItemType::try_new("   "), Err(ItemTypeError::Empty));
}

#[test]
fn readout_shows_size_color_and_type_of_the_selected_item() {
    let item = frame_item();
    let readout = InspectReadout::from_item(&item);

    assert_eq!(item.size().width(), 128.0);
    assert_eq!(item.size().height(), 32.5);
    assert_eq!(item.size().text(), "128 × 32.5");
    assert_eq!(item.color().channels(), [0x3B, 0x82, 0xF6, 255]);
    assert_eq!(item.kind().as_str(), "Frame");

    assert_eq!(readout.size_text(), item.size().text());
    assert_eq!(readout.color_text(), item.color().text());
    assert_eq!(readout.type_text(), item.kind().as_str());
    assert_eq!(readout.color_rgba(), item.color().channels());
    assert_eq!(readout.color_imgui(), item.color().imgui());
}

#[test]
fn no_selection_has_no_readout() {
    let state = InspectState::new();
    assert!(state.selection().is_none());
    assert!(state.readout().is_none());

    let mut state = InspectState::new();
    let item = frame_item();
    state.set_selection(Some(item.clone()));
    assert_eq!(state.selection(), Some(&item));
    let readout = state.readout().expect("readout");
    assert_eq!(readout.size_text(), "128 × 32.5");
    assert_eq!(readout.color_text(), "#3B82F6");
    assert_eq!(readout.type_text(), "Frame");

    state.set_selection(None);
    assert!(state.selection().is_none());
    assert!(state.readout().is_none());
}

#[test]
fn inspect_draws_for_a_selection_and_for_none() {
    let mut ctx = dear_imgui_rs::Context::create();
    ctx.io_mut().set_display_size([640.0, 480.0]);
    ctx.io_mut().set_delta_time(1.0 / 60.0);
    ctx.font_atlas()
        .try_claim_legacy_renderer()
        .expect("legacy font atlas")
        .build();

    let mut state = InspectState::new();
    frame_draw(&mut ctx, &state);

    state.set_selection(Some(frame_item()));
    frame_draw(&mut ctx, &state);
}

fn frame_draw(ctx: &mut dear_imgui_rs::Context, state: &InspectState) {
    ctx.io_mut().set_display_size([640.0, 480.0]);
    ctx.io_mut().set_delta_time(1.0 / 60.0);
    {
        let ui = ctx.frame();
        state.draw(ui);
    }
    let _frame = ctx.render_legacy();
}
