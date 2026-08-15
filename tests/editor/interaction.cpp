#include "editor/interaction.hpp"

#include <cstdint>
#include <iostream>
#include <limits>

#include "editor/state.hpp"

namespace {

    constexpr auto pad_left = 0.0F;
    constexpr auto pad_right = 1.0F;
    constexpr auto pad_center = 0.5F;
    constexpr auto vowel_midpoint_pad_y = 0.25F;
    constexpr auto pad_y_for_low_vowel = 1.0F;
    constexpr auto pad_y_for_high_vowel = 0.0F;
    constexpr auto fixed_pad_host_note = 40;
    constexpr auto fixed_pad_internal_note = 28;
    constexpr auto internal_low_note = 4;
    constexpr auto internal_high_note = 72;
    constexpr auto no_note = -1;
    constexpr auto idle_visual_state = 5U;
    constexpr auto fixed_pad_low_vowel_state = 6U;
    constexpr auto fixed_pad_mid_vowel_state = 23U;
    constexpr auto fixed_pad_high_vowel_state = 29U;
    constexpr auto internal_default_vowel_state = 17U;
    constexpr auto idle_frame = 3U;
    constexpr auto atlas_frame_count = 30.0F;
    constexpr auto idle_selector = static_cast<float>(idle_frame) / atlas_frame_count;
    constexpr auto default_vowel = 0.5F;
    constexpr auto vowel_above_maximum = 2.0F;
    constexpr auto down_failure_exit_code = 1;
    constexpr auto drag_failure_exit_code = 2;
    constexpr auto up_failure_exit_code = 3;
    constexpr auto idle_failure_exit_code = 4;
    constexpr auto release_state_failure_exit_code = 6;
    constexpr auto external_state_failure_exit_code = 7;
    constexpr auto success_exit_code = 0;

    auto external_state_contract() -> bool {
        using delaylama::host::EditorModel;
        using delaylama::host::VisualState;

        EditorModel model;
        model.apply_external_state(
            VisualState {.note = fixed_pad_internal_note, .gate = true, .vowel = 0.0F});
        if (model.animation_frame() != fixed_pad_low_vowel_state
            || model.animation_frame()
                   != EditorModel::select_visual_state(fixed_pad_internal_note, true, 0.0F)) {
            return false;
        }

        EditorModel equivalent_model;
        equivalent_model.apply_external_state(
            VisualState {.note = fixed_pad_internal_note, .gate = true, .vowel = 0.0F});
        if (equivalent_model.animation_frame() != model.animation_frame()) {
            return false;
        }

        model.apply_external_state(
            VisualState {.note = fixed_pad_internal_note, .gate = true, .vowel = -1.0F});
        if (model.animation_frame() != fixed_pad_low_vowel_state) {
            return false;
        }
        model.apply_external_state(
            VisualState {
                .note = fixed_pad_internal_note,
                .gate = true,
                .vowel = vowel_above_maximum});
        if (model.animation_frame() != fixed_pad_high_vowel_state) {
            return false;
        }
        model.apply_external_state(
            VisualState {
                .note = fixed_pad_internal_note,
                .gate = true,
                .vowel = std::numeric_limits<float>::quiet_NaN()});
        if (model.animation_frame() != internal_default_vowel_state) {
            return false;
        }

        model.apply_external_state(
            VisualState {.note = internal_low_note, .gate = true, .vowel = 0.0F});
        const auto low_note_frame = model.animation_frame();
        if (low_note_frame != fixed_pad_low_vowel_state
            || low_note_frame != EditorModel::select_visual_state(internal_low_note, true, 0.0F)) {
            return false;
        }
        model.apply_external_state(
            VisualState {.note = internal_high_note, .gate = true, .vowel = 0.0F});
        if (model.animation_frame() != low_note_frame
            || model.animation_frame()
                   != EditorModel::select_visual_state(internal_high_note, true, 0.0F)) {
            return false;
        }

        model.apply_external_state(
            VisualState {
                .note = no_note,
                .gate = false,
                .vowel = default_vowel,
                .atlas_selector = idle_selector});
        if (model.animation_frame() != idle_frame) {
            return false;
        }

        model.apply_external_state(
            VisualState {.note = internal_low_note - 1, .gate = true, .vowel = default_vowel});
        if (model.animation_frame() != idle_visual_state) {
            return false;
        }
        model.apply_external_state(
            VisualState {.note = internal_high_note + 1, .gate = true, .vowel = default_vowel});
        return model.animation_frame() == idle_visual_state;
    }

    auto release_contract() -> bool {
        using delaylama::host::EditorModel;
        using delaylama::host::VisualState;

        EditorModel model;
        model.apply_external_state(
            VisualState {.note = fixed_pad_internal_note, .gate = true, .vowel = default_vowel});
        if (model.animation_frame() == idle_visual_state) {
            return false;
        }
        model.apply_external_state(
            VisualState {.note = fixed_pad_internal_note, .gate = false, .vowel = 1.0F});
        if (model.animation_frame() != idle_visual_state) {
            return false;
        }
        model.apply_external_state(
            VisualState {.note = fixed_pad_internal_note, .gate = false, .vowel = 0.0F});
        return model.animation_frame() == idle_visual_state;
    }

    auto pointer_state_is_independent() -> bool {
        using delaylama::host::EditorModel;
        using delaylama::host::PadGesture;
        using delaylama::host::VisualState;

        EditorModel model;
        model.apply_external_state(
            VisualState {.note = fixed_pad_internal_note, .gate = true, .vowel = default_vowel});
        const auto result = model.handle_gesture(pad_left, pad_y_for_low_vowel, PadGesture::Down);
        return result.note == fixed_pad_host_note && result.note_on
               && result.note_on_note == fixed_pad_host_note && !result.note_off;
    }

}

auto main() -> std::int32_t {
    using delaylama::host::EditorModel;
    using delaylama::host::PadGesture;

    EditorModel model;

    const auto initial_idle_state = model.animation_frame();
    const auto idle_up = model.handle_gesture(pad_center, vowel_midpoint_pad_y, PadGesture::Up);
    if (initial_idle_state != idle_visual_state || model.animation_frame() != initial_idle_state
        || idle_up.note != fixed_pad_host_note || idle_up.note_on || idle_up.note_on_note != no_note
        || !idle_up.note_off || idle_up.note_off_note != fixed_pad_host_note) {
        return idle_failure_exit_code;
    }

    const auto down = model.handle_gesture(pad_left, vowel_midpoint_pad_y, PadGesture::Down);
    if (down.note != fixed_pad_host_note || !down.note_on
        || down.note_on_note != fixed_pad_host_note || down.note_off
        || down.note_off_note != no_note || model.animation_frame() != fixed_pad_mid_vowel_state
        || model.animation_frame()
               != EditorModel::select_visual_state(
                   fixed_pad_internal_note,
                   true,
                   1.0F - vowel_midpoint_pad_y)) {
        return down_failure_exit_code;
    }

    const auto drag = model.handle_gesture(pad_right, pad_y_for_low_vowel, PadGesture::Drag);
    if (drag.note != fixed_pad_host_note || drag.note_on || drag.note_on_note != no_note
        || drag.note_off || drag.note_off_note != no_note
        || model.animation_frame() != fixed_pad_low_vowel_state) {
        return drag_failure_exit_code;
    }

    const auto high_vowel_drag =
        model.handle_gesture(pad_right, pad_y_for_high_vowel, PadGesture::Drag);
    if (high_vowel_drag.note != fixed_pad_host_note || high_vowel_drag.note_on
        || high_vowel_drag.note_on_note != no_note || high_vowel_drag.note_off
        || high_vowel_drag.note_off_note != no_note
        || model.animation_frame() != fixed_pad_high_vowel_state) {
        return drag_failure_exit_code;
    }

    const auto up = model.handle_gesture(pad_right, vowel_midpoint_pad_y, PadGesture::Up);
    if (up.note != fixed_pad_host_note || up.note_on || up.note_on_note != no_note || !up.note_off
        || up.note_off_note != fixed_pad_host_note
        || model.animation_frame() != idle_visual_state) {
        return up_failure_exit_code;
    }

    if (!external_state_contract() || !pointer_state_is_independent()) {
        return external_state_failure_exit_code;
    }
    if (!release_contract()) {
        return release_state_failure_exit_code;
    }

    const auto released_idle_up =
        model.handle_gesture(pad_center, vowel_midpoint_pad_y, PadGesture::Up);
    if (released_idle_up.note != fixed_pad_host_note || released_idle_up.note_on
        || released_idle_up.note_on_note != no_note || !released_idle_up.note_off
        || released_idle_up.note_off_note != fixed_pad_host_note
        || model.animation_frame() != idle_visual_state) {
        return release_state_failure_exit_code;
    }

    std::cout << "Delay Lama fixed-pad gesture tests passed\n";
    return success_exit_code;
}
