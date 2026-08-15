#include <algorithm>
#include <array>
#include <cassert>
#include <cmath>
#include <cstddef>
#include <span>

#include "dsp/constants.hpp"
#include "dsp/midi.hpp"
#include "support.hpp"

namespace delaylama::tests {
    namespace {

        auto test_note_event_is_applied_at_its_sample() -> void {
            SynthEngine engine;
            engine.prepare(k_sample_rate, k_standard_block_samples, k_channels);
            constexpr auto offset = 96;
            StereoBlock block(k_standard_block_samples);
            const auto events = std::array {note_on(k_note, offset)};
            engine.process(block.outputs(), block.left.size(), events);
            for (auto index = std::size_t {0}; index < offset; ++index) {
                assert(std::abs(sample_at(block.left, index)) < k_tolerance);
                assert(std::abs(sample_at(block.right, index)) < k_tolerance);
            }
            assert(has_signal(std::span<const float> {block.left}.subspan(offset)));
        }

        auto test_mid_period_head_change_forces_overlap() -> void {
            SynthEngine changed;
            SynthEngine baseline;
            changed.prepare(k_sample_rate, k_standard_block_samples, k_channels);
            baseline.prepare(k_sample_rate, k_standard_block_samples, k_channels);
            constexpr auto change_offset = 64;
            const auto changed_events = std::array {
                note_on(),
                Event {
                    .type = EventType::ControlChange,
                    .sample_offset = change_offset,
                    .note = -1,
                    .value = 1.0F,
                    .controller = delaylama::midi_protocol::control_change::voice,
                }};
            const auto baseline_events = std::array {note_on()};
            StereoBlock changed_block(k_standard_block_samples);
            StereoBlock baseline_block(k_standard_block_samples);
            changed.process(changed_block.outputs(), changed_block.left.size(), changed_events);
            baseline.process(baseline_block.outputs(), baseline_block.left.size(), baseline_events);
            for (auto index = std::size_t {0}; index < change_offset; ++index) {
                assert(
                    std::abs(
                        sample_at(changed_block.left, index)
                        - sample_at(baseline_block.left, index))
                    < k_tolerance);
            }
            auto changed_after_event = false;
            for (auto index = std::size_t {change_offset}; index < changed_block.left.size();
                 ++index) {
                changed_after_event = changed_after_event
                                      || std::abs(
                                             sample_at(changed_block.left, index)
                                             - sample_at(baseline_block.left, index))
                                             > k_tolerance;
            }
            assert(changed_after_event);
        }

        auto test_zero_offset_bend_is_deferred_to_sample_one() -> void {
            SynthEngine deferred;
            SynthEngine explicit_one;
            deferred.prepare(k_sample_rate, k_standard_block_samples, k_channels);
            explicit_one.prepare(k_sample_rate, k_standard_block_samples, k_channels);
            const auto deferred_events = std::array {note_on(), pitch_bend(0.0F, 0)};
            const auto explicit_events = std::array {note_on(), pitch_bend(0.0F, 1)};
            StereoBlock deferred_block(k_standard_block_samples);
            StereoBlock explicit_block(k_standard_block_samples);
            deferred.process(deferred_block.outputs(), deferred_block.left.size(), deferred_events);
            explicit_one.process(
                explicit_block.outputs(),
                explicit_block.left.size(),
                explicit_events);
            for (auto index = std::size_t {0}; index < deferred_block.left.size(); ++index) {
                assert(
                    std::abs(
                        sample_at(deferred_block.left, index)
                        - sample_at(explicit_block.left, index))
                    < k_tolerance);
                assert(
                    std::abs(
                        sample_at(deferred_block.right, index)
                        - sample_at(explicit_block.right, index))
                    < k_tolerance);
            }
        }

        auto test_note_stack_releases_one_newest_occurrence() -> void {
            SynthEngine engine;
            engine.prepare(k_sample_rate, k_note_stack_block_samples, k_channels);
            StereoBlock block(1);
            const auto first = std::array {note_on(k_stack_initial_note)};
            engine.process(block.outputs(), block.left.size(), first);
            const auto second = std::array {note_on(k_stack_newest_note)};
            engine.process(block.outputs(), block.left.size(), second);
            const auto duplicate = std::array {note_on(k_stack_newest_note)};
            engine.process(block.outputs(), block.left.size(), duplicate);
            const auto release = std::array {note_off(k_stack_newest_note)};
            engine.process(block.outputs(), block.left.size(), release);
            assert(engine.voice_state().gate);
            assert(engine.voice_state().current_note == k_stack_newest_note);
            engine.process(block.outputs(), block.left.size(), release);
            assert(engine.voice_state().current_note == k_stack_initial_note);
        }

        auto test_short_retrigger_waits_for_remaining_grain_period() -> void {
            SynthEngine retriggered;
            SynthEngine released;
            retriggered.prepare(k_sample_rate, k_standard_block_samples, k_channels);
            released.prepare(k_sample_rate, k_standard_block_samples, k_channels);
            const auto start = std::array {note_on()};
            StereoBlock onset_a(k_retrigger_onset_samples);
            StereoBlock onset_b(k_retrigger_onset_samples);
            retriggered.process(onset_a.outputs(), onset_a.left.size(), start);
            released.process(onset_b.outputs(), onset_b.left.size(), start);
            const auto stop = std::array {note_off()};
            StereoBlock rest_a(k_retrigger_rest_samples);
            StereoBlock rest_b(k_retrigger_rest_samples);
            retriggered.process(rest_a.outputs(), rest_a.left.size(), stop);
            released.process(rest_b.outputs(), rest_b.left.size(), stop);

            StereoBlock retrigger_block(k_retrigger_comparison_samples);
            StereoBlock released_block(k_retrigger_comparison_samples);
            retriggered.process(retrigger_block.outputs(), retrigger_block.left.size(), start);
            released.process(released_block.outputs(), released_block.left.size(), {});
            for (auto index = std::size_t {0}; index < k_retrigger_unchanged_samples; ++index) {
                assert(
                    std::abs(
                        sample_at(retrigger_block.left, index)
                        - sample_at(released_block.left, index))
                    < k_tolerance);
            }
            auto diverged_when_due = false;
            for (auto index = std::size_t {k_retrigger_divergence_start};
                 index < retrigger_block.left.size();
                 ++index) {
                diverged_when_due = diverged_when_due
                                    || std::abs(
                                           sample_at(retrigger_block.left, index)
                                           - sample_at(released_block.left, index))
                                           > k_tolerance;
            }
            assert(diverged_when_due);
        }

        auto test_long_idle_advances_vibrato_refresh_clock() -> void {
            SynthEngine idled_one;
            SynthEngine idled_two;
            SynthEngine fresh;
            for (auto* const engine : {&idled_one, &idled_two, &fresh}) {
                engine->prepare(k_sample_rate, k_standard_block_samples, k_channels);
                auto parameters = engine->parameters();
                parameters.vibrato = 1.0F;
                engine->set_parameters(parameters);
            }

            const auto idle_samples =
                static_cast<std::size_t>(
                    k_sample_rate * delaylama::dsp_detail::k_vibrato_refresh_seconds)
                + 1;
            auto elapsed = std::size_t {0};
            while (elapsed < idle_samples) {
                const auto block_samples =
                    std::min(std::size_t {k_standard_block_samples}, idle_samples - elapsed);
                StereoBlock first_idle(block_samples);
                StereoBlock second_idle(block_samples);
                idled_one.process(first_idle.outputs(), block_samples, {});
                idled_two.process(second_idle.outputs(), block_samples, {});
                elapsed += block_samples;
            }

            StereoBlock first_note(k_max_test_samples);
            StereoBlock second_note(k_max_test_samples);
            StereoBlock fresh_note(k_max_test_samples);
            const auto start = std::array {note_on()};
            idled_one.process(first_note.outputs(), first_note.left.size(), start);
            idled_two.process(second_note.outputs(), second_note.left.size(), start);
            fresh.process(fresh_note.outputs(), fresh_note.left.size(), start);

            auto differs_from_fresh = false;
            for (auto index = std::size_t {0}; index < first_note.left.size(); ++index) {
                assert(
                    std::abs(sample_at(first_note.left, index) - sample_at(second_note.left, index))
                    < k_tolerance);
                differs_from_fresh =
                    differs_from_fresh
                    || std::abs(
                           sample_at(first_note.left, index) - sample_at(fresh_note.left, index))
                           > k_tolerance;
            }
            assert(differs_from_fresh);
        }

        auto test_reset_restores_deterministic_grain_and_random_state() -> void {
            SynthEngine warmed;
            SynthEngine fresh;
            warmed.prepare(k_sample_rate, k_large_block_samples, k_channels);
            fresh.prepare(k_sample_rate, k_large_block_samples, k_channels);
            StereoBlock warm(k_warmup_samples);
            const auto warm_events = std::array {note_on()};
            warmed.process(warm.outputs(), warm.left.size(), warm_events);
            warmed.reset();

            StereoBlock reset_block(k_large_block_samples);
            StereoBlock fresh_block(k_large_block_samples);
            warmed.process(reset_block.outputs(), reset_block.left.size(), warm_events);
            fresh.process(fresh_block.outputs(), fresh_block.left.size(), warm_events);
            for (auto index = std::size_t {0}; index < reset_block.left.size(); ++index) {
                assert(
                    std::abs(
                        sample_at(reset_block.left, index) - sample_at(fresh_block.left, index))
                    < k_tolerance);
                assert(
                    std::abs(
                        sample_at(reset_block.right, index) - sample_at(fresh_block.right, index))
                    < k_tolerance);
            }
        }

        auto test_delay_taps_are_independent_and_preserve_dry() -> void {
            SynthEngine dry;
            SynthEngine delayed;
            dry.prepare(k_sample_rate, k_standard_block_samples, k_channels);
            delayed.prepare(k_sample_rate, k_standard_block_samples, k_channels);
            auto dry_parameters = dry.parameters();
            dry_parameters.delay_mix = 0.0F;
            dry.set_parameters(dry_parameters);
            auto delayed_parameters = delayed.parameters();
            delayed_parameters.delay_mix = 1.0F;
            delayed.set_parameters(delayed_parameters);

            const auto left_tap = static_cast<std::size_t>(
                k_sample_rate * delaylama::dsp_detail::k_delay_left_seconds);
            const auto right_tap = static_cast<std::size_t>(
                k_sample_rate * delaylama::dsp_detail::k_delay_right_seconds);
            const auto total_samples = right_tap + 1024;
            auto global_sample = std::size_t {0};
            auto left_diverged = false;
            auto right_diverged = false;
            while (global_sample < total_samples) {
                const auto block_samples =
                    std::min(std::size_t {k_standard_block_samples}, total_samples - global_sample);
                StereoBlock dry_block(block_samples);
                StereoBlock delayed_block(block_samples);
                if (global_sample == 0) {
                    const auto start = std::array {note_on()};
                    dry.process(dry_block.outputs(), block_samples, start);
                    delayed.process(delayed_block.outputs(), block_samples, start);
                } else {
                    dry.process(dry_block.outputs(), block_samples, {});
                    delayed.process(delayed_block.outputs(), block_samples, {});
                }
                for (auto index = std::size_t {0}; index < block_samples; ++index) {
                    const auto absolute = global_sample + index;
                    const auto left_difference = std::abs(
                        sample_at(dry_block.left, index) - sample_at(delayed_block.left, index));
                    const auto right_difference = std::abs(
                        sample_at(dry_block.right, index) - sample_at(delayed_block.right, index));
                    assert(absolute >= left_tap || left_difference < k_tolerance);
                    assert(absolute >= right_tap || right_difference < k_tolerance);
                    left_diverged =
                        left_diverged || (absolute >= left_tap && left_difference > k_tolerance);
                    right_diverged =
                        right_diverged || (absolute >= right_tap && right_difference > k_tolerance);
                }
                global_sample += block_samples;
            }
            assert(left_diverged);
            assert(right_diverged);
        }

        auto test_sample_time_atlas_selector() -> void {
            constexpr auto frame_scale = 1.0F / 30.0F;
            const auto tick = static_cast<std::size_t>(
                k_sample_rate * delaylama::dsp_detail::k_atlas_tick_seconds);

            SynthEngine active;
            active.prepare(k_sample_rate, k_atlas_test_block_samples, k_channels);
            const auto active_event = std::array {note_on()};
            active.process({}, 1, active_event);
            assert(std::abs(active.atlas_selector() - k_active_atlas_selector) < k_tolerance);
            const auto release_event = std::array {note_off()};
            active.process({}, 1, release_event);
            assert(
                std::abs(active.atlas_selector() - (k_release_atlas_frame * frame_scale))
                < k_tolerance);
            active.process({}, (k_seven_tick_mark * tick) - k_pre_boundary_samples, {});
            assert(
                std::abs(active.atlas_selector() - (k_release_atlas_frame * frame_scale))
                < k_tolerance);
            active.process({}, 1, {});
            assert(
                std::abs(active.atlas_selector() - (k_seven_tick_atlas_frame * frame_scale))
                < k_tolerance);

            SynthEngine seven_tick_idle;
            seven_tick_idle.prepare(k_sample_rate, k_atlas_test_block_samples, k_channels);
            seven_tick_idle.process({}, (k_seven_tick_mark * tick) + 1, {});
            assert(
                std::abs(
                    seven_tick_idle.atlas_selector() - (k_seven_tick_atlas_frame * frame_scale))
                < k_tolerance);

            SynthEngine eight_tick_idle;
            eight_tick_idle.prepare(k_sample_rate, k_atlas_test_block_samples, k_channels);
            const auto eight_and_half = static_cast<std::size_t>(static_cast<double>(tick) * 8.5);
            eight_tick_idle.process({}, eight_and_half + 1, {});
            assert(
                std::abs(eight_tick_idle.atlas_selector() - (k_release_atlas_frame * frame_scale))
                < k_tolerance);

            SynthEngine timeline;
            timeline.prepare(k_sample_rate, k_atlas_test_block_samples, k_channels);
            timeline.process({}, (k_timeline_start_tick * tick) + 1, {});
            assert(
                std::abs(timeline.atlas_selector() - (k_release_atlas_frame * frame_scale))
                < k_tolerance);
            timeline.process({}, tick, {});
            assert(
                std::abs(timeline.atlas_selector() - (k_timeline_first_frame * frame_scale))
                < k_tolerance);
        }

        auto test_dsp_constants_are_exactly_owned() -> void {
            static_assert(delaylama::dsp_detail::k_dry_ring_samples == k_expected_dry_ring_samples);
            static_assert(
                delaylama::dsp_detail::k_delay_ring_samples == k_expected_delay_ring_samples);
            static_assert(
                delaylama::dsp_detail::k_sine_table_samples == k_expected_sine_table_samples);
            static_assert(
                delaylama::dsp_detail::k_frequency_table_samples
                == k_expected_frequency_table_samples);
            static_assert(
                delaylama::dsp_detail::k_formant_table_samples == k_expected_formant_table_samples);
            static_assert(delaylama::dsp_detail::k_bend_step_count == k_expected_bend_step_count);
            static_assert(
                delaylama::dsp_detail::k_excitation_decay_step_one
                == k_expected_excitation_decay_one);
            static_assert(
                delaylama::dsp_detail::k_excitation_decay_step_two
                == k_expected_excitation_decay_two);
            static_assert(
                delaylama::dsp_detail::k_formant_decay_step_one == k_expected_formant_decay_one);
            static_assert(
                delaylama::dsp_detail::k_formant_decay_step_two == k_expected_formant_decay_two);
            static_assert(
                delaylama::dsp_detail::k_formant_decay_step_three
                == k_expected_formant_decay_three);
            static_assert(
                delaylama::dsp_detail::k_formant_one_points == k_expected_formant_one_points);
            static_assert(
                delaylama::dsp_detail::k_formant_two_points == k_expected_formant_two_points);
            static_assert(
                delaylama::dsp_detail::k_formant_three_points == k_expected_formant_three_points);
        }

    }

    void run_render_tests() {
        test_dsp_constants_are_exactly_owned();
        test_note_event_is_applied_at_its_sample();
        test_mid_period_head_change_forces_overlap();
        test_zero_offset_bend_is_deferred_to_sample_one();
        test_note_stack_releases_one_newest_occurrence();
        test_short_retrigger_waits_for_remaining_grain_period();
        test_long_idle_advances_vibrato_refresh_clock();
        test_reset_restores_deterministic_grain_and_random_state();
        test_sample_time_atlas_selector();
        test_delay_taps_are_independent_and_preserve_dry();
    }

}
