#include "assets.hpp"

#include <cstddef>
#include <span>

namespace {

    template<std::size_t extent>
    consteval auto png_view(const unsigned char (&bytes)[extent]) noexcept
        -> std::span<const unsigned char> {
        return std::span<const unsigned char> {bytes, extent - 1U};
    }

    const unsigned char control_panel_png_data[] = {
#include "control_panel.png.h"
    };
    const unsigned char help_panel_png_data[] = {
#include "help_panel.png.h"
    };
    const unsigned char monk_sprite_sheet_png_data[] = {
#include "monk_sprite_sheet.png.h"
    };
    const unsigned char knob_strip_a_png_data[] = {
#include "knob_strip_a.png.h"
    };
    const unsigned char knob_strip_b_png_data[] = {
#include "knob_strip_b.png.h"
    };
    const unsigned char scene_background_png_data[] = {
#include "scene_background.png.h"
    };
    const unsigned char ui_tile_a_png_data[] = {
#include "ui_tile_a.png.h"
    };
    const unsigned char ui_tile_b_png_data[] = {
#include "ui_tile_b.png.h"
    };
    const unsigned char ui_arrow_png_data[] = {
#include "ui_arrow.png.h"
    };

}
namespace delay_lama_assets {
    constexpr std::span<const unsigned char> control_panel_png = png_view(control_panel_png_data);
    constexpr std::span<const unsigned char> help_panel_png = png_view(help_panel_png_data);
    constexpr std::span<const unsigned char> monk_sprite_sheet_png =
        png_view(monk_sprite_sheet_png_data);
    constexpr std::span<const unsigned char> knob_strip_a_png = png_view(knob_strip_a_png_data);
    constexpr std::span<const unsigned char> knob_strip_b_png = png_view(knob_strip_b_png_data);
    constexpr std::span<const unsigned char> scene_background_png =
        png_view(scene_background_png_data);
    constexpr std::span<const unsigned char> ui_tile_a_png = png_view(ui_tile_a_png_data);
    constexpr std::span<const unsigned char> ui_tile_b_png = png_view(ui_tile_b_png_data);
    constexpr std::span<const unsigned char> ui_arrow_png = png_view(ui_arrow_png_data);

}
