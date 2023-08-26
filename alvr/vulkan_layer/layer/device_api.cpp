#include "device_api.hpp"
#include "private_data.hpp"
#include "wsi/display.hpp"
#include "settings.h"

#include <vector>

static const char *alvr_display_name = "ALVR display";

const struct {
} alvr_display;
const VkDisplayKHR alvr_display_handle = (VkDisplayKHR_T *)&alvr_display;

const struct {
} alvr_display_mode;
const VkDisplayModeKHR alvr_display_mode_handle = (VkDisplayModeKHR_T *)&alvr_display_mode;

extern "C" {

VKAPI_ATTR VkResult VKAPI_CALL wsi_layer_vkGetPhysicalDeviceDisplayPropertiesKHR(
    VkPhysicalDevice device, uint32_t *pPropertyCount, VkDisplayPropertiesKHR *pProperties) {
    if (!pProperties) {
        *pPropertyCount = 1;
        return VK_SUCCESS;
    }
    if (*pPropertyCount < 1) {
        return VK_INCOMPLETE;
    }
    pProperties[0].display = alvr_display_handle;
    pProperties[0].displayName = alvr_display_name;
    pProperties[0].physicalDimensions = VkExtent2D{20, 20};
    pProperties[0].physicalResolution = VkExtent2D{Settings::Instance().m_renderWidth, Settings::Instance().m_renderHeight};
    pProperties[0].supportedTransforms = VK_SURFACE_TRANSFORM_IDENTITY_BIT_KHR;
    pProperties[0].planeReorderPossible = VK_FALSE;
    pProperties[0].persistentContent = VK_TRUE;
    return VK_SUCCESS;
}

VKAPI_ATTR VkResult VKAPI_CALL wsi_layer_vkGetDisplayModePropertiesKHR(
    VkPhysicalDevice device, VkDisplayKHR display, uint32_t *pPropertyCount,
    VkDisplayModePropertiesKHR *pProperties) {
    if (display != alvr_display_handle) {
        *pPropertyCount = 0;
        return VK_ERROR_OUT_OF_HOST_MEMORY;
    }
    if (!pProperties) {
        *pPropertyCount = 1;
        return VK_SUCCESS;
    }
    if (*pPropertyCount < 1) {
        return VK_INCOMPLETE;
    }
    pProperties[0].displayMode = alvr_display_mode_handle;
    pProperties[0].parameters.visibleRegion = VkExtent2D{Settings::Instance().m_renderWidth, Settings::Instance().m_renderHeight};
    pProperties[0].parameters.refreshRate = Settings::Instance().m_refreshRate * 1000;
    return VK_SUCCESS;
}

VKAPI_ATTR VkResult VKAPI_CALL wsi_layer_vkGetPhysicalDeviceDisplayPlanePropertiesKHR(
    VkPhysicalDevice device, uint32_t *pPropertyCount, VkDisplayPlanePropertiesKHR *pProperties) {
    if (!pProperties) {
        *pPropertyCount = 1;
        return VK_SUCCESS;
    }
    if (*pPropertyCount < 1) {
        return VK_INCOMPLETE;
    }
    pProperties[0].currentDisplay = alvr_display_handle;
    pProperties[0].currentStackIndex = 0;
    return VK_SUCCESS;
}

VKAPI_ATTR VkResult VKAPI_CALL wsi_layer_vkAcquireXlibDisplayEXT(VkPhysicalDevice device,
                                                                 Display *dpy,
                                                                 VkDisplayKHR display) {
    if (display != alvr_display_handle) {
        return VK_ERROR_OUT_OF_HOST_MEMORY;
    }
    return VK_SUCCESS;
}

VKAPI_ATTR VkResult VKAPI_CALL wsi_layer_vkGetDrmDisplayEXT(VkPhysicalDevice physicalDevice,
                                                            int32_t drmFd,
                                                            uint32_t connectorId,
                                                            VkDisplayKHR *display) {
    *display = alvr_display_handle;
    return VK_SUCCESS;
}

VKAPI_ATTR VkResult VKAPI_CALL wsi_layer_vkAcquireDrmDisplayEXT(VkPhysicalDevice physicalDevice,
                                                                int32_t drmFd,
                                                                VkDisplayKHR display) {
    if (display != alvr_display_handle) {
        return VK_ERROR_INITIALIZATION_FAILED;
    }
    return VK_SUCCESS;
}

VKAPI_ATTR VkResult VKAPI_CALL wsi_layer_vkGetDisplayPlaneSupportedDisplaysKHR(
    VkPhysicalDevice physicalDevice, uint32_t planeIndex, uint32_t *pDisplayCount,
    VkDisplayKHR *pDisplays) {
    if (planeIndex != 0) {
        return VK_ERROR_OUT_OF_HOST_MEMORY;
    }
    if (!pDisplays) {
        *pDisplayCount = 1;
        return VK_SUCCESS;
    }
    pDisplays[0] = alvr_display_handle;
    return VK_SUCCESS;
}

VKAPI_ATTR VkResult VKAPI_CALL wsi_layer_vkCreateDisplayPlaneSurfaceKHR(
    VkInstance vkinstance, const VkDisplaySurfaceCreateInfoKHR * /*pCreateInfo*/,
    const VkAllocationCallbacks *pAllocator, VkSurfaceKHR *pSurface) {
    auto &instance = layer::instance_private_data::get(vkinstance);
    VkHeadlessSurfaceCreateInfoEXT createInfo = {};
    createInfo.sType = VK_STRUCTURE_TYPE_HEADLESS_SURFACE_CREATE_INFO_EXT;
    auto res =
        instance.disp.CreateHeadlessSurfaceEXT(vkinstance, &createInfo, pAllocator, pSurface);
    if (*pSurface == NULL)
        std::abort();
    instance.add_surface(*pSurface);
    return res;
}

VKAPI_ATTR VkResult VKAPI_CALL wsi_layer_vkReleaseDisplayEXT(VkPhysicalDevice physicalDevice,
                                                             VkDisplayKHR display) {
    return VK_SUCCESS;
}

VKAPI_ATTR void VKAPI_CALL wsi_layer_vkDestroySurfaceKHR(VkInstance vkinstance,
                                                         VkSurfaceKHR surface,
                                                         const VkAllocationCallbacks *pAllocator) {
    auto &instance = layer::instance_private_data::get(vkinstance);
    if (instance.should_layer_handle_surface(surface)) {
        return;
    }
    return instance.disp.DestroySurfaceKHR(vkinstance, surface, pAllocator);
}

VKAPI_ATTR VkResult VKAPI_CALL wsi_layer_vkRegisterDisplayEventEXT(
    VkDevice device, VkDisplayKHR display, const VkDisplayEventInfoEXT *pDisplayEventInfo,
    const VkAllocationCallbacks *pAllocator, VkFence *pFence) {
    if (display != alvr_display_handle) {
        return VK_ERROR_OUT_OF_HOST_MEMORY;
    }

    auto &instance = layer::device_private_data::get(device);
    *pFence = instance.display->get_vsync_fence();

    return VK_SUCCESS;
}

VKAPI_ATTR void VKAPI_CALL wsi_layer_vkDestroyFence(VkDevice device, VkFence fence,
                                                    const VkAllocationCallbacks *pAllocator) {
    auto &instance = layer::device_private_data::get(device);
    auto alvr_fence = instance.display->peek_vsync_fence();
    if (fence == alvr_fence) {
        return;
    }
    instance.disp.DestroyFence(device, fence, pAllocator);
}

VKAPI_ATTR VkResult VKAPI_CALL wsi_layer_vkWaitForFences(VkDevice device, uint32_t fenceCount,
                                                         const VkFence *pFences, VkBool32 waitAll,
                                                         uint64_t timeout) {
    auto &instance = layer::device_private_data::get(device);
    auto alvr_fence = instance.display->peek_vsync_fence();
    for (uint32_t i = 0; i < fenceCount; ++i) {
        if (pFences[i] == alvr_fence) {
            assert(fenceCount == 1); // only our fence
            return instance.display->wait_for_vsync(timeout) ? VK_SUCCESS : VK_TIMEOUT;
        }
    }
    return instance.disp.WaitForFences(device, fenceCount, pFences, waitAll, timeout);
}

VKAPI_ATTR VkResult wsi_layer_vkGetFenceStatus(VkDevice device, VkFence fence)
{
    auto &instance = layer::device_private_data::get(device);
    auto alvr_fence = instance.display->peek_vsync_fence();
    if (fence == alvr_fence) {
        return instance.display->is_signaled() ? VK_SUCCESS : VK_NOT_READY;
    }
    return instance.disp.GetFenceStatus(device, fence);
}

VKAPI_ATTR VkResult VKAPI_CALL wsi_layer_vkCreateDisplayModeKHR(
    VkPhysicalDevice                            physicalDevice,
    VkDisplayKHR                                display,
    const VkDisplayModeCreateInfoKHR*           pCreateInfo,
    const VkAllocationCallbacks*                pAllocator,
    VkDisplayModeKHR*                           pMode)
{
  return VK_ERROR_INITIALIZATION_FAILED;
}

}
