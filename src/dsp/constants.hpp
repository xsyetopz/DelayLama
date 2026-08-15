#ifndef DELAYLAMA_DSP_CONSTANTS_HPP
#define DELAYLAMA_DSP_CONSTANTS_HPP

#include <array>
#include <cmath>
#include <concepts>
#include <cstddef>
#include <cstdint>
#include <iterator>
#include <numbers>

namespace delaylama::dsp_detail {

    constexpr auto k_pi = std::numbers::pi_v<double>;
    constexpr auto k_two_pi = 2.0 * k_pi;
    constexpr auto k_min_sample_rate = 8000.0;
    constexpr auto k_max_sample_rate = 384000.0;
    constexpr auto k_default_sample_rate = 44100.0;
    constexpr auto k_grain_seconds = 0.02;
    constexpr auto k_dry_ring_samples = std::size_t {10240};
    constexpr auto k_delay_ring_samples = std::size_t {20000};
    constexpr auto k_sine_table_samples = std::size_t {1024};
    constexpr auto k_frequency_table_samples = std::size_t {4096};
    constexpr auto k_formant_table_samples = std::size_t {1280};
    constexpr auto k_formant_segment_samples = std::size_t {320};
    constexpr auto k_formant_segment_count = std::size_t {4};
    constexpr auto k_formant_control_point_count = std::size_t {5};
    constexpr auto k_formant_point_two_index = std::size_t {2};
    constexpr auto k_formant_point_three_index = std::size_t {3};
    constexpr auto k_formant_point_four_index = std::size_t {4};
    constexpr auto k_cubic_curve_difference_weight = 3;
    constexpr auto k_cubic_curve_divisor = 2.0;
    constexpr auto k_raised_cosine_scale = 0.5;
    constexpr auto k_exponential_table_grain_multiple = std::size_t {4};

    constexpr auto k_exponential_rate = 157.0796327;
    constexpr auto k_excitation_frequency_one = 4950.0;
    constexpr auto k_excitation_frequency_two = 3800.0;
    constexpr auto k_excitation_decay_step_one = 3.0F;
    constexpr auto k_excitation_decay_step_two = 3.6F;
    constexpr auto k_excitation_mix = 0.5F;
    constexpr auto k_window_attack_seconds = 0.0018;
    constexpr auto k_window_tail_start_seconds = 0.013;
    constexpr auto k_window_tail_phase_seconds = 0.007;

    constexpr auto k_frequency_base_hz = 8.175798916;
    constexpr auto k_semitone_ratio = 1.059463094;
    constexpr auto k_frequency_steps_per_semitone = 32.0;
    constexpr auto k_vowel_table_scale = 1279.0F;
    constexpr auto k_head_scale_base = 0.75F;
    constexpr auto k_head_scale_amount = 0.5F;
    constexpr auto k_formant_decay_step_one = 0.65F;
    constexpr auto k_formant_decay_step_two = 0.95F;
    constexpr auto k_formant_decay_step_three = 1.25F;

    constexpr auto k_delay_left_seconds = 0.309592;
    constexpr auto k_delay_right_seconds = 0.398435;
    constexpr auto k_delay_feedback = 0.5F;
    constexpr auto k_output_pitch_base = 2.0F;
    constexpr auto k_output_pitch_divisor = 72.0F;

    constexpr auto k_glide_snap_semitones = 0.2;
    constexpr auto k_glide_octave_semitones = 12.0;
    constexpr auto k_glide_time_floor = 0.01;
    constexpr auto k_initial_pitch = 36.0;

    constexpr auto k_vibrato_depth_floor = 0.2F;
    constexpr auto k_vibrato_rate_scale = 0.2F;
    constexpr auto k_initial_vibrato_rate_hz = 4.0F;
    constexpr auto k_vibrato_random_base_hz = 5.0F;
    constexpr auto k_vibrato_random_range_hz = 2.0F;
    constexpr auto k_vibrato_refresh_seconds = 0.104;

    constexpr auto k_random_multiplier = std::uint32_t {1664525};
    constexpr auto k_random_increment = std::uint32_t {1013904223};
    constexpr auto k_random_divisor = 4294967296.0F;

    constexpr auto k_bend_update_seconds = 0.01;
    constexpr auto k_bend_step_count = 10;
    constexpr auto k_bend_center = 8192;
    constexpr auto k_bend_divisor = 16384.0F;
    constexpr auto k_bend_maximum = 16383;
    constexpr auto k_zero_offset_bend_first_sample = std::size_t {1};
    constexpr auto k_zero_offset_bend_padding = std::size_t {2};

    constexpr auto k_stereo_channels = std::size_t {2};
    constexpr auto k_atlas_tick_seconds = 0.208;
    constexpr auto k_atlas_selector_scale = 1.0F / 30.0F;
    constexpr auto k_atlas_active_base = 0.2F;
    constexpr auto k_atlas_active_vowel_scale = 0.8F;
    constexpr auto k_atlas_frame_zero = 0;
    constexpr auto k_atlas_frame_one = 1;
    constexpr auto k_atlas_frame_two = 2;
    constexpr auto k_atlas_frame_three = 3;
    constexpr auto k_atlas_frame_four = 4;
    constexpr auto k_atlas_frame_five = 5;
    constexpr auto k_atlas_seven_tick_mark = 7;
    constexpr auto k_atlas_eight_and_half_tick_mark = 8.5;
    constexpr auto k_atlas_fifteen_tick_mark = 15;
    constexpr auto k_atlas_seventeen_tick_mark = 17;
    constexpr auto k_atlas_timeline_start_tick = 23;
    constexpr auto k_atlas_idle_frames = std::array {
        k_atlas_frame_five, k_atlas_frame_three, k_atlas_frame_four, k_atlas_frame_three,
        k_atlas_frame_two,  k_atlas_frame_one,   k_atlas_frame_zero, k_atlas_frame_one,
        k_atlas_frame_five, k_atlas_frame_three, k_atlas_frame_four, k_atlas_frame_three,
        k_atlas_frame_five, k_atlas_frame_one,   k_atlas_frame_zero, k_atlas_frame_one,
        k_atlas_frame_two,  k_atlas_frame_three, k_atlas_frame_four, k_atlas_frame_three,
        k_atlas_frame_five, k_atlas_frame_one,   k_atlas_frame_zero, k_atlas_frame_one};

    constexpr auto k_formant_one_points = std::array {280, 450, 800, 350, 270};
    constexpr auto k_formant_two_points = std::array {600, 800, 1150, 2000, 2140};
    constexpr auto k_formant_three_points = std::array {2240, 2830, 2900, 2800, 2950};

    template<std::floating_point Value>
    [[nodiscard]] constexpr auto finite_or(const Value value, const Value fallback) noexcept
        -> Value {
        return std::isfinite(value) ? value : fallback;
    }

    template<typename Container>
    [[nodiscard]] inline auto element_at(Container& container, const std::size_t index) noexcept
        -> decltype(auto) {
        return *std::next(container.begin(), static_cast<std::ptrdiff_t>(index));
    }

    template<typename Container>
    [[nodiscard]] inline auto element_at(
        const Container& container,
        const std::size_t index) noexcept -> decltype(auto) {
        return *std::next(container.begin(), static_cast<std::ptrdiff_t>(index));
    }

    [[nodiscard]] inline auto wrap_index(
        const std::ptrdiff_t value,
        const std::size_t size) noexcept -> std::size_t {
        const auto signed_size = static_cast<std::ptrdiff_t>(size);
        auto wrapped = value % signed_size;
        if (wrapped < 0) {
            wrapped += signed_size;
        }
        return static_cast<std::size_t>(wrapped);
    }

}

#endif
