//! Embedded-artwork contract tests.

fn qoi_dimensions(bytes: &[u8]) -> Option<(u32, u32)> {
    let mut decoder = qoi::Decoder::new(bytes)
        .ok()?
        .with_channels(qoi::Channels::Rgba);
    let dimensions = (decoder.header().width, decoder.header().height);
    let pixel_count = dimensions.0.checked_mul(dimensions.1)?.checked_mul(4)?;
    let decoded_length = usize::try_from(pixel_count).ok()?;
    (decoder.decode_to_vec().ok()?.len() == decoded_length).then_some(dimensions)
}

#[test]
fn asset_pixel_dimensions_are_preserved() {
    let assets: [(&[u8], (u32, u32)); 10] = [
        (include_bytes!("../assets/source_surface.qoi"), (360, 510)),
        (include_bytes!("../assets/scene_background.qoi"), (360, 311)),
        (include_bytes!("../assets/control_panel.qoi"), (360, 220)),
        (
            include_bytes!("../assets/monk_sprite_sheet.qoi"),
            (1570, 1866),
        ),
        (include_bytes!("../assets/knob_strip_a.qoi"), (50, 3000)),
        (include_bytes!("../assets/knob_strip_b.qoi"), (50, 3000)),
        (include_bytes!("../assets/ui_arrow.qoi"), (20, 17)),
        (include_bytes!("../assets/ui_tile_a.qoi"), (10, 10)),
        (include_bytes!("../assets/ui_tile_b.qoi"), (10, 10)),
        (include_bytes!("../assets/help_panel.qoi"), (253, 275)),
    ];
    for (bytes, dimensions) in assets {
        assert_eq!(qoi_dimensions(bytes), Some(dimensions));
    }
}
