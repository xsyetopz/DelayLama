#include <juce_core/juce_core.h>
#include <juce_events/juce_events.h>
#include <juce_graphics/juce_graphics.h>
#include <juce_gui_basics/juce_gui_basics.h>

#include <array>
#include <atomic>
#include <cmath>
#include <cstddef>
#include <fstream>
#include <iostream>
#include <memory>
#include <string_view>

#include "juce_adapter/processor.hpp"
#include "module_exports.hpp"

namespace {

    constexpr auto editor_width = 360;
    constexpr auto editor_height = 510;
    constexpr auto delay_left = 104;
    constexpr auto delay_top = 479;
    constexpr auto delay_width = 152;
    constexpr auto delay_height = 25;
    constexpr auto help_artwork_width = 253;
    constexpr auto help_artwork_height = 275;
    constexpr auto normalized_drag_start = 0.2F;
    constexpr auto normalized_drag_end = 0.65F;
    constexpr auto normalized_marker_end = 0.8F;
    constexpr auto normalized_midpoint = 0.5F;
    constexpr auto parameter_tolerance = 1.0E-5F;
    constexpr auto parameter_step_tolerance = 5.1E-4F;
    constexpr auto file_line_buffer_size = 512U;
    constexpr auto midi_test_sample_rate = 44100.0;
    constexpr auto midi_test_block_size = 16;
    constexpr auto midi_test_channel_count = 2;
    constexpr auto midi_test_channel = 1U;
    constexpr auto midi_message_byte_count = std::size_t {3};
    constexpr auto midi_sync_event_count = 4;
    constexpr auto port_time_controller = 5U;
    constexpr auto port_time_value = 32U;
    constexpr auto bend_low_byte = 0U;
    constexpr auto bend_high_byte = 32U;
    constexpr auto delay_mix_controller = 12U;
    constexpr auto delay_mix_value = 64U;
    constexpr auto voice_controller = 13U;
    constexpr auto voice_value = 96U;
    constexpr auto seven_bit_maximum = 127.0F;

    auto require(const bool condition, const std::string_view label) -> bool {
        if (!condition) {
            std::cerr << "editor interaction assertion failed: " << label << '\n';
        }
        return condition;
    }

    auto find_component(juce::Component& root, const juce::String& component_id)
        -> juce::Component* {
        if (root.getComponentID() == component_id) {
            return &root;
        }
        for (auto index = 0; index < root.getNumChildComponents(); ++index) {
            if (auto* const match = find_component(*root.getChildComponent(index), component_id)) {
                return match;
            }
        }
        return nullptr;
    }

    auto find_visible_help_window(juce::Component& editor) -> juce::DocumentWindow* {
        for (auto index = 0; index < editor.getNumChildComponents(); ++index) {
            auto* const child = editor.getChildComponent(index);
            auto* const window = dynamic_cast<juce::DocumentWindow*>(child);
            if (window != nullptr && window->isVisible()) {
                return window;
            }
        }
        return nullptr;
    }

    auto make_mouse_event(
        juce::Component& target,
        const juce::Point<float> position,
        const juce::Point<float> mouse_down_position,
        const bool dragged) -> juce::MouseEvent {
        const auto source = juce::Desktop::getInstance().getMainMouseSource();
        const auto now = juce::Time::getCurrentTime();
        return {
            source,
            position,
            juce::ModifierKeys::leftButtonModifier,
            juce::MouseInputSource::defaultPressure,
            0.0F,
            0.0F,
            0.0F,
            0.0F,
            &target,
            &target,
            now,
            mouse_down_position,
            now,
            1,
            dragged};
    }

    auto file_contains_token(const char* const path, const std::string_view token) -> bool {
        auto stream = std::ifstream {path};
        if (!stream) {
            return false;
        }
        auto line = std::array<char, file_line_buffer_size> {};
        while (stream.getline(line.data(), static_cast<std::streamsize>(line.size()))) {
            if (std::string_view {line.data()}.find(token) != std::string_view::npos) {
                return true;
            }
        }
        return false;
    }

    auto render_component(juce::Component& component) -> juce::Image {
        auto image =
            juce::Image {juce::Image::ARGB, component.getWidth(), component.getHeight(), true};
        auto graphics = juce::Graphics {image};
        component.paintEntireComponent(graphics, true);
        return image;
    }

    auto images_differ(const juce::Image& left, const juce::Image& right) -> bool {
        if (left.getBounds() != right.getBounds()) {
            return true;
        }
        for (auto row = 0; row < left.getHeight(); ++row) {
            for (auto column = 0; column < left.getWidth(); ++column) {
                if (left.getPixelAt(column, row) != right.getPixelAt(column, row)) {
                    return true;
                }
            }
        }
        return false;
    }

    auto image_is_transparent(const juce::Image& image) -> bool {
        for (auto row = 0; row < image.getHeight(); ++row) {
            for (auto column = 0; column < image.getWidth(); ++column) {
                if (image.getPixelAt(column, row).getAlpha() != 0U) {
                    return false;
                }
            }
        }
        return true;
    }

    auto trigger_button(juce::Button& button) -> void {
        const auto position = button.getLocalBounds().getCentre().toFloat();
        const auto event = make_mouse_event(button, position, position, false);
        auto& component = static_cast<juce::Component&>(button);
        component.mouseDown(event);
        component.mouseUp(event);
    }

    auto generated_idle_animation_contract() -> bool {
        const auto source_contract = std::array<std::string_view, 3> {
            "Present published atlas state so UI timing cannot drift from audio timing",
            "animation_phase_ = model_.animation_frame();",
            "visual_state.atlas_selector - last_visual_state_.atlas_selector"};
        for (const auto token : source_contract) {
            if (!file_contains_token(DELAYLAMA_GENERATED_EDITOR_SOURCE, token)) {
                return false;
            }
        }
        const auto legacy_clock = std::array<std::string_view, 2> {
            "idle_animation_sequence_",
            "idle_animation_position_"};
        for (const auto token : legacy_clock) {
            if (file_contains_token(DELAYLAMA_GENERATED_EDITOR_HEADER, token)
                || file_contains_token(DELAYLAMA_GENERATED_EDITOR_SOURCE, token)) {
                return false;
            }
        }
        return file_contains_token(
                   DELAYLAMA_GENERATED_EDITOR_HEADER,
                   "animation_frame_capacity_ = 30U")
               && file_contains_token(
                   DELAYLAMA_GENERATED_EDITOR_HEADER,
                   "release_atlas_frame_ = 5U");
    }

    class HostNotificationProbe final : public delaylama::juce_test::AudioProcessorListener {
    public:
        auto audioProcessorParameterChanged(
            [[maybe_unused]] delaylama::juce_test::AudioProcessor* const processor,
            [[maybe_unused]] const int parameter_index,
            [[maybe_unused]] const float new_value) -> void override {
            ++parameter_notifications;
        }

        auto audioProcessorChanged(
            [[maybe_unused]] delaylama::juce_test::AudioProcessor* const processor,
            [[maybe_unused]] const ChangeDetails& details) -> void override {}

        int parameter_notifications = 0;
    };

    auto midi_parameter_sync_contract() -> bool {
        constexpr auto first_bend_tick_blocks = 27U;
        constexpr auto bend_target = 4096.0F / 16383.0F;
        auto processor = DelayLamaAudioProcessor {};
        auto probe = HostNotificationProbe {};
        processor.addListener(&probe);
        processor.prepareToPlay(midi_test_sample_rate, midi_test_block_size);

        auto audio =
            delaylama::juce_test::AudioBuffer {midi_test_channel_count, midi_test_block_size};
        auto midi = delaylama::juce_test::MidiBuffer {};
        constexpr auto cc_status = static_cast<unsigned char>(0xB0U | (midi_test_channel - 1));
        constexpr auto bend_status = static_cast<unsigned char>(0xE0U | (midi_test_channel - 1));
        constexpr auto messages = std::array {
            std::array<unsigned char, midi_message_byte_count> {
                cc_status,
                port_time_controller,
                port_time_value},
            std::array<unsigned char, midi_message_byte_count> {
                bend_status,
                bend_low_byte,
                bend_high_byte},
            std::array<unsigned char, midi_message_byte_count> {
                cc_status,
                delay_mix_controller,
                delay_mix_value},
            std::array<unsigned char, midi_message_byte_count> {
                cc_status,
                voice_controller,
                voice_value}};
        for (const auto& message : messages) {
            midi.addEvent(message.data(), static_cast<int>(message.size()), 0);
        }
        if (!require(
                midi.getNumEvents() == midi_sync_event_count,
                "MIDI sync fixture contains four events")) {
            processor.removeListener(&probe);
            return false;
        }
        processor.processBlock(audio, midi);

        const auto read = [&processor](const char* const parameter_id) -> float {
            const auto* const parameter = processor.parameter_state().getParameter(parameter_id);
            return parameter == nullptr ? -1.0F : parameter->getValue();
        };
        const auto seven_bit = [](const int value) -> float {
            return static_cast<float>(value) / seven_bit_maximum;
        };
        if (!require(
                std::abs(
                    read(DelayLamaAudioProcessor::port_time_parameter_id)
                    - seven_bit(static_cast<int>(port_time_value)))
                    <= parameter_step_tolerance,
                "CC5 silently persists PortTime")
            || !require(
                std::abs(
                    read(DelayLamaAudioProcessor::delay_mix_parameter_id)
                    - seven_bit(static_cast<int>(delay_mix_value)))
                    <= parameter_step_tolerance,
                "CC12 silently persists Delay")
            || !require(
                std::abs(
                    read(DelayLamaAudioProcessor::voice_parameter_id)
                    - seven_bit(static_cast<int>(voice_value)))
                    <= parameter_step_tolerance,
                "CC13 silently persists HeadSize")
            || !require(
                std::abs(read(DelayLamaAudioProcessor::vowel_parameter_id) - normalized_midpoint)
                    <= parameter_step_tolerance,
                "E0 does not publish its target before the first 10 ms tick")) {
            processor.removeListener(&probe);
            return false;
        }

        for (auto index = 0U; index < first_bend_tick_blocks; ++index) {
            processor.processBlock(audio, midi);
        }
        const auto stepped_vowel = read(DelayLamaAudioProcessor::vowel_parameter_id);
        processor.removeListener(&probe);
        return require(
                   stepped_vowel < normalized_midpoint && stepped_vowel > bend_target,
                   "E0 publishes the first stepped Vowel value after the 10 ms tick")
               && require(
                   probe.parameter_notifications == 0,
                   "silent MIDI persistence does not notify the host");
    }

    auto test_editor_interaction_boundaries() -> bool {
        const auto juce_initialiser = juce::ScopedJuceInitialiser_GUI {};
        const auto processor = std::make_unique<DelayLamaAudioProcessor>();
        const auto editor = std::unique_ptr<juce::AudioProcessorEditor> {processor->createEditor()};
        if (!require(editor != nullptr, "processor creates an editor")) {
            return false;
        }
        editor->setSize(editor_width, editor_height);
        editor->setVisible(true);

        auto* const delay_component = find_component(*editor, "delaylama.delay");
        auto* const delay_slider = dynamic_cast<juce::Slider*>(delay_component);
        if (!require(delay_slider != nullptr, "delay rail has a slider component")) {
            return false;
        }
        const auto expected_delay_bounds =
            juce::Rectangle<int> {delay_left, delay_top, delay_width, delay_height};
        if (!require(
                delay_slider->getBounds() == expected_delay_bounds,
                "delay rail uses the native bounds")) {
            return false;
        }
        const auto hit_position = expected_delay_bounds.getCentre();
        if (!require(
                editor->getComponentAt(hit_position) == delay_slider
                    && delay_slider->getComponentID() == "delaylama.delay",
                "delay rail hit resolves to delaylama.delay instead of XYPad")) {
            return false;
        }

        const auto* const delay_parameter = processor->parameter_state().getRawParameterValue(
            DelayLamaAudioProcessor::delay_mix_parameter_id);
        if (!require(delay_parameter != nullptr, "delay parameter is exposed by APVTS")) {
            return false;
        }
        const auto value_before = delay_parameter->load(std::memory_order_relaxed);
        delay_slider->setValue(normalized_drag_start, juce::dontSendNotification);
        const auto low_marker = render_component(*delay_slider);
        delay_slider->setValue(normalized_marker_end, juce::dontSendNotification);
        const auto high_marker = render_component(*delay_slider);
        delay_slider->setValue(value_before, juce::dontSendNotification);
        if (!require(
                images_differ(low_marker, high_marker),
                "delay marker follows the delay parameter instead of remaining pinned")) {
            return false;
        }
        const auto slider_y = static_cast<float>(delay_slider->getHeight()) * normalized_midpoint;
        const auto drag_start = juce::Point<float> {
            static_cast<float>(delay_slider->getWidth()) * normalized_drag_start,
            slider_y};
        const auto drag_end = juce::Point<float> {
            static_cast<float>(delay_slider->getWidth()) * normalized_drag_end,
            slider_y};
        delay_slider->mouseDown(make_mouse_event(*delay_slider, drag_start, drag_start, false));
        delay_slider->mouseDrag(make_mouse_event(*delay_slider, drag_end, drag_start, true));
        delay_slider->mouseUp(make_mouse_event(*delay_slider, drag_end, drag_start, true));
        const auto value_after = delay_parameter->load(std::memory_order_relaxed);
        if (!require(
                std::abs(value_after - value_before) > parameter_tolerance,
                "delay rail drag changes delay_mix")) {
            return false;
        }
        if (!require(
                !processor->visual_state_snapshot().gate,
                "delay rail drag does not route through XYPad")) {
            return false;
        }

        auto* const help_component = find_component(*editor, "delaylama.help");
        auto* const help_button = dynamic_cast<juce::Button*>(help_component);
        if (!require(help_button != nullptr, "help target has a button component")) {
            return false;
        }
        if (!require(
                !help_button->getWantsKeyboardFocus()
                    && !help_button->getMouseClickGrabsKeyboardFocus(),
                "help target does not take keyboard focus")) {
            return false;
        }
        if (!require(
                image_is_transparent(render_component(*help_button)),
                "question-mark hit target draws no native button chrome")) {
            return false;
        }
        trigger_button(*help_button);
        auto* const help_overlay = find_visible_help_window(*editor);
        if (!require(help_overlay != nullptr, "help trigger creates a visible overlay")) {
            return false;
        }
        auto* const artwork_close_component =
            find_component(*help_overlay, "delaylama.help.close-artwork");
        auto* const artwork_close_button = dynamic_cast<juce::Button*>(artwork_close_component);
        const auto expected_close_bounds =
            juce::Rectangle<int> {0, 0, help_artwork_width, help_artwork_height};
        if (!require(
                artwork_close_button != nullptr,
                "help artwork close text has a native hit target")) {
            return false;
        }
        if (!require(
                artwork_close_button->getBounds() == expected_close_bounds,
                "help artwork close target covers the panel")) {
            return false;
        }
        if (!require(
                image_is_transparent(render_component(*artwork_close_button)),
                "help artwork close target draws no native button chrome")) {
            return false;
        }
        trigger_button(*artwork_close_button);
        if (!require(
                !help_overlay->isVisible(),
                "help artwork close text hides the same overlay as the title button")) {
            return false;
        }
        trigger_button(*help_button);
        auto* const reopened_help_overlay = find_visible_help_window(*editor);
        return require(
            reopened_help_overlay == help_overlay
                && reopened_help_overlay->getParentComponent() == editor.get()
                && reopened_help_overlay->getBoundsInParent().intersects(editor->getLocalBounds()),
            "help overlay reopens in the editor and overlaps its bounds");
    }

}

auto main() noexcept -> int {
    if (!require(
            generated_idle_animation_contract(),
            "generated editor consumes processor-owned atlas state")) {
        return 1;
    }
    if (!require(
            midi_parameter_sync_contract(),
            "MIDI-mapped parameters persist without host notification")) {
        return 1;
    }
    return test_editor_interaction_boundaries() ? 0 : 1;
}
