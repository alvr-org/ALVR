#pragma once

#include <vulkan/vk_layer.h>

#ifdef __cplusplus
extern "C" {
#endif

VK_LAYER_EXPORT PFN_vkVoidFunction VKAPI_CALL wsi_layer_vkGetDeviceProcAddr(VkDevice device, const char *funcName);
VK_LAYER_EXPORT VKAPI_ATTR PFN_vkVoidFunction VKAPI_CALL wsi_layer_vkGetInstanceProcAddr(VkInstance instance, const char *funcName);

#ifdef __cplusplus
}
#endif
