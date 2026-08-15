#ifndef DELAYLAMA_TESTS_DSP_SUPPORT_HPP
#define DELAYLAMA_TESTS_DSP_SUPPORT_HPP

#include <algorithm>
#include <array>
#include <cmath>
#include <cstddef>
#include <iterator>
#include <span>

#include "dsp/constants.hpp"
#include "dsp/engine.hpp"
#include "dsp/midi.hpp"

namespace delaylama::tests {

    using delaylama::AudioChannel;
    using delaylama::Event;
    using delaylama::EventType;
    using delaylama::OutputChannels;
    using delaylama::SynthEngine;

    inline constexpr auto k_sample_rate = 44100.0;
    inline constexpr auto k_channels = 2;
    inline constexpr auto k_tolerance = 1.0e-6F;
    inline constexpr auto k_note = 48;

    inline constexpr auto k_max_test_samples = std::size_t {8192};
    inline constexpr auto k_standard_block_samples = 512;
    inline constexpr auto k_large_block_samples = 1024;
    inline constexpr auto k_bend_settle_samples = std::size_t {5000};
    inline constexpr auto k_bend_pre_tick_samples = std::size_t {440};
    inline constexpr auto k_bend_tick_completion_samples = std::size_t {4};
    inline constexpr auto k_default_vowel = 0.5F;
    inline constexpr auto k_expected_low_bend = 2.0F / 16384.0F;
    inline constexpr auto k_expected_high_bend = 16382.0F / 16384.0F;
    inline constexpr auto k_note_stack_block_samples = 32;
    inline constexpr auto k_stack_initial_note = 40;
    inline constexpr auto k_stack_newest_note = 52;
    inline constexpr auto k_retrigger_onset_samples = std::size_t {64};
    inline constexpr auto k_retrigger_rest_samples = std::size_t {16};
    inline constexpr auto k_retrigger_comparison_samples = std::size_t {400};
    inline constexpr auto k_retrigger_unchanged_samples = std::size_t {200};
    inline constexpr auto k_retrigger_divergence_start = std::size_t {250};
    inline constexpr auto k_expected_first_route_step = 0.1F;
    inline constexpr auto k_ramp_settle_samples = std::size_t {4500};
    inline constexpr auto k_first_ramp_step_samples = std::size_t {444};
    inline constexpr auto k_second_ramp_step_samples = std::size_t {442};
    inline constexpr auto k_host_delay_mix = 0.25F;
    inline constexpr auto k_host_vowel = 0.75F;
    inline constexpr auto k_pad_pitch_value = 0.75F;
    inline constexpr auto k_pad_vowel_value = 0.25F;
    inline constexpr auto k_pad_vowel_upper_bound = 0.26F;
    inline constexpr auto k_pad_vowel_lower_bound = 0.24F;
    inline constexpr auto k_warmup_samples = std::size_t {4096};
    inline constexpr auto k_atlas_test_block_samples = 64;
    inline constexpr auto k_active_atlas_selector = 0.6F;
    inline constexpr auto k_release_atlas_frame = 5.0F;
    inline constexpr auto k_seven_tick_mark = std::size_t {7};
    inline constexpr auto k_pre_boundary_samples = std::size_t {2};
    inline constexpr auto k_seven_tick_atlas_frame = 2.0F;
    inline constexpr auto k_timeline_start_tick = std::size_t {23};
    inline constexpr auto k_timeline_first_frame = 3.0F;
    inline constexpr auto k_expected_dry_ring_samples = std::size_t {10240};
    inline constexpr auto k_expected_delay_ring_samples = std::size_t {20000};
    inline constexpr auto k_expected_sine_table_samples = std::size_t {1024};
    inline constexpr auto k_expected_frequency_table_samples = std::size_t {4096};
    inline constexpr auto k_expected_formant_table_samples = std::size_t {1280};
    inline constexpr auto k_expected_bend_step_count = 10;
    inline constexpr auto k_expected_excitation_decay_one = 3.0F;
    inline constexpr auto k_expected_excitation_decay_two = 3.6F;
    inline constexpr auto k_expected_formant_decay_one = 0.65F;
    inline constexpr auto k_expected_formant_decay_two = 0.95F;
    inline constexpr auto k_expected_formant_decay_three = 1.25F;
    inline constexpr auto k_expected_formant_one_a = 280;
    inline constexpr auto k_expected_formant_one_e = 450;
    inline constexpr auto k_expected_formant_one_i = 800;
    inline constexpr auto k_expected_formant_one_o = 350;
    inline constexpr auto k_expected_formant_one_u = 270;
    inline constexpr auto k_expected_formant_two_a = 600;
    inline constexpr auto k_expected_formant_two_e = 800;
    inline constexpr auto k_expected_formant_two_i = 1150;
    inline constexpr auto k_expected_formant_two_o = 2000;
    inline constexpr auto k_expected_formant_two_u = 2140;
    inline constexpr auto k_expected_formant_three_a = 2240;
    inline constexpr auto k_expected_formant_three_e = 2830;
    inline constexpr auto k_expected_formant_three_i = 2900;
    inline constexpr auto k_expected_formant_three_o = 2800;
    inline constexpr auto k_expected_formant_three_u = 2950;
    inline constexpr auto k_expected_formant_one_points = std::array {
        k_expected_formant_one_a,
        k_expected_formant_one_e,
        k_expected_formant_one_i,
        k_expected_formant_one_o,
        k_expected_formant_one_u};
    inline constexpr auto k_expected_formant_two_points = std::array {
        k_expected_formant_two_a,
        k_expected_formant_two_e,
        k_expected_formant_two_i,
        k_expected_formant_two_o,
        k_expected_formant_two_u};
    inline constexpr auto k_expected_formant_three_points = std::array {
        k_expected_formant_three_a,
        k_expected_formant_three_e,
        k_expected_formant_three_i,
        k_expected_formant_three_o,
        k_expected_formant_three_u};

    struct StereoBlock {
        std::array<float, k_max_test_samples> left_storage {};
        std::array<float, k_max_test_samples> right_storage {};
        std::span<float> left;
        std::span<float> right;
        std::array<AudioChannel, k_channels> channels;

        explicit StereoBlock(const std::size_t samples) noexcept
            : left(std::span<float> {left_storage}.first(samples))
            , right(std::span<float> {right_storage}.first(samples))
            , channels {AudioChannel {left}, AudioChannel {right}} {}

        auto outputs() noexcept -> OutputChannels {
            return OutputChannels {channels};
        }
    };

    inline auto sample_at(const std::span<const float> samples, const std::size_t index) noexcept
        -> float {
        return *std::next(samples.begin(), static_cast<std::ptrdiff_t>(index));
    }

    inline auto note_on(const int note = k_note, const int offset = 0, const float velocity = 1.0F)
        -> Event {
        return Event {
            .type = EventType::NoteOn,
            .sample_offset = offset,
            .note = note,
            .value = velocity,
        };
    }

    inline auto note_off(const int note = k_note, const int offset = 0) -> Event {
        return Event {
            .type = EventType::NoteOff,
            .sample_offset = offset,
            .note = note,
        };
    }

    inline auto pitch_bend(const float value, const int offset = 0) -> Event {
        return Event {
            .type = EventType::PitchBend,
            .sample_offset = offset,
            .note = -1,
            .value = value,
        };
    }

    inline auto has_signal(const std::span<const float> samples) -> bool {
        return std::ranges::any_of(samples, [](const float sample) -> bool {
            return std::abs(sample) > k_tolerance;
        });
    }

    void run_control_tests();
    void run_render_tests();

}

#endif
