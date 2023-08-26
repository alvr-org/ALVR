#pragma once

#include <vulkan/vulkan.h>

extern "C" {

VKAPI_ATTR VkResult VKAPI_CALL wsi_layer_vkGetPhysicalDeviceDisplayPropertiesKHR(
    VkPhysicalDevice device, uint32_t *pPropertyCount, VkDisplayPropertiesKHR *pProperties);

VKAPI_ATTR VkResult VKAPI_CALL wsi_layer_vkGetDisplayModePropertiesKHR(
    VkPhysicalDevice device, VkDisplayKHR display, uint32_t *pPropertyCount,
    VkDisplayModePropertiesKHR *pProperties);

VKAPI_ATTR VkResult VKAPI_CALL wsi_layer_vkGetPhysicalDeviceDisplayPlanePropertiesKHR(
    VkPhysicalDevice device, uint32_t *pPropertyCount, VkDisplayPlanePropertiesKHR *pProperties);

VKAPI_ATTR VkResult VKAPI_CALL wsi_layer_vkAcquireXlibDisplayEXT(VkPhysicalDevice device,
                                                                 Display *dpy,
                                                                 VkDisplayKHR display);

VKAPI_ATTR VkResult VKAPI_CALL wsi_layer_vkGetDrmDisplayEXT(VkPhysicalDevice physicalDevice,
                                                            int32_t drmFd,
                                                            uint32_t connectorId,
                                                            VkDisplayKHR *display);

VKAPI_ATTR VkResult VKAPI_CALL wsi_layer_vkAcquireDrmDisplayEXT(VkPhysicalDevice physicalDevice,
                                                                int32_t drmFd,
                                                                VkDisplayKHR display);

VKAPI_ATTR VkResult VKAPI_CALL wsi_layer_vkGetDisplayPlaneSupportedDisplaysKHR(
    VkPhysicalDevice physicalDevice, uint32_t planeIndex, uint32_t *pDisplayCount,
    VkDisplayKHR *pDisplays);

VKAPI_ATTR VkResult VKAPI_CALL wsi_layer_vkCreateDisplayPlaneSurfaceKHR(
    VkInstance instance, const VkDisplaySurfaceCreateInfoKHR *pCreateInfo,
    const VkAllocationCallbacks *pAllocator, VkSurfaceKHR *pSurface);

VKAPI_ATTR VkResult VKAPI_CALL wsi_layer_vkReleaseDisplayEXT(VkPhysicalDevice physicalDevice,
                                                             VkDisplayKHR display);

VKAPI_ATTR void VKAPI_CALL wsi_layer_vkDestroySurfaceKHR(VkInstance instance, VkSurfaceKHR surface,
                                                         const VkAllocationCallbacks *pAllocator);

VKAPI_ATTR VkResult VKAPI_CALL wsi_layer_vkRegisterDisplayEventEXT(
    VkDevice device, VkDisplayKHR display, const VkDisplayEventInfoEXT *pDisplayEventInfo,
    const VkAllocationCallbacks *pAllocator, VkFence *pFence);

VKAPI_ATTR void VKAPI_CALL wsi_layer_vkDestroyFence(VkDevice device, VkFence fence,
                                                    const VkAllocationCallbacks *pAllocator);

VKAPI_ATTR VkResult VKAPI_CALL wsi_layer_vkWaitForFences(VkDevice device, uint32_t fenceCount,
                                                         const VkFence *pFences, VkBool32 waitAll,
                                                         uint64_t timeout);

VKAPI_ATTR VkResult wsi_layer_vkGetFenceStatus(VkDevice device, VkFence fence);

VKAPI_ATTR VkResult VKAPI_CALL wsi_layer_vkCreateDisplayModeKHR(
    VkPhysicalDevice                            physicalDevice,
    VkDisplayKHR                                display,
    const VkDisplayModeCreateInfoKHR*           pCreateInfo,
    const VkAllocationCallbacks*                pAllocator,
    VkDisplayModeKHR*                           pMode);
}
