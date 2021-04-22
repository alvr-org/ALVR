/*
 * Copyright (c) 2021 Arm Limited.
 *
 * SPDX-License-Identifier: MIT
 *
 * Permission is hereby granted, free of charge, to any person obtaining a copy
 * of this software and associated documentation files (the "Software"), to
 * deal in the Software without restriction, including without limitation the
 * rights to use, copy, modify, merge, publish, distribute, sublicense, and/or
 * sell copies of the Software, and to permit persons to whom the Software is
 * furnished to do so, subject to the following conditions:
 *
 * The above copyright notice and this permission notice shall be included in all
 * copies or substantial portions of the Software.
 *
 * THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
 * IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
 * FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
 * AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
 * LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
 * OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
 * SOFTWARE.
 */

#pragma once

#include <cassert>
#include <stdint.h>

#include <vulkan/vk_icd.h>

namespace util {

/**
 * @brief Set of WSI platforms.
 * @note This could be implemented via std::unordered_set, but would require handling allocation
 * callbacks and would therefore be less convenient to use. Instead, we can store all info in the
 * bits of uint64_t.
 */
class wsi_platform_set {
  public:
    void add(VkIcdWsiPlatform p) { m_platforms |= (static_cast<uint64_t>(1) << to_int(p)); }

    bool contains(VkIcdWsiPlatform p) const {
        return (m_platforms & (static_cast<uint64_t>(1) << to_int(p))) != 0;
    }

  private:
    /**
     * @brief Convert a VkIcdWsiPlatform to an integer between 0-63.
     */
    static int to_int(VkIcdWsiPlatform p) {
        assert(static_cast<int>(p) >= 0 && static_cast<int>(p) < 64);
        return static_cast<int>(p);
    }

    uint64_t m_platforms = 0;
};

} /* namespace util */
