import re
import unittest

from scripts.adapter.processor import processor_header, processor_source

EXPECTED_PARAMETER_COUNT = 4


class ProcessorContractTests(unittest.TestCase):
    def test_parameter_layout_has_stable_order_names_and_defaults(self) -> None:
        header = processor_header("Delay Lama")
        source = processor_source()
        expected = [
            ("port_time_parameter_id", "PortTime", "default_port_time"),
            ("vowel_parameter_id", "Vowel", "default_vowel"),
            ("delay_mix_parameter_id", "Delay", "default_delay_mix"),
            ("voice_parameter_id", "HeadSize", "default_voice"),
        ]
        layout = re.findall(
            r'layout\.add\(make_parameter\((\w+), "([^"]+)", (\w+)\)\);',
            source,
        )
        self.assertEqual(layout, expected)
        self.assertEqual(
            re.findall(
                r"constexpr auto default_(port_time|vowel|delay_mix|voice) = ([0-9.]+)F;",
                source,
            ),
            [
                ("port_time", "0.50"),
                ("vowel", "0.50"),
                ("delay_mix", "0.80"),
                ("voice", "0.50"),
            ],
        )
        self.assertEqual(
            source.count("layout.add(make_parameter("), EXPECTED_PARAMETER_COUNT
        )
        self.assertIn("auto getNumPrograms() -> int override", header)
        self.assertIn("factory_program_count = 5", header)

    def test_hidden_midi_controls_are_not_host_parameters(self) -> None:
        header = processor_header("Delay Lama")
        source = processor_source()
        self.assertNotIn("vibrato_parameter_id", header)
        self.assertNotIn("volume_parameter_id", header)
        self.assertNotIn('"Vibrato"', source)
        self.assertNotIn('"Volume"', source)
        self.assertIn("internal_vibrato_", header)
        self.assertIn("internal_volume_", header)
        self.assertIn("default_volume = 0.1F", header)
        self.assertIn("internal_xy_routing_", header)
        self.assertIn("xy_routing = internal_xy_routing_", source)
        self.assertIn("internal_vibrato_ = current.vibrato", source)
        self.assertIn("internal_volume_ = current.volume", source)
        self.assertIn("internal_xy_routing_ = current.xy_routing", source)

    def test_pad_midi_protocol_is_queued_without_new_host_parameters(self) -> None:
        header = processor_header("Delay Lama")
        source = processor_source()
        self.assertIn(
            "enqueue_pad_controls(float first_axis, float second_axis)", header
        )
        self.assertIn("bool has_local_event = false;", header)
        self.assertIn("bool is_local_pad = false;", header)
        self.assertIn("Event local_event {}", header)
        self.assertIn("pad_midi::pitch_message", source)
        self.assertIn("pad_midi::controller_message", source)
        self.assertIn("EventType::PadPitch", source)
        self.assertIn("EventType::PadVowel", source)
        self.assertNotIn("EventType::PadAxes", source)
        self.assertIn("scheduled.has_local_event", source)
        self.assertIn("scheduled.is_local_pad", source)
        self.assertIn(".is_local_pad = true", source)
        self.assertIn(".local_pad = true", source)
        self.assertIn("pad_midi::to_seven_bit(value)", source)
        self.assertIn(".value = controller_wire_value(first_axis)", source)
        self.assertIn("midi_pitch_msb_scale = 128.0F", source)
        self.assertIn("midi_protocol::fourteen_bit_max", source)
        self.assertIn(
            ".value = pitch_wire_value(delaylama::host::pad_midi::vowel_axis_value(second_axis))",
            source,
        )
        self.assertIn("enqueue(delaylama::host::pad_midi::controller_message(", source)
        self.assertIn("enqueue(delaylama::host::pad_midi::pitch_message(", source)
        self.assertLess(
            source.index(
                "enqueue(delaylama::host::pad_midi::controller_message(\n"
                "                pad_midi_channel,\n"
                "                first_axis),\n"
                "                pitch_event"
            ),
            source.index(
                "enqueue(delaylama::host::pad_midi::pitch_message(\n"
                "                pad_midi_channel,\n"
                "                second_axis),\n"
                "                vowel_event"
            ),
        )
        self.assertIn("vowel_event", source)
        self.assertIn("to_core_event(", source)
        self.assertIn(".sample_offset = 0", source)
        self.assertNotIn("set_vowel_from_pad", header + source)
        self.assertEqual(
            source.count("layout.add(make_parameter("), EXPECTED_PARAMETER_COUNT
        )

    def test_state_and_program_contract_are_canonical(self) -> None:
        header = processor_header("Delay Lama")
        source = processor_source()
        self.assertIn("factory_program_count = 5", header)
        for name in ("Rabten", "Dorje", "Ngawang", "Jamyang", "Tinley"):
            self.assertIn(f'"{name}"', header)
        expected_factory_values = {
            "rabten_port_time": "0.5F",
            "rabten_delay": "0.8F",
            "rabten_voice": "0.5F",
            "dorje_port_time": "0.4F",
            "dorje_delay": "0.3F",
            "dorje_voice": "0.0F",
            "ngawang_port_time": "0.8F",
            "ngawang_delay": "0.6F",
            "ngawang_voice": "0.25F",
            "jamyang_port_time": "0.5F",
            "jamyang_delay": "0.0F",
            "jamyang_voice": "0.75F",
            "tinley_port_time": "1.0F",
            "tinley_delay": "0.9F",
            "tinley_voice": "1.0F",
        }
        for name, value in expected_factory_values.items():
            self.assertIn(f"{name} = {value}", header)
        self.assertIn(
            "std::array {rabten_port_time, rabten_delay, rabten_voice}", header
        )
        self.assertIn("values.at(port_time_program_index)", source)
        self.assertIn("values.at(delay_program_index)", source)
        self.assertIn("values.at(voice_program_index)", source)
        self.assertIn("parameters_.copyState()", source)
        self.assertIn(
            'child.setProperty("value", parameter->getValue(), nullptr)', source
        )
        self.assertIn("return parameter->getValue();", source)
        self.assertIn('state.setProperty("program", current_program_, nullptr)', source)
        self.assertIn(
            'canonical.setProperty("program", current_program_, nullptr)', source
        )
        self.assertIn("parameters_.replaceState(canonical)", source)
        self.assertIn(".withProgramChanged(true)", source)
        self.assertLess(
            source.index("parameters_.replaceState(canonical)"),
            source.index(".withProgramChanged(true)"),
        )
        self.assertIn(
            "capture_internal_midi_controls(control_capture_samples)",
            source,
        )
        self.assertIn("static_cast<std::size_t>(num_samples) + 1U", source)
        self.assertIn(
            "apply_boundary_events(static_cast<std::size_t>(num_samples))", source
        )
        self.assertIn("boundary_event.sample_offset = 0", source)
        self.assertIn("sample_offset > value.sample_offset", source)

    def test_midi_mapped_parameters_are_synchronised_without_host_notification(
        self,
    ) -> None:

        source = processor_source()
        capture = source.split(
            "auto DelayLamaAudioProcessor::capture_internal_midi_controls",
            maxsplit=1,
        )[1].split("auto DelayLamaAudioProcessor::publish_visual_state", maxsplit=1)[0]
        self.assertIn("parameter->setValue(normalised)", capture)
        self.assertNotIn("setValueNotifyingHost", capture)
        self.assertIn("const auto current = model_.parameters();", capture)
        for parameter_id, field in (
            ("port_time_parameter_id", "current.port_time"),
            ("vowel_parameter_id", "current.vowel"),
            ("delay_mix_parameter_id", "current.delay_mix"),
            ("voice_parameter_id", "current.voice"),
        ):
            self.assertIn(parameter_id, capture)
            self.assertIn(field, capture)
        self.assertIn("internal_vibrato_ = current.vibrato", capture)
        self.assertIn("internal_volume_ = current.volume", capture)
        self.assertIn("internal_xy_routing_ = current.xy_routing", capture)
        self.assertIn(
            "last_parameter_snapshot_.xy_routing = internal_xy_routing_", capture
        )
        self.assertNotIn("xy_routing_parameter_id", capture)

    def test_visual_state_is_published_after_audio_events(self) -> None:
        header = processor_header("Delay Lama")
        source = processor_source()
        self.assertIn(
            "visual_state_snapshot() const noexcept",
            header,
        )
        self.assertIn("std::atomic<int> visual_note_", header)
        self.assertIn("std::atomic<bool> visual_gate_", header)
        self.assertIn("std::atomic<float> visual_vowel_", header)
        self.assertIn("std::atomic<float> visual_atlas_selector_", header)
        self.assertIn("publish_visual_state();", source)
        self.assertIn("model_.release();", source)
        self.assertIn("auto DelayLamaAudioProcessor::releaseResources()", source)
        self.assertIn("sequence + visual_state_sequence_step", source)
        self.assertIn("next_state.atlas_selector", source)
        self.assertIn("visual_atlas_selector_.store", source)
        self.assertNotIn("juce::ChangeBroadcaster", header + source)
        self.assertNotIn("AsyncUpdater", header + source)
        self.assertNotIn("sendChangeMessage", header + source)
        self.assertNotIn("triggerAsyncUpdate", header + source)

    def test_audio_visual_state_publication_never_notifies_the_message_queue(
        self,
    ) -> None:

        source = processor_source()
        marker = (
            "auto DelayLamaAudioProcessor::publish_visual_state() noexcept -> void {"
        )
        publish_body = source.split(marker, maxsplit=1)[1].split(
            "auto JUCE_CALLTYPE createPluginFilter()", maxsplit=1
        )[0]
        for forbidden in (
            "ChangeBroadcaster",
            "AsyncUpdater",
            "sendChangeMessage",
            "triggerAsyncUpdate",
            "MessageManager::callAsync",
        ):
            self.assertNotIn(forbidden, publish_body)


if __name__ == "__main__":
    unittest.main()
