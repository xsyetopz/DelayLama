#ifndef DELAYLAMA_HOST_PROCESSOR_HPP
#define DELAYLAMA_HOST_PROCESSOR_HPP

#include <cstddef>
#include <span>

#include "dsp/engine.hpp"
#include "editor/state.hpp"

/// Format-independent processor API.
namespace delaylama::host {

    /// Owns the format-independent synthesis lifecycle.
    class ProcessorModel final {
    public:
        /// Creates an unprepared processor with default parameters.
        ProcessorModel() noexcept = default;
        /// Releases engine state.
        ~ProcessorModel() = default;

        /// Processor state cannot be copied.
        ProcessorModel(const ProcessorModel&) = delete;
        /// Processor state cannot be copy-assigned.
        auto operator=(const ProcessorModel&) -> ProcessorModel& = delete;
        /// Transfers engine and parameter state.
        ProcessorModel(ProcessorModel&&) noexcept = default;
        /// Replaces this processor with transferred state.
        auto operator=(ProcessorModel&&) noexcept -> ProcessorModel& = default;

        /// Prepares stereo rendering for a sample rate and maximum block size.
        auto prepare(double sample_rate, int max_block_size) noexcept -> void;
        /// Clears rendering state without discarding parameters.
        auto release() noexcept -> void;
        /// Renders a block and applies events at their sample offsets.
        auto process(
            delaylama::OutputChannels outputs,
            std::size_t num_samples,
            std::span<const delaylama::Event> events) noexcept -> void;

        /// Returns the active parameter snapshot.
        [[nodiscard]] auto parameters() const noexcept -> delaylama::Parameters;
        /// Applies parameters to subsequent samples.
        auto set_parameters(const delaylama::Parameters& parameters) noexcept -> void;
        /// Returns the latest editor state.
        [[nodiscard]] auto visual_state() const noexcept -> VisualState;

    private:
        delaylama::SynthEngine engine_;
        delaylama::Parameters parameters_ {};
    };

}

#endif
