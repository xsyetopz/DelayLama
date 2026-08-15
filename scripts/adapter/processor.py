import json
from textwrap import dedent
from typing import TYPE_CHECKING

__all__ = ("processor_header", "processor_source")

if TYPE_CHECKING:
    from processor_runtime import processor_source
else:
    try:
        from .processor_runtime import processor_source
    except ImportError:
        from processor_runtime import processor_source


def processor_header(product_name: str) -> str:
    content = dedent(
        r"""
        #ifndef DELAYLAMA_JUCE_ADAPTER_PROCESSOR_HPP
        #define DELAYLAMA_JUCE_ADAPTER_PROCESSOR_HPP

        #include <juce_audio_processors/juce_audio_processors.h>

        #include <array>
        #include <atomic>
        #include <cstddef>
        #include <cstdint>

        #include "host/processor.hpp"

        /// Adapts JUCE host callbacks to the processor model.
        class DelayLamaAudioProcessor final : public juce::AudioProcessor {
        public:
            /// Creates the processor and parameter state.
            DelayLamaAudioProcessor();
            /// Destroys the processor adapter.
            ~DelayLamaAudioProcessor() override = default;

            /// Prepares rendering for the host configuration.
            auto prepareToPlay(double sample_rate, int samples_per_block) -> void override;
            /// Clears rendering state while retaining parameters.
            auto releaseResources() -> void override;
            /// Accepts supported output-bus layouts.
            auto isBusesLayoutSupported(const BusesLayout& layouts) const -> bool override;
            /// Processes bounded audio and MIDI events at their sample offsets.
            auto processBlock(juce::AudioBuffer<float>&, juce::MidiBuffer&) -> void override;
            /// Exposes JUCE's alternate processing overloads.
            using juce::AudioProcessor::processBlock;

            /// Creates the editor component.
            auto createEditor() -> juce::AudioProcessorEditor* override;
            /// Reports editor availability.
            auto hasEditor() const -> bool override {
                return true;
            }

            /// Returns the product name.
            auto getName() const -> const juce::String override {
                return @@PRODUCT_NAME@@;
            }
            /// Reports MIDI input support.
            auto acceptsMidi() const -> bool override {
                return true;
            }
            /// Reports MIDI output support.
            auto producesMidi() const -> bool override {
                return true;
            }
            /// Reports that this processor is not a MIDI effect.
            auto isMidiEffect() const -> bool override {
                return false;
            }
            /// Returns the declared tail length.
            auto getTailLengthSeconds() const -> double override {
                return tail_length_seconds;
            }

            /// Returns the factory program count.
            auto getNumPrograms() -> int override {
                return factory_program_count;
            }
            /// Returns the active program index.
            auto getCurrentProgram() -> int override {
                return current_program_;
            }
            /// Applies a factory program.
            auto setCurrentProgram(int) -> void override;
            /// Returns a factory program name.
            auto getProgramName(int program_index) -> const juce::String override {
                const auto bounded_index = juce::jlimit(
                    0, factory_program_count - 1, program_index);
                return factory_program_names.at(static_cast<std::size_t>(bounded_index));
            }
            /// Keeps factory program names immutable.
            auto changeProgramName(int, const juce::String&) -> void override {}

            /// Serializes parameter state.
            auto getStateInformation(juce::MemoryBlock& destination) -> void override;
            /// Restores validated parameter state.
            auto setStateInformation(const void* data, int size_in_bytes) -> void override;

            /// Returns mutable parameter state.
            auto parameter_state() noexcept -> juce::AudioProcessorValueTreeState& {
                return parameters_;
            }
            /// Returns read-only parameter state.
            auto parameter_state() const noexcept -> const juce::AudioProcessorValueTreeState& {
                return parameters_;
            }

            /// Returns the latest editor snapshot.
            [[nodiscard]] auto visual_state_snapshot() const noexcept
                -> delaylama::host::VisualState;

            /// Queues a pad note for the next block.
            auto enqueue_pad_note(int midi_note, bool is_down) -> void;
            /// Queues the pad pitch and vowel controls for the next block.
            auto enqueue_pad_controls(float first_axis, float second_axis) -> void;

            /// Portamento parameter identifier.
            static constexpr auto port_time_parameter_id = "port_time";
            /// Vowel parameter identifier.
            static constexpr auto vowel_parameter_id = "vowel";
            /// Delay parameter identifier.
            static constexpr auto delay_mix_parameter_id = "delay_mix";
            /// Voice parameter identifier.
            static constexpr auto voice_parameter_id = "voice";

        private:
            static constexpr auto tail_length_seconds = 2.0;
            static constexpr auto factory_program_count = 5;
            static constexpr auto port_time_program_index = 0U;
            static constexpr auto delay_program_index = 1U;
            static constexpr auto voice_program_index = 2U;
            static constexpr auto factory_program_names = std::array {
                "Rabten", "Dorje", "Ngawang", "Jamyang", "Tinley"};
            static constexpr auto rabten_port_time = 0.5F;
            static constexpr auto rabten_delay = 0.8F;
            static constexpr auto rabten_voice = 0.5F;
            static constexpr auto dorje_port_time = 0.4F;
            static constexpr auto dorje_delay = 0.3F;
            static constexpr auto dorje_voice = 0.0F;
            static constexpr auto ngawang_port_time = 0.8F;
            static constexpr auto ngawang_delay = 0.6F;
            static constexpr auto ngawang_voice = 0.25F;
            static constexpr auto jamyang_port_time = 0.5F;
            static constexpr auto jamyang_delay = 0.0F;
            static constexpr auto jamyang_voice = 0.75F;
            static constexpr auto tinley_port_time = 1.0F;
            static constexpr auto tinley_delay = 0.9F;
            static constexpr auto tinley_voice = 1.0F;
            static constexpr auto factory_program_values = std::array {
                std::array {rabten_port_time, rabten_delay, rabten_voice},
                std::array {dorje_port_time, dorje_delay, dorje_voice},
                std::array {ngawang_port_time, ngawang_delay, ngawang_voice},
                std::array {jamyang_port_time, jamyang_delay, jamyang_voice},
                std::array {tinley_port_time, tinley_delay, tinley_voice}};
            static constexpr auto default_vibrato = 0.0F;
            static constexpr auto default_volume = 0.1F;
            static constexpr auto default_xy_routing = 0.0F;
            static constexpr auto default_visual_note = -1;
            static constexpr auto default_visual_gate = false;
            static constexpr auto default_visual_vowel = 0.5F;
            static constexpr auto default_visual_atlas_selector
                = delaylama::host::visual_state_derived_atlas_selector;
            static constexpr auto visual_state_sequence_write_bit = std::uint64_t {1U};
            static constexpr auto visual_state_sequence_step = std::uint64_t {2U};
            static constexpr std::size_t max_realtime_events_ = 4096U;
            static constexpr std::size_t max_pending_pad_events_ = 257U;
            static constexpr std::size_t max_midi_message_bytes_ = 3U;
            static constexpr std::size_t output_channel_count_ = 2U;
            static constexpr std::size_t midi_buffer_event_overhead_bytes_
                = sizeof(std::int32_t) + sizeof(std::uint16_t);
            static constexpr std::size_t max_midi_buffer_bytes_
                = max_realtime_events_
                * (max_midi_message_bytes_ + midi_buffer_event_overhead_bytes_);

            struct PendingMidi {
                juce::MidiMessage message;
                delaylama::Event local_event {};
                int sample_offset = 0;
                bool has_local_event = false;
                bool is_local_pad = false;
            };

            struct ScheduledMessage {
                juce::MidiMessage message;
                delaylama::Event local_event {};
                int sample_offset = 0;
                bool emit = false;
                bool has_local_event = false;
                bool is_local_pad = false;
            };

            static auto create_parameter_layout()
                -> juce::AudioProcessorValueTreeState::ParameterLayout;
            [[nodiscard]] auto parameter_snapshot() const -> delaylama::Parameters;
            auto update_core_parameters() -> void;
            // Defer block-end events so JUCE boundary timestamps are not dropped.
            auto apply_boundary_events(std::size_t num_samples) noexcept -> void;
            auto capture_internal_midi_controls(std::size_t num_samples) noexcept -> void;
            auto publish_visual_state() noexcept -> void;

            delaylama::host::ProcessorModel model_;
            juce::AudioProcessorValueTreeState parameters_;
            delaylama::Parameters last_parameter_snapshot_ {};
            bool has_parameter_snapshot_ = false;
            float internal_vibrato_ = default_vibrato;
            float internal_volume_ = default_volume;
            float internal_xy_routing_ = default_xy_routing;
            int current_program_ = 0;
            std::atomic<int> visual_note_ {default_visual_note};
            std::atomic<bool> visual_gate_ {default_visual_gate};
            std::atomic<float> visual_vowel_ {default_visual_vowel};
            std::atomic<float> visual_atlas_selector_ {default_visual_atlas_selector};
            std::atomic<std::uint64_t> visual_state_sequence_ {0U};

            std::array<PendingMidi, max_pending_pad_events_> pending_midi_ {};
            std::atomic<std::size_t> pending_midi_write_ {0};
            std::atomic<std::size_t> pending_midi_read_ {0};
            std::array<ScheduledMessage, max_realtime_events_> message_scratch_ {};
            std::size_t message_count_ = 0U;
            std::array<delaylama::Event, max_realtime_events_> event_scratch_ {};
            std::size_t event_count_ = 0U;
            std::array<delaylama::AudioChannel, output_channel_count_> output_channels_ {};

        JUCE_DECLARE_NON_COPYABLE_WITH_LEAK_DETECTOR(DelayLamaAudioProcessor)
        };

        #endif

        """
    ).lstrip()
    return content.replace("@@PRODUCT_NAME@@", json.dumps(product_name))
