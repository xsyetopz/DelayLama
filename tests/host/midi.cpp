#include "host/midi.hpp"

#include <juce_audio_basics/juce_audio_basics.h>

#include <array>
#include <cmath>
#include <cstddef>
#include <iterator>
#include <span>
#include <utility>

#include "dsp/engine.hpp"
#include "dsp/midi.hpp"
#include "support.hpp"

namespace delaylama::tests {
    namespace {

        auto message_byte(const juce::MidiMessage& message, const int index) -> unsigned int {
            const auto bytes = std::span<const unsigned char> {
                message.getRawData(),
                static_cast<std::size_t>(message.getRawDataSize())};
            const auto iterator =
                std::ranges::next(bytes.begin(), static_cast<std::ptrdiff_t>(index));
            return static_cast<unsigned int>(*iterator);
        }

        auto pad_protocol_contract() -> bool {
            using delaylama::host::pad_midi::controller_message;
            using delaylama::host::pad_midi::note_message;
            using delaylama::host::pad_midi::pitch_message;

            const auto messages = std::array {
                std::pair {note_message(midi_channel, true), pad_note_on_status},
                std::pair {pitch_message(midi_channel, pad_axis_midpoint), pad_pitch_bend_status},
                std::pair {
                    controller_message(midi_channel, pad_axis_midpoint),
                    pad_controller_status},
                std::pair {note_message(midi_channel, false), pad_note_off_status}};
            for (const auto& [message, status] : messages) {
                if ((message_byte(message, pad_status_byte_index) & pad_status_mask) != status) {
                    return false;
                }
            }

            const auto& note_on = std::get<0>(messages).first;
            if (message_byte(note_on, pad_data1_byte_index)
                    != delaylama::host::pad_midi::note_number
                || message_byte(note_on, pad_data2_byte_index)
                       != delaylama::host::pad_midi::note_velocity) {
                return false;
            }
            const auto& pitch = std::get<1>(messages).first;
            if (message_byte(pitch, pad_status_byte_index) != pad_pitch_bend_status
                || message_byte(pitch, pad_data1_byte_index) != pad_pitch_lsb
                || message_byte(pitch, pad_data2_byte_index) != pad_midpoint_pitch_msb) {
                return false;
            }
            const auto decoded_pitch = delaylama::host::to_core_event(pitch, 0);
            const auto expected_pitch_value =
                static_cast<float>(message_byte(pitch, pad_data2_byte_index) * pitch_bend_msb_radix)
                / static_cast<float>(delaylama::midi_protocol::fourteen_bit_max);
            if (!decoded_pitch.has_value()
                || std::abs(decoded_pitch->value - expected_pitch_value) >= value_tolerance) {
                return false;
            }
            const auto low_vowel = pitch_message(midi_channel, pad_y_axis_low);
            const auto high_vowel = pitch_message(midi_channel, pad_y_axis_high);
            if (message_byte(low_vowel, pad_data1_byte_index) != pad_pitch_lsb
                || message_byte(low_vowel, pad_data2_byte_index)
                       != delaylama::host::pad_midi::to_seven_bit(
                           delaylama::host::pad_midi::vowel_axis_value(pad_y_axis_low))
                || message_byte(high_vowel, pad_data1_byte_index) != pad_pitch_lsb
                || message_byte(high_vowel, pad_data2_byte_index)
                       != delaylama::host::pad_midi::to_seven_bit(
                           delaylama::host::pad_midi::vowel_axis_value(pad_y_axis_high))) {
                return false;
            }
            const auto& controller = std::get<2>(messages).first;
            if (message_byte(controller, pad_data1_byte_index)
                    != delaylama::host::pad_midi::xy_selector
                || message_byte(controller, pad_data2_byte_index) != pad_midpoint_seven_bit) {
                return false;
            }
            const auto low_controller = controller_message(midi_channel, pad_x_axis_low);
            const auto high_controller = controller_message(midi_channel, pad_x_axis_high);
            if (message_byte(low_controller, pad_data1_byte_index)
                    != delaylama::host::pad_midi::xy_selector
                || message_byte(low_controller, pad_data2_byte_index)
                       != delaylama::host::pad_midi::to_seven_bit(pad_x_axis_low)
                || message_byte(high_controller, pad_data1_byte_index)
                       != delaylama::host::pad_midi::xy_selector
                || message_byte(high_controller, pad_data2_byte_index)
                       != delaylama::host::pad_midi::to_seven_bit(pad_x_axis_high)
                || message_byte(low_vowel, pad_data2_byte_index)
                       <= message_byte(high_vowel, pad_data2_byte_index)) {
                return false;
            }
            const auto& note_off = std::get<3>(messages).first;
            return message_byte(note_off, pad_data1_byte_index)
                       == delaylama::host::pad_midi::note_number
                   && message_byte(note_off, pad_data2_byte_index)
                          == delaylama::host::pad_midi::note_velocity;
        }

        auto midi_event_contract() -> bool {
            using delaylama::EventType;
            using delaylama::host::to_core_event;
            using delaylama::midi_protocol::fourteen_bit_max;
            using delaylama::midi_protocol::seven_bit_max;

            const auto note_on = to_core_event(
                juce::MidiMessage::noteOn(midi_channel, raw_note_number, note_velocity),
                note_on_sample_offset);
            if (!note_on.has_value() || note_on->type != EventType::NoteOn
                || note_on->sample_offset != note_on_sample_offset
                || note_on->note != internal_note_number
                || std::abs(note_on->value - note_velocity) >= value_tolerance) {
                return false;
            }

            const auto note_off = to_core_event(
                juce::MidiMessage::noteOff(midi_channel, raw_note_number),
                note_off_sample_offset);
            if (!note_off.has_value() || note_off->type != EventType::NoteOff
                || note_off->sample_offset != note_off_sample_offset
                || note_off->note != internal_note_number) {
                return false;
            }

            for (const auto raw_bend :
                 std::array {pitch_bend_low_value, pitch_bend_center_value, fourteen_bit_max}) {
                const auto bend = to_core_event(
                    juce::MidiMessage::pitchWheel(midi_channel, raw_bend),
                    pitch_bend_sample_offset);
                const auto expected =
                    static_cast<float>(raw_bend) / static_cast<float>(fourteen_bit_max);
                if (!bend.has_value() || bend->type != EventType::PitchBend
                    || bend->sample_offset != pitch_bend_sample_offset
                    || std::abs(bend->value - expected) >= value_tolerance) {
                    return false;
                }
            }

            const auto documented_controllers = std::array {
                delaylama::midi_protocol::control_change::vibrato,
                delaylama::midi_protocol::control_change::port_time,
                delaylama::midi_protocol::control_change::volume,
                delaylama::midi_protocol::control_change::delay_mix,
                delaylama::midi_protocol::control_change::voice};
            for (const auto controller : documented_controllers) {
                const auto cc = to_core_event(
                    juce::MidiMessage::controllerEvent(
                        midi_channel,
                        static_cast<int>(controller),
                        controller_value),
                    controller_sample_offset);
                if (!cc.has_value() || cc->type != EventType::ControlChange
                    || cc->controller != controller || cc->sample_offset != controller_sample_offset
                    || std::abs(
                           cc->value
                           - (static_cast<float>(controller_value)
                              / static_cast<float>(seven_bit_max)))
                           >= value_tolerance) {
                    return false;
                }
            }

            const auto internal_xy = to_core_event(
                juce::MidiMessage::controllerEvent(
                    midi_channel,
                    static_cast<int>(delaylama::midi_protocol::control_change::xy_routing),
                    static_cast<int>(controller_value)),
                controller_sample_offset);
            if (!internal_xy.has_value()
                || internal_xy->controller != delaylama::midi_protocol::control_change::xy_routing
                || std::abs(
                       internal_xy->value
                       - (static_cast<float>(controller_value) / static_cast<float>(seven_bit_max)))
                       >= value_tolerance) {
                return false;
            }

            if (to_core_event(juce::MidiMessage::midiStart(), unsupported_message_sample_offset)
                    .has_value()) {
                return false;
            }
            if (!pad_protocol_contract()) {
                return false;
            }
            return true;
        }

    }

    [[nodiscard]] auto run_midi_contract() -> bool {
        return midi_event_contract();
    }

}
