#include "host/processor.hpp"

#include <juce_audio_basics/juce_audio_basics.h>

#include <array>
#include <cmath>
#include <span>

#include "dsp/engine.hpp"
#include "dsp/midi.hpp"
#include "host/midi.hpp"
#include "support.hpp"

namespace delaylama::tests {
    namespace {

        auto processor_visual_state_contract() -> bool {
            using delaylama::AudioChannel;
            using delaylama::Event;
            using delaylama::EventType;
            using delaylama::OutputChannels;
            using delaylama::host::ProcessorModel;

            ProcessorModel processor;
            processor.prepare(processor_sample_rate, processor_max_block_size);
            std::array<float, processor_block_sample_count> left {};
            std::array<float, processor_block_sample_count> right {};
            auto channels = std::array<AudioChannel, processor_output_channel_count> {
                AudioChannel {left},
                AudioChannel {right}};

            const auto note_on = std::array<Event, 1> {Event {
                .type = EventType::NoteOn,
                .sample_offset = processor_note_on_offset,
                .note = processor_midi_note,
                .value = processor_note_velocity}};
            processor.process(
                OutputChannels {channels},
                processor_block_sample_count,
                std::span<const Event> {note_on});
            const auto held_state = processor.visual_state();
            if (held_state.note != processor_midi_note || !held_state.gate
                || std::abs(held_state.vowel - processor_default_vowel)
                       > processor_value_tolerance) {
                return false;
            }

            const auto bend = std::array<Event, 1> {Event {
                .type = EventType::PitchBend,
                .sample_offset = processor_pitch_bend_offset,
                .note = processor_no_note,
                .value = processor_full_vowel}};
            processor.process(
                OutputChannels {channels},
                processor_block_sample_count,
                std::span<const Event> {bend});
            const auto bent_state = processor.visual_state();
            if (bent_state.note != processor_midi_note || !bent_state.gate
                || std::abs(bent_state.vowel - processor_default_vowel)
                       > processor_value_tolerance) {
                return false;
            }

            for (auto index = 0U; index < processor_pitch_bend_first_tick_blocks; ++index) {
                processor.process(OutputChannels {channels}, processor_block_sample_count, {});
            }
            const auto ticked_state = processor.visual_state();
            if (ticked_state.note != processor_midi_note || !ticked_state.gate
                || ticked_state.vowel <= processor_default_vowel
                || ticked_state.vowel >= processor_full_vowel) {
                return false;
            }

            constexpr auto cc11_target = 0.25F;
            constexpr auto cc11_first_tick_blocks = 27U;
            const auto xy_before_cc11 = processor.parameters().xy_routing;
            const auto cc11 = std::array<Event, 1> {Event {
                .type = EventType::ControlChange,
                .sample_offset = 0,
                .value = cc11_target,
                .controller = delaylama::midi_protocol::control_change::xy_routing}};
            processor.process(
                OutputChannels {channels},
                processor_block_sample_count,
                std::span<const Event> {cc11});
            if (std::abs(processor.parameters().xy_routing - xy_before_cc11)
                > processor_value_tolerance) {
                return false;
            }
            for (auto index = 0U; index < cc11_first_tick_blocks; ++index) {
                processor.process(OutputChannels {channels}, processor_block_sample_count, {});
            }
            const auto stepped_xy = processor.parameters().xy_routing;
            if (stepped_xy <= xy_before_cc11 || stepped_xy >= cc11_target) {
                return false;
            }
            const auto vowel_before_release = processor.visual_state().vowel;

            const auto note_off = std::array<Event, 1> {Event {
                .type = EventType::NoteOff,
                .sample_offset = processor_note_off_offset,
                .note = processor_midi_note,
                .value = 0.0F}};
            processor.process(
                OutputChannels {channels},
                processor_block_sample_count,
                std::span<const Event> {note_off});
            const auto released_state = processor.visual_state();
            return released_state.note == processor_no_note && !released_state.gate
                   && std::abs(released_state.vowel - vowel_before_release)
                          <= processor_value_tolerance;
        }

        auto host_note_contract() -> bool {
            using delaylama::AudioChannel;
            using delaylama::Event;
            using delaylama::OutputChannels;
            using delaylama::host::ProcessorModel;
            using delaylama::host::to_core_event;

            const auto low = to_core_event(
                juce::MidiMessage::noteOn(midi_channel, host_note_low, note_velocity),
                note_on_sample_offset);
            const auto high = to_core_event(
                juce::MidiMessage::noteOn(midi_channel, host_note_high, note_velocity),
                note_on_sample_offset);
            if (!low.has_value() || low->note != host_note_low_internal || !high.has_value()
                || high->note != host_note_high_internal) {
                return false;
            }

            if (to_core_event(
                    juce::MidiMessage::noteOn(midi_channel, host_note_below_minimum, note_velocity),
                    note_on_sample_offset)
                    .has_value()
                || to_core_event(
                       juce::MidiMessage::noteOff(midi_channel, host_note_above_maximum),
                       note_off_sample_offset)
                       .has_value()) {
                return false;
            }

            ProcessorModel processor;
            processor.prepare(processor_sample_rate, processor_max_block_size);
            std::array<float, processor_block_sample_count> left {};
            std::array<float, processor_block_sample_count> right {};
            auto channels = std::array<AudioChannel, processor_output_channel_count> {
                AudioChannel {left},
                AudioChannel {right}};
            processor.process(
                OutputChannels {channels},
                processor_block_sample_count,
                std::span<const Event> {&*low, 1U});
            const auto held_state = processor.visual_state();
            if (held_state.note != host_note_low_internal || !held_state.gate) {
                return false;
            }
            processor.process(OutputChannels {channels}, processor_block_sample_count, {});
            const auto unchanged_state = processor.visual_state();
            return unchanged_state.note == host_note_low_internal && unchanged_state.gate;
        }

        auto processor_pad_state_contract() -> bool {
            using delaylama::AudioChannel;
            using delaylama::Event;
            using delaylama::EventType;
            using delaylama::OutputChannels;
            using delaylama::host::ProcessorModel;

            ProcessorModel processor;
            processor.prepare(processor_sample_rate, processor_max_block_size);
            std::array<float, processor_block_sample_count> left {};
            std::array<float, processor_block_sample_count> right {};
            auto channels = std::array<AudioChannel, processor_output_channel_count> {
                AudioChannel {left},
                AudioChannel {right}};
            const auto events = std::array {
                Event {
                    .type = EventType::NoteOn,
                    .sample_offset = processor_note_on_offset,
                    .note = processor_pad_internal_note,
                    .value = processor_note_velocity,
                    .local_pad = true},
                Event {
                    .type = EventType::PadPitch,
                    .sample_offset = processor_note_on_offset,
                    .value = processor_pad_pitch,
                    .local_pad = true},
                Event {
                    .type = EventType::PadVowel,
                    .sample_offset = processor_note_on_offset,
                    .value = processor_pad_vowel,
                    .local_pad = true}};
            processor.process(
                OutputChannels {channels},
                processor_block_sample_count,
                std::span<const Event> {events});
            const auto state = processor.visual_state();
            if (state.note != processor_pad_internal_note || !state.gate
                || std::abs(state.vowel - processor_pad_vowel) > processor_value_tolerance) {
                return false;
            }

            const auto host_collision = delaylama::host::to_core_event(
                juce::MidiMessage::noteOn(
                    midi_channel,
                    delaylama::host::pad_midi::note_number,
                    note_velocity),
                processor_note_on_offset);
            if (!host_collision.has_value() || host_collision->local_pad
                || host_collision->note != processor_pad_internal_note) {
                return false;
            }
            processor.process(
                OutputChannels {channels},
                processor_block_sample_count,
                std::span<const Event> {&*host_collision, 1U});
            const auto collision_state = processor.visual_state();
            return collision_state.note == processor_pad_internal_note && collision_state.gate
                   && std::abs(collision_state.vowel - processor_default_vowel)
                          <= processor_value_tolerance;
        }

    }

    [[nodiscard]] auto run_processor_contract() -> bool {
        if (!processor_visual_state_contract()) {
            return false;
        }
        if (!host_note_contract()) {
            return false;
        }
        if (!processor_pad_state_contract()) {
            return false;
        }
        return true;
    }

}
