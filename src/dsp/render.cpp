#include <algorithm>
#include <array>
#include <bit>
#include <cmath>
#include <cstddef>
#include <cstdint>
#include <iterator>
#include <span>
#include <utility>
#include <vector>

#include "constants.hpp"
#include "engine.hpp"

namespace delaylama {
    namespace {
        using dsp_detail::element_at;
        using dsp_detail::k_atlas_active_base;
        using dsp_detail::k_atlas_active_vowel_scale;
        using dsp_detail::k_atlas_eight_and_half_tick_mark;
        using dsp_detail::k_atlas_fifteen_tick_mark;
        using dsp_detail::k_atlas_frame_five;
        using dsp_detail::k_atlas_frame_two;
        using dsp_detail::k_atlas_idle_frames;
        using dsp_detail::k_atlas_selector_scale;
        using dsp_detail::k_atlas_seven_tick_mark;
        using dsp_detail::k_atlas_seventeen_tick_mark;
        using dsp_detail::k_atlas_timeline_start_tick;
        using dsp_detail::k_cubic_curve_difference_weight;
        using dsp_detail::k_cubic_curve_divisor;
        using dsp_detail::k_delay_feedback;
        using dsp_detail::k_delay_ring_samples;
        using dsp_detail::k_dry_ring_samples;
        using dsp_detail::k_excitation_decay_step_one;
        using dsp_detail::k_excitation_decay_step_two;
        using dsp_detail::k_excitation_frequency_one;
        using dsp_detail::k_excitation_frequency_two;
        using dsp_detail::k_excitation_mix;
        using dsp_detail::k_exponential_rate;
        using dsp_detail::k_exponential_table_grain_multiple;
        using dsp_detail::k_formant_control_point_count;
        using dsp_detail::k_formant_decay_step_one;
        using dsp_detail::k_formant_decay_step_three;
        using dsp_detail::k_formant_decay_step_two;
        using dsp_detail::k_formant_one_points;
        using dsp_detail::k_formant_point_four_index;
        using dsp_detail::k_formant_point_three_index;
        using dsp_detail::k_formant_point_two_index;
        using dsp_detail::k_formant_segment_count;
        using dsp_detail::k_formant_segment_samples;
        using dsp_detail::k_formant_table_samples;
        using dsp_detail::k_formant_three_points;
        using dsp_detail::k_formant_two_points;
        using dsp_detail::k_frequency_base_hz;
        using dsp_detail::k_frequency_steps_per_semitone;
        using dsp_detail::k_frequency_table_samples;
        using dsp_detail::k_grain_seconds;
        using dsp_detail::k_head_scale_amount;
        using dsp_detail::k_head_scale_base;
        using dsp_detail::k_output_pitch_base;
        using dsp_detail::k_output_pitch_divisor;
        using dsp_detail::k_pi;
        using dsp_detail::k_raised_cosine_scale;
        using dsp_detail::k_random_divisor;
        using dsp_detail::k_random_increment;
        using dsp_detail::k_random_multiplier;
        using dsp_detail::k_semitone_ratio;
        using dsp_detail::k_sine_table_samples;
        using dsp_detail::k_two_pi;
        using dsp_detail::k_vibrato_depth_floor;
        using dsp_detail::k_vibrato_random_base_hz;
        using dsp_detail::k_vibrato_random_range_hz;
        using dsp_detail::k_vibrato_rate_scale;
        using dsp_detail::k_vowel_table_scale;
        using dsp_detail::k_window_attack_seconds;
        using dsp_detail::k_window_tail_phase_seconds;
        using dsp_detail::k_window_tail_start_seconds;
        using dsp_detail::k_zero_offset_bend_padding;
    }

    auto SynthEngine::build_formant_curve(
        const std::array<int, k_formant_control_point_count>& points,
        std::vector<float>& output) -> void {
        const auto extended = std::array {
            points.front(),
            element_at(points, 0),
            element_at(points, 1),
            element_at(points, k_formant_point_two_index),
            element_at(points, k_formant_point_three_index),
            element_at(points, k_formant_point_four_index),
            points.back()};
        output.resize(k_formant_table_samples);
        for (auto segment = std::size_t {0}; segment < k_formant_segment_count; ++segment) {
            const auto previous = element_at(extended, segment);
            const auto current = element_at(extended, segment + 1);
            const auto next = element_at(extended, segment + 2);
            const auto after_next = element_at(extended, segment + 3);
            const auto coefficient_a = static_cast<float>(
                ((k_cubic_curve_difference_weight * (current - next)) - previous + after_next)
                / k_cubic_curve_divisor);
            const auto integer_half = ((5 * current) + after_next) / 2;
            const auto coefficient_b = static_cast<float>((2 * next) + previous - integer_half);
            const auto coefficient_c = static_cast<float>(next - previous) * 0.5F;
            for (auto sample = std::size_t {0}; sample < k_formant_segment_samples; ++sample) {
                const auto time =
                    static_cast<float>(sample) / static_cast<float>(k_formant_segment_samples);
                element_at(output, (segment * k_formant_segment_samples) + sample) =
                    (((((coefficient_a * time) + coefficient_b) * time) + coefficient_c) * time)
                    + static_cast<float>(current);
            }
        }
    }

    auto SynthEngine::initialise_tables() -> void {
        grain_samples_ =
            std::max(std::size_t {1}, static_cast<std::size_t>(sample_rate_ * k_grain_seconds));
        grain_.assign(grain_samples_, 0.0F);
        dry_ring_.assign(k_dry_ring_samples, 0.0F);
        delay_left_.assign(k_delay_ring_samples, 0.0F);
        delay_right_.assign(k_delay_ring_samples, 0.0F);

        exponential_table_.resize(grain_samples_ * k_exponential_table_grain_multiple);
        for (auto index = std::size_t {0}; index < exponential_table_.size(); ++index) {
            element_at(exponential_table_, index) = static_cast<float>(
                std::exp(-static_cast<double>(index) * k_exponential_rate / sample_rate_));
        }

        sine_table_.resize(k_sine_table_samples);
        vibrato_sine_table_.resize(k_sine_table_samples);
        for (auto index = std::size_t {0}; index < k_sine_table_samples; ++index) {
            const auto value = static_cast<float>(std::sin(
                k_two_pi * static_cast<double>(index) / static_cast<double>(k_sine_table_samples)));
            element_at(sine_table_, index) = value;
            element_at(vibrato_sine_table_, index) = value;
        }

        excitation_table_.resize(grain_samples_);
        for (auto index = std::size_t {0}; index < grain_samples_; ++index) {
            const auto time = static_cast<double>(index) / sample_rate_;
            const auto decay_one = std::min(
                exponential_table_.size() - 1,
                static_cast<std::size_t>(static_cast<double>(index) * k_excitation_decay_step_one));
            const auto decay_two = std::min(
                exponential_table_.size() - 1,
                static_cast<std::size_t>(static_cast<double>(index) * k_excitation_decay_step_two));
            element_at(excitation_table_, index) =
                (static_cast<float>(std::sin(k_two_pi * k_excitation_frequency_one * time))
                 * element_at(exponential_table_, decay_one))
                + (static_cast<float>(std::sin(k_two_pi * k_excitation_frequency_two * time))
                   * element_at(exponential_table_, decay_two));
        }

        window_table_.assign(grain_samples_, 1.0F);
        const auto attack = std::max(
            std::size_t {1},
            static_cast<std::size_t>(sample_rate_ * k_window_attack_seconds));
        for (auto index = std::size_t {0}; index < std::min(attack, grain_samples_); ++index) {
            element_at(window_table_, index) = static_cast<float>(
                (1.0 - std::cos(k_pi * static_cast<double>(index) / static_cast<double>(attack)))
                * k_raised_cosine_scale);
        }
        const auto tail_start =
            static_cast<std::size_t>(sample_rate_ * k_window_tail_start_seconds);
        const auto tail_phase = std::max(
            std::size_t {1},
            static_cast<std::size_t>(sample_rate_ * k_window_tail_phase_seconds));
        for (auto index = std::min(tail_start, grain_samples_); index < grain_samples_; ++index) {
            element_at(window_table_, index) = static_cast<float>(
                (1.0
                 - std::cos(
                     k_pi * static_cast<double>(index + tail_phase)
                     / static_cast<double>(tail_phase)))
                * k_raised_cosine_scale);
        }

        frequency_table_.resize(k_frequency_table_samples);
        for (auto index = std::size_t {0}; index < k_frequency_table_samples; ++index) {
            element_at(frequency_table_, index) = static_cast<float>(
                k_frequency_base_hz
                * std::pow(
                    k_semitone_ratio,
                    static_cast<double>(index) / k_frequency_steps_per_semitone));
        }

        build_formant_curve(k_formant_one_points, formant_one_);
        build_formant_curve(k_formant_two_points, formant_two_);
        build_formant_curve(k_formant_three_points, formant_three_);
    }

    auto SynthEngine::rebuild_grain() noexcept -> void {
        const auto vowel = parameters_.vowel;
        const auto vowel_index = std::min(
            k_formant_table_samples - 1,
            static_cast<std::size_t>(clamp01(vowel) * k_vowel_table_scale));
        const auto head_scale = k_head_scale_base + (k_head_scale_amount * parameters_.voice);
        const auto table_rate = static_cast<double>(k_sine_table_samples) / sample_rate_;
        const auto phase_step_one =
            static_cast<double>(element_at(formant_one_, vowel_index) * head_scale) * table_rate;
        const auto phase_step_two =
            static_cast<double>(element_at(formant_two_, vowel_index) * head_scale) * table_rate;
        const auto phase_step_three =
            static_cast<double>(element_at(formant_three_, vowel_index) * head_scale) * table_rate;

        auto phase_one = 0.0;
        auto phase_two = 0.0;
        auto phase_three = 0.0;
        auto decay_one = 0.0;
        auto decay_two = 0.0;
        auto decay_three = 0.0;
        for (auto index = std::size_t {0}; index < grain_samples_; ++index) {
            const auto sine_one =
                element_at(sine_table_, static_cast<std::size_t>(phase_one) % k_sine_table_samples);
            const auto sine_two =
                element_at(sine_table_, static_cast<std::size_t>(phase_two) % k_sine_table_samples);
            const auto sine_three = element_at(
                sine_table_,
                static_cast<std::size_t>(phase_three) % k_sine_table_samples);
            const auto exponential_one = element_at(
                exponential_table_,
                std::min(exponential_table_.size() - 1, static_cast<std::size_t>(decay_one)));
            const auto exponential_two = element_at(
                exponential_table_,
                std::min(exponential_table_.size() - 1, static_cast<std::size_t>(decay_two)));
            const auto exponential_three = element_at(
                exponential_table_,
                std::min(exponential_table_.size() - 1, static_cast<std::size_t>(decay_three)));
            const auto formants = (sine_one * exponential_one) + (sine_two * exponential_two)
                                  + (sine_three * exponential_three);
            element_at(grain_, index) =
                (formants + (k_excitation_mix * element_at(excitation_table_, index)))
                * element_at(window_table_, index);

            phase_one += phase_step_one;
            phase_two += phase_step_two;
            phase_three += phase_step_three;
            if (phase_one >= static_cast<double>(k_sine_table_samples)) {
                phase_one -= static_cast<double>(k_sine_table_samples);
            }
            if (phase_two >= static_cast<double>(k_sine_table_samples)) {
                phase_two -= static_cast<double>(k_sine_table_samples);
            }
            if (phase_three >= static_cast<double>(k_sine_table_samples)) {
                phase_three -= static_cast<double>(k_sine_table_samples);
            }
            decay_one += k_formant_decay_step_one;
            decay_two += k_formant_decay_step_two;
            decay_three += k_formant_decay_step_three;
        }
        grain_rebuild_required_ = false;
    }

    auto SynthEngine::overlap_grain(const std::size_t offset) noexcept -> void {
        for (auto index = std::size_t {0}; index < grain_samples_; ++index) {
            const auto destination = (offset + index) % dry_ring_.size();
            element_at(dry_ring_, destination) += element_at(grain_, index);
        }
    }

    auto SynthEngine::next_random() noexcept -> float {
        random_state_ = (random_state_ * k_random_multiplier) + k_random_increment;
        const auto signed_state = std::bit_cast<std::int32_t>(random_state_);
        return static_cast<float>(signed_state) / k_random_divisor;
    }

    auto SynthEngine::advance_vibrato() noexcept -> float {
        if (vibrato_refresh_counter_ >= vibrato_refresh_interval_) {
            vibrato_refresh_counter_ = 0;
            vibrato_rate_hz_ =
                k_vibrato_random_base_hz + (k_vibrato_random_range_hz * next_random());
        }
        const auto table_index = static_cast<std::size_t>(vibrato_phase_) % k_sine_table_samples;
        const auto sample = (parameters_.vibrato + k_vibrato_depth_floor)
                            * element_at(vibrato_sine_table_, table_index);
        const auto rate_scale = 1.0F + (k_vibrato_rate_scale * parameters_.vibrato);
        vibrato_phase_ += static_cast<double>(
            rate_scale * vibrato_rate_hz_ * static_cast<float>(k_sine_table_samples)
            / static_cast<float>(sample_rate_));
        while (vibrato_phase_ >= static_cast<double>(k_sine_table_samples)) {
            vibrato_phase_ -= static_cast<double>(k_sine_table_samples);
        }
        return sample;
    }

    auto SynthEngine::current_frequency(const float pitch) const noexcept -> float {
        const auto table_index = std::clamp(
            static_cast<std::size_t>(std::max(0.0F, pitch) * k_frequency_steps_per_semitone),
            std::size_t {0},
            frequency_table_.size() - 1);
        return element_at(frequency_table_, table_index);
    }

    auto SynthEngine::advance_atlas_state() noexcept -> void {
        if (gate_) {
            atlas_idle_elapsed_ = 1;
            if (atlas_dirty_) {
                atlas_selector_ =
                    k_atlas_active_base + (k_atlas_active_vowel_scale * parameters_.vowel);
                atlas_dirty_ = false;
            }
            ++atlas_tick_counter_;
            return;
        }

        const auto seven_ticks = atlas_tick_samples_ * k_atlas_seven_tick_mark;
        const auto eight_and_half_ticks = static_cast<int>(
            static_cast<double>(atlas_tick_samples_) * k_atlas_eight_and_half_tick_mark);
        const auto fifteen_ticks = atlas_tick_samples_ * k_atlas_fifteen_tick_mark;
        const auto seventeen_ticks = atlas_tick_samples_ * k_atlas_seventeen_tick_mark;
        const auto timeline_start = atlas_tick_samples_ * k_atlas_timeline_start_tick;
        if (atlas_idle_elapsed_ == seven_ticks || atlas_idle_elapsed_ == fifteen_ticks) {
            atlas_selector_ = static_cast<float>(k_atlas_frame_two) * k_atlas_selector_scale;
        }
        if (atlas_idle_elapsed_ == eight_and_half_ticks || atlas_idle_elapsed_ == seventeen_ticks) {
            atlas_selector_ = static_cast<float>(k_atlas_frame_five) * k_atlas_selector_scale;
        }
        if (atlas_tick_counter_ >= atlas_tick_samples_ && atlas_idle_elapsed_ >= timeline_start) {
            atlas_tick_counter_ = 0;
            if (atlas_idle_index_ >= k_atlas_idle_frames.size()) {
                atlas_idle_index_ = 0;
            }
            atlas_selector_ = static_cast<float>(element_at(k_atlas_idle_frames, atlas_idle_index_))
                              * k_atlas_selector_scale;
            ++atlas_idle_index_;
            atlas_idle_elapsed_ = timeline_start;
        }
        ++atlas_tick_counter_;
        ++atlas_idle_elapsed_;
    }

    auto SynthEngine::render_voice_pass(
        const std::size_t num_samples,
        const std::span<const Event> events,
        const std::size_t dry_start) noexcept -> void {
        const auto bend_count =
            static_cast<std::size_t>(std::ranges::count_if(events, [](const Event& event) -> bool {
                return event.type == EventType::PitchBend && event.sample_offset == 0;
            }));
        const auto available = num_samples > k_zero_offset_bend_padding
                                   ? num_samples - k_zero_offset_bend_padding
                                   : std::size_t {0};
        const auto spacing = bend_count == 0 ? std::size_t {0} : available / bend_count;

        if (std::cmp_greater_equal(samples_since_grain_, dry_ring_.size())) {
            samples_since_grain_ -= static_cast<int>(dry_ring_.size());
        }
        for (auto sample = std::size_t {0}; sample < num_samples; ++sample) {
            apply_regular_events_at(events, sample);
            apply_scheduled_zero_bends_at(events, sample, spacing);
            advance_pitch_bend();
            advance_atlas_state();
            if (gate_) {
                advance_glide();
                const auto working_pitch =
                    static_cast<float>(current_pitch_ + static_cast<double>(advance_vibrato()));
                const auto frequency = current_frequency(working_pitch);
                const auto period = std::max(1, static_cast<int>(sample_rate_ / frequency));
                const auto force_grain = grain_rebuild_required_;
                if (force_grain) {
                    rebuild_grain();
                }
                if (force_grain || samples_since_grain_ >= period) {
                    overlap_grain((dry_start + sample) % dry_ring_.size());
                    samples_since_grain_ = 0;
                }
            }
            ++vibrato_refresh_counter_;
            ++samples_since_grain_;
        }
    }

    auto SynthEngine::render_output_pass(
        const OutputChannels outputs,
        const std::size_t num_samples,
        const std::size_t dry_start) noexcept -> void {
        for (auto sample = std::size_t {0}; sample < num_samples; ++sample) {
            const auto dry_index = (dry_start + sample) % dry_ring_.size();
            const auto dry = element_at(dry_ring_, dry_index);
            const auto left_feedback = element_at(delay_left_, delay_left_tap_);
            const auto right_feedback = element_at(delay_right_, delay_right_tap_);
            element_at(delay_left_, delay_write_) =
                (dry + (k_delay_feedback * left_feedback)) * parameters_.delay_mix;
            element_at(delay_right_, delay_write_) =
                (dry + (k_delay_feedback * right_feedback)) * parameters_.delay_mix;
            const auto left_output_tap = element_at(delay_left_, delay_left_tap_);
            const auto right_output_tap = element_at(delay_right_, delay_right_tap_);

            const auto output_gain =
                (k_output_pitch_base
                 - (static_cast<float>(current_pitch_) / k_output_pitch_divisor))
                * parameters_.volume;
            const auto left = (dry + left_output_tap) * output_gain;
            const auto right = (dry + right_output_tap) * output_gain;
            write_output(outputs, sample, left, right);

            element_at(dry_ring_, dry_index) = 0.0F;
            delay_write_ = (delay_write_ + 1) % k_delay_ring_samples;
            delay_left_tap_ = (delay_left_tap_ + 1) % k_delay_ring_samples;
            delay_right_tap_ = (delay_right_tap_ + 1) % k_delay_ring_samples;
        }
    }

    auto SynthEngine::write_output(
        const OutputChannels outputs,
        const std::size_t sample_offset,
        const float left,
        const float right) noexcept -> void {
        auto channel_index = std::size_t {0};
        for (auto channel : outputs) {
            if (sample_offset < channel.size()) {
                const auto value =
                    channel_index % dsp_detail::k_stereo_channels == 0 ? left : right;
                *std::ranges::next(channel.begin(), static_cast<std::ptrdiff_t>(sample_offset)) =
                    value;
            }
            ++channel_index;
        }
    }

}
