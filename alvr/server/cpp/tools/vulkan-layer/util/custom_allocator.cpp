/*
 * Copyright (c) 2020-2021 Arm Limited.
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

#include "custom_allocator.hpp"

extern "C" {

static void *default_allocation(void *, size_t size, size_t, VkSystemAllocationScope) {
    return malloc(size);
}

static void *default_reallocation(void *, void *pOriginal, size_t size, size_t,
                                  VkSystemAllocationScope) {
    return realloc(pOriginal, size);
}

static void default_free(void *, void *pMemory) { free(pMemory); }
}

namespace util {

const allocator &allocator::get_generic() {
    static allocator generic{nullptr, VK_SYSTEM_ALLOCATION_SCOPE_COMMAND};
    return generic;
}

allocator::allocator(const allocator &other, VkSystemAllocationScope new_scope)
    : allocator{other.get_original_callbacks(), new_scope} {}

/* If callbacks is already populated by vulkan then use those specified as default. */
allocator::allocator(const VkAllocationCallbacks *callbacks, VkSystemAllocationScope scope) {
    m_scope = scope;
    if (callbacks != nullptr) {
        m_callbacks = *callbacks;
    } else {
        m_callbacks = {};
        m_callbacks.pfnAllocation = default_allocation;
        m_callbacks.pfnReallocation = default_reallocation;
        m_callbacks.pfnFree = default_free;
    }
}

const VkAllocationCallbacks *allocator::get_original_callbacks() const {
    return m_callbacks.pfnAllocation == default_allocation ? nullptr : &m_callbacks;
}

} /* namespace util */
