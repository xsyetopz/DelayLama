#ifndef DELAYLAMA_EDITOR_STATE_HPP
#define DELAYLAMA_EDITOR_STATE_HPP

/// Processor state consumed by the editor.
namespace delaylama::host {

    /// Neutral vowel used before the first host event reaches the editor.
    inline constexpr auto visual_state_default_vowel = 0.5F;
    /// Sentinel requesting derivation when no processor atlas state is available.
    inline constexpr auto visual_state_derived_atlas_selector = -1.0F;

    /// Post-event note, gate, vowel, and atlas state consumed by the visual editor.
    struct VisualState {
        /// Internal voice-stack note 4..72, or a negative value when inactive.
        int note = -1;

        /// Whether the selected note is currently held.
        bool gate = false;

        /// Normalised vowel value after host event application.
        float vowel = visual_state_default_vowel;

        /// Processor-owned selector 6 value, or the derivation sentinel.
        float atlas_selector = visual_state_derived_atlas_selector;
    };

}

#endif
