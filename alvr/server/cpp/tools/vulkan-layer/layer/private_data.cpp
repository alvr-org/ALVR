/*
 * Copyright (c) 2018-2021 Arm Limited.
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

#include "private_data.hpp"

#include "wsi/wsi_factory.hpp"

#include <unordered_map>

namespace layer {

static std::mutex g_data_lock;
static std::unordered_map<void *, std::unique_ptr<instance_private_data>> g_instance_data;
static std::unordered_map<void *, std::unique_ptr<device_private_data>> g_device_data;

template <typename object_type, typename get_proc_type>
static PFN_vkVoidFunction get_proc_helper(object_type obj, get_proc_type get_proc,
                                          const char *proc_name, bool required, bool &ok) {
    PFN_vkVoidFunction ret = get_proc(obj, proc_name);
    if (nullptr == ret && required) {
        ok = false;
    }
    return ret;
}

VkResult instance_dispatch_table::populate(VkInstance instance,
                                           PFN_vkGetInstanceProcAddr get_proc) {
    bool ok = true;
#define REQUIRED(x)                                                                                \
    x = reinterpret_cast<PFN_vk##x>(get_proc_helper(instance, get_proc, "vk" #x, true, ok));
#define OPTIONAL(x)                                                                                \
    x = reinterpret_cast<PFN_vk##x>(get_proc_helper(instance, get_proc, "vk" #x, false, ok));
    INSTANCE_ENTRYPOINTS_LIST(REQUIRED, OPTIONAL);
#undef REQUIRED
#undef OPTIONAL
    return ok ? VK_SUCCESS : VK_ERROR_INITIALIZATION_FAILED;
}

VkResult device_dispatch_table::populate(VkDevice device, PFN_vkGetDeviceProcAddr get_proc) {
    bool ok = true;
#define REQUIRED(x)                                                                                \
    x = reinterpret_cast<PFN_vk##x>(get_proc_helper(device, get_proc, "vk" #x, true, ok));
#define OPTIONAL(x)                                                                                \
    x = reinterpret_cast<PFN_vk##x>(get_proc_helper(device, get_proc, "vk" #x, false, ok));
    DEVICE_ENTRYPOINTS_LIST(REQUIRED, OPTIONAL);
#undef REQUIRED
#undef OPTIONAL
    return ok ? VK_SUCCESS : VK_ERROR_INITIALIZATION_FAILED;
}

instance_private_data::instance_private_data(const instance_dispatch_table &table,
                                             PFN_vkSetInstanceLoaderData set_loader_data,
                                             util::wsi_platform_set enabled_layer_platforms)
    : disp(table), SetInstanceLoaderData(set_loader_data),
      enabled_layer_platforms(enabled_layer_platforms) {}

template <typename dispatchable_type>
static inline void *get_key(dispatchable_type dispatchable_object) {
    return *reinterpret_cast<void **>(dispatchable_object);
}

void instance_private_data::set(VkInstance inst, std::unique_ptr<instance_private_data> inst_data) {
    scoped_mutex lock(g_data_lock);
    g_instance_data[get_key(inst)] = std::move(inst_data);
}

template <typename dispatchable_type>
static instance_private_data &get_instance_private_data(dispatchable_type dispatchable_object) {
    scoped_mutex lock(g_data_lock);
    return *g_instance_data[get_key(dispatchable_object)];
}

instance_private_data &instance_private_data::get(VkInstance instance) {
    return get_instance_private_data(instance);
}

instance_private_data &instance_private_data::get(VkPhysicalDevice phys_dev) {
    return get_instance_private_data(phys_dev);
}

static VkIcdWsiPlatform get_platform_of_surface(VkSurfaceKHR surface) {
    VkIcdSurfaceBase *surface_base = reinterpret_cast<VkIcdSurfaceBase *>(surface);
    return surface_base->platform;
}

bool instance_private_data::does_layer_support_surface(VkSurfaceKHR surface) {
    return enabled_layer_platforms.contains(get_platform_of_surface(surface));
}

bool instance_private_data::do_icds_support_surface(VkPhysicalDevice, VkSurfaceKHR) {
    /* For now assume ICDs do not support VK_KHR_surface. This means that the layer will handle all
     * the surfaces it can handle (even if the ICDs can handle the surface) and only call down for
     * surfaces it cannot handle. In the future we may allow system integrators to configure which
     * ICDs have precedence handling which platforms.
     */
    return false;
}

bool instance_private_data::should_layer_handle_surface(VkSurfaceKHR surface) {
    return surfaces.find(surface) != surfaces.end();
}

void instance_private_data::destroy(VkInstance inst) {
    scoped_mutex lock(g_data_lock);
    g_instance_data.erase(get_key(inst));
}

void instance_private_data::add_surface(VkSurfaceKHR surface) {
    scoped_mutex lock(g_data_lock);
    surfaces.insert(surface);
}

device_private_data::device_private_data(instance_private_data &inst_data,
                                         VkPhysicalDevice phys_dev, VkDevice dev,
                                         const device_dispatch_table &table,
                                         PFN_vkSetDeviceLoaderData set_loader_data)
    : disp{table}, instance_data{inst_data}, SetDeviceLoaderData{set_loader_data},
      physical_device{phys_dev}, device{dev} {}

void device_private_data::set(VkDevice dev, std::unique_ptr<device_private_data> dev_data) {
    scoped_mutex lock(g_data_lock);
    g_device_data[get_key(dev)] = std::move(dev_data);
}

template <typename dispatchable_type>
static device_private_data &get_device_private_data(dispatchable_type dispatchable_object) {
    scoped_mutex lock(g_data_lock);
    return *g_device_data[get_key(dispatchable_object)];
}

device_private_data &device_private_data::get(VkDevice device) {
    return get_device_private_data(device);
}

device_private_data &device_private_data::get(VkQueue queue) {
    return get_device_private_data(queue);
}

void device_private_data::add_layer_swapchain(VkSwapchainKHR swapchain) {
    scoped_mutex lock(swapchains_lock);
    swapchains.insert(swapchain);
}

bool device_private_data::layer_owns_all_swapchains(const VkSwapchainKHR *swapchain,
                                                    uint32_t swapchain_count) const {
    scoped_mutex lock(swapchains_lock);
    for (uint32_t i = 0; i < swapchain_count; i++) {
        if (swapchains.find(swapchain[i]) == swapchains.end()) {
            return false;
        }
    }
    return true;
}

bool device_private_data::should_layer_create_swapchain(VkSurfaceKHR vk_surface) {
    return instance_data.should_layer_handle_surface(vk_surface);
}

bool device_private_data::can_icds_create_swapchain(VkSurfaceKHR vk_surface) {
    return disp.CreateSwapchainKHR != nullptr;
}

void device_private_data::destroy(VkDevice dev) {
    scoped_mutex lock(g_data_lock);
    g_device_data.erase(get_key(dev));
}
} /* namespace layer */
