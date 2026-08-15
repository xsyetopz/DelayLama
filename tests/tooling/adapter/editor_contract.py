import re
import struct
import unittest
import zlib
from pathlib import Path

from scripts.adapter.editor_surface import editor_header
from scripts.adapter.editor_template import editor_source

PNG_SIGNATURE = b"\x89PNG\r\n\x1a\n"
PNG_HEADER_SIZE = 13
PNG_FILTER_NONE = 0
PNG_FILTER_SUB = 1
PNG_FILTER_UP = 2
PNG_FILTER_AVERAGE = 3
PNG_FILTER_PAETH = 4
RGB_CHANNELS = 3
BYTE_LIMIT = 256
PNG_CHUNK_TYPE_START = 4
PNG_CHUNK_TYPE_END = 8
PNG_CHUNK_DATA_START = 8
PNG_CHUNK_CRC_BYTES = 12
PNG_BIT_DEPTH = 8
PNG_COLOUR_TYPE_RGB = 2
PNG_NON_INTERLACED = 0
PNG_AVERAGE_DIVISOR = 2
REPOSITORY_ROOT_PARENT_DEPTH = 3
SOURCE_PANEL_TOP = 290
SOURCE_WIDTH = 360
SOURCE_HEIGHT = 510
SCENE_HEIGHT = 311
CONTROL_PANEL_HEIGHT = 220


def _paeth_predictor(left: int, above: int, upper_left: int) -> int:
    estimate = left + above - upper_left
    left_distance = abs(estimate - left)
    above_distance = abs(estimate - above)
    upper_left_distance = abs(estimate - upper_left)
    if left_distance <= above_distance and left_distance <= upper_left_distance:
        return left
    if above_distance <= upper_left_distance:
        return above
    return upper_left


def _decode_rgb_png(path: Path) -> list[bytes]:
    payload = path.read_bytes()
    if payload[: len(PNG_SIGNATURE)] != PNG_SIGNATURE:
        raise ValueError(f"not a PNG: {path}")
    offset = len(PNG_SIGNATURE)
    width = 0
    height = 0
    compressed = bytearray()
    while offset < len(payload):
        length = struct.unpack_from(">I", payload, offset)[0]
        chunk_type = payload[
            offset + PNG_CHUNK_TYPE_START : offset + PNG_CHUNK_TYPE_END
        ]
        chunk = payload[
            offset + PNG_CHUNK_DATA_START : offset + PNG_CHUNK_DATA_START + length
        ]
        offset += PNG_CHUNK_CRC_BYTES + length
        if chunk_type == b"IHDR":
            width, height, bit_depth, colour_type, _, _, interlace = struct.unpack(
                ">IIBBBBB", chunk[:PNG_HEADER_SIZE]
            )
            if (bit_depth, colour_type, interlace) != (
                PNG_BIT_DEPTH,
                PNG_COLOUR_TYPE_RGB,
                PNG_NON_INTERLACED,
            ):
                raise ValueError(f"unsupported PNG format: {path}")
        elif chunk_type == b"IDAT":
            compressed.extend(chunk)
        elif chunk_type == b"IEND":
            break

    decoded = zlib.decompress(compressed)
    stride = width * RGB_CHANNELS
    rows: list[bytes] = []
    previous = bytes(stride)
    cursor = 0
    for _ in range(height):
        filter_type = decoded[cursor]
        cursor += 1
        encoded = decoded[cursor : cursor + stride]
        cursor += stride
        row = bytearray(stride)
        for index, value in enumerate(encoded):
            left = row[index - RGB_CHANNELS] if index >= RGB_CHANNELS else 0
            above = previous[index]
            upper_left = previous[index - RGB_CHANNELS] if index >= RGB_CHANNELS else 0
            if filter_type == PNG_FILTER_NONE:
                prediction = 0
            elif filter_type == PNG_FILTER_SUB:
                prediction = left
            elif filter_type == PNG_FILTER_UP:
                prediction = above
            elif filter_type == PNG_FILTER_AVERAGE:
                prediction = (left + above) // PNG_AVERAGE_DIVISOR
            elif filter_type == PNG_FILTER_PAETH:
                prediction = _paeth_predictor(left, above, upper_left)
            else:
                raise ValueError(f"unsupported PNG filter: {filter_type}")
            row[index] = (value + prediction) % BYTE_LIMIT
        rows.append(bytes(row))
        previous = bytes(row)
    return rows


class EditorSurfaceContractTests(unittest.TestCase):
    def test_native_geometry_is_explicit(self) -> None:
        source = editor_source("Delay Lama")
        expected = {
            "source_frame_left": "22.0F",
            "source_frame_top": "5.0F",
            "source_frame_width": "314.0F",
            "source_frame_height": "311.0F",
            "source_glide_left": "21.0F",
            "source_glide_top": "448.0F",
            "source_voice_left": "293.0F",
            "source_voice_top": "447.0F",
            "source_knob_extent": "50.0F",
            "source_delay_left": "104.0F",
            "source_delay_top": "479.0F",
            "source_delay_width": "152.0F",
            "source_delay_height": "25.0F",
            "source_help_left": "284.0F",
            "source_help_top": "300.0F",
            "source_help_width": "43.0F",
            "source_help_height": "35.0F",
        }
        for name, value in expected.items():
            self.assertRegex(source, rf"constexpr auto {name} = {re.escape(value)};")

        self.assertIn("constexpr auto source_width = 360.0F;", source)
        self.assertIn("constexpr auto source_height = 510.0F;", source)
        self.assertIn("const auto available = bounds.toFloat();", source)
        self.assertNotIn("source_surface_inset", source)

    def test_help_window_is_an_editor_owned_overlay(self) -> None:
        source = editor_source("Delay Lama")
        self.assertIn("setContentNonOwned(&artwork_component_, true);", source)
        self.assertIn("HelpWindow::~HelpWindow()", source)
        self.assertIn("clearContentComponent();", source)
        self.assertNotIn("setContentOwned(&artwork_component_", source)
        self.assertIn("juce::DocumentWindow::closeButton,\n          false)", source)
        self.assertIn("addAndMakeVisible(help_window_.get());", source)
        self.assertIn(
            'artwork_close_button_.setComponentID("delaylama.help.close-artwork");',
            source,
        )
        self.assertIn(
            "artwork_close_button_.onClick = [this]() { closeButtonPressed(); };",
            source,
        )
        self.assertIn(
            "artwork_component_.addAndMakeVisible(artwork_close_button_);", source
        )
        self.assertIn("artwork_close_button_.setBounds(artwork.getBounds());", source)
        self.assertIn("getLocalBounds().withSizeKeepingCentre(", source)
        self.assertIn("setUsingNativeTitleBar(false);", source)
        self.assertIn(
            "setSize(artwork.getWidth(), artwork.getHeight() + getTitleBarHeight());",
            source,
        )
        self.assertNotIn("centreWithSize(", source)

    def test_renderer_uses_product_assets_and_processor_owned_animation(self) -> None:
        source = editor_source("Delay Lama")
        for asset_name in (
            "scene_background_png",
            "control_panel_png",
            "monk_sprite_sheet_png",
            "knob_strip_a_png",
            "knob_strip_b_png",
            "ui_arrow_png",
            "help_panel_png",
        ):
            self.assertIn(asset_name, source)
        self.assertIn("drawImageTransformed", source)
        self.assertIn("const auto index = (column * atlas_rows) + row;", source)
        self.assertIn("advance_animation", source)
        self.assertIn(
            "Present published atlas state so UI timing cannot drift from audio timing",
            source,
        )
        self.assertIn("animation_phase_ = model_.animation_frame();", source)
        self.assertNotIn("idle_animation_sequence_", source)
        self.assertNotIn("idle_animation_position_", source)
        self.assertNotIn("% animation_frame_capacity_", source)
        self.assertNotIn("std::rand", source)
        self.assertNotIn("std::mt19937", source)
        for vector_fallback in (
            "drawRect(",
            "drawRoundedRectangle(",
            "drawEllipse(",
            "drawText(",
        ):
            self.assertNotIn(vector_fallback, source)

    def test_processor_visual_state_bridge_is_message_thread_driven(self) -> None:
        header = editor_header()
        source = editor_source("Delay Lama")
        self.assertIn("private juce::Timer", header)
        self.assertIn("startTimerHz(visual_state_poll_hz);", source)
        self.assertIn("stopTimer();", source)
        self.assertIn("auto DelayLamaAudioProcessorEditor::timerCallback()", source)
        self.assertIn(
            "const auto visual_state = processor_.visual_state_snapshot();", source
        )
        self.assertIn("editor_model_.apply_external_state(visual_state);", source)
        self.assertIn(
            "visual_state.atlas_selector - last_visual_state_.atlas_selector", source
        )
        self.assertIn("xy_pad_.repaint();", source)
        self.assertIn("xy_pad_.advance_animation(changed, !visual_state.gate);", source)
        self.assertNotIn("if (!changed) {\n                return;", source)
        self.assertNotIn("ChangeBroadcaster", header + source)
        self.assertNotIn("sendChangeMessage", header + source)

    def test_delay_slider_intercepts_and_snaps_to_mouse(self) -> None:
        source = editor_source("Delay Lama")
        self.assertIn("delay_slider_.setInterceptsMouseClicks(true, false);", source)
        self.assertIn("delay_slider_.setSliderSnapsToMousePosition(true);", source)
        self.assertIn("delay_attachment_", source)
        self.assertIn(
            "slider_position - (static_cast<float>(arrow_width) * half_extent_ratio)",
            source,
        )
        self.assertNotIn(
            "slider_position * static_cast<float>(width - arrow_width)", source
        )
        self.assertLess(
            source.index("addAndMakeVisible(xy_pad_);"),
            source.index("addAndMakeVisible(delay_slider_);"),
        )

    def test_help_hit_target_is_pointer_only(self) -> None:
        source = editor_source("Delay Lama")
        header = editor_header()
        self.assertIn("class ImageHitTarget final : public juce::Button", header)
        self.assertIn('ImageHitTarget help_button_ {"Help"};', header)
        self.assertIn("setWantsKeyboardFocus(false);", source)
        self.assertIn("setMouseClickGrabsKeyboardFocus(false);", source)
        self.assertIn("setAccessible(true);", source)
        self.assertIn("ImageHitTarget::paintButton(", source)
        self.assertIn("help_button_.onClick = [this]() { show_help(); };", source)
        self.assertNotIn("help_button_.setColour", source)

    def test_pad_gestures_forward_midi_lifecycle(self) -> None:
        source = editor_source("Delay Lama")
        self.assertIn("processor_.enqueue_pad_note(result.note_on_note, true);", source)
        self.assertIn(
            "processor_.enqueue_pad_controls(result.position_x, result.position_y);",
            source,
        )
        self.assertIn(
            "    if (gesture != XYPad::Gesture::Up) {\n"
            "        processor_.enqueue_pad_controls(result.position_x, result.position_y);\n"
            "    }",
            source,
        )
        self.assertNotIn("set_vowel_from_pad", source)
        self.assertIn(
            "processor_.enqueue_pad_note(result.note_off_note, false);", source
        )
        self.assertNotIn("if (result.note_off)", source)
        self.assertNotIn("if (result.note_on)", source)

    def test_surface_slices_share_one_asset(self) -> None:
        root = Path(__file__).resolve().parents[REPOSITORY_ROOT_PARENT_DEPTH]
        source = _decode_rgb_png(root / "assets" / "source_surface.png")
        scene = _decode_rgb_png(root / "assets" / "scene_background.png")
        panel = _decode_rgb_png(root / "assets" / "control_panel.png")
        self.assertEqual(len(source), SOURCE_HEIGHT)
        self.assertEqual(len(source[0]), SOURCE_WIDTH * RGB_CHANNELS)
        self.assertEqual(len(scene), SCENE_HEIGHT)
        self.assertEqual(len(panel), CONTROL_PANEL_HEIGHT)
        self.assertEqual(scene, source[: len(scene)])
        self.assertEqual(
            panel, source[SOURCE_PANEL_TOP : SOURCE_PANEL_TOP + len(panel)]
        )


if __name__ == "__main__":
    unittest.main()
