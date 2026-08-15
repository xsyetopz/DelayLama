//! Compile-time artwork bundle and asset completeness checks.
/// The artwork contract used by the original editor.
#[derive(Clone, Copy, Debug)]
/// Embedded artwork files required by the original editor.
pub struct Artwork {
    /// Embedded PNG bytes for the source surface asset.
    pub source_surface: &'static [u8],
    /// Embedded PNG bytes for the scene background asset.
    pub scene_background: &'static [u8],
    /// Embedded PNG bytes for the control panel asset.
    pub control_panel: &'static [u8],
    /// Embedded PNG bytes for the monk sprite sheet asset.
    pub monk_sprite_sheet: &'static [u8],
    /// Embedded PNG bytes for the knob strip a asset.
    pub knob_strip_a: &'static [u8],
    /// Embedded PNG bytes for the knob strip b asset.
    pub knob_strip_b: &'static [u8],
    /// Embedded PNG bytes for the ui arrow asset.
    pub ui_arrow: &'static [u8],
    /// Embedded PNG bytes for the ui tile a asset.
    pub ui_tile_a: &'static [u8],
    /// Embedded PNG bytes for the ui tile b asset.
    pub ui_tile_b: &'static [u8],
    /// Embedded PNG bytes for the help panel asset.
    pub help_panel: &'static [u8],
}

impl Artwork {
    /// Complete artwork bundle loaded from the repository assets.
    pub const ORIGINAL: Self = Self {
        source_surface: include_bytes!("../../../assets/source_surface.png"),
        scene_background: include_bytes!("../../../assets/scene_background.png"),
        control_panel: include_bytes!("../../../assets/control_panel.png"),
        monk_sprite_sheet: include_bytes!("../../../assets/monk_sprite_sheet.png"),
        knob_strip_a: include_bytes!("../../../assets/knob_strip_a.png"),
        knob_strip_b: include_bytes!("../../../assets/knob_strip_b.png"),
        ui_arrow: include_bytes!("../../../assets/ui_arrow.png"),
        ui_tile_a: include_bytes!("../../../assets/ui_tile_a.png"),
        ui_tile_b: include_bytes!("../../../assets/ui_tile_b.png"),
        help_panel: include_bytes!("../../../assets/help_panel.png"),
    };

    /// Returns whether every embedded asset contains data.
    pub const fn is_complete(self) -> bool {
        !self.source_surface.is_empty()
            && !self.scene_background.is_empty()
            && !self.control_panel.is_empty()
            && !self.monk_sprite_sheet.is_empty()
            && !self.knob_strip_a.is_empty()
            && !self.knob_strip_b.is_empty()
            && !self.ui_arrow.is_empty()
            && !self.ui_tile_a.is_empty()
            && !self.ui_tile_b.is_empty()
            && !self.help_panel.is_empty()
    }
}
