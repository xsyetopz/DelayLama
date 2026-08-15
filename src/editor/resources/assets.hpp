#ifndef DELAYLAMA_EDITOR_RESOURCES_ASSETS_HPP
#define DELAYLAMA_EDITOR_RESOURCES_ASSETS_HPP

#include <span>

/// Embedded editor artwork.
namespace delay_lama_assets {

    /// PNG bytes for the editor's control-panel surface.
    extern const std::span<const unsigned char> control_panel_png;
    /// PNG bytes for the quick-help/about panel.
    extern const std::span<const unsigned char> help_panel_png;
    /// PNG bytes for the monk atlas.
    extern const std::span<const unsigned char> monk_sprite_sheet_png;
    /// PNG bytes for the first rotary-control strip.
    extern const std::span<const unsigned char> knob_strip_a_png;
    /// PNG bytes for the second rotary-control strip.
    extern const std::span<const unsigned char> knob_strip_b_png;
    /// PNG bytes for the upper-surface background.
    extern const std::span<const unsigned char> scene_background_png;
    /// PNG bytes for the first tiled editor-surface texture.
    extern const std::span<const unsigned char> ui_tile_a_png;
    /// PNG bytes for the second tiled editor-surface texture.
    extern const std::span<const unsigned char> ui_tile_b_png;
    /// PNG bytes for the editor's directional arrow.
    extern const std::span<const unsigned char> ui_arrow_png;

}

#endif
