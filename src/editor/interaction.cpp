#include "interaction.hpp"

#include <algorithm>
#include <cmath>
#include <cstddef>

#include "editor/state.hpp"

namespace {

    // Local bounds prevent adapters from defining a competing coordinate protocol.
    constexpr auto normalized_min = 0.0F;
    constexpr auto normalized_max = 1.0F;
    constexpr auto atlas_frame_count = 30U;
    constexpr auto active_atlas_offset = 0.2F;
    constexpr auto active_atlas_vowel_scale = 0.8F;
    constexpr auto nearest_frame_offset = 0.5F;
    constexpr auto release_atlas_selector = 5.0F / 30.0F;

    constexpr auto quantize_atlas_selector(const float selector) noexcept -> std::size_t {
        return std::min(
            static_cast<std::size_t>(
                (selector * static_cast<float>(atlas_frame_count - 1U)) + nearest_frame_offset),
            static_cast<std::size_t>(atlas_frame_count - 1U));
    }

}

namespace delaylama::host {

    auto EditorModel::sanitise_visual_state(VisualState state) noexcept -> VisualState {
        state.vowel = std::isfinite(state.vowel)
                          ? std::clamp(state.vowel, normalized_min, normalized_max)
                          : default_pad_vowel;
        if (!state.gate || state.note < internal_min_note || state.note > internal_max_note) {
            state.note = inactive_pad_note;
            state.gate = false;
        }
        if (!std::isfinite(state.atlas_selector) || state.atlas_selector < normalized_min) {
            state.atlas_selector =
                state.gate ? active_atlas_offset + (active_atlas_vowel_scale * state.vowel)
                           : release_atlas_selector;
        } else {
            state.atlas_selector = std::clamp(state.atlas_selector, normalized_min, normalized_max);
        }
        return state;
    }

    auto EditorModel::select_visual_state(
        const int note,
        const bool gate,
        const float vowel) noexcept -> std::size_t {
        const auto state =
            sanitise_visual_state(VisualState {.note = note, .gate = gate, .vowel = vowel});
        return quantize_atlas_selector(state.atlas_selector);
    }

    auto EditorModel::apply_external_state(const VisualState state) noexcept -> void {
        const auto sanitised = sanitise_visual_state(state);
        visual_state_ = quantize_atlas_selector(sanitised.atlas_selector);
    }

    auto EditorModel::handle_gesture(
        const float position_x,
        const float position_y,
        const PadGesture gesture) noexcept -> GestureResult {
        const auto clamped_x = std::clamp(position_x, normalized_min, normalized_max);
        const auto clamped_y = std::clamp(position_y, normalized_min, normalized_max);

        auto result = GestureResult {
            .position_x = clamped_x,
            .position_y = clamped_y,
            .vowel = normalized_max - clamped_y,
            .note = pad_host_note,
            .note_on_note = inactive_pad_note,
            .note_off_note = inactive_pad_note,
            .note_on = false,
            .note_off = false};

        if (gesture == PadGesture::Down) {
            result.note_on_note = pad_host_note;
            result.note_on = true;
            pad_gate_ = true;
        } else if (gesture == PadGesture::Up) {
            result.note_off_note = pad_host_note;
            result.note_off = true;
            pad_gate_ = false;
        }

        active_pad_vowel_ = result.vowel;
        apply_external_state(
            VisualState {.note = pad_internal_note, .gate = pad_gate_, .vowel = active_pad_vowel_});
        return result;
    }

    auto EditorModel::animation_frame() const noexcept -> std::size_t {
        return visual_state_;
    }

}
