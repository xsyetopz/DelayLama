from textwrap import dedent


def editor_header() -> str:
    return dedent(
        r"""
        #ifndef DELAYLAMA_JUCE_ADAPTER_EDITOR_HPP
        #define DELAYLAMA_JUCE_ADAPTER_EDITOR_HPP

        #include <juce_audio_processors/juce_audio_processors.h>
        #include <juce_events/juce_events.h>
        #include <juce_gui_basics/juce_gui_basics.h>

        #include <array>
        #include <cstddef>
        #include <functional>
        #include <memory>
        #include <span>

        #include "editor/interaction.hpp"
        #include "juce_adapter/processor.hpp"

        /// JUCE editor for the Delay Lama controls.
        class DelayLamaAudioProcessorEditor final : public juce::AudioProcessorEditor,
                                                    private juce::Timer {
        public:
            /// Creates an editor for its processor.
            explicit DelayLamaAudioProcessorEditor(DelayLamaAudioProcessor& owner);
            /// Destroys the editor and child components.
            ~DelayLamaAudioProcessorEditor() override;

            /// Paints the editor artwork.
            auto paint(juce::Graphics& graphics) -> void override;
            /// Lays out controls after resizing.
            auto resized() -> void override;

        private:
            class ArtworkLookAndFeel final : public juce::LookAndFeel_V4 {
            public:
                ArtworkLookAndFeel();
                auto drawRotarySlider(
                    juce::Graphics& graphics,
                    int x,
                    int y,
                    int width,
                    int height,
                    float slider_position,
                    float rotary_start_angle,
                    float rotary_end_angle,
                    juce::Slider& slider) -> void override;
                auto drawLinearSlider(
                    juce::Graphics& graphics,
                    int x,
                    int y,
                    int width,
                    int height,
                    float slider_position,
                    float min_slider_position,
                    float max_slider_position,
                    juce::Slider::SliderStyle style,
                    juce::Slider& slider) -> void override;

            private:
                auto frame_image(const juce::Image& strip, float position) const
                    -> juce::Image;

                juce::Image knob_strip_a_;
                juce::Image knob_strip_b_;
                juce::Image arrow_;
            };
            class ImageHitTarget final : public juce::Button {
            public:
                explicit ImageHitTarget(const juce::String& action_name);

            private:
                auto paintButton(
                    juce::Graphics& graphics,
                    bool is_mouse_over,
                    bool is_button_down) -> void override;
            };
            class HelpWindow final : public juce::DocumentWindow {
            public:
                explicit HelpWindow(juce::Image artwork);
                ~HelpWindow() override;
                auto closeButtonPressed() -> void override;

            private:
                juce::ImageComponent artwork_component_;
                ImageHitTarget artwork_close_button_ {"Close help"};
            };
            class XYPad final : public juce::Component {
            public:
                using Gesture = delaylama::host::PadGesture;
                explicit XYPad(delaylama::host::EditorModel& model);
                std::function<void(float, float, Gesture)> on_gesture;
                auto paint(juce::Graphics& graphics) -> void override;
                auto advance_animation(bool state_changed, bool idle) -> void;
                auto mouseDown(const juce::MouseEvent& event) -> void override;
                auto mouseDrag(const juce::MouseEvent& event) -> void override;
                auto mouseUp(const juce::MouseEvent& event) -> void override;

            private:
                static constexpr auto animation_frame_capacity_ = 30U;
                static constexpr auto release_atlas_frame_ = 5U;
                static constexpr auto default_pad_position_ = 0.5F;
                auto send_gesture(const juce::MouseEvent& event, Gesture gesture) -> void;
                auto artwork_bounds() const -> juce::Rectangle<float>;
                auto pad_bounds() const -> juce::Rectangle<float>;

                delaylama::host::EditorModel& model_;
                float marker_x_ = default_pad_position_;
                float marker_y_ = default_pad_position_;
                juce::Image scene_background_;
                juce::Image control_panel_;
                juce::Image arrow_;
                std::array<juce::Image, animation_frame_capacity_> monk_frames_;
                std::size_t animation_phase_ = release_atlas_frame_;
            };

            auto configure_rotary(
                juce::Slider& slider,
                const juce::String& component_id,
                const juce::String& title,
                const juce::String& description) -> void;
            auto configure_delay_slider() -> void;
            auto handle_pad_gesture(float, float, XYPad::Gesture) -> void;
            // Poll on JUCE's message thread because audio callbacks must not post to the OS queue.
            auto timerCallback() -> void override;
            auto show_help() -> void;

            delaylama::host::EditorModel editor_model_;
            DelayLamaAudioProcessor& processor_;
            XYPad xy_pad_;
            ArtworkLookAndFeel look_and_feel_;
            juce::Slider glide_slider_;
            juce::Slider delay_slider_;
            juce::Slider voice_slider_;
            ImageHitTarget help_button_ {"Help"};

            std::unique_ptr<juce::AudioProcessorValueTreeState::SliderAttachment>
                glide_attachment_;
            std::unique_ptr<juce::AudioProcessorValueTreeState::SliderAttachment>
                delay_attachment_;
            std::unique_ptr<juce::AudioProcessorValueTreeState::SliderAttachment>
                voice_attachment_;
            std::unique_ptr<HelpWindow> help_window_;
            delaylama::host::VisualState last_visual_state_ {};

        JUCE_DECLARE_NON_COPYABLE_WITH_LEAK_DETECTOR(DelayLamaAudioProcessorEditor)
        };

        #endif

        """
    ).lstrip()
