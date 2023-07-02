#pragma once

#include <vulkan/vk_layer.h>

extern "C" const char *g_sessionPath;

extern "C" VKAPI_ATTR VkResult VKAPI_CALL wsi_layer_Negotiate(VkNegotiateLayerInterface *nli);
