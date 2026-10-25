#pragma once

#include <chrono>
#include <cstdint>

namespace alvr {

// The callback imports/waits for one advertised timeline. The first failed
// confirmation rejects the whole job. Never multiply the wait budget by the
// number of textures, and still accept an already-ready value at the deadline.
// Missing handles belong to the separate implicit-fence path.
// A clock callable makes deadline/exhaustion behavior testable without sleeps.
template <typename Waits, typename Wait, typename Now>
bool waitForTextureTimelines(
    Waits const& waits, std::chrono::nanoseconds budget, Wait&& wait, Now&& now
) {
    auto const deadline = now() + budget;
    for (auto const& texture : waits) {
        if (texture.semaphore == 0) {
            continue;
        }
        auto const remaining = std::chrono::duration_cast<std::chrono::nanoseconds>(
            deadline - now()
        ).count();
        if (!wait(texture, remaining > 0 ? static_cast<uint64_t>(remaining) : 0)) {
            return false;
        }
    }
    return true;
}

template <typename Waits, typename Wait>
bool waitForTextureTimelines(
    Waits const& waits, std::chrono::nanoseconds budget, Wait&& wait
) {
    return waitForTextureTimelines(
        waits, budget, wait, [] { return std::chrono::steady_clock::now(); }
    );
}

} // namespace alvr

