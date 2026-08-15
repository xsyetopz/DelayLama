#include <algorithm>
#include <cmath>
#include <cstddef>
#include <span>
#include <utility>

#include "constants.hpp"
#include "engine.hpp"
#include "midi.hpp"

namespace delaylama {
    namespace {
        using dsp_detail::element_at;
        using dsp_detail::finite_or;
        using dsp_detail::k_atlas_frame_five;
        using dsp_detail::k_atlas_selector_scale;
        using dsp_detail::k_atlas_tick_seconds;
        using dsp_detail::k_bend_center;
        using dsp_detail::k_bend_divisor;
        using dsp_detail::k_bend_maximum;
        using dsp_detail::k_bend_step_count;
        using dsp_detail::k_bend_update_seconds;
        using dsp_detail::k_default_sample_rate;
        using dsp_detail::k_delay_left_seconds;
        using dsp_detail::k_delay_right_seconds;
        using dsp_detail::k_glide_octave_semitones;
        using dsp_detail::k_glide_snap_semitones;
        using dsp_detail::k_glide_time_floor;
        using dsp_detail::k_initial_pitch;
        using dsp_detail::k_initial_vibrato_rate_hz;
        using dsp_detail::k_max_sample_rate;
        using dsp_detail::k_min_sample_rate;
        using dsp_detail::k_vibrato_refresh_seconds;
    }

    auto SynthEngine::clamp01(const float value, const float fallback) noexcept -> float {
        return std::clamp(finite_or(value, fallback), 0.0F, 1.0F);
    }

    auto SynthEngine::sanitise_parameters(const Parameters& parameters) noexcept -> Parameters {
        return Parameters {
            .vowel = clamp01(parameters.vowel, parameter_defaults::vowel),
            .port_time = clamp01(parameters.port_time, parameter_defaults::port_time),
            .delay_mix = clamp01(parameters.delay_mix, parameter_defaults::delay_mix),
            .voice = clamp01(parameters.voice, parameter_defaults::voice),
            .vibrato = clamp01(parameters.vibrato, parameter_defaults::vibrato),
            .volume = clamp01(parameters.volume, parameter_defaults::volume),
            .xy_routing = clamp01(parameters.xy_routing, parameter_defaults::xy_routing),
        };
    }

    auto SynthEngine::normalise_seven_bit(const float value) noexcept -> float {
        const auto finite_value = finite_or(value, 0.0F);
        const auto normalised =
            finite_value > 1.0F ? finite_value / static_cast<float>(midi_protocol::seven_bit_max)
                                : finite_value;
        return clamp01(normalised);
    }

    auto SynthEngine::normalise_pitch_bend_to_integer(const float value) noexcept -> int {
        const auto finite_value = finite_or(value, 0.0F);
        const auto raw =
            finite_value > 1.0F ? finite_value : finite_value * static_cast<float>(k_bend_maximum);
        return std::clamp(static_cast<int>(raw), 0, k_bend_maximum);
    }

    auto SynthEngine::prepare(
        const double sample_rate,
        const int max_block_size,
        const int num_channels) -> void {
        const auto configured = parameters_;
        sample_rate_ = std::clamp(
            finite_or(sample_rate, k_default_sample_rate),
            k_min_sample_rate,
            k_max_sample_rate);
        max_block_size_ = std::max(0, max_block_size);
        prepared_channels_ = std::max(1, num_channels);
        initialise_tables();
        reset();
        set_parameters(configured);
    }

    auto SynthEngine::reset() noexcept -> void {
        parameters_ = Parameters {};
        for (auto& note : notes_) {
            note = NoteSlot {};
        }
        note_age_ = 0;
        current_note_ = no_note;
        gate_ = false;
        current_pitch_ = k_initial_pitch;
        target_pitch_ = k_initial_pitch;
        legato_glide_enabled_ = false;

        bend_current_ = k_bend_center;
        bend_target_ = k_bend_center;
        bend_increment_ = 0;
        bend_steps_remaining_ = 0;
        bend_update_counter_ = 0;
        bend_update_interval_ = std::max(1, static_cast<int>(sample_rate_ * k_bend_update_seconds));
        route_current_ = static_cast<float>(k_initial_pitch);
        route_target_ = route_current_;
        route_increment_ = 0.0F;
        route_steps_remaining_ = 0;

        pad_pitch_modulation_ = parameter_defaults::pad_pitch_modulation;
        pad_vowel_ = parameter_defaults::vowel;
        pad_active_ = false;

        random_state_ = 0;
        vibrato_rate_hz_ = k_initial_vibrato_rate_hz;
        vibrato_phase_ = 0.0;
        vibrato_refresh_counter_ = 0;
        vibrato_refresh_interval_ =
            std::max(1, static_cast<int>(sample_rate_ * k_vibrato_refresh_seconds));

        atlas_selector_ = 0.0F;
        atlas_dirty_ = true;
        atlas_tick_samples_ = std::max(1, static_cast<int>(sample_rate_ * k_atlas_tick_seconds));
        atlas_idle_elapsed_ = 0;
        atlas_tick_counter_ = 0;
        atlas_idle_index_ = 0;

        grain_rebuild_required_ = true;
        samples_since_grain_ = 0;
        dry_cursor_ = 0;
        delay_write_ = 0;
        const auto left_offset = static_cast<std::ptrdiff_t>(sample_rate_ * -k_delay_left_seconds);
        const auto right_offset =
            static_cast<std::ptrdiff_t>(sample_rate_ * -k_delay_right_seconds);
        delay_left_tap_ = dsp_detail::wrap_index(left_offset, dsp_detail::k_delay_ring_samples);
        delay_right_tap_ = dsp_detail::wrap_index(right_offset, dsp_detail::k_delay_ring_samples);

        std::ranges::fill(grain_, 0.0F);
        std::ranges::fill(dry_ring_, 0.0F);
        std::ranges::fill(delay_left_, 0.0F);
        std::ranges::fill(delay_right_, 0.0F);
    }

    auto SynthEngine::set_parameters(const Parameters& parameters) noexcept -> void {
        const auto sanitised = sanitise_parameters(parameters);
        const auto vowel_changed = sanitised.vowel != parameters_.vowel;
        const auto head_changed = sanitised.voice != parameters_.voice;
        parameters_ = sanitised;
        if (vowel_changed) {
            atlas_dirty_ = true;
        }
        grain_rebuild_required_ = grain_rebuild_required_ || vowel_changed || head_changed;
    }

    auto SynthEngine::parameters() const noexcept -> Parameters {
        return parameters_;
    }

    auto SynthEngine::voice_state() const noexcept -> VoiceState {
        return VoiceState {.current_note = current_note_, .gate = gate_};
    }

    auto SynthEngine::pad_state() const noexcept -> PadState {
        return PadState {
            .pitch_modulation = pad_pitch_modulation_,
            .vowel = pad_vowel_,
            .active = pad_active_,
        };
    }

    auto SynthEngine::atlas_selector() const noexcept -> float {
        return atlas_selector_;
    }

    auto SynthEngine::start_pitch_bend(const float value) noexcept -> void {
        bend_target_ = normalise_pitch_bend_to_integer(value);
        const auto delta = bend_target_ - bend_current_;
        bend_increment_ = delta / k_bend_step_count;
        bend_steps_remaining_ = k_bend_step_count;
    }

    auto SynthEngine::advance_pitch_bend() noexcept -> void {
        if (bend_update_counter_ < bend_update_interval_) {
            ++bend_update_counter_;
            return;
        }
        bend_update_counter_ = 0;
        if (bend_steps_remaining_ > 0) {
            bend_current_ += bend_increment_;
            --bend_steps_remaining_;
            parameters_.vowel = static_cast<float>(bend_current_) / k_bend_divisor;
            atlas_dirty_ = true;
            grain_rebuild_required_ = true;
        }
        if (route_steps_remaining_ > 0) {
            route_current_ += route_increment_;
            --route_steps_remaining_;
            parameters_.xy_routing = static_cast<float>(
                (static_cast<double>(route_current_) - k_initial_pitch) / k_glide_octave_semitones);
            target_pitch_ = route_current_;
            if (!legato_glide_enabled_) {
                current_pitch_ = target_pitch_;
            }
        }
    }

    auto SynthEngine::note_on(const int note) noexcept -> void {
        if (note < midi_protocol::minimum_note || note > midi_protocol::maximum_note) {
            return;
        }
        if (!notes_.back().held) {
            for (auto index = notes_.size() - 1; index > 0; --index) {
                if (element_at(notes_, index - 1).held) {
                    element_at(notes_, index) = element_at(notes_, index - 1);
                }
            }
        }
        notes_.front() = NoteSlot {.note = note, .age = ++note_age_, .held = true};
        choose_current_note();
    }

    auto SynthEngine::note_off(const int note) noexcept -> void {
        for (auto index = std::size_t {0}; index < notes_.size(); ++index) {
            if (!element_at(notes_, index).held || element_at(notes_, index).note != note) {
                continue;
            }
            while ((index + 1) < notes_.size() && element_at(notes_, index + 1).held) {
                element_at(notes_, index) = element_at(notes_, index + 1);
                ++index;
            }
            element_at(notes_, index) = NoteSlot {};
            break;
        }
        choose_current_note();
    }

    auto SynthEngine::choose_current_note() noexcept -> void {
        set_current_note(notes_.front().held ? notes_.front().note : no_note);
    }

    auto SynthEngine::set_pitch_target(const double target) noexcept -> void {
        target_pitch_ = target;
        if (!legato_glide_enabled_) {
            current_pitch_ = target_pitch_;
        }
    }

    auto SynthEngine::set_current_note(const int note) noexcept -> void {
        const auto was_gated = gate_;
        const auto changed = note != current_note_;
        current_note_ = note;
        gate_ = note != no_note;
        if (!gate_) {
            legato_glide_enabled_ = false;
            if (was_gated) {
                atlas_selector_ = static_cast<float>(k_atlas_frame_five) * k_atlas_selector_scale;
                atlas_idle_index_ = 0;
                atlas_dirty_ = true;
            }
            return;
        }
        if (!was_gated) {
            target_pitch_ = static_cast<double>(note);
            current_pitch_ = target_pitch_;
            legato_glide_enabled_ = false;
            atlas_dirty_ = true;
            return;
        }
        if (was_gated && element_at(notes_, std::size_t {1}).held) {
            legato_glide_enabled_ = true;
        }
        if (changed) {
            set_pitch_target(static_cast<double>(note));
        }
    }

    auto SynthEngine::advance_glide() noexcept -> void {
        if (!legato_glide_enabled_) {
            current_pitch_ = target_pitch_;
            return;
        }
        const auto distance = target_pitch_ - current_pitch_;
        if (std::abs(distance) <= k_glide_snap_semitones) {
            current_pitch_ = target_pitch_;
            return;
        }
        const auto denominator =
            (static_cast<double>(parameters_.port_time) + k_glide_time_floor) * sample_rate_;
        current_pitch_ += std::copysign(k_glide_octave_semitones / denominator, distance);
    }

    auto SynthEngine::apply_event(const Event& event) noexcept -> void {
        switch (event.type) {
            case EventType::NoteOn:
                if (!event.local_pad) {
                    pad_active_ = false;
                }
                if (normalise_seven_bit(event.value) == 0.0F) {
                    note_off(event.note);
                } else {
                    note_on(event.note);
                }
                break;
            case EventType::NoteOff:
                if (event.local_pad) {
                    pad_active_ = false;
                }
                note_off(event.note);
                break;
            case EventType::PitchBend:
                start_pitch_bend(event.value);
                break;
            case EventType::PadPitch:
                pad_pitch_modulation_ =
                    clamp01(event.value, parameter_defaults::pad_pitch_modulation);
                pad_active_ = true;
                route_target_ = static_cast<float>(
                    k_initial_pitch + (k_glide_octave_semitones * pad_pitch_modulation_));
                route_increment_ =
                    (route_target_ - route_current_) / static_cast<float>(k_bend_step_count);
                route_steps_remaining_ = k_bend_step_count;
                break;
            case EventType::PadVowel:
                pad_vowel_ = clamp01(event.value, parameter_defaults::vowel);
                pad_active_ = true;
                start_pitch_bend(pad_vowel_);
                break;
            case EventType::ControlChange: {
                const auto value = normalise_seven_bit(event.value);
                switch (event.controller) {
                    case midi_protocol::control_change::vibrato:
                        parameters_.vibrato = value;
                        break;
                    case midi_protocol::control_change::port_time:
                        parameters_.port_time = value;
                        break;
                    case midi_protocol::control_change::volume:
                        parameters_.volume = value * midi_protocol::cc7_volume_scale;
                        break;
                    case midi_protocol::control_change::xy_routing:
                        pad_pitch_modulation_ = value;
                        route_target_ = static_cast<float>(
                            k_initial_pitch + (k_glide_octave_semitones * value));
                        route_increment_ = (route_target_ - route_current_)
                                           / static_cast<float>(k_bend_step_count);
                        route_steps_remaining_ = k_bend_step_count;
                        break;
                    case midi_protocol::control_change::delay_mix:
                        parameters_.delay_mix = value;
                        break;
                    case midi_protocol::control_change::voice:
                        parameters_.voice = value;
                        grain_rebuild_required_ = true;
                        break;
                    default:
                        break;
                }
                break;
            }
        }
    }

    auto SynthEngine::apply_regular_events_at(
        const std::span<const Event> events,
        const std::size_t sample_offset) noexcept -> void {
        for (const auto& event : events) {
            if (event.type == EventType::PitchBend && event.sample_offset == 0) {
                continue;
            }
            if (event.sample_offset >= 0 && std::cmp_equal(event.sample_offset, sample_offset)) {
                apply_event(event);
            }
        }
    }

    auto SynthEngine::apply_scheduled_zero_bends_at(
        const std::span<const Event> events,
        const std::size_t sample_offset,
        const std::size_t spacing) noexcept -> void {
        auto bend_index = std::size_t {0};
        for (const auto& event : events) {
            if (event.type != EventType::PitchBend || event.sample_offset != 0) {
                continue;
            }
            const auto scheduled =
                dsp_detail::k_zero_offset_bend_first_sample + (bend_index * spacing);
            if (scheduled == sample_offset) {
                apply_event(event);
            }
            ++bend_index;
        }
    }

    auto SynthEngine::apply_zero_length_events(const std::span<const Event> events) noexcept
        -> void {
        for (const auto& event : events) {
            if (event.sample_offset <= 0) {
                apply_event(event);
            }
        }
    }

    auto SynthEngine::process(
        const OutputChannels outputs,
        const std::size_t num_samples,
        const std::span<const Event> events) noexcept -> void {
        if (num_samples == 0) {
            apply_zero_length_events(events);
            return;
        }
        const auto dry_start = dry_cursor_;
        render_voice_pass(num_samples, events, dry_start);
        render_output_pass(outputs, num_samples, dry_start);
        dry_cursor_ = (dry_start + num_samples) % dry_ring_.size();
    }

}
