from textwrap import dedent


def processor_source() -> str:
    return dedent(
        r"""
#include <atomic>
#include <cmath>
#include <cstddef>
#include <memory>
#include <span>
#include <utility>

#include "host/midi.hpp"
#include "juce_adapter/editor.hpp"
#include "juce_adapter/processor.hpp"
#include "dsp/midi.hpp"
#include "dsp/engine.hpp"

        namespace {

            using ParameterLayout = juce::AudioProcessorValueTreeState::ParameterLayout;

            constexpr auto parameter_version = 1;
            constexpr auto parameter_minimum = 0.0F;
            constexpr auto parameter_maximum = 1.0F;
            constexpr auto parameter_step = 0.001F;
            constexpr auto default_port_time = 0.50F;
            // Vowel stays separate because factory records contain only three controls.
            constexpr auto default_vowel = 0.50F;
            constexpr auto default_delay_mix = 0.80F;
            constexpr auto default_voice = 0.50F;
            constexpr auto parameter_change_tolerance = 1.0E-6F;
            constexpr auto stereo_channel_count = 2;
            constexpr auto midi_status_mask = 0xF0U;
            constexpr auto note_on_status = 0x90;
            constexpr auto note_off_status = 0x80;
            constexpr auto controller_status = 0xB0;
            constexpr auto pitch_bend_status = 0xE0;
            constexpr auto pad_midi_channel = 1;

            auto make_parameter(const char* const id, const char* const name, const float default_value)
                -> std::unique_ptr<juce::AudioParameterFloat> {
                return std::make_unique<juce::AudioParameterFloat>(
                    juce::ParameterID {id, parameter_version},
                    name,
                    juce::NormalisableRange<float> {
                        parameter_minimum, parameter_maximum, parameter_step},
                    default_value,
                    juce::AudioParameterFloatAttributes {}.withLabel("normalized"));
            }

            template <typename Type>
            auto release_juce_owned(auto&&... arguments) -> Type* {
                return std::make_unique<Type>(
                    std::forward<decltype(arguments)>(arguments)...)
                    .release();
            }

        }

        DelayLamaAudioProcessor::DelayLamaAudioProcessor()
            : juce::AudioProcessor(
                  BusesProperties().withOutput("Output", juce::AudioChannelSet::stereo(), true))
            , parameters_(*this, nullptr, "PARAMETERS", create_parameter_layout()) {}

        auto DelayLamaAudioProcessor::visual_state_snapshot() const noexcept
            -> delaylama::host::VisualState {
            for (;;) {
                const auto sequence_before = visual_state_sequence_.load(
                    std::memory_order_acquire);
                if ((sequence_before & visual_state_sequence_write_bit) != 0U) {
                    continue;
                }
                const auto state = delaylama::host::VisualState {
                    .note = visual_note_.load(std::memory_order_relaxed),
                    .gate = visual_gate_.load(std::memory_order_relaxed),
                    .vowel = visual_vowel_.load(std::memory_order_relaxed),
                    .atlas_selector = visual_atlas_selector_.load(std::memory_order_relaxed)};
                if (sequence_before == visual_state_sequence_.load(
                        std::memory_order_acquire)) {
                    return state;
                }
            }
        }

        auto DelayLamaAudioProcessor::setCurrentProgram(const int program_index) -> void {
            const auto bounded_index = juce::jlimit(
                0, factory_program_count - 1, program_index);
            const auto& values = factory_program_values.at(
                static_cast<std::size_t>(bounded_index));
            const auto set_parameter = [this](const char* const id, const float value) {
                if (auto* const parameter = parameters_.getParameter(id)) {
                    parameter->setValueNotifyingHost(value);
                }
            };
            set_parameter(port_time_parameter_id, values.at(port_time_program_index));
            set_parameter(delay_mix_parameter_id, values.at(delay_program_index));
            set_parameter(voice_parameter_id, values.at(voice_program_index));
            current_program_ = bounded_index;
            parameters_.state.setProperty("program", current_program_, nullptr);
            has_parameter_snapshot_ = false;
        }

        auto DelayLamaAudioProcessor::create_parameter_layout() -> ParameterLayout {
            auto layout = ParameterLayout {};
            layout.add(make_parameter(port_time_parameter_id, "PortTime", default_port_time));
            layout.add(make_parameter(vowel_parameter_id, "Vowel", default_vowel));
            layout.add(make_parameter(delay_mix_parameter_id, "Delay", default_delay_mix));
            layout.add(make_parameter(voice_parameter_id, "HeadSize", default_voice));
            return layout;
        }

        auto DelayLamaAudioProcessor::isBusesLayoutSupported(const BusesLayout& layouts) const
            -> bool {
            const auto output = layouts.getMainOutputChannelSet();
            return output == juce::AudioChannelSet::mono()
                || output == juce::AudioChannelSet::stereo();
        }

        auto DelayLamaAudioProcessor::prepareToPlay(const double sample_rate, const int samples_per_block)
            -> void {
            message_count_ = 0U;
            event_count_ = 0U;
            has_parameter_snapshot_ = false;
            internal_vibrato_ = default_vibrato;
            internal_volume_ = default_volume;
            internal_xy_routing_ = default_xy_routing;
            last_parameter_snapshot_ = delaylama::Parameters {};
            pending_midi_read_.store(0U, std::memory_order_relaxed);
            pending_midi_write_.store(0U, std::memory_order_relaxed);
            model_.prepare(sample_rate, samples_per_block);
            update_core_parameters();
            publish_visual_state();
        }

        auto DelayLamaAudioProcessor::releaseResources() -> void {
            model_.release();
            publish_visual_state();
        }

        auto DelayLamaAudioProcessor::parameter_snapshot() const -> delaylama::Parameters {
            const auto read_parameter = [this](const char* const id) -> float {
                if (const auto* const parameter = parameters_.getParameter(id)) {
                    return parameter->getValue();
                }
                return 0.0F;
            };

            return {
                .vowel = read_parameter(vowel_parameter_id),
                .port_time = read_parameter(port_time_parameter_id),
                .delay_mix = read_parameter(delay_mix_parameter_id),
                .voice = read_parameter(voice_parameter_id),
                .vibrato = internal_vibrato_,
                .volume = internal_volume_,
                .xy_routing = internal_xy_routing_};
        }

        auto DelayLamaAudioProcessor::update_core_parameters() -> void {
            const auto snapshot = parameter_snapshot();
            const auto differs = [](const float left, const float right) -> bool {
                return std::abs(left - right) > parameter_change_tolerance;
            };
            const auto changed = !has_parameter_snapshot_
                || differs(snapshot.vowel, last_parameter_snapshot_.vowel)
                || differs(snapshot.port_time, last_parameter_snapshot_.port_time)
                || differs(snapshot.delay_mix, last_parameter_snapshot_.delay_mix)
                || differs(snapshot.voice, last_parameter_snapshot_.voice);
            if (changed) {
                model_.set_parameters(snapshot);
                last_parameter_snapshot_ = snapshot;
                has_parameter_snapshot_ = true;
            }
        }

        auto DelayLamaAudioProcessor::processBlock(
            juce::AudioBuffer<float>& buffer,
            juce::MidiBuffer& midi) -> void {
            const auto num_samples = buffer.getNumSamples();
            const auto num_channels = juce::jmin(buffer.getNumChannels(), stereo_channel_count);
            buffer.clear();
            message_count_ = 0U;
            event_count_ = 0U;

            const auto append_message = [this](
                                                   const juce::MidiMessage& message,
                                                   const int sample_offset,
                                                   const bool emit,
                                                   const delaylama::Event local_event,
                                                   const bool has_local_event,
                                                   const bool is_local_pad) {
                    if (message_count_ < max_realtime_events_) {
                        message_scratch_.at(message_count_++) = ScheduledMessage {
                            .message = message,
                            .local_event = local_event,
                            .sample_offset = sample_offset,
                            .emit = emit,
                            .has_local_event = has_local_event,
                            .is_local_pad = is_local_pad};
                    }
                };

            for (const auto metadata : midi) {
                const auto status = metadata.data != nullptr && metadata.numBytes > 0
                    ? static_cast<int>(static_cast<unsigned char>(*metadata.data) & midi_status_mask)
                    : 0;
            if (metadata.numBytes != static_cast<int>(max_midi_message_bytes_)
                    || (status != note_off_status && status != note_on_status
                        && status != controller_status && status != pitch_bend_status)) {
                    continue;
                }
                append_message(
                    metadata.getMessage(),
                    juce::jlimit(0, num_samples, metadata.samplePosition),
                    false,
                    {},
                    false,
                    false);
            }

            auto read_index = pending_midi_read_.load(std::memory_order_relaxed);
            const auto write_index = pending_midi_write_.load(std::memory_order_acquire);
            while (read_index != write_index) {
                const auto& pending = pending_midi_.at(read_index);
                append_message(
                    pending.message,
                    pending.sample_offset,
                    true,
                    pending.local_event,
                    pending.has_local_event,
                    pending.is_local_pad);
                read_index = (read_index + 1U) % max_pending_pad_events_;
            }
            pending_midi_read_.store(read_index, std::memory_order_release);

            for (auto index = std::size_t {1U}; index < message_count_; ++index) {
                const auto value = message_scratch_.at(index);
                auto position = index;
                while (position > 0U
                       && message_scratch_.at(position - 1U).sample_offset > value.sample_offset) {
                    message_scratch_.at(position) = message_scratch_.at(position - 1U);
                    --position;
                }
                message_scratch_.at(position) = value;
            }

            for (auto index = std::size_t {0U}; index < message_count_; ++index) {
                const auto& scheduled = message_scratch_.at(index);
                if (scheduled.has_local_event) {
                    if (event_count_ < max_realtime_events_) {
                        auto event = scheduled.local_event;
                        event.sample_offset = scheduled.sample_offset;
                        event_scratch_.at(event_count_++) = event;
                    }
                } else if (!scheduled.is_local_pad) {
                    if (const auto event = delaylama::host::to_core_event(
                               scheduled.message,
                               scheduled.sample_offset)) {
                        if (event_count_ < max_realtime_events_) {
                            event_scratch_.at(event_count_++) = *event;
                        }
                    }
                }
            }

            update_core_parameters();
            for (auto channel = 0; channel < num_channels; ++channel) {
                output_channels_.at(static_cast<std::size_t>(channel)) = delaylama::AudioChannel {
                    buffer.getWritePointer(channel),
                    static_cast<std::size_t>(num_samples)};
            }
            model_.process(
                std::span<delaylama::AudioChannel> {
                    output_channels_.data(),
                    static_cast<std::size_t>(num_channels)},
                static_cast<std::size_t>(num_samples),
                std::span<const delaylama::Event> {event_scratch_.data(), event_count_});
            const auto control_capture_samples = num_samples == 0
                ? std::size_t {0U}
                : static_cast<std::size_t>(num_samples) + 1U;
            apply_boundary_events(static_cast<std::size_t>(num_samples));
            capture_internal_midi_controls(control_capture_samples);
            publish_visual_state();

            // Reserve the bounded record footprint so addEvent cannot grow on the audio thread.
            midi.clear();
            midi.ensureSize(max_midi_buffer_bytes_);
            for (auto index = std::size_t {0U}; index < message_count_; ++index) {
                const auto& scheduled = message_scratch_.at(index);
                if (scheduled.emit) {
                    midi.addEvent(scheduled.message, scheduled.sample_offset);
                }
            }
        }

        auto DelayLamaAudioProcessor::enqueue_pad_note(const int midi_note, const bool is_down) -> void {
            const auto message = delaylama::host::pad_midi::note_message(
                pad_midi_channel,
                is_down);
            const auto has_local_event = delaylama::midi_protocol::is_host_note(midi_note);
            const auto local_event = delaylama::Event {
                .type = is_down ? delaylama::EventType::NoteOn : delaylama::EventType::NoteOff,
                .sample_offset = 0,
                .note = has_local_event
                    ? delaylama::midi_protocol::to_internal_note(midi_note)
                    : -1,
                .value = static_cast<float>(delaylama::host::pad_midi::note_velocity)
                         / static_cast<float>(delaylama::midi_protocol::seven_bit_max),
                .local_pad = true};
            const auto write_index = pending_midi_write_.load(std::memory_order_relaxed);
            const auto next_write_index = (write_index + 1U) % max_pending_pad_events_;
            const auto read_index = pending_midi_read_.load(std::memory_order_acquire);
            if (next_write_index == read_index) {
                return;
            }
            pending_midi_.at(write_index) = PendingMidi {
                .message = message,
                .local_event = local_event,
                .sample_offset = 0,
                .has_local_event = has_local_event,
                .is_local_pad = true};
            pending_midi_write_.store(next_write_index, std::memory_order_release);
        }

        auto DelayLamaAudioProcessor::enqueue_pad_controls(
            const float first_axis,
            const float second_axis) -> void {
            const auto controller_wire_value = [](const float value) -> float {
                return static_cast<float>(delaylama::host::pad_midi::to_seven_bit(value))
                       / static_cast<float>(delaylama::midi_protocol::seven_bit_max);
            };
            const auto pitch_wire_value = [](const float value) -> float {
                constexpr auto midi_pitch_msb_scale = 128.0F;
                return (static_cast<float>(delaylama::host::pad_midi::to_seven_bit(value))
                        * midi_pitch_msb_scale)
                       / static_cast<float>(delaylama::midi_protocol::fourteen_bit_max);
            };
            const auto pitch_event = delaylama::Event {
                .type = delaylama::EventType::PadPitch,
                .sample_offset = 0,
                .value = controller_wire_value(first_axis),
                .local_pad = true};
            const auto vowel_event = delaylama::Event {
                .type = delaylama::EventType::PadVowel,
                .sample_offset = 0,
                .value = pitch_wire_value(delaylama::host::pad_midi::vowel_axis_value(second_axis)),
                .local_pad = true};
            const auto enqueue = [this](
                                     const auto& message,
                                     const auto& event,
                                     const bool has_local_event) {
                const auto write_index = pending_midi_write_.load(std::memory_order_relaxed);
                const auto next_write_index = (write_index + 1U) % max_pending_pad_events_;
                const auto read_index = pending_midi_read_.load(std::memory_order_acquire);
                if (next_write_index == read_index) {
                    return;
                }
                pending_midi_.at(write_index) = PendingMidi {
                    .message = message,
                    .local_event = event,
                    .sample_offset = 0,
                    .has_local_event = has_local_event,
                    .is_local_pad = true};
                pending_midi_write_.store(next_write_index, std::memory_order_release);
            };

            enqueue(delaylama::host::pad_midi::controller_message(
                pad_midi_channel,
                first_axis),
                pitch_event,
                true);
            enqueue(delaylama::host::pad_midi::pitch_message(
                pad_midi_channel,
                second_axis),
                vowel_event,
                true);
        }

        auto DelayLamaAudioProcessor::apply_boundary_events(const std::size_t num_samples) noexcept
            -> void {
            if (num_samples == 0U) {
                return;
            }

            for (const auto& event : std::span<const delaylama::Event> {
                     event_scratch_.data(), event_count_}) {
                if (event.sample_offset < 0
                    || !std::cmp_equal(event.sample_offset, num_samples)) {
                    continue;
                }

                auto boundary_event = event;
                boundary_event.sample_offset = 0;
                model_.process(
                    {},
                    0U,
                    std::span<const delaylama::Event> {&boundary_event, 1U});
            }
        }

        auto DelayLamaAudioProcessor::createEditor() -> juce::AudioProcessorEditor* {
            return release_juce_owned<DelayLamaAudioProcessorEditor>(*this);
        }

        auto DelayLamaAudioProcessor::getStateInformation(juce::MemoryBlock& destination) -> void {
            auto state = parameters_.copyState();
            for (auto child : state) {
                const auto parameter_id = child.getProperty("id").toString();
                if (const auto* const parameter = parameters_.getParameter(parameter_id)) {
                    child.setProperty("value", parameter->getValue(), nullptr);
                }
            }
            state.setProperty("program", current_program_, nullptr);
            if (const auto xml = state.createXml()) {
                copyXmlToBinary(*xml, destination);
            }
        }

        auto DelayLamaAudioProcessor::setStateInformation(
            const void* const state_data,
            const int size_in_bytes) -> void {
            if (const auto xml = getXmlFromBinary(state_data, size_in_bytes)) {
                const auto restored = juce::ValueTree::fromXml(*xml);
                if (restored.isValid() && restored.getType() == parameters_.state.getType()) {
                    // Rebuild state so obsolete children cannot restore removed host parameters.
                    auto canonical = parameters_.copyState();
                    for (auto child : canonical) {
                        const auto parameter_id = child.getProperty("id").toString();
                        for (const auto restored_child : restored) {
                            if (restored_child.getProperty("id").toString() == parameter_id
                                && restored_child.hasProperty("value")) {
                                child.setProperty(
                                    "value", restored_child.getProperty("value"), nullptr);
                                break;
                            }
                        }
                    }
                    const auto restored_program = restored.hasProperty("program")
                        ? restored.getProperty("program").toString().getIntValue()
                        : 0;
                    current_program_ = juce::jlimit(
                        0, factory_program_count - 1, restored_program);
                    canonical.setProperty("program", current_program_, nullptr);
                    parameters_.replaceState(canonical);
                    updateHostDisplay(
                        juce::AudioProcessorListener::ChangeDetails {}
                            .withProgramChanged(true));
                    internal_vibrato_ = default_vibrato;
                    internal_volume_ = default_volume;
                    internal_xy_routing_ = default_xy_routing;
                    has_parameter_snapshot_ = false;
                }
            }
        }

        auto DelayLamaAudioProcessor::capture_internal_midi_controls(
            [[maybe_unused]] const std::size_t num_samples) noexcept -> void {
            const auto synchronise_host_parameter = [this](const char* const parameter_id, const float value) {
                const auto normalised = juce::jlimit(
                    parameter_minimum, parameter_maximum, value);
                if (auto* const parameter = parameters_.getParameter(parameter_id)) {
                    // Mirror engine values without generating duplicate host automation.
                    parameter->setValue(normalised);
                    return parameter->getValue();
                }
                return normalised;
            };

            // Publish engine ramp state rather than unsmoothed MIDI targets.
            const auto current = model_.parameters();
            last_parameter_snapshot_.port_time = synchronise_host_parameter(
                port_time_parameter_id, current.port_time);
            last_parameter_snapshot_.vowel = synchronise_host_parameter(
                vowel_parameter_id, current.vowel);
            last_parameter_snapshot_.delay_mix = synchronise_host_parameter(
                delay_mix_parameter_id, current.delay_mix);
            last_parameter_snapshot_.voice = synchronise_host_parameter(
                voice_parameter_id, current.voice);
            internal_vibrato_ = current.vibrato;
            internal_volume_ = current.volume;
            internal_xy_routing_ = current.xy_routing;
            last_parameter_snapshot_.vibrato = internal_vibrato_;
            last_parameter_snapshot_.volume = internal_volume_;
            last_parameter_snapshot_.xy_routing = internal_xy_routing_;
            has_parameter_snapshot_ = true;
        }

        auto DelayLamaAudioProcessor::publish_visual_state() noexcept -> void {
            const auto next_state = model_.visual_state();
            const auto previous_state = visual_state_snapshot();
            const auto changed = next_state.note != previous_state.note
                || next_state.gate != previous_state.gate
                || std::abs(next_state.vowel - previous_state.vowel)
                    > parameter_change_tolerance
                || std::abs(next_state.atlas_selector - previous_state.atlas_selector)
                    > parameter_change_tolerance;
            if (!changed) {
                return;
            }

            const auto sequence = visual_state_sequence_.fetch_add(
                visual_state_sequence_write_bit,
                std::memory_order_acq_rel);
            visual_note_.store(next_state.note, std::memory_order_relaxed);
            visual_gate_.store(next_state.gate, std::memory_order_relaxed);
            visual_vowel_.store(next_state.vowel, std::memory_order_relaxed);
            visual_atlas_selector_.store(
                next_state.atlas_selector, std::memory_order_relaxed);
            visual_state_sequence_.store(
                sequence + visual_state_sequence_step,
                std::memory_order_release);
        }

        /// Creates the JUCE plug-in instance requested by the host.
        auto JUCE_CALLTYPE createPluginFilter() -> juce::AudioProcessor* {
            return release_juce_owned<DelayLamaAudioProcessor>();
        }
        """
    ).lstrip()
