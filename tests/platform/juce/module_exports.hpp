#ifndef DELAYLAMA_TESTS_MODULE_EXPORTS_HPP
#define DELAYLAMA_TESTS_MODULE_EXPORTS_HPP

// Use JUCE's umbrella because its module internals are not standalone headers.
#include <juce_audio_processors/juce_audio_processors.h>  // IWYU pragma: export

namespace delaylama::juce_test {
    using AudioBuffer = juce::AudioBuffer<float>;
    using AudioProcessor = juce::AudioProcessor;
    using AudioProcessorListener = juce::AudioProcessorListener;
    using MidiBuffer = juce::MidiBuffer;
}

#endif
