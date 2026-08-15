#include <cstdint>
#include <iostream>

#include "support.hpp"

auto main() noexcept -> std::int32_t {
    delaylama::tests::run_control_tests();
    delaylama::tests::run_render_tests();
    std::cout << "Delay Lama synthesis tests passed\n";
    return 0;
}
