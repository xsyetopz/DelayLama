#include <cstdint>

#include "support.hpp"

auto main() -> std::int32_t {
    if (!delaylama::tests::run_midi_contract()) {
        return delaylama::tests::failure_exit_code;
    }
    if (!delaylama::tests::run_processor_contract()) {
        return delaylama::tests::failure_exit_code;
    }
    return delaylama::tests::success_exit_code;
}
