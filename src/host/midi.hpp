#ifndef DELAYLAMA_HOST_MIDI_HPP
#define DELAYLAMA_HOST_MIDI_HPP

#include <juce_audio_basics/juce_audio_basics.h>

#include <algorithm>
#include <cmath>
#include <cstdint>
#include <optional>

#include "dsp/engine.hpp"
#include "dsp/midi.hpp"

/// Host-boundary models and MIDI conversion for the Delay Lama processor.
namespace delaylama::host {

    /// Pad MIDI messages and their fixed protocol values.
    namespace pad_midi {
        /// MIDI note emitted by the pad begin/end sentinels.
        inline constexpr auto note_number = 40;

        /// MIDI velocity used by both the pad note-on and note-off bytes.
        inline constexpr auto note_velocity = 64;

        /// Internal dispatch selector represented by the pitch-bend status.
        inline constexpr auto pitch_bend_selector = 10;

        /// Raw MIDI status byte used by the pitch-bend callback.
        inline constexpr auto pitch_bend_status = 0xE0U;

        /// Mask retaining the channel nibble when composing a raw status byte.
        inline constexpr auto channel_mask = 0x0FU;

        /// Internal dispatch selector represented by CC11 for local X pitch modulation.
        inline constexpr auto xy_selector = 11;

        /// Callback scalar for the pad end sentinel.
        inline constexpr auto end_sentinel = 200;

        /// Callback scalar for the pad begin sentinel.
        inline constexpr auto begin_sentinel = 201;

        /// Base scalar used before pitch-bend inversion.
        inline constexpr auto pitch_bend_input_base = 100.0F;

        /// Output scalar for pitch-bend inversion.
        inline constexpr auto pitch_bend_output_base = 101.0F;

        /// Return a finite normalized axis value in the closed unit interval.
        inline auto clamp_axis(const float value) noexcept -> float {
            const auto finite_value = std::isfinite(value) ? value : 0.0F;
            return std::clamp(finite_value, 0.0F, 1.0F);
        }

        /// Convert a normalized axis value using integer truncation.
        inline auto to_seven_bit(const float value) noexcept -> int {
            // Truncation keeps the midpoint at 63; rounding would produce 64.
            return static_cast<int>(
                clamp_axis(value) * static_cast<float>(delaylama::midi_protocol::seven_bit_max));
        }

        /// Convert the second pad Y axis through the 101-input inversion.
        inline auto vowel_axis_value(const float value) noexcept -> float {
            const auto input = pitch_bend_input_base + clamp_axis(value);
            return clamp_axis(pitch_bend_output_base - input);
        }

        /// Build the fixed note-on or note-off message emitted by a pad sentinel.
        inline auto note_message(const int channel, const bool is_down) -> juce::MidiMessage {
            const auto velocity = static_cast<std::uint8_t>(note_velocity);
            return is_down ? juce::MidiMessage::noteOn(channel, note_number, velocity)
                           : juce::MidiMessage::noteOff(channel, note_number, velocity);
        }

        /// Build the CC11 message emitted directly for the first normalized X axis.
        inline auto controller_message(const int channel, const float value) -> juce::MidiMessage {
            return juce::MidiMessage::controllerEvent(
                channel,
                static_cast<int>(xy_selector),
                to_seven_bit(value));
        }

        /// Build the pitch-bend message emitted for the second normalized Y axis.
        inline auto pitch_message(const int channel, const float value) -> juce::MidiMessage {
            // The low byte stays clear because the editor axis occupies the high seven bits.
            const auto high_byte = to_seven_bit(vowel_axis_value(value));
            const auto status = static_cast<int>(
                pitch_bend_status | (static_cast<unsigned int>(channel - 1) & channel_mask));
            return {status, 0, high_byte};
        }
    }

    /// Converts supported MIDI messages to sample-offset synthesis events.
    inline auto to_core_event(const juce::MidiMessage& message, const int sample_offset)
        -> std::optional<delaylama::Event> {
        const auto make_note_event = [&message, sample_offset](const auto type) -> auto {
            const auto raw_note = message.getNoteNumber();
            if (!delaylama::midi_protocol::is_host_note(raw_note)) {
                return std::optional<delaylama::Event> {};
            }

            auto event = delaylama::Event {};
            event.type = type;
            event.sample_offset = sample_offset;
            event.note = delaylama::midi_protocol::to_internal_note(raw_note);
            event.value = static_cast<float>(message.getVelocity())
                          / static_cast<float>(delaylama::midi_protocol::seven_bit_max);
            event.local_pad = false;
            return std::optional<delaylama::Event> {event};
        };

        if (message.isNoteOn()) {
            return make_note_event(delaylama::EventType::NoteOn);
        }

        if (message.isNoteOff()) {
            return make_note_event(delaylama::EventType::NoteOff);
        }

        if (message.isPitchWheel()) {
            auto event = delaylama::Event {};
            event.type = delaylama::EventType::PitchBend;
            event.sample_offset = sample_offset;
            // Dividing by the full range keeps zero and one as the bend endpoints.
            event.value = static_cast<float>(message.getPitchWheelValue())
                          / static_cast<float>(delaylama::midi_protocol::fourteen_bit_max);
            return event;
        }

        if (message.isController()) {
            auto event = delaylama::Event {};
            event.type = delaylama::EventType::ControlChange;
            event.controller = message.getControllerNumber();
            event.sample_offset = sample_offset;
            event.value = static_cast<float>(message.getControllerValue())
                          / static_cast<float>(delaylama::midi_protocol::seven_bit_max);
            return event;
        }

        return std::nullopt;
    }

}

#endif
