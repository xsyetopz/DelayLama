//! Compile-time artwork bundle and asset completeness checks.

const REFERENCE_ASSETS: [&[u8]; 3] = [
    include_bytes!("../../../assets/source_surface.qoi"),
    include_bytes!("../../../assets/ui_tile_a.qoi"),
    include_bytes!("../../../assets/ui_tile_b.qoi"),
];

/// Scene artwork used by the editor surface.
#[derive(Clone, Copy, Debug)]
pub(super) struct SceneArtwork {
    /// Embedded QOI bytes for the scene background asset.
    pub(super) background: &'static [u8],
    /// Embedded QOI bytes for the monk sprite sheet asset.
    pub(super) monk_sprite_sheet: &'static [u8],
}

/// Main editor-surface artwork.
#[derive(Clone, Copy, Debug)]
pub(super) struct SurfaceArtwork {
    /// Scene artwork drawn above the source surface.
    pub(super) scene: SceneArtwork,
    /// Embedded QOI bytes for the control panel asset.
    pub(super) control_panel: &'static [u8],
}

/// Artwork for the editor's rotary controls.
#[derive(Clone, Copy, Debug)]
pub(super) struct ControlArtwork {
    /// Embedded QOI strips for the two rotary-control styles.
    pub(super) knob_strips: [&'static [u8]; 2],
}

/// Artwork layered over the editor's main controls.
#[derive(Clone, Copy, Debug)]
pub(super) struct InterfaceArtwork {
    /// Embedded QOI bytes for the UI arrow asset.
    pub(super) arrow: &'static [u8],
    /// Embedded QOI bytes for the help panel asset.
    pub(super) help_panel: &'static [u8],
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum TextureSlot {
    Scene,
    Monk,
    Panel,
    PortamentoKnob,
    VoiceKnob,
    Arrow,
    Help,
}

impl TextureSlot {
    pub(super) const fn index(self) -> usize {
        match self {
            Self::Scene => 0,
            Self::Monk => 1,
            Self::Panel => 2,
            Self::PortamentoKnob => 3,
            Self::VoiceKnob => 4,
            Self::Arrow => 5,
            Self::Help => 6,
        }
    }
}

/// Embedded image files used by the asset editor.
#[derive(Clone, Copy, Debug)]
pub(super) struct Artwork {
    /// Artwork belonging to the main editor surface.
    pub(super) surface: SurfaceArtwork,
    /// Artwork belonging to rotary controls.
    pub(super) controls: ControlArtwork,
    /// Artwork layered over the controls.
    pub(super) interface: InterfaceArtwork,
}

impl Artwork {
    /// Complete artwork bundle loaded from the repository assets.
    pub(super) const EMBEDDED: Self = Self {
        surface: SurfaceArtwork {
            scene: SceneArtwork {
                background: include_bytes!("../../../assets/scene_background.qoi"),
                monk_sprite_sheet: include_bytes!("../../../assets/monk_sprite_sheet.qoi"),
            },
            control_panel: include_bytes!("../../../assets/control_panel.qoi"),
        },
        controls: ControlArtwork {
            knob_strips: [
                include_bytes!("../../../assets/knob_strip_a.qoi"),
                include_bytes!("../../../assets/knob_strip_b.qoi"),
            ],
        },
        interface: InterfaceArtwork {
            arrow: include_bytes!("../../../assets/ui_arrow.qoi"),
            help_panel: include_bytes!("../../../assets/help_panel.qoi"),
        },
    };

    pub(super) const fn reference_assets_complete() -> bool {
        let [source_surface, ui_tile_a, ui_tile_b] = REFERENCE_ASSETS;
        !source_surface.is_empty() && !ui_tile_a.is_empty() && !ui_tile_b.is_empty()
    }

    pub(super) const fn rendered_assets(self) -> [(TextureSlot, &'static [u8]); 7] {
        let [portamento_knob, voice_knob] = self.controls.knob_strips;
        [
            (TextureSlot::Scene, self.surface.scene.background),
            (TextureSlot::Monk, self.surface.scene.monk_sprite_sheet),
            (TextureSlot::Panel, self.surface.control_panel),
            (TextureSlot::PortamentoKnob, portamento_knob),
            (TextureSlot::VoiceKnob, voice_knob),
            (TextureSlot::Arrow, self.interface.arrow),
            (TextureSlot::Help, self.interface.help_panel),
        ]
    }
}
