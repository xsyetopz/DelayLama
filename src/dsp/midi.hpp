#ifndef DELAYLAMA_DSP_MIDI_HPP
#define DELAYLAMA_DSP_MIDI_HPP

/// MIDI limits and note/controller mappings.
namespace delaylama::midi_protocol {

    /// Largest value representable by a seven-bit MIDI field.
    inline constexpr auto seven_bit_max = 127;

    /// Largest value representable by a fourteen-bit MIDI field.
    inline constexpr auto fourteen_bit_max = 16383;

    /// Raw host note number at the bottom of the Delay Lama range.
    inline constexpr auto host_note_minimum = 16;

    /// Raw host note number at the top of the Delay Lama range.
    inline constexpr auto host_note_maximum = 84;

    /// Offset removed from raw host notes before entering the voice stack.
    inline constexpr auto host_note_offset = 12;

    /// Lowest internal note number accepted by the synthesis event contract.
    inline constexpr auto minimum_note = host_note_minimum - host_note_offset;

    /// Highest internal note number accepted by the synthesis event contract.
    inline constexpr auto maximum_note = host_note_maximum - host_note_offset;

    /// Internal note selected by the editor pad's fixed host note.
    inline constexpr auto pad_internal_note = 28;

    /// Return whether a raw host note belongs to the voice range.
    constexpr auto is_host_note(const int note) noexcept -> bool {
        return note >= host_note_minimum && note <= host_note_maximum;
    }

    /// Convert a validated raw host note to the internal stack note number.
    constexpr auto to_internal_note(const int note) noexcept -> int {
        return note - host_note_offset;
    }

    /// Controller identifiers mapped to the synthesis parameter contract.
    namespace control_change {
        /// Controller identifier for vibrato depth.
        inline constexpr auto vibrato = 1;

        /// Controller identifier for portamento time.
        inline constexpr auto port_time = 5;

        /// Controller identifier for output volume.
        inline constexpr auto volume = 7;

        /// Controller identifier for the pad's internal routing path.
        inline constexpr auto xy_routing = 11;

        /// Controller identifier for delay wet mix.
        inline constexpr auto delay_mix = 12;

        /// Controller identifier for the HeadSize formant-frequency scale.
        inline constexpr auto voice = 13;
    }

    /// Internal gain scale for the CC7 controller path.
    inline constexpr auto cc7_volume_scale = 0.127F;

}

#endif
