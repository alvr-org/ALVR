#pragma once

#include <vulkan/vk_layer.h>

extern "C" const char *g_sessionPath;

extern "C" VK_LAYER_EXPORT PFN_vkVoidFunction VKAPI_CALL wsi_layer_vkGetDeviceProcAddr(VkDevice device, const char *funcName);
extern "C" VK_LAYER_EXPORT VKAPI_ATTR PFN_vkVoidFunction VKAPI_CALL wsi_layer_vkGetInstanceProcAddr(VkInstance instance, const char *funcName);
