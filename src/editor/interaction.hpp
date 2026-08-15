#ifndef DELAYLAMA_EDITOR_INTERACTION_HPP
#define DELAYLAMA_EDITOR_INTERACTION_HPP

#include <cstddef>
#include <cstdint>

#include "editor/state.hpp"

/// Editor gesture and visual-state API.
namespace delaylama::host {

    /// Pointer lifecycle on the XY pad.
    enum class PadGesture : std::uint8_t {
        /// Starts a pad gesture.
        Down,
        /// Updates a held gesture.
        Drag,
        /// Ends a pad gesture.
        Up
    };

    /// Note and control changes produced by a pad gesture.
    struct GestureResult {
        /// Clamped horizontal position.
        float position_x = 0.0F;
        /// Clamped vertical position.
        float position_y = 0.0F;
        /// Vowel control value.
        float vowel = 0.0F;
        /// Pad host note.
        int note = -1;
        /// Note-on target, or -1.
        int note_on_note = -1;
        /// Note-off target, or -1.
        int note_off_note = -1;
        /// Whether to emit note-on.
        bool note_on = false;
        /// Whether to emit note-off.
        bool note_off = false;
    };

    /// Owns pad gesture and atlas-frame state.
    class EditorModel final {
    public:
        /// Creates an idle model.
        EditorModel() noexcept = default;
        /// Releases gesture and visual state.
        ~EditorModel() = default;

        /// Model state cannot be copied.
        EditorModel(const EditorModel&) = delete;
        /// Model state cannot be copy-assigned.
        auto operator=(const EditorModel&) -> EditorModel& = delete;
        /// Transfers gesture and visual state.
        EditorModel(EditorModel&&) noexcept = default;
        /// Replaces this model with transferred state.
        auto operator=(EditorModel&&) noexcept -> EditorModel& = default;

        /// Clamps a pad gesture and returns its note and control changes.
        [[nodiscard]] auto handle_gesture(
            float position_x,
            float position_y,
            PadGesture gesture) noexcept -> GestureResult;

        /// Applies a sanitized host snapshot; callers must cross the audio/UI boundary safely.
        auto apply_external_state(VisualState state) noexcept -> void;

        /// Maps note, gate, and vowel state to an atlas frame.
        [[nodiscard]] static auto select_visual_state(int note, bool gate, float vowel) noexcept
            -> std::size_t;

        /// Returns the current atlas frame.
        [[nodiscard]] auto animation_frame() const noexcept -> std::size_t;

    private:
        // Kept private so adapters cannot define a competing note domain.
        static constexpr auto internal_min_note = 4;
        static constexpr auto internal_max_note = 72;
        static constexpr auto pad_host_note = 40;
        static constexpr auto pad_internal_note = 28;
        static constexpr auto inactive_pad_note = -1;
        static constexpr auto default_pad_vowel = 0.5F;
        static constexpr auto idle_visual_state = 5U;

        static auto sanitise_visual_state(VisualState state) noexcept -> VisualState;

        bool pad_gate_ = false;
        float active_pad_vowel_ = default_pad_vowel;
        std::size_t visual_state_ = idle_visual_state;
    };

}

#endif
