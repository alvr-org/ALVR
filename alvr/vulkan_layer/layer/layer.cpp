/*
 * Copyright (c) 2016-2021 Arm Limited.
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
#include <cstdio>
#include <cstring>
#include <fstream>
#include <iostream>

#include <vulkan/vk_layer.h>

#include "settings.h"
#include "device_api.hpp"
#include "private_data.hpp"
#include "surface_api.hpp"
#include "swapchain_api.hpp"
#include "util/custom_allocator.hpp"
#include "util/extension_list.hpp"
#include "wsi/wsi_factory.hpp"
#include "layer.h"

#define VK_LAYER_API_VERSION VK_MAKE_VERSION(1, 0, VK_HEADER_VERSION)

namespace layer {

static const VkLayerProperties global_layer = {
    "VK_LAYER_ALVR_capture",
    VK_LAYER_API_VERSION,
    1,
    "ALVR capture layer",
};
static const VkExtensionProperties device_extension[] = {
    {VK_KHR_SWAPCHAIN_EXTENSION_NAME, VK_KHR_SWAPCHAIN_SPEC_VERSION}};
static const VkExtensionProperties instance_extension[] = {
    {VK_KHR_SURFACE_EXTENSION_NAME, VK_KHR_SURFACE_SPEC_VERSION}};

VKAPI_ATTR VkResult extension_properties(const uint32_t count,
                                         const VkExtensionProperties *layer_ext, uint32_t *pCount,
                                         VkExtensionProperties *pProp) {
    uint32_t size;

    if (pProp == NULL || layer_ext == NULL) {
        *pCount = count;
        return VK_SUCCESS;
    }

    size = *pCount < count ? *pCount : count;
    memcpy(pProp, layer_ext, size * sizeof(VkExtensionProperties));
    *pCount = size;
    if (size < count) {
        return VK_INCOMPLETE;
    }

    return VK_SUCCESS;
}

VKAPI_ATTR VkResult layer_properties(const uint32_t count, const VkLayerProperties *layer_prop,
                                     uint32_t *pCount, VkLayerProperties *pProp) {
    uint32_t size;

    if (pProp == NULL || layer_prop == NULL) {
        *pCount = count;
        return VK_SUCCESS;
    }

    size = *pCount < count ? *pCount : count;
    memcpy(pProp, layer_prop, size * sizeof(VkLayerProperties));
    *pCount = size;
    if (size < count) {
        return VK_INCOMPLETE;
    }

    return VK_SUCCESS;
}

VKAPI_ATTR VkLayerInstanceCreateInfo *get_chain_info(const VkInstanceCreateInfo *pCreateInfo,
                                                     VkLayerFunction func) {
    VkLayerInstanceCreateInfo *chain_info = (VkLayerInstanceCreateInfo *)pCreateInfo->pNext;
    while (chain_info && !(chain_info->sType == VK_STRUCTURE_TYPE_LOADER_INSTANCE_CREATE_INFO &&
                           chain_info->function == func)) {
        chain_info = (VkLayerInstanceCreateInfo *)chain_info->pNext;
    }

    return chain_info;
}

VKAPI_ATTR VkLayerDeviceCreateInfo *get_chain_info(const VkDeviceCreateInfo *pCreateInfo,
                                                   VkLayerFunction func) {
    VkLayerDeviceCreateInfo *chain_info = (VkLayerDeviceCreateInfo *)pCreateInfo->pNext;
    while (chain_info && !(chain_info->sType == VK_STRUCTURE_TYPE_LOADER_DEVICE_CREATE_INFO &&
                           chain_info->function == func)) {
        chain_info = (VkLayerDeviceCreateInfo *)chain_info->pNext;
    }

    return chain_info;
}

/* This is where the layer is initialised and the instance dispatch table is constructed. */
VKAPI_ATTR VkResult create_instance(const VkInstanceCreateInfo *pCreateInfo,
                                    const VkAllocationCallbacks *pAllocator,
                                    VkInstance *pInstance) {
    // Make sure settings are loaded before we access them
    Settings::Instance().Load();

    VkLayerInstanceCreateInfo *layerCreateInfo = get_chain_info(pCreateInfo, VK_LAYER_LINK_INFO);
    PFN_vkSetInstanceLoaderData loader_callback =
        get_chain_info(pCreateInfo, VK_LOADER_DATA_CALLBACK)->u.pfnSetInstanceLoaderData;

    if (nullptr == layerCreateInfo || nullptr == layerCreateInfo->u.pLayerInfo) {
        return VK_ERROR_INITIALIZATION_FAILED;
    }

    PFN_vkGetInstanceProcAddr fpGetInstanceProcAddr =
        layerCreateInfo->u.pLayerInfo->pfnNextGetInstanceProcAddr;

    PFN_vkCreateInstance fpCreateInstance =
        (PFN_vkCreateInstance)fpGetInstanceProcAddr(nullptr, "vkCreateInstance");
    if (nullptr == fpCreateInstance) {
        return VK_ERROR_INITIALIZATION_FAILED;
    }

    /* Advance the link info for the next element on the chain. */
    layerCreateInfo->u.pLayerInfo = layerCreateInfo->u.pLayerInfo->pNext;

    /* The layer needs some Vulkan 1.2 functionality in order to operate correctly.
     * We thus change the application info to require this API version, if necessary.
     * This may have consequences for ICDs whose behaviour depends on apiVersion.
     */
    const uint32_t minimum_required_vulkan_version = VK_API_VERSION_1_2;
    VkApplicationInfo modified_app_info{};
    if (nullptr != pCreateInfo->pApplicationInfo) {
        modified_app_info = *pCreateInfo->pApplicationInfo;
        if (modified_app_info.apiVersion < minimum_required_vulkan_version) {
            modified_app_info.apiVersion = minimum_required_vulkan_version;
        }
    } else {
        modified_app_info.sType = VK_STRUCTURE_TYPE_APPLICATION_INFO;
        modified_app_info.apiVersion = minimum_required_vulkan_version;
    }

    // Hijack one extension name
    // the headless extension can't be added as a new parameter, because the loader performs a copy before
    // calling the createInstance functions. The loader must know we activated this function because it
    // will enable bits in the wsi part, so we switch to vulkan 1.1, and replace one of the extentions
    // that has been promoted, with a const_cast.
    for (uint32_t i = 0 ; i < pCreateInfo->enabledExtensionCount ; ++i) {
      if (strcmp("VK_KHR_external_memory_capabilities", pCreateInfo->ppEnabledExtensionNames[i]) == 0)
      {
        const char** ext = const_cast<const char**>(pCreateInfo->ppEnabledExtensionNames + i);
        *ext = VK_EXT_HEADLESS_SURFACE_EXTENSION_NAME;
      }
    }

    auto createInfo = *pCreateInfo;
    createInfo.pApplicationInfo = &modified_app_info;

    /* Now call create instance on the chain further down the list.
     * Note that we do not remove the extensions that the layer supports from
     * modified_info.ppEnabledExtensionNames. Layers have to abide the rule that vkCreateInstance
     * must not generate an error for unrecognized extension names. Also, the loader filters the
     * extension list to ensure that ICDs do not see extensions that they do not support.
     */
    VkResult result;
    result = fpCreateInstance(&createInfo, pAllocator, pInstance);
    if (result != VK_SUCCESS) {
        return result;
    }

    instance_dispatch_table table;
    result = table.populate(*pInstance, fpGetInstanceProcAddr);
    if (result != VK_SUCCESS) {
        return result;
    }

    /* Find all the platforms that the layer can handle based on
     * pCreateInfo->ppEnabledExtensionNames. */
    auto layer_platforms_to_enable = wsi::find_enabled_layer_platforms(pCreateInfo);

    std::unique_ptr<instance_private_data> inst_data{
        new instance_private_data{table, loader_callback, layer_platforms_to_enable}};
    instance_private_data::set(*pInstance, std::move(inst_data));
    return VK_SUCCESS;
}

VKAPI_ATTR VkResult create_device(VkPhysicalDevice physicalDevice,
                                  const VkDeviceCreateInfo *pCreateInfo,
                                  const VkAllocationCallbacks *pAllocator, VkDevice *pDevice) {
    VkLayerDeviceCreateInfo *layerCreateInfo = get_chain_info(pCreateInfo, VK_LAYER_LINK_INFO);
    PFN_vkSetDeviceLoaderData loader_callback =
        get_chain_info(pCreateInfo, VK_LOADER_DATA_CALLBACK)->u.pfnSetDeviceLoaderData;

    if (nullptr == layerCreateInfo || nullptr == layerCreateInfo->u.pLayerInfo) {
        return VK_ERROR_INITIALIZATION_FAILED;
    }

    /* Retrieve the vkGetDeviceProcAddr and the vkCreateDevice function pointers for the next layer
     * in the chain. */
    PFN_vkGetInstanceProcAddr fpGetInstanceProcAddr =
        layerCreateInfo->u.pLayerInfo->pfnNextGetInstanceProcAddr;
    PFN_vkGetDeviceProcAddr fpGetDeviceProcAddr =
        layerCreateInfo->u.pLayerInfo->pfnNextGetDeviceProcAddr;
    PFN_vkCreateDevice fpCreateDevice =
        (PFN_vkCreateDevice)fpGetInstanceProcAddr(VK_NULL_HANDLE, "vkCreateDevice");
    if (nullptr == fpCreateDevice) {
        return VK_ERROR_INITIALIZATION_FAILED;
    }

    /* Advance the link info for the next element on the chain. */
    layerCreateInfo->u.pLayerInfo = layerCreateInfo->u.pLayerInfo->pNext;

    /* Copy the extension to a util::extension_list. */
    util::allocator allocator{pAllocator, VK_SYSTEM_ALLOCATION_SCOPE_COMMAND};
    util::extension_list enabled_extensions{allocator};
    VkResult result;
    result = enabled_extensions.add(pCreateInfo->ppEnabledExtensionNames,
                                    pCreateInfo->enabledExtensionCount);
    if (result != VK_SUCCESS) {
        return result;
    }

    /* Add the extensions required by the platforms that are being enabled in the layer. */
    auto &inst_data = instance_private_data::get(physicalDevice);
    const util::wsi_platform_set &enabled_platforms = inst_data.get_enabled_platforms();
    result = wsi::add_extensions_required_by_layer(physicalDevice, enabled_platforms,
                                                   enabled_extensions);
    if (result != VK_SUCCESS) {
        return result;
    }

    util::vector<const char *> modified_enabled_extensions{allocator};
    if (!enabled_extensions.get_extension_strings(modified_enabled_extensions)) {
        return VK_ERROR_OUT_OF_HOST_MEMORY;
    }

    /* Now call create device on the chain further down the list. */
    VkDeviceCreateInfo modified_info = *pCreateInfo;
    modified_info.ppEnabledExtensionNames = modified_enabled_extensions.data();
    modified_info.enabledExtensionCount = modified_enabled_extensions.size();

    // Enable timeline semaphores
    VkPhysicalDeviceFeatures2 features = {};
    features.sType = VK_STRUCTURE_TYPE_PHYSICAL_DEVICE_FEATURES_2;

    VkPhysicalDeviceVulkan12Features features12 = {};
    features12.sType = VK_STRUCTURE_TYPE_PHYSICAL_DEVICE_VULKAN_1_2_FEATURES;

    VkPhysicalDeviceFeatures2 *features_ptr = nullptr;
    VkPhysicalDeviceVulkan12Features *features12_ptr = nullptr;

    VkDeviceCreateInfo *next = &modified_info;
    while (next->pNext) {
        if (next->sType == VK_STRUCTURE_TYPE_PHYSICAL_DEVICE_FEATURES_2) {
            features_ptr = (VkPhysicalDeviceFeatures2*)next;
        } else if (next->sType == VK_STRUCTURE_TYPE_PHYSICAL_DEVICE_VULKAN_1_2_FEATURES) {
            features12_ptr = (VkPhysicalDeviceVulkan12Features*)next;
        }
        next = (VkDeviceCreateInfo*)next->pNext;
    }
    if (!features_ptr) {
        features_ptr = &features;
        next->pNext = features_ptr;
        next = (VkDeviceCreateInfo*)features_ptr;
    }
    if (!features12_ptr) {
        features12_ptr = &features12;
        next->pNext = features12_ptr;
        next = (VkDeviceCreateInfo*)features12_ptr;
    }
    features12_ptr->timelineSemaphore = true;

    if (modified_info.pEnabledFeatures) {
        features_ptr->features = *modified_info.pEnabledFeatures;
        modified_info.pEnabledFeatures = nullptr;
    }

    result = fpCreateDevice(physicalDevice, &modified_info, pAllocator, pDevice);
    if (result != VK_SUCCESS) {
        return result;
    }

    device_dispatch_table table;
    result = table.populate(*pDevice, fpGetDeviceProcAddr);
    if (result != VK_SUCCESS) {
        return result;
    }

    std::unique_ptr<device_private_data> device{
        new device_private_data{inst_data, physicalDevice, *pDevice, table, loader_callback}};
    device->display = std::make_unique<wsi::display>();
    device_private_data::set(*pDevice, std::move(device));
    return VK_SUCCESS;
}

/* Clean up the dispatch table for this instance. */
VKAPI_ATTR void VKAPI_CALL
wsi_layer_vkDestroyInstance(VkInstance instance, const VkAllocationCallbacks *pAllocator) {
    assert(instance);
    layer::instance_private_data::get(instance).disp.DestroyInstance(instance, pAllocator);
    layer::instance_private_data::destroy(instance);
}

VKAPI_ATTR void VKAPI_CALL
wsi_layer_vkDestroyDevice(VkDevice device, const VkAllocationCallbacks *pAllocator) {
    layer::device_private_data::destroy(device);
}

VKAPI_ATTR VkResult VKAPI_CALL
wsi_layer_vkCreateInstance(const VkInstanceCreateInfo *pCreateInfo,
                           const VkAllocationCallbacks *pAllocator, VkInstance *pInstance) {
    return layer::create_instance(pCreateInfo, pAllocator, pInstance);
}

VKAPI_ATTR VkResult VKAPI_CALL
wsi_layer_vkCreateDevice(VkPhysicalDevice physicalDevice, const VkDeviceCreateInfo *pCreateInfo,
                         const VkAllocationCallbacks *pAllocator, VkDevice *pDevice) {
    return layer::create_device(physicalDevice, pCreateInfo, pAllocator, pDevice);
}

VKAPI_ATTR VkResult VKAPI_CALL wsi_layer_vkEnumerateDeviceExtensionProperties(
    VkPhysicalDevice physicalDevice, const char *pLayerName, uint32_t *pCount,
    VkExtensionProperties *pProperties) {
    if (pLayerName && !strcmp(pLayerName, layer::global_layer.layerName))
        return layer::extension_properties(1, layer::device_extension, pCount, pProperties);

    assert(physicalDevice);
    return layer::instance_private_data::get(physicalDevice)
        .disp.EnumerateDeviceExtensionProperties(physicalDevice, pLayerName, pCount, pProperties);
}

VKAPI_ATTR VkResult VKAPI_CALL wsi_layer_vkEnumerateInstanceExtensionProperties(
    const char *pLayerName, uint32_t *pCount, VkExtensionProperties *pProperties) {
    if (pLayerName && !strcmp(pLayerName, layer::global_layer.layerName))
        return layer::extension_properties(1, layer::instance_extension, pCount, pProperties);

    return VK_ERROR_LAYER_NOT_PRESENT;
}

VKAPI_ATTR VkResult VKAPI_CALL
wsi_layer_vkEnumerateInstanceLayerProperties(uint32_t *pCount, VkLayerProperties *pProperties) {
    return layer::layer_properties(1, &layer::global_layer, pCount, pProperties);
}

#define GET_PROC_ADDR(func)                                                                        \
    if (!strcmp(funcName, #func))                                                                  \
        return (PFN_vkVoidFunction)&wsi_layer_##func;


PFN_vkVoidFunction VKAPI_CALL wsi_layer_vkGetDeviceProcAddr(VkDevice device,
                                                                            const char *funcName) {
    GET_PROC_ADDR(vkCreateSwapchainKHR);
    GET_PROC_ADDR(vkDestroySwapchainKHR);
    GET_PROC_ADDR(vkGetSwapchainImagesKHR);
    GET_PROC_ADDR(vkAcquireNextImageKHR);
    GET_PROC_ADDR(vkQueuePresentKHR);
    GET_PROC_ADDR(vkGetSwapchainCounterEXT);
    GET_PROC_ADDR(vkRegisterDisplayEventEXT);
    GET_PROC_ADDR(vkDestroyFence);
    GET_PROC_ADDR(vkWaitForFences);
    GET_PROC_ADDR(vkGetFenceStatus);

    return layer::device_private_data::get(device).disp.GetDeviceProcAddr(device, funcName);
}

VKAPI_ATTR PFN_vkVoidFunction VKAPI_CALL
wsi_layer_vkGetInstanceProcAddr(VkInstance instance, const char *funcName) {
    GET_PROC_ADDR(vkGetDeviceProcAddr);
    GET_PROC_ADDR(vkGetInstanceProcAddr);
    GET_PROC_ADDR(vkCreateInstance);
    GET_PROC_ADDR(vkDestroyInstance);
    GET_PROC_ADDR(vkCreateDevice);
    GET_PROC_ADDR(vkDestroyDevice);
    GET_PROC_ADDR(vkGetPhysicalDeviceSurfaceSupportKHR);
    GET_PROC_ADDR(vkGetPhysicalDeviceSurfaceCapabilitiesKHR);
    GET_PROC_ADDR(vkGetPhysicalDeviceSurfaceFormatsKHR);
    GET_PROC_ADDR(vkGetPhysicalDeviceSurfacePresentModesKHR);
    GET_PROC_ADDR(vkEnumerateDeviceExtensionProperties);
    GET_PROC_ADDR(vkEnumerateInstanceExtensionProperties);
    GET_PROC_ADDR(vkEnumerateInstanceLayerProperties);

    GET_PROC_ADDR(vkGetPhysicalDeviceDisplayPropertiesKHR);
    GET_PROC_ADDR(vkGetDisplayModePropertiesKHR);
    GET_PROC_ADDR(vkGetPhysicalDeviceDisplayPlanePropertiesKHR);
    GET_PROC_ADDR(vkAcquireXlibDisplayEXT);
    GET_PROC_ADDR(vkGetDrmDisplayEXT);
    GET_PROC_ADDR(vkAcquireDrmDisplayEXT);
    GET_PROC_ADDR(vkGetDisplayPlaneSupportedDisplaysKHR);
    GET_PROC_ADDR(vkCreateDisplayPlaneSurfaceKHR);
    GET_PROC_ADDR(vkCreateDisplayModeKHR);
    GET_PROC_ADDR(vkReleaseDisplayEXT);

    return layer::instance_private_data::get(instance).disp.GetInstanceProcAddr(instance, funcName);
}

} /* namespace layer */

const char *g_sessionPath;

VKAPI_ATTR VkResult VKAPI_CALL wsi_layer_Negotiate(VkNegotiateLayerInterface *nli)
{
    if (nli->loaderLayerInterfaceVersion < 2)
        return VK_ERROR_INITIALIZATION_FAILED;

    nli->loaderLayerInterfaceVersion = 2;
    nli->pfnGetInstanceProcAddr = layer::wsi_layer_vkGetInstanceProcAddr;
    nli->pfnGetDeviceProcAddr = layer::wsi_layer_vkGetDeviceProcAddr;

    return VK_SUCCESS;
}
