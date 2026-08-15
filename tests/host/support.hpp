#ifndef DELAYLAMA_TESTS_HOST_SUPPORT_HPP
#define DELAYLAMA_TESTS_HOST_SUPPORT_HPP

namespace delaylama::tests {

    inline constexpr auto midi_channel = 1;
    inline constexpr auto raw_note_number = 60;
    inline constexpr auto internal_note_number = 48;
    inline constexpr auto note_velocity = 1.0F;
    inline constexpr auto note_on_sample_offset = 7;
    inline constexpr auto note_off_sample_offset = 13;
    inline constexpr auto pitch_bend_sample_offset = 3;
    inline constexpr auto controller_sample_offset = 5;
    inline constexpr auto controller_value = 64;
    inline constexpr auto pitch_bend_low_value = 0;
    inline constexpr auto pitch_bend_center_value = 8192;
    inline constexpr auto value_tolerance = 1.0E-6F;
    inline constexpr auto unsupported_message_sample_offset = 0;
    inline constexpr auto processor_sample_rate = 44100.0;
    inline constexpr auto processor_max_block_size = 32;
    inline constexpr auto processor_block_sample_count = 16U;
    inline constexpr auto processor_midi_note = 60;
    inline constexpr auto processor_note_on_offset = 0;
    inline constexpr auto processor_pitch_bend_offset = 4;
    inline constexpr auto processor_note_off_offset = 2;
    inline constexpr auto processor_note_velocity = 1.0F;
    inline constexpr auto processor_full_vowel = 1.0F;
    inline constexpr auto processor_default_vowel = 0.5F;
    inline constexpr auto processor_pad_internal_note = 28;
    inline constexpr auto processor_pad_pitch = 0.25F;
    inline constexpr auto processor_pad_vowel = 0.75F;
    inline constexpr auto processor_pitch_bend_first_tick_blocks = 26U;
    inline constexpr auto processor_no_note = -1;
    inline constexpr auto processor_output_channel_count = 2U;
    inline constexpr auto processor_value_tolerance = 1.0E-6F;
    inline constexpr auto pad_status_mask = 0xF0U;
    inline constexpr auto pad_note_on_status = 0x90U;
    inline constexpr auto pad_note_off_status = 0x80U;
    inline constexpr auto pad_controller_status = 0xB0U;
    inline constexpr auto pad_pitch_bend_status = 0xE0U;
    inline constexpr auto pad_x_axis_low = 0.0F;
    inline constexpr auto pad_x_axis_high = 1.0F;
    inline constexpr auto pad_y_axis_low = 0.0F;
    inline constexpr auto pad_y_axis_high = 1.0F;
    inline constexpr auto pad_axis_midpoint = 0.5F;
    inline constexpr auto pad_status_byte_index = 0;
    inline constexpr auto pad_data1_byte_index = 1;
    inline constexpr auto pad_data2_byte_index = 2;
    inline constexpr auto pad_midpoint_seven_bit = 63U;
    inline constexpr auto pad_pitch_lsb = 0U;
    inline constexpr auto pad_midpoint_pitch_msb = 63U;
    inline constexpr auto pitch_bend_msb_radix = 128U;
    inline constexpr auto host_note_below_minimum = 15;
    inline constexpr auto host_note_above_maximum = 85;
    inline constexpr auto host_note_low = 16;
    inline constexpr auto host_note_high = 84;
    inline constexpr auto host_note_low_internal = 4;
    inline constexpr auto host_note_high_internal = 72;
    inline constexpr auto failure_exit_code = 1;
    inline constexpr auto success_exit_code = 0;

    [[nodiscard]] auto run_midi_contract() -> bool;
    [[nodiscard]] auto run_processor_contract() -> bool;

}

#endif
