#ifndef DELAYLAMA_DSP_ENGINE_HPP
#define DELAYLAMA_DSP_ENGINE_HPP

#include <array>
#include <cstddef>
#include <cstdint>
#include <span>
#include <vector>

#include "constants.hpp"

/// Delay Lama synthesis API.
namespace delaylama {

    /// Event kinds accepted by SynthEngine::process.
    enum class EventType : std::uint8_t {
        NoteOn,         ///< Starts or retriggers a note.
        NoteOff,        ///< Releases a note.
        PitchBend,      ///< Sets the normalized 14-bit bend value.
        ControlChange,  ///< Applies a MIDI controller value.
        PadPitch,       ///< Sets local pad pitch modulation.
        PadVowel,       ///< Sets local pad vowel modulation.
    };

    /// One block-relative synthesis event.
    struct Event {
        EventType type = EventType::NoteOff;  ///< Event operation.
        int sample_offset = 0;                ///< Zero-based frame within the block.
        int note = -1;                        ///< Internal note number, or -1 when unused.
        float value = 0.0F;                   ///< Normalized event value.
        int controller = 0;                   ///< MIDI controller number when applicable.
        bool local_pad = false;               ///< Whether the event came from the editor pad.
    };

    /// Initial synthesis values.
    namespace parameter_defaults {
        inline constexpr auto vowel = 0.5F;                 ///< Vowel position.
        inline constexpr auto port_time = 0.5F;             ///< Portamento time.
        inline constexpr auto delay_mix = 0.8F;             ///< Delay level.
        inline constexpr auto voice = 0.5F;                 ///< Voice character.
        inline constexpr auto vibrato = 0.0F;               ///< Vibrato depth.
        inline constexpr auto volume = 0.1F;                ///< Output gain.
        inline constexpr auto xy_routing = 0.0F;            ///< Local pad pitch route.
        inline constexpr auto pad_pitch_modulation = 0.5F;  ///< Neutral pad pitch.
    }

    /// Normalized controls applied to the synthesis engine.
    struct Parameters {
        float vowel = parameter_defaults::vowel;            ///< Vowel position.
        float port_time = parameter_defaults::port_time;    ///< Portamento time.
        float delay_mix = parameter_defaults::delay_mix;    ///< Delay level.
        float voice = parameter_defaults::voice;            ///< Voice character.
        float vibrato = parameter_defaults::vibrato;        ///< Vibrato depth.
        float volume = parameter_defaults::volume;          ///< Output gain.
        float xy_routing = parameter_defaults::xy_routing;  ///< Local pad pitch route.
    };

    /// Current monophonic note state.
    struct VoiceState {
        int current_note = -1;  ///< Internal note number, or -1 while idle.
        bool gate = false;      ///< Whether a note is held.
    };

    /// Current editor-pad modulation state.
    struct PadState {
        float pitch_modulation = parameter_defaults::pad_pitch_modulation;  ///< Pad pitch value.
        float vowel = parameter_defaults::vowel;                            ///< Pad vowel value.
        bool active = false;  ///< Whether the pad is held.
    };

    /// Mutable samples for one output channel.
    using AudioChannel = std::span<float>;
    /// Output channels supplied to SynthEngine::process.
    using OutputChannels = std::span<AudioChannel>;

    /// Stateful grain/overlap-add synthesizer.
    class SynthEngine {
    public:
        /// Creates an unprepared engine with default parameters.
        SynthEngine() noexcept = default;
        /// Releases prepared storage.
        ~SynthEngine() = default;
        /// Engine state cannot be copied.
        SynthEngine(const SynthEngine&) = delete;
        /// Engine state cannot be copy-assigned.
        auto operator=(const SynthEngine&) -> SynthEngine& = delete;
        /// Transfers prepared storage and synthesis state.
        SynthEngine(SynthEngine&&) noexcept = default;
        /// Replaces this engine with transferred synthesis state.
        auto operator=(SynthEngine&&) noexcept -> SynthEngine& = default;

        /// Allocates state for the sample rate, maximum block size, and channel count.
        auto prepare(double sample_rate, int max_block_size, int num_channels) -> void;
        /// Clears note, modulation, delay, and rendering state without reallocating.
        auto reset() noexcept -> void;
        /// Applies sanitized controls to subsequent samples.
        auto set_parameters(const Parameters& parameters) noexcept -> void;
        /// Returns the active sanitized controls.
        [[nodiscard]] auto parameters() const noexcept -> Parameters;
        /// Returns the current note and gate.
        [[nodiscard]] auto voice_state() const noexcept -> VoiceState;
        /// Returns the current editor-pad modulation state.
        [[nodiscard]] auto pad_state() const noexcept -> PadState;
        /// Returns the current normalized atlas selector.
        [[nodiscard]] auto atlas_selector() const noexcept -> float;
        /// Renders a block and applies events at their sample offsets.
        auto process(
            OutputChannels outputs,
            std::size_t num_samples,
            std::span<const Event> events) noexcept -> void;

    private:
        static constexpr auto note_slot_count = std::size_t {128};
        static constexpr auto no_note = -1;

        struct NoteSlot {
            int note = no_note;
            std::uint64_t age = 0;
            bool held = false;
        };

        static auto clamp01(float value, float fallback = 0.0F) noexcept -> float;
        static auto sanitise_parameters(const Parameters& parameters) noexcept -> Parameters;
        static auto normalise_seven_bit(float value) noexcept -> float;
        static auto normalise_pitch_bend_to_integer(float value) noexcept -> int;

        auto apply_event(const Event& event) noexcept -> void;
        auto apply_regular_events_at(
            std::span<const Event> events,
            std::size_t sample_offset) noexcept -> void;
        auto apply_scheduled_zero_bends_at(
            std::span<const Event> events,
            std::size_t sample_offset,
            std::size_t spacing) noexcept -> void;
        auto apply_zero_length_events(std::span<const Event> events) noexcept -> void;
        auto start_pitch_bend(float value) noexcept -> void;
        auto advance_pitch_bend() noexcept -> void;
        auto note_on(int note) noexcept -> void;
        auto note_off(int note) noexcept -> void;
        auto choose_current_note() noexcept -> void;
        auto set_current_note(int note) noexcept -> void;
        auto advance_glide() noexcept -> void;
        auto advance_atlas_state() noexcept -> void;
        auto set_pitch_target(double target) noexcept -> void;

        auto initialise_tables() -> void;
        static auto build_formant_curve(
            const std::array<int, dsp_detail::k_formant_control_point_count>& points,
            std::vector<float>& output) -> void;
        auto rebuild_grain() noexcept -> void;
        auto overlap_grain(std::size_t offset) noexcept -> void;
        auto advance_vibrato() noexcept -> float;
        auto next_random() noexcept -> float;
        [[nodiscard]] auto current_frequency(float pitch) const noexcept -> float;
        auto render_voice_pass(
            std::size_t num_samples,
            std::span<const Event> events,
            std::size_t dry_start) noexcept -> void;
        auto render_output_pass(
            OutputChannels outputs,
            std::size_t num_samples,
            std::size_t dry_start) noexcept -> void;
        static auto write_output(
            OutputChannels outputs,
            std::size_t sample_offset,
            float left,
            float right) noexcept -> void;

        double sample_rate_ = dsp_detail::k_default_sample_rate;
        int max_block_size_ = 0;
        int prepared_channels_ = static_cast<int>(dsp_detail::k_stereo_channels);
        Parameters parameters_ {};

        std::array<NoteSlot, note_slot_count> notes_ {};
        std::uint64_t note_age_ = 0;
        int current_note_ = no_note;
        bool gate_ = false;
        double current_pitch_ = dsp_detail::k_initial_pitch;
        double target_pitch_ = dsp_detail::k_initial_pitch;
        bool legato_glide_enabled_ = false;

        int bend_current_ = dsp_detail::k_bend_center;
        int bend_target_ = dsp_detail::k_bend_center;
        int bend_increment_ = 0;
        int bend_steps_remaining_ = 0;
        int bend_update_counter_ = 0;
        int bend_update_interval_ = 1;
        float route_current_ = static_cast<float>(dsp_detail::k_initial_pitch);
        float route_target_ = static_cast<float>(dsp_detail::k_initial_pitch);
        float route_increment_ = 0.0F;
        int route_steps_remaining_ = 0;

        float pad_pitch_modulation_ = parameter_defaults::pad_pitch_modulation;
        float pad_vowel_ = parameter_defaults::vowel;
        bool pad_active_ = false;

        std::uint32_t random_state_ = 0;
        float vibrato_rate_hz_ = dsp_detail::k_initial_vibrato_rate_hz;
        double vibrato_phase_ = 0.0;
        int vibrato_refresh_counter_ = 0;
        int vibrato_refresh_interval_ = 1;

        float atlas_selector_ = 0.0F;
        bool atlas_dirty_ = true;
        int atlas_tick_samples_ = 1;
        int atlas_idle_elapsed_ = 0;
        int atlas_tick_counter_ = 0;
        std::size_t atlas_idle_index_ = 0;

        bool grain_rebuild_required_ = true;
        int samples_since_grain_ = 0;
        std::size_t grain_samples_ = 0;
        std::size_t dry_cursor_ = 0;
        std::size_t delay_write_ = 0;
        std::size_t delay_left_tap_ = 0;
        std::size_t delay_right_tap_ = 0;

        std::vector<float> grain_;
        std::vector<float> dry_ring_;
        std::vector<float> delay_left_;
        std::vector<float> delay_right_;
        std::vector<float> exponential_table_;
        std::vector<float> sine_table_;
        std::vector<float> vibrato_sine_table_;
        std::vector<float> excitation_table_;
        std::vector<float> window_table_;
        std::vector<float> frequency_table_;
        std::vector<float> formant_one_;
        std::vector<float> formant_two_;
        std::vector<float> formant_three_;
    };

}

#endif
