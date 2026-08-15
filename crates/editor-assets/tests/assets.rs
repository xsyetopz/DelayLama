use delaylama_editor_assets::Artwork;

fn png_dimensions(bytes: &[u8]) -> Option<(u32, u32)> {
    let width = u32::from_be_bytes(bytes.get(16..20)?.try_into().ok()?);
    let height = u32::from_be_bytes(bytes.get(20..24)?.try_into().ok()?);
    Some((width, height))
}

#[test]
fn original_asset_pixel_dimensions_are_preserved() {
    let artwork = Artwork::ORIGINAL;
    assert_eq!(png_dimensions(artwork.source_surface), Some((360, 510)));
    assert_eq!(png_dimensions(artwork.scene_background), Some((360, 311)));
    assert_eq!(png_dimensions(artwork.control_panel), Some((360, 220)));
    assert_eq!(
        png_dimensions(artwork.monk_sprite_sheet),
        Some((1570, 1866))
    );
    assert_eq!(png_dimensions(artwork.knob_strip_a), Some((50, 3000)));
    assert_eq!(png_dimensions(artwork.knob_strip_b), Some((50, 3000)));
    assert_eq!(png_dimensions(artwork.ui_arrow), Some((20, 17)));
    assert_eq!(png_dimensions(artwork.help_panel), Some((253, 275)));
}
