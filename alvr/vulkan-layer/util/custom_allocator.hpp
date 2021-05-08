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

#include <cassert>
#include <new>
#include <string>
#include <vector>

#include <vulkan/vulkan.h>

#pragma once

namespace util {

/**
 * @brief Minimalistic wrapper of VkAllocationCallbacks.
 */
class allocator {
  public:
    /**
     * @brief Get an allocator that can be used if VkAllocationCallbacks are not provided.
     */
    static const allocator &get_generic();

    /**
     * @brief Construct a new wrapper for the given VK callbacks and scope.
     * @param callbacks Pointer to allocation callbacks. If this is @c nullptr, then default
     *   allocation callbacks are used. These can be accessed through #m_callbacks.
     * @param scope The scope to use for this allocator.
     */
    allocator(const VkAllocationCallbacks *callbacks, VkSystemAllocationScope scope);

    /**
     * @brief Copy the given allocator, but change the allocation scope.
     */
    allocator(const allocator &other, VkSystemAllocationScope new_scope);

    /**
     * @brief Get a pointer to the allocation callbacks provided while constructing this object.
     * @return a copy of the #VkAllocationCallback argument provided in the allocator constructor
     *   or @c nullptr if this argument was provided as @c nullptr.
     * @note The #m_callbacks member is always populated with callable pointers for pfnAllocation,
     *   pfnReallocation and pfnFree.
     */
    const VkAllocationCallbacks *get_original_callbacks() const;

    /**
     * @brief Helper method to allocate and construct objects with a custom allocator.
     * @param num_objects Number of objects to create.
     * @return Pointer to the newly created objects or @c nullptr if allocation failed.
     */
    template <typename T, typename... arg_types>
    T *create(size_t num_objects, arg_types &&...args) const noexcept;

    /**
     * @brief Helper method to destroy and deallocate objects constructed with allocator::create().
     * @param num_objects Number of objects to destroy.
     */
    template <typename T> void destroy(size_t num_objects, T *obj) const noexcept;

    VkAllocationCallbacks m_callbacks;
    VkSystemAllocationScope m_scope;
};

/**
 * @brief Implementation of an allocator that can be used with STL containers.
 */
template <typename T> class custom_allocator {
  public:
    using value_type = T;
    using pointer = T *;

    custom_allocator(const allocator &alloc) : m_alloc(alloc) {}

    template <typename U>
    custom_allocator(const custom_allocator<U> &other) : m_alloc(other.get_data()) {}

    const allocator &get_data() const { return m_alloc; }

    pointer allocate(size_t n) const {
        size_t size = n * sizeof(T);
        auto &cb = m_alloc.m_callbacks;
        void *ret = cb.pfnAllocation(cb.pUserData, size, alignof(T), m_alloc.m_scope);
        if (ret == nullptr)
            throw std::bad_alloc();
        return reinterpret_cast<pointer>(ret);
    }

    pointer allocate(size_t n, void *ptr) const {
        size_t size = n * sizeof(T);
        auto &cb = m_alloc.m_callbacks;
        void *ret = cb.pfnReallocation(cb.pUserData, ptr, size, alignof(T), m_alloc.m_scope);
        if (ret == nullptr)
            throw std::bad_alloc();
        return reinterpret_cast<pointer>(ret);
    }

    void deallocate(void *ptr, size_t) const noexcept {
        m_alloc.m_callbacks.pfnFree(m_alloc.m_callbacks.pUserData, ptr);
    }

  private:
    const allocator m_alloc;
};

template <typename T, typename U>
bool operator==(const custom_allocator<T> &, const custom_allocator<U> &) {
    return true;
}

template <typename T, typename U>
bool operator!=(const custom_allocator<T> &, const custom_allocator<U> &) {
    return false;
}

template <typename T, typename... arg_types>
T *allocator::create(size_t num_objects, arg_types &&...args) const noexcept {
    if (num_objects < 1) {
        return nullptr;
    }

    custom_allocator<T> allocator(*this);
    T *ptr;
    try {
        ptr = allocator.allocate(num_objects);
    } catch (...) {
        return nullptr;
    }

    size_t objects_constructed = 0;
    try {
        while (objects_constructed < num_objects) {
            T *next_object = &ptr[objects_constructed];
            new (next_object) T(std::forward<arg_types>(args)...);
            objects_constructed++;
        }
    } catch (...) {
        /* We catch all exceptions thrown while constructing the object, not just
         * std::bad_alloc.
         */
        while (objects_constructed > 0) {
            objects_constructed--;
            ptr[objects_constructed].~T();
        }
        allocator.deallocate(ptr, num_objects);
        return nullptr;
    }
    return ptr;
}

template <typename T> void allocator::destroy(size_t num_objects, T *objects) const noexcept {
    assert((objects == nullptr) == (num_objects == 0));
    if (num_objects == 0) {
        return;
    }

    custom_allocator<T> allocator(*this);
    for (size_t i = 0; i < num_objects; i++) {
        objects[i].~T();
    }
    allocator.deallocate(objects, num_objects);
}

template <typename T> void destroy_custom(T *obj) { T::destroy(obj); }

/**
 * @brief Vector using a Vulkan custom allocator to allocate its elements.
 * @note The vector must be passed a custom_allocator during construction and it takes a copy
 *   of it, meaning that the user is free to destroy the custom_allocator after constructing the
 *   vector.
 */
template <typename T> class vector : public std::vector<T, custom_allocator<T>> {
  public:
    using base = std::vector<T, custom_allocator<T>>;
    using base::base;

    /* Delete all methods that can cause allocation failure, i.e. can throw std::bad_alloc.
     *
     * Rationale: we want to force users to use our corresponding try_... method instead:
     * this makes the API slightly more annoying to use, but hopefully safer as it encourages
     * users to check for allocation failures, which is important for Vulkan.
     *
     * Note: deleting each of these methods (below) deletes all its overloads from the base class,
     *   to be precise: the deleted method covers the methods (all overloads) in the base class.
     * Note: clear() is already noexcept since C++11.
     */
    void insert() = delete;
    void emplace() = delete;
    void emplace_back() = delete;
    void push_back() = delete;
    void resize() = delete;
    void reserve() = delete;

    /* Note pop_back(), erase(), clear() do not throw std::bad_alloc exceptions. */

    /* @brief Like std::vector::push_back, but non throwing.
     * @return @c false iff the operation could not be performed due to an allocation failure.
     */
    template <typename... arg_types> bool try_push_back(arg_types &&...args) noexcept {
        try {
            base::push_back(std::forward<arg_types>(args)...);
            return true;
        } catch (const std::bad_alloc &e) {
            return false;
        }
    }

    /* @brief push back multiple elements at once
     * @return @c false iff the operation could not be performed due to an allocation failure.
     */
    bool try_push_back_many(const T *begin, const T *end) noexcept {
        for (const T *it = begin; it != end; ++it) {
            if (!try_push_back(*it)) {
                return false;
            }
        }
        return true;
    }

    /* @brief Like std::vector::resize, but non throwing.
     * @return @c false iff the operation could not be performed due to an allocation failure.
     */
    template <typename... arg_types> bool try_resize(arg_types &&...args) noexcept {
        try {
            base::resize(std::forward<arg_types>(args)...);
            return true;
        } catch (const std::bad_alloc &e) {
            return false;
        }
    }
};

} /* namespace util */
