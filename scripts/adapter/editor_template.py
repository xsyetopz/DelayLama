import json
from textwrap import dedent


def editor_source(product_name: str) -> str:
    content = dedent(
        r"""
        #include "juce_adapter/editor.hpp"

        #include <cmath>
        #include <cstddef>
        #include <span>

        #include "editor/resources/assets.hpp"

        namespace {
            constexpr auto editor_default_width = 360;
            constexpr auto editor_default_height = 510;
            constexpr auto editor_min_width = 360;
            constexpr auto editor_min_height = 510;
            constexpr auto editor_max_width = 1440;
            constexpr auto editor_max_height = 2040;
            constexpr auto source_width = 360.0F;
            constexpr auto source_height = 510.0F;
            constexpr auto source_scene_height = 311.0F;
            constexpr auto source_panel_top = 290.0F;
            constexpr auto source_panel_height = 220.0F;
            constexpr auto source_frame_width = 314.0F;
            constexpr auto source_frame_height = 311.0F;
            constexpr auto source_frame_left = 22.0F;
            constexpr auto source_frame_top = 5.0F;
            constexpr auto source_pad_left = 96.0F;
            constexpr auto source_pad_top = 362.0F;
            constexpr auto source_pad_width = 166.0F;
            constexpr auto source_pad_height = 84.0F;
            constexpr auto source_glide_left = 21.0F;
            constexpr auto source_glide_top = 448.0F;
            constexpr auto source_voice_left = 293.0F;
            constexpr auto source_voice_top = 447.0F;
            constexpr auto source_knob_extent = 50.0F;
            constexpr auto source_delay_left = 104.0F;
            constexpr auto source_delay_top = 479.0F;
            constexpr auto source_delay_width = 152.0F;
            constexpr auto source_delay_height = 25.0F;
            constexpr auto source_help_left = 284.0F;
            constexpr auto source_help_top = 300.0F;
            constexpr auto source_help_width = 43.0F;
            constexpr auto source_help_height = 35.0F;
            constexpr auto half_extent_ratio = 0.5F;
            constexpr auto normalized_minimum = 0.0F;
            constexpr auto normalized_maximum = 1.0F;
            constexpr auto normalized_step = 0.001F;
            constexpr auto visual_state_poll_hz = 30;
            constexpr auto visual_state_change_tolerance = 1.0E-6F;
            constexpr auto rotary_frame_count = 60;
            constexpr auto arrow_width = 20;
            constexpr auto arrow_height = 17;
            constexpr auto slider_center_divisor = 2;
            constexpr auto knob_slider_text_box_width = 1;
            constexpr auto knob_slider_text_box_height = 1;
            constexpr auto slider_component_prefix = "delaylama.";

            auto surface_bounds(const juce::Rectangle<int> bounds) -> juce::Rectangle<float> {
                const auto available = bounds.toFloat();
                const auto scale = juce::jmin(
                    available.getWidth() / source_width,
                    available.getHeight() / source_height);
                const auto width = source_width * scale;
                const auto height = source_height * scale;
                return {
                    available.getCentreX() - (width * half_extent_ratio),
                    available.getCentreY() - (height * half_extent_ratio),
                    width,
                    height};
            }

            auto component_bounds(
                const juce::Rectangle<float>& surface,
                const float left,
                const float top,
                const float width,
                const float height) -> juce::Rectangle<int> {
                const auto scale = surface.getWidth() / source_width;
                return juce::Rectangle<int> {
                    juce::roundToInt(surface.getX() + (left * scale)),
                    juce::roundToInt(surface.getY() + (top * scale)),
                    juce::jmax(1, juce::roundToInt(width * scale)),
                    juce::jmax(1, juce::roundToInt(height * scale))};
            }

            auto image_from_bytes(const std::span<const unsigned char> bytes) -> juce::Image {
                return juce::ImageCache::getFromMemory(bytes.data(), bytes.size());
            }

            auto keyed_image_from_bytes(const std::span<const unsigned char> bytes) -> juce::Image {
                const auto source = image_from_bytes(bytes);
                if (!source.isValid()) {
                    return {};
                }
                auto keyed = juce::Image(
                    juce::Image::ARGB,
                    source.getWidth(),
                    source.getHeight(),
                    true);
                const auto graphics = juce::Graphics {keyed};
                graphics.drawImageAt(source, 0, 0);
                for (auto y = 0; y < keyed.getHeight(); ++y) {
                    for (auto x = 0; x < keyed.getWidth(); ++x) {
                        const auto pixel = keyed.getPixelAt(x, y);
                        if (pixel == juce::Colours::white) {
                            keyed.setPixelAt(x, y, juce::Colours::transparentBlack);
                        }
                    }
                }
                return keyed;
            }
        }

        DelayLamaAudioProcessorEditor::DelayLamaAudioProcessorEditor(
            DelayLamaAudioProcessor& owner)
            : AudioProcessorEditor(&owner)
            , processor_(owner)
            , xy_pad_(editor_model_)
            , look_and_feel_() {
            setOpaque(true);
            setResizable(true, true);
            setResizeLimits(
                editor_min_width, editor_min_height, editor_max_width, editor_max_height);

            xy_pad_.on_gesture = [this](const float position_x, const float position_y, const XYPad::Gesture gesture) {
                handle_pad_gesture(position_x, position_y, gesture);
            };
            addAndMakeVisible(xy_pad_);

            configure_rotary(
                glide_slider_,
                "glide",
                "Glide",
                "Portamento time for MIDI keyboard notes.");
            configure_delay_slider();
            configure_rotary(
                voice_slider_,
                "voice",
                "Voice",
                "Voice character from baritone through soprano.");

            glide_attachment_ = std::make_unique<
                juce::AudioProcessorValueTreeState::SliderAttachment>(
                processor_.parameter_state(),
                DelayLamaAudioProcessor::port_time_parameter_id,
                glide_slider_);
            delay_attachment_ = std::make_unique<
                juce::AudioProcessorValueTreeState::SliderAttachment>(
                processor_.parameter_state(),
                DelayLamaAudioProcessor::delay_mix_parameter_id,
                delay_slider_);
            voice_attachment_ = std::make_unique<
                juce::AudioProcessorValueTreeState::SliderAttachment>(
                processor_.parameter_state(),
                DelayLamaAudioProcessor::voice_parameter_id,
                voice_slider_);

            help_button_.setDescription("Open the Delay Lama help panel.");
            help_button_.setComponentID("delaylama.help");
            help_button_.onClick = [this]() { show_help(); };
            addAndMakeVisible(help_button_);
            setSize(editor_default_width, editor_default_height);
            last_visual_state_ = processor_.visual_state_snapshot();
            editor_model_.apply_external_state(last_visual_state_);
            startTimerHz(visual_state_poll_hz);
        }

        DelayLamaAudioProcessorEditor::~DelayLamaAudioProcessorEditor() {
            stopTimer();
            glide_slider_.setLookAndFeel(nullptr);
            delay_slider_.setLookAndFeel(nullptr);
            voice_slider_.setLookAndFeel(nullptr);
        }

        DelayLamaAudioProcessorEditor::ImageHitTarget::ImageHitTarget(
            const juce::String& action_name)
            : Button(action_name) {
            setTitle(action_name);
            setAccessible(true);
            setWantsKeyboardFocus(false);
            setMouseClickGrabsKeyboardFocus(false);
        }

        auto DelayLamaAudioProcessorEditor::ImageHitTarget::paintButton(
            [[maybe_unused]] juce::Graphics& graphics,
            [[maybe_unused]] const bool is_mouse_over,
            [[maybe_unused]] const bool is_button_down) -> void {}

        DelayLamaAudioProcessorEditor::ArtworkLookAndFeel::ArtworkLookAndFeel()
            : knob_strip_a_(image_from_bytes(delay_lama_assets::knob_strip_a_png))
            , knob_strip_b_(image_from_bytes(delay_lama_assets::knob_strip_b_png))
            , arrow_(keyed_image_from_bytes(delay_lama_assets::ui_arrow_png)) {}

        auto DelayLamaAudioProcessorEditor::ArtworkLookAndFeel::frame_image(
            const juce::Image& strip,
            const float position) const -> juce::Image {
            if (!strip.isValid()) {
                return {};
            }
            const auto frame = juce::jlimit(
                0,
                rotary_frame_count - 1,
                juce::roundToInt(position * static_cast<float>(rotary_frame_count - 1)));
            return strip.getClippedImage(
                juce::Rectangle<int> {0, frame * strip.getWidth(), strip.getWidth(), strip.getWidth()});
        }

        auto DelayLamaAudioProcessorEditor::ArtworkLookAndFeel::drawRotarySlider(
            juce::Graphics& graphics,
            const int x,
            const int y,
            const int width,
            const int height,
            const float slider_position,
            [[maybe_unused]] const float rotary_start_angle,
            [[maybe_unused]] const float rotary_end_angle,
            juce::Slider& slider) -> void {
            const auto& strip = slider.getComponentID() == "delaylama.voice"
                ? knob_strip_b_
                : knob_strip_a_;
            const auto frame = frame_image(strip, slider_position);
            if (frame.isValid()) {
                graphics.drawImageWithin(
                    frame,
                    x,
                    y,
                    width,
                    height,
                    juce::RectanglePlacement::stretchToFit,
                    false);
            }
        }

        auto DelayLamaAudioProcessorEditor::ArtworkLookAndFeel::drawLinearSlider(
            juce::Graphics& graphics,
            const int x,
            const int y,
            const int width,
            const int height,
            const float slider_position,
            [[maybe_unused]] const float min_slider_position,
            [[maybe_unused]] const float max_slider_position,
            [[maybe_unused]] const juce::Slider::SliderStyle style,
            [[maybe_unused]] juce::Slider& slider) -> void {
            if (!arrow_.isValid()) {
                return;
            }
            const auto marker_x = juce::jlimit(
                static_cast<float>(x),
                static_cast<float>(x + width - arrow_width),
                slider_position - (static_cast<float>(arrow_width) * half_extent_ratio));
            graphics.drawImageWithin(
                arrow_,
                juce::roundToInt(marker_x),
                y + juce::jmax(0, (height - arrow_height) / slider_center_divisor),
                arrow_width,
                arrow_height,
                juce::RectanglePlacement::stretchToFit,
                false);
        }

        auto DelayLamaAudioProcessorEditor::configure_rotary(
            juce::Slider& slider,
            const juce::String& component_id,
            const juce::String& title,
            const juce::String& description) -> void {
            slider.setSliderStyle(juce::Slider::RotaryHorizontalVerticalDrag);
            slider.setTextBoxStyle(
                juce::Slider::NoTextBox,
                false,
                knob_slider_text_box_width,
                knob_slider_text_box_height);
            slider.setRange(normalized_minimum, normalized_maximum, normalized_step);
            slider.setComponentID(slider_component_prefix + component_id);
            slider.setTitle(title);
            slider.setDescription(description);
            slider.setAccessible(true);
            slider.setTooltip(description);
            slider.setLookAndFeel(&look_and_feel_);
            addAndMakeVisible(slider);
        }

        auto DelayLamaAudioProcessorEditor::configure_delay_slider() -> void {
            delay_slider_.setSliderStyle(juce::Slider::LinearHorizontal);
            delay_slider_.setTextBoxStyle(
                juce::Slider::NoTextBox,
                false,
                knob_slider_text_box_width,
                knob_slider_text_box_height);
            delay_slider_.setRange(normalized_minimum, normalized_maximum, normalized_step);
            delay_slider_.setComponentID("delaylama.delay");
            delay_slider_.setTitle("Delay");
            delay_slider_.setDescription("Delayed signal level from anechoic to echoing.");
            delay_slider_.setAccessible(true);
            delay_slider_.setInterceptsMouseClicks(true, false);
            delay_slider_.setSliderSnapsToMousePosition(true);
            delay_slider_.setTooltip("Delayed signal level from anechoic to echoing.");
            delay_slider_.setLookAndFeel(&look_and_feel_);
            addAndMakeVisible(delay_slider_);
        }

        auto DelayLamaAudioProcessorEditor::paint(juce::Graphics& graphics) -> void {
            graphics.fillAll(juce::Colours::black);
        }

        auto DelayLamaAudioProcessorEditor::resized() -> void {
            xy_pad_.setBounds(getLocalBounds());
            const auto surface = surface_bounds(getLocalBounds());
            glide_slider_.setBounds(component_bounds(
                surface,
                source_glide_left,
                source_glide_top,
                source_knob_extent,
                source_knob_extent));
            voice_slider_.setBounds(component_bounds(
                surface,
                source_voice_left,
                source_voice_top,
                source_knob_extent,
                source_knob_extent));
            delay_slider_.setBounds(component_bounds(
                surface,
                source_delay_left,
                source_delay_top,
                source_delay_width,
                source_delay_height));
            help_button_.setBounds(component_bounds(
                surface,
                source_help_left,
                source_help_top,
                source_help_width,
                source_help_height));
            if (help_window_ != nullptr && help_window_->isVisible()) {
                help_window_->setBounds(getLocalBounds().withSizeKeepingCentre(
                    help_window_->getWidth(),
                    help_window_->getHeight()));
            }
        }

        auto DelayLamaAudioProcessorEditor::handle_pad_gesture(
            const float position_x,
            const float position_y,
            const XYPad::Gesture gesture) -> void {
            const auto result = editor_model_.handle_gesture(position_x, position_y, gesture);
            if (gesture == XYPad::Gesture::Down) {
                processor_.enqueue_pad_note(result.note_on_note, true);
            }
            if (gesture != XYPad::Gesture::Up) {
                processor_.enqueue_pad_controls(result.position_x, result.position_y);
            }
            if (gesture == XYPad::Gesture::Up) {
                processor_.enqueue_pad_note(result.note_off_note, false);
            }
            xy_pad_.advance_animation(true, gesture == XYPad::Gesture::Up);
            xy_pad_.repaint();
        }

        auto DelayLamaAudioProcessorEditor::timerCallback() -> void {
            const auto visual_state = processor_.visual_state_snapshot();
            const auto changed = visual_state.note != last_visual_state_.note
                || visual_state.gate != last_visual_state_.gate
                || std::abs(visual_state.vowel - last_visual_state_.vowel)
                    > visual_state_change_tolerance
                || std::abs(visual_state.atlas_selector - last_visual_state_.atlas_selector)
                    > visual_state_change_tolerance;
            if (changed) {
                last_visual_state_ = visual_state;
                editor_model_.apply_external_state(visual_state);
            }
            xy_pad_.advance_animation(changed, !visual_state.gate);
            xy_pad_.repaint();
        }

        auto DelayLamaAudioProcessorEditor::show_help() -> void {
            if (help_window_ == nullptr) {
                const auto artwork = image_from_bytes(delay_lama_assets::help_panel_png);
                if (artwork.isValid()) {
                    help_window_ = std::make_unique<HelpWindow>(artwork);
                    addAndMakeVisible(help_window_.get());
                }
            }
            if (help_window_ != nullptr) {
                help_window_->setBounds(getLocalBounds().withSizeKeepingCentre(
                    help_window_->getWidth(),
                    help_window_->getHeight()));
                help_window_->setVisible(true);
                help_window_->toFront(false);
            }
        }

        DelayLamaAudioProcessorEditor::HelpWindow::HelpWindow(const juce::Image artwork)
            : DocumentWindow(
                  @@PRODUCT_NAME@@ " Help",
                  juce::Colours::black,
                  juce::DocumentWindow::closeButton,
                  false)
            , artwork_component_("Delay Lama help artwork") {
            artwork_component_.setImage(artwork, juce::RectanglePlacement::centred);
            artwork_component_.setAccessible(true);
            artwork_component_.setTitle("Delay Lama help artwork");
            artwork_component_.setDescription("Delay Lama quick-help panel.");
            artwork_close_button_.setDescription(
                "Close the Delay Lama help panel.");
            artwork_close_button_.setComponentID("delaylama.help.close-artwork");
            artwork_close_button_.setBounds(artwork.getBounds());
            artwork_close_button_.onClick = [this]() { closeButtonPressed(); };
            artwork_component_.addAndMakeVisible(artwork_close_button_);
            setContentNonOwned(&artwork_component_, true);
            setResizable(false, false);
            setUsingNativeTitleBar(false);
            setSize(artwork.getWidth(), artwork.getHeight() + getTitleBarHeight());
            setVisible(false);
        }

        DelayLamaAudioProcessorEditor::HelpWindow::~HelpWindow() {
            clearContentComponent();
        }

        auto DelayLamaAudioProcessorEditor::HelpWindow::closeButtonPressed() -> void {
            setVisible(false);
        }

        DelayLamaAudioProcessorEditor::XYPad::XYPad(delaylama::host::EditorModel& model)
            : model_(model)
            , scene_background_(image_from_bytes(delay_lama_assets::scene_background_png))
            , control_panel_(image_from_bytes(delay_lama_assets::control_panel_png))
            , arrow_(keyed_image_from_bytes(delay_lama_assets::ui_arrow_png)) {
            const auto monk_sheet = image_from_bytes(delay_lama_assets::monk_sprite_sheet_png);
            jassert(scene_background_.isValid());
            jassert(control_panel_.isValid());
            jassert(monk_sheet.isValid());
            jassert(arrow_.isValid());

            if (monk_sheet.isValid()) {
                constexpr auto atlas_columns = 5;
                constexpr auto atlas_rows = 6;
                const auto frame_width = monk_sheet.getWidth() / atlas_columns;
                const auto frame_height = monk_sheet.getHeight() / atlas_rows;
                for (auto column = 0; column < atlas_columns; ++column) {
                    for (auto row = 0; row < atlas_rows; ++row) {
                        const auto index = (column * atlas_rows) + row;
                        monk_frames_.at(static_cast<std::size_t>(index)) = monk_sheet.getClippedImage(
                            juce::Rectangle<int> {
                                column * frame_width,
                                row * frame_height,
                                frame_width,
                                frame_height});
                    }
                }
            }
        }

        auto DelayLamaAudioProcessorEditor::XYPad::paint(juce::Graphics& graphics) -> void {
            const auto area = artwork_bounds();
            graphics.fillAll(juce::Colours::black);
            graphics.setImageResamplingQuality(juce::Graphics::highResamplingQuality);

            if (scene_background_.isValid()) {
                const auto scale = area.getWidth() / source_width;
                graphics.drawImageWithin(
                    scene_background_,
                    juce::roundToInt(area.getX()),
                    juce::roundToInt(area.getY()),
                    juce::roundToInt(area.getWidth()),
                    juce::roundToInt(area.getHeight() * (source_scene_height / source_height)),
                    juce::RectanglePlacement::stretchToFit,
                    false);
            }
            if (monk_frames_.front().isValid()) {
                const auto scale = area.getWidth() / source_width;
                graphics.drawImageWithin(
                    monk_frames_.at(animation_phase_),
                    juce::roundToInt(area.getX() + (source_frame_left * scale)),
                    juce::roundToInt(area.getY() + (source_frame_top * scale)),
                    juce::roundToInt(source_frame_width * scale),
                    juce::roundToInt(source_frame_height * scale),
                    juce::RectanglePlacement::stretchToFit,
                    false);
            }
            if (control_panel_.isValid()) {
                const auto scale = area.getWidth() / source_width;
                graphics.drawImageWithin(
                    control_panel_,
                    juce::roundToInt(area.getX()),
                    juce::roundToInt(area.getY() + (source_panel_top * scale)),
                    juce::roundToInt(area.getWidth()),
                    juce::roundToInt(source_panel_height * scale),
                    juce::RectanglePlacement::stretchToFit,
                    false);
            }

            if (arrow_.isValid()) {
                const auto scale = area.getWidth() / source_width;
                const auto pad = pad_bounds();
                const auto marker = juce::Point<float> {
                    pad.getX() + (marker_x_ * pad.getWidth()),
                    pad.getY() + (marker_y_ * pad.getHeight())};
                graphics.drawImageWithin(
                    arrow_,
                    juce::roundToInt(marker.x - ((arrow_width * scale) * half_extent_ratio)),
                    juce::roundToInt(pad.getY() - (arrow_height * scale)),
                    juce::roundToInt(arrow_width * scale),
                    juce::roundToInt(arrow_height * scale),
                    juce::RectanglePlacement::stretchToFit,
                    false);
                graphics.drawImageTransformed(
                    arrow_,
                    juce::AffineTransform::rotation(-juce::MathConstants<float>::halfPi)
                        .scaled(scale)
                        .translated(
                            pad.getX() - (arrow_height * scale),
                            marker.y + ((arrow_width * scale) * half_extent_ratio)),
                    false);
            }
        }

        auto DelayLamaAudioProcessorEditor::XYPad::advance_animation(
            [[maybe_unused]] const bool state_changed,
            [[maybe_unused]] const bool idle) -> void {
            // Present published atlas state so UI timing cannot drift from audio timing.
            animation_phase_ = model_.animation_frame();
        }

        auto DelayLamaAudioProcessorEditor::XYPad::send_gesture(
            const juce::MouseEvent& event,
            const Gesture gesture) -> void {
            const auto pad = pad_bounds();
            if (gesture != Gesture::Up && !pad.contains(event.position)) {
                return;
            }
            const auto position_x = juce::jlimit(
                normalized_minimum,
                normalized_maximum,
                (event.position.x - pad.getX()) / juce::jmax(1.0F, pad.getWidth()));
            const auto position_y = juce::jlimit(
                normalized_minimum,
                normalized_maximum,
                (event.position.y - pad.getY()) / juce::jmax(1.0F, pad.getHeight()));
            marker_x_ = position_x;
            marker_y_ = position_y;
            if (on_gesture != nullptr) {
                on_gesture(position_x, position_y, gesture);
            }
            repaint();
        }

        auto DelayLamaAudioProcessorEditor::XYPad::mouseDown(const juce::MouseEvent& event)
            -> void {
            send_gesture(event, Gesture::Down);
        }

        auto DelayLamaAudioProcessorEditor::XYPad::mouseDrag(const juce::MouseEvent& event)
            -> void {
            send_gesture(event, Gesture::Drag);
        }

        auto DelayLamaAudioProcessorEditor::XYPad::mouseUp(const juce::MouseEvent& event) -> void {
            send_gesture(event, Gesture::Up);
        }

        auto DelayLamaAudioProcessorEditor::XYPad::artwork_bounds() const
            -> juce::Rectangle<float> {
            return surface_bounds(getLocalBounds());
        }

        auto DelayLamaAudioProcessorEditor::XYPad::pad_bounds() const
            -> juce::Rectangle<float> {
            const auto area = artwork_bounds();
            const auto scale = area.getWidth() / source_width;
            return {
                area.getX() + (source_pad_left * scale),
                area.getY() + (source_pad_top * scale),
                source_pad_width * scale,
                source_pad_height * scale};
        }
        """
    ).lstrip()
    product_literal = json.dumps(product_name)
    return content.replace("@@PRODUCT_NAME@@", product_literal)
