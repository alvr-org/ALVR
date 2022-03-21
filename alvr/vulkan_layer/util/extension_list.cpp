/*
 * Copyright (c) 2019, 2021 Arm Limited.
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

#include "extension_list.hpp"
#include <cassert>
#include <layer/private_data.hpp>
#include <string.h>

namespace util {

extension_list::extension_list(const util::allocator &allocator)
    : m_alloc{allocator}, m_ext_props(allocator) {}

VkResult extension_list::add(const struct VkEnumerateInstanceExtensionPropertiesChain *chain) {
    uint32_t count;
    VkResult m_error = chain->CallDown(nullptr, &count, nullptr);
    if (m_error == VK_SUCCESS) {
        if (!m_ext_props.try_resize(count)) {
            return VK_ERROR_OUT_OF_HOST_MEMORY;
        }
        m_error = chain->CallDown(nullptr, &count, m_ext_props.data());
    }
    return m_error;
}

VkResult extension_list::add(VkPhysicalDevice dev) {
    layer::instance_private_data &inst_data = layer::instance_private_data::get(dev);
    uint32_t count;
    VkResult m_error =
        inst_data.disp.EnumerateDeviceExtensionProperties(dev, nullptr, &count, nullptr);

    if (m_error == VK_SUCCESS) {
        if (!m_ext_props.try_resize(count)) {
            return VK_ERROR_OUT_OF_HOST_MEMORY;
        }
        m_error = inst_data.disp.EnumerateDeviceExtensionProperties(dev, nullptr, &count,
                                                                    m_ext_props.data());
    }
    return m_error;
}

VkResult extension_list::add(
    PFN_vkEnumerateInstanceExtensionProperties fpEnumerateInstanceExtensionProperties) {
    uint32_t count = 0;
    VkResult m_error = fpEnumerateInstanceExtensionProperties(nullptr, &count, nullptr);

    if (m_error == VK_SUCCESS) {
        if (!m_ext_props.try_resize(count)) {
            return VK_ERROR_OUT_OF_HOST_MEMORY;
        }
        m_error = fpEnumerateInstanceExtensionProperties(nullptr, &count, m_ext_props.data());
    }
    return m_error;
}

VkResult extension_list::add(const char *const *extensions, uint32_t count) {
    for (uint32_t i = 0; i < count; i++) {
        VkExtensionProperties props = {};
        strncpy(props.extensionName, extensions[i], sizeof(props.extensionName) - 1);
        props.extensionName[sizeof(props.extensionName) - 1] = '\0';
        if (!m_ext_props.try_push_back(props)) {
            return VK_ERROR_OUT_OF_HOST_MEMORY;
        }
    }
    return VK_SUCCESS;
}

VkResult extension_list::add(const VkExtensionProperties *props, uint32_t count) {
    if (!m_ext_props.try_push_back_many(props, props + count)) {
        return VK_ERROR_OUT_OF_HOST_MEMORY;
    }
    return VK_SUCCESS;
}

VkResult extension_list::add(const char *ext) {
    if (!contains(ext)) {
        VkExtensionProperties props = {};
        strncpy(props.extensionName, ext, sizeof(props.extensionName) - 1);
        props.extensionName[sizeof(props.extensionName) - 1] = '\0';
        if (!m_ext_props.try_push_back(props)) {
            return VK_ERROR_OUT_OF_HOST_MEMORY;
        }
    }
    return VK_SUCCESS;
}

VkResult extension_list::add(VkExtensionProperties ext_prop) {
    if (!contains(ext_prop.extensionName)) {
        if (!m_ext_props.try_push_back(ext_prop)) {
            return VK_ERROR_OUT_OF_HOST_MEMORY;
        }
    }
    return VK_SUCCESS;
}

VkResult extension_list::add(const char **ext_list, uint32_t count) {
    for (uint32_t i = 0; i < count; i++) {
        if (add(ext_list[i]) != VK_SUCCESS) {
            return VK_ERROR_OUT_OF_HOST_MEMORY;
        }
    }
    return VK_SUCCESS;
}

VkResult extension_list::add(const extension_list &ext_list) {
    util::vector<VkExtensionProperties> ext_vect = ext_list.get_extension_props();
    for (auto &ext : ext_vect) {
        if (add(ext) != VK_SUCCESS) {
            return VK_ERROR_OUT_OF_HOST_MEMORY;
        }
    }
    return VK_SUCCESS;
}

bool extension_list::get_extension_strings(util::vector<const char *> &out) const {
    size_t old_size = out.size();
    size_t new_size = old_size + m_ext_props.size();
    if (!out.try_resize(new_size)) {
        return false;
    }

    for (size_t i = old_size; i < new_size; i++) {
        out[i] = m_ext_props[i - old_size].extensionName;
    }
    return true;
}

bool extension_list::contains(const extension_list &req) const {
    for (const auto &req_ext : req.m_ext_props) {
        if (!contains(req_ext.extensionName)) {
            return false;
        }
    }
    return true;
}

bool extension_list::contains(const char *extension_name) const {
    for (const auto &p : m_ext_props) {
        if (strcmp(p.extensionName, extension_name) == 0) {
            return true;
        }
    }
    return false;
}

void extension_list::remove(const char *ext) {
    m_ext_props.erase(std::remove_if(m_ext_props.begin(), m_ext_props.end(),
                                     [&ext](VkExtensionProperties ext_prop) {
                                         return (strcmp(ext_prop.extensionName, ext) == 0);
                                     }));
}
} // namespace util
