#include "processor.hpp"

#include <cstddef>
#include <span>

#include "dsp/engine.hpp"
#include "editor/state.hpp"

namespace {

    // Keep stereo policy here so SynthEngine remains channel-count agnostic.
    constexpr auto stereo_channel_count = 2;

}

namespace delaylama::host {

    auto ProcessorModel::prepare(const double sample_rate, const int max_block_size) noexcept
        -> void {
        engine_.prepare(sample_rate, max_block_size, stereo_channel_count);
        engine_.reset();
        engine_.set_parameters(parameters_);
    }

    auto ProcessorModel::release() noexcept -> void {
        engine_.reset();
    }

    auto ProcessorModel::process(
        const delaylama::OutputChannels outputs,
        const std::size_t num_samples,
        const std::span<const delaylama::Event> events) noexcept -> void {
        engine_.process(outputs, num_samples, events);
    }

    auto ProcessorModel::parameters() const noexcept -> delaylama::Parameters {
        return engine_.parameters();
    }

    auto ProcessorModel::set_parameters(const delaylama::Parameters& parameters) noexcept -> void {
        parameters_ = parameters;
        engine_.set_parameters(parameters_);
    }

    auto ProcessorModel::visual_state() const noexcept -> VisualState {
        const auto voice = engine_.voice_state();
        const auto parameters = engine_.parameters();
        const auto pad = engine_.pad_state();
        const auto vowel = voice.gate && pad.active ? pad.vowel : parameters.vowel;
        return VisualState {
            .note = voice.current_note,
            .gate = voice.gate,
            .vowel = vowel,
            .atlas_selector = engine_.atlas_selector()};
    }

}
