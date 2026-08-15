#include <array>
#include <cassert>
#include <cmath>
#include <cstddef>
#include <iterator>
#include <span>

#include "dsp/engine.hpp"
#include "dsp/midi.hpp"
#include "support.hpp"

namespace delaylama::tests::control_detail {
    namespace {

        using delaylama::AudioChannel;
        using delaylama::Event;
        using delaylama::EventType;
        using delaylama::OutputChannels;
        using delaylama::Parameters;
        using delaylama::SynthEngine;

        namespace parameter_defaults = delaylama::parameter_defaults;

        constexpr auto k_max_block_samples = 512U;
        constexpr auto k_output_channel_count = 2;
        constexpr auto k_block_start_offset = 0;
        constexpr auto k_no_note = -1;
        constexpr auto k_no_controller = 0;
        constexpr auto k_zero_value = 0.0F;
        constexpr auto k_full_normalized_value = 1.0F;
        constexpr auto k_full_midi_value =
            static_cast<float>(delaylama::midi_protocol::seven_bit_max);
        constexpr auto k_controller_midpoint_value = 64.0F;
        constexpr auto k_volume_raw_value = 32.0F;
        constexpr auto k_pitch_bend_center_value = 8191.5F;
        constexpr auto k_sample_boundary_tolerance = 1.0e-7F;
        constexpr auto k_signal_change_tolerance = 1.0e-6F;
        constexpr auto k_parameter_tolerance = 1.0e-6F;
        constexpr auto k_vowel_tolerance = 2.0e-4F;
        constexpr auto k_controller_test_sample_rate = 44100.0;
        constexpr auto k_controller_test_max_block_size = 32;
        constexpr auto k_controller_test_block_samples = 1U;
        constexpr auto k_middle_c_note = 60;
        constexpr auto k_default_volume = 0.1F;

        auto make_note_on(
            const int note,
            const int sample_offset = k_block_start_offset,
            const float value = k_full_normalized_value) -> Event {
            return Event {
                .type = EventType::NoteOn,
                .sample_offset = sample_offset,
                .note = note,
                .value = value,
                .controller = k_no_controller};
        }

        auto make_control_change(
            const int controller,
            const float value,
            const int sample_offset = k_block_start_offset) -> Event {
            return Event {
                .type = EventType::ControlChange,
                .sample_offset = sample_offset,
                .note = k_no_note,
                .value = value,
                .controller = controller};
        }

        [[nodiscard]] auto sample_at(
            const std::span<const float> samples,
            const std::size_t index) noexcept {
            const auto iterator =
                std::ranges::next(samples.begin(), static_cast<std::ptrdiff_t>(index));
            return *iterator;
        }

        struct StereoBlock {
            std::array<float, k_max_block_samples> left_storage {};
            std::array<float, k_max_block_samples> right_storage {};
            std::span<float> left;
            std::span<float> right;
            std::array<AudioChannel, k_output_channel_count> channels;

            explicit StereoBlock(const std::size_t samples) noexcept
                : left(std::span<float> {left_storage}.first(samples))
                , right(std::span<float> {right_storage}.first(samples))
                , channels {AudioChannel {left}, AudioChannel {right}} {}

            [[nodiscard]] auto outputs() noexcept {
                return OutputChannels {channels};
            }
        };

        auto test_controller_mappings() {
            SynthEngine engine;
            engine.prepare(
                k_controller_test_sample_rate,
                k_controller_test_max_block_size,
                k_output_channel_count);
            const auto defaults = engine.parameters();
            assert(
                std::abs(defaults.port_time - parameter_defaults::port_time)
                < k_parameter_tolerance);
            assert(
                std::abs(defaults.delay_mix - parameter_defaults::delay_mix)
                < k_parameter_tolerance);
            assert(std::abs(defaults.vowel - parameter_defaults::vowel) < k_parameter_tolerance);
            assert(std::abs(defaults.voice - parameter_defaults::voice) < k_parameter_tolerance);
            assert(std::abs(defaults.volume - k_default_volume) < k_parameter_tolerance);
            const auto events = std::array {
                make_control_change(
                    delaylama::midi_protocol::control_change::vibrato,
                    k_full_midi_value),
                make_control_change(
                    delaylama::midi_protocol::control_change::port_time,
                    k_controller_midpoint_value),
                make_control_change(
                    static_cast<int>(delaylama::midi_protocol::control_change::volume),
                    k_volume_raw_value),
                make_control_change(
                    delaylama::midi_protocol::control_change::xy_routing,
                    k_controller_midpoint_value),
                make_control_change(
                    delaylama::midi_protocol::control_change::delay_mix,
                    k_full_midi_value),
                make_control_change(delaylama::midi_protocol::control_change::voice, k_zero_value),
                Event {
                    .type = EventType::PitchBend,
                    .sample_offset = k_block_start_offset,
                    .note = k_no_note,
                    .value = k_pitch_bend_center_value,
                    .controller = k_no_controller},
            };
            StereoBlock block(k_controller_test_block_samples);
            engine.process(block.outputs(), block.left.size(), events);
            const auto result = engine.parameters();
            assert(std::abs(result.vibrato - k_full_normalized_value) < k_parameter_tolerance);
            assert(
                std::abs(result.port_time - (k_controller_midpoint_value / k_full_midi_value))
                < k_parameter_tolerance);
            assert(
                std::abs(
                    result.volume
                    - ((k_volume_raw_value / k_full_midi_value)
                       * delaylama::midi_protocol::cc7_volume_scale))
                < k_parameter_tolerance);
            // CC11 routing advances on the shared ten-millisecond ramp,
            // so a one-sample block has not published its first route step yet.
            assert(std::abs(result.xy_routing) < k_parameter_tolerance);
            assert(std::abs(result.delay_mix - k_full_normalized_value) < k_parameter_tolerance);
            assert(std::abs(result.voice) < k_parameter_tolerance);
            assert(
                std::abs(result.vowel - delaylama::parameter_defaults::vowel) < k_vowel_tolerance);
        }

        auto test_nonzero_velocity_is_a_gate_not_an_amplitude() {
            SynthEngine low_velocity_engine;
            SynthEngine high_velocity_engine;
            low_velocity_engine.prepare(
                k_controller_test_sample_rate,
                static_cast<int>(k_max_block_samples),
                k_output_channel_count);
            high_velocity_engine.prepare(
                k_controller_test_sample_rate,
                static_cast<int>(k_max_block_samples),
                k_output_channel_count);

            constexpr auto low_nonzero_velocity = 1.0F / k_full_midi_value;
            const auto low_note = std::array {
                make_note_on(k_middle_c_note, k_block_start_offset, low_nonzero_velocity)};
            const auto high_note =
                std::array {make_note_on(k_middle_c_note, k_block_start_offset, k_full_midi_value)};
            StereoBlock low_block(k_max_block_samples);
            StereoBlock high_block(k_max_block_samples);
            low_velocity_engine.process(low_block.outputs(), low_block.left.size(), low_note);
            high_velocity_engine.process(high_block.outputs(), high_block.left.size(), high_note);

            for (auto index = 0U; index < k_max_block_samples; ++index) {
                assert(
                    std::abs(sample_at(low_block.left, index) - sample_at(high_block.left, index))
                    < k_sample_boundary_tolerance);
                assert(
                    std::abs(sample_at(low_block.right, index) - sample_at(high_block.right, index))
                    < k_sample_boundary_tolerance);
            }
        }

        auto test_delay_level_preserves_the_dry_voice() {
            SynthEngine dry_engine;
            SynthEngine full_delay_engine;
            dry_engine.prepare(
                k_controller_test_sample_rate,
                static_cast<int>(k_max_block_samples),
                k_output_channel_count);
            full_delay_engine.prepare(
                k_controller_test_sample_rate,
                static_cast<int>(k_max_block_samples),
                k_output_channel_count);

            auto dry_parameters = Parameters {};
            dry_parameters.delay_mix = k_zero_value;
            dry_engine.set_parameters(dry_parameters);
            auto full_delay_parameters = Parameters {};
            full_delay_parameters.delay_mix = k_full_normalized_value;
            full_delay_engine.set_parameters(full_delay_parameters);

            const auto note = std::array {make_note_on(k_middle_c_note)};
            StereoBlock dry_block(k_max_block_samples);
            StereoBlock full_delay_block(k_max_block_samples);
            dry_engine.process(dry_block.outputs(), dry_block.left.size(), note);
            full_delay_engine.process(
                full_delay_block.outputs(),
                full_delay_block.left.size(),
                note);
            // Compare before the first echo; a dry/wet crossfade would silence Delay=1.
            auto has_dry_signal = false;
            for (auto index = 0U; index < k_max_block_samples; ++index) {
                assert(
                    std::abs(
                        sample_at(dry_block.left, index) - sample_at(full_delay_block.left, index))
                    < k_sample_boundary_tolerance);
                assert(
                    std::abs(
                        sample_at(dry_block.right, index)
                        - sample_at(full_delay_block.right, index))
                    < k_sample_boundary_tolerance);
                has_dry_signal = has_dry_signal
                                 || std::abs(sample_at(full_delay_block.left, index))
                                        > k_signal_change_tolerance;
            }
            assert(has_dry_signal);
        }

    }
}

namespace delaylama::tests::control_detail {
    namespace {

        void run_voice_engine_control_tests() {
            test_controller_mappings();
            test_nonzero_velocity_is_a_gate_not_an_amplitude();
            test_delay_level_preserves_the_dry_voice();
        }

    }
}

namespace delaylama::tests {
    namespace {

        auto settled_bend(const float target) -> float {
            SynthEngine engine;
            engine.prepare(k_sample_rate, k_max_test_samples, k_channels);
            StereoBlock block(k_bend_settle_samples);
            const auto event = std::array {pitch_bend(target)};
            engine.process(block.outputs(), block.left.size(), event);
            return engine.parameters().vowel;
        }

        auto test_pitch_bend_uses_ten_step_ramp() -> void {
            SynthEngine engine;
            engine.prepare(k_sample_rate, k_large_block_samples, k_channels);
            StereoBlock short_block(k_bend_pre_tick_samples);
            const auto event = std::array {pitch_bend(0.0F)};
            engine.process(short_block.outputs(), short_block.left.size(), event);
            assert(std::abs(engine.parameters().vowel - k_default_vowel) < k_tolerance);

            StereoBlock first_step(k_bend_tick_completion_samples);
            engine.process(first_step.outputs(), first_step.left.size(), {});
            const auto expected_first = static_cast<float>(8192 - 819) / 16384.0F;
            assert(std::abs(engine.parameters().vowel - expected_first) < k_tolerance);

            const auto low = settled_bend(0.0F);
            const auto high = settled_bend(16383.0F);
            assert(std::abs(low - k_expected_low_bend) < k_tolerance);
            assert(std::abs(high - k_expected_high_bend) < k_tolerance);
        }

        auto test_cc11_route_uses_shared_ten_millisecond_cadence() -> void {
            SynthEngine engine;
            engine.prepare(k_sample_rate, k_large_block_samples, k_channels);
            StereoBlock empty(0);
            const auto route = std::array {Event {
                .type = EventType::PadPitch,
                .sample_offset = 0,
                .note = -1,
                .value = 1.0F,
                .local_pad = true,
            }};
            engine.process(empty.outputs(), 0, route);
            assert(std::abs(engine.parameters().xy_routing) < k_tolerance);
            engine.process({}, k_bend_pre_tick_samples, {});
            assert(std::abs(engine.parameters().xy_routing) < k_tolerance);
            engine.process({}, k_bend_tick_completion_samples, {});
            assert(
                std::abs(engine.parameters().xy_routing - k_expected_first_route_step)
                < k_tolerance);
            engine.process({}, k_ramp_settle_samples, {});
            assert(std::abs(engine.parameters().xy_routing - 1.0F) < k_tolerance);
        }

        auto test_parameter_bundle_does_not_cancel_bend_ramp() -> void {
            SynthEngine engine;
            engine.prepare(k_sample_rate, k_large_block_samples, k_channels);
            StereoBlock empty(0);
            const auto bend = std::array {pitch_bend(0.0F)};
            engine.process(empty.outputs(), 0, bend);
            engine.process({}, k_first_ramp_step_samples, {});
            auto parameters = engine.parameters();
            parameters.delay_mix = k_host_delay_mix;
            engine.set_parameters(parameters);
            engine.process({}, k_ramp_settle_samples, {});
            assert(std::abs(engine.parameters().vowel - k_expected_low_bend) < k_tolerance);
        }

        auto test_host_vowel_does_not_cancel_independent_bend_ramp() -> void {
            SynthEngine engine;
            engine.prepare(k_sample_rate, k_large_block_samples, k_channels);
            StereoBlock empty(0);
            const auto bend = std::array {pitch_bend(0.0F)};
            engine.process(empty.outputs(), 0, bend);
            engine.process({}, k_first_ramp_step_samples, {});
            auto parameters = engine.parameters();
            parameters.vowel = k_host_vowel;
            engine.set_parameters(parameters);
            assert(std::abs(engine.parameters().vowel - k_host_vowel) < k_tolerance);
            engine.process({}, k_second_ramp_step_samples, {});
            const auto expected_second_bend_step = static_cast<float>(8192 - (2 * 819)) / 16384.0F;
            assert(std::abs(engine.parameters().vowel - expected_second_bend_step) < k_tolerance);
        }

        auto test_local_pad_routes_pitch_and_vowel_separately() -> void {
            SynthEngine engine;
            engine.prepare(k_sample_rate, k_large_block_samples, k_channels);
            StereoBlock empty(0);
            const auto pitch = std::array {Event {
                .type = EventType::PadPitch,
                .sample_offset = 0,
                .note = -1,
                .value = k_pad_pitch_value,
                .local_pad = true,
            }};
            engine.process(empty.outputs(), 0, pitch);
            assert(std::abs(engine.pad_state().pitch_modulation - k_pad_pitch_value) < k_tolerance);
            assert(std::abs(engine.parameters().vowel - k_default_vowel) < k_tolerance);

            const auto vowel = std::array {Event {
                .type = EventType::PadVowel,
                .sample_offset = 0,
                .note = -1,
                .value = k_pad_vowel_value,
                .local_pad = true,
            }};
            engine.process(empty.outputs(), 0, vowel);
            StereoBlock settle(k_bend_settle_samples);
            engine.process(settle.outputs(), settle.left.size(), {});
            assert(engine.parameters().vowel < k_pad_vowel_upper_bound);
            assert(engine.parameters().vowel > k_pad_vowel_lower_bound);
        }

    }

    void run_control_tests() {
        control_detail::run_voice_engine_control_tests();
        test_pitch_bend_uses_ten_step_ramp();
        test_cc11_route_uses_shared_ten_millisecond_cadence();
        test_parameter_bundle_does_not_cancel_bend_ramp();
        test_host_vowel_does_not_cancel_independent_bend_ramp();
        test_local_pad_routes_pitch_and_vowel_separately();
    }

}
