// 
// Notice Regarding Standards.  AMD does not provide a license or sublicense to
// any Intellectual Property Rights relating to any standards, including but not
// limited to any audio and/or video codec technologies such as MPEG-2, MPEG-4;
// AVC/H.264; HEVC/H.265; AAC decode/FFMPEG; AAC encode/FFMPEG; VC-1; and MP3
// (collectively, the "Media Technologies"). For clarity, you will pay any
// royalties due for such third party technologies, which may include the Media
// Technologies that are owed as a result of AMD providing the Software to you.
// 
// MIT license 
// 
// Copyright (c) 2018 Advanced Micro Devices, Inc. All rights reserved.
//
// Permission is hereby granted, free of charge, to any person obtaining a copy
// of this software and associated documentation files (the "Software"), to deal
// in the Software without restriction, including without limitation the rights
// to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
// copies of the Software, and to permit persons to whom the Software is
// furnished to do so, subject to the following conditions:
//
// The above copyright notice and this permission notice shall be included in
// all copies or substantial portions of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
// IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT.  IN NO EVENT SHALL THE
// AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
// LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
// OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN
// THE SOFTWARE.
//
///-------------------------------------------------------------------------
///  @file   VulkanImportTable.cpp
///  @brief  Vulkan import table
///-------------------------------------------------------------------------
#include "VulkanImportTable.h"
#include "../common/TraceAdapter.h"
#include "Thread.h"

using namespace amf;

//-------------------------------------------------------------------------------------------------

//
#define GET_DLL_ENTRYPOINT(h, w) w = reinterpret_cast<PFN_##w>(amf_get_proc_address(h, #w)); if(w==nullptr) \
	{ AMFTraceError(L"VulkanImportTable", L"Failed to aquire entrypoint " + *#w); return AMF_FAIL; };
#define GET_INSTANCE_ENTRYPOINT(i, w) w = reinterpret_cast<PFN_##w>(vkGetInstanceProcAddr(i, #w)); if(w==nullptr) \
	{ AMFTraceError(L"VulkanImportTable", L"Failed to aquire entrypoint " + *#w); return AMF_FAIL; };
#define GET_INSTANCE_ENTRYPOINT_NORETURN(i, w) w = reinterpret_cast<PFN_##w>(vkGetInstanceProcAddr(i, #w));
#define GET_DEVICE_ENTRYPOINT(i, w) w = reinterpret_cast<PFN_##w>(vkGetDeviceProcAddr(i, #w)); if(w==nullptr) \
	{ AMFTraceError(L"VulkanImportTable", L"Failed to aquire entrypoint " + *#w); return AMF_FAIL; };

VulkanImportTable::VulkanImportTable() :
	m_hVulkanDll(nullptr),
	vkCreateInstance(nullptr),
	vkDestroyInstance(nullptr),
	vkEnumeratePhysicalDevices(nullptr),
	vkGetPhysicalDeviceFeatures(nullptr),
	vkGetPhysicalDeviceFormatProperties(nullptr),
	vkGetPhysicalDeviceImageFormatProperties(nullptr),
	vkGetPhysicalDeviceProperties(nullptr),
	vkGetPhysicalDeviceQueueFamilyProperties(nullptr),
	vkGetPhysicalDeviceMemoryProperties(nullptr),
	vkGetInstanceProcAddr(nullptr),
	vkGetDeviceProcAddr(nullptr),
	vkCreateDevice(nullptr),
	vkDestroyDevice(nullptr),
	vkEnumerateInstanceExtensionProperties(nullptr),
	vkEnumerateDeviceExtensionProperties(nullptr),
	vkEnumerateInstanceLayerProperties(nullptr),
	vkEnumerateDeviceLayerProperties(nullptr),
	vkGetDeviceQueue(nullptr),
	vkQueueSubmit(nullptr),
	vkQueueWaitIdle(nullptr),
	vkDeviceWaitIdle(nullptr),
	vkAllocateMemory(nullptr),
	vkFreeMemory(nullptr),
	vkMapMemory(nullptr),
	vkUnmapMemory(nullptr),
	vkFlushMappedMemoryRanges(nullptr),
	vkInvalidateMappedMemoryRanges(nullptr),
	vkGetDeviceMemoryCommitment(nullptr),
	vkBindBufferMemory(nullptr),
	vkBindImageMemory(nullptr),
	vkGetBufferMemoryRequirements(nullptr),
	vkGetImageMemoryRequirements(nullptr),
	vkGetImageSparseMemoryRequirements(nullptr),
	vkGetPhysicalDeviceSparseImageFormatProperties(nullptr),
	vkQueueBindSparse(nullptr),
	vkCreateFence(nullptr),
	vkDestroyFence(nullptr),
	vkResetFences(nullptr),
	vkGetFenceStatus(nullptr),
	vkWaitForFences(nullptr),
	vkCreateSemaphore(nullptr),
	vkDestroySemaphore(nullptr),
	vkCreateEvent(nullptr),
	vkDestroyEvent(nullptr),
	vkGetEventStatus(nullptr),
	vkSetEvent(nullptr),
	vkResetEvent(nullptr),
	vkCreateQueryPool(nullptr),
	vkDestroyQueryPool(nullptr),
	vkGetQueryPoolResults(nullptr),
	vkCreateBuffer(nullptr),
	vkDestroyBuffer(nullptr),
	vkCreateBufferView(nullptr),
	vkDestroyBufferView(nullptr),
	vkCreateImage(nullptr),
	vkDestroyImage(nullptr),
	vkGetImageSubresourceLayout(nullptr),
	vkCreateImageView(nullptr),
	vkDestroyImageView(nullptr),
	vkCreateShaderModule(nullptr),
	vkDestroyShaderModule(nullptr),
	vkCreatePipelineCache(nullptr),
	vkDestroyPipelineCache(nullptr),
	vkGetPipelineCacheData(nullptr),
	vkMergePipelineCaches(nullptr),
	vkCreateGraphicsPipelines(nullptr),
	vkCreateComputePipelines(nullptr),
	vkDestroyPipeline(nullptr),
	vkCreatePipelineLayout(nullptr),
	vkDestroyPipelineLayout(nullptr),
	vkCreateSampler(nullptr),
	vkDestroySampler(nullptr),
	vkCreateDescriptorSetLayout(nullptr),
	vkDestroyDescriptorSetLayout(nullptr),
	vkCreateDescriptorPool(nullptr),
	vkDestroyDescriptorPool(nullptr),
	vkResetDescriptorPool(nullptr),
	vkAllocateDescriptorSets(nullptr),
	vkFreeDescriptorSets(nullptr),
	vkUpdateDescriptorSets(nullptr),
	vkCreateFramebuffer(nullptr),
	vkDestroyFramebuffer(nullptr),
	vkCreateRenderPass(nullptr),
	vkDestroyRenderPass(nullptr),
	vkGetRenderAreaGranularity(nullptr),
	vkCreateCommandPool(nullptr),
	vkDestroyCommandPool(nullptr),
	vkResetCommandPool(nullptr),
	vkAllocateCommandBuffers(nullptr),
	vkFreeCommandBuffers(nullptr),
	vkBeginCommandBuffer(nullptr),
	vkEndCommandBuffer(nullptr),
	vkResetCommandBuffer(nullptr),
	vkCmdBindPipeline(nullptr),
	vkCmdSetViewport(nullptr),
	vkCmdSetScissor(nullptr),
	vkCmdSetLineWidth(nullptr),
	vkCmdSetDepthBias(nullptr),
	vkCmdSetBlendConstants(nullptr),
	vkCmdSetDepthBounds(nullptr),
	vkCmdSetStencilCompareMask(nullptr),
	vkCmdSetStencilWriteMask(nullptr),
	vkCmdSetStencilReference(nullptr),
	vkCmdBindDescriptorSets(nullptr),
	vkCmdBindIndexBuffer(nullptr),
	vkCmdBindVertexBuffers(nullptr),
	vkCmdDraw(nullptr),
	vkCmdDrawIndexed(nullptr),
	vkCmdDrawIndirect(nullptr),
	vkCmdDrawIndexedIndirect(nullptr),
	vkCmdDispatch(nullptr),
	vkCmdDispatchIndirect(nullptr),
	vkCmdCopyBuffer(nullptr),
	vkCmdCopyImage(nullptr),
	vkCmdBlitImage(nullptr),
	vkCmdCopyBufferToImage(nullptr),
	vkCmdCopyImageToBuffer(nullptr),
	vkCmdUpdateBuffer(nullptr),
	vkCmdFillBuffer(nullptr),
	vkCmdClearColorImage(nullptr),
	vkCmdClearDepthStencilImage(nullptr),
	vkCmdClearAttachments(nullptr),
	vkCmdResolveImage(nullptr),
	vkCmdSetEvent(nullptr),
	vkCmdResetEvent(nullptr),
	vkCmdWaitEvents(nullptr),
	vkCmdPipelineBarrier(nullptr),
	vkCmdBeginQuery(nullptr),
	vkCmdEndQuery(nullptr),
	vkCmdResetQueryPool(nullptr),
	vkCmdWriteTimestamp(nullptr),
	vkCmdCopyQueryPoolResults(nullptr),
	vkCmdPushConstants(nullptr),
	vkCmdBeginRenderPass(nullptr),
	vkCmdNextSubpass(nullptr),
	vkCmdEndRenderPass(nullptr),
	vkCmdExecuteCommands(nullptr),
	vkDestroySurfaceKHR(nullptr),
	vkGetPhysicalDeviceSurfaceSupportKHR(nullptr),
	vkGetPhysicalDeviceSurfaceCapabilitiesKHR(nullptr),
	vkGetPhysicalDeviceSurfaceFormatsKHR(nullptr),
	vkGetPhysicalDeviceSurfacePresentModesKHR(nullptr),
	vkCreateSwapchainKHR(nullptr),
	vkDestroySwapchainKHR(nullptr),
	vkGetSwapchainImagesKHR(nullptr),
	vkAcquireNextImageKHR(nullptr),
	vkQueuePresentKHR(nullptr),
#if defined(_WIN32)
	vkCreateWin32SurfaceKHR(nullptr),
#endif
	vkCreateDebugReportCallbackEXT(nullptr),
	vkDebugReportMessageEXT(nullptr),
	vkDestroyDebugReportCallbackEXT(nullptr)
{
}

VulkanImportTable::~VulkanImportTable()
{
	if (m_hVulkanDll != nullptr)
	{
		amf_free_library(m_hVulkanDll);
	}
	m_hVulkanDll = nullptr;
}

AMF_RESULT VulkanImportTable::LoadFunctionsTable()
{
	if (m_hVulkanDll != nullptr)
	{
		return AMF_OK;
	}
#if defined(_WIN32)
	m_hVulkanDll = amf_load_library(L"vulkan-1.dll");
#elif defined(__linux__)
	m_hVulkanDll = amf_load_library(L"libvulkan.so.1");
#endif

	if (m_hVulkanDll == nullptr)
	{
		AMFTraceError(L"VulkanImportTable", L"amf_load_library() failed to load vulkan dll!");
		return AMF_FAIL;
	}
	GET_DLL_ENTRYPOINT(m_hVulkanDll, vkCreateInstance);
	GET_DLL_ENTRYPOINT(m_hVulkanDll, vkCreateInstance);
	GET_DLL_ENTRYPOINT(m_hVulkanDll, vkDestroyInstance);
	GET_DLL_ENTRYPOINT(m_hVulkanDll, vkEnumeratePhysicalDevices);
	GET_DLL_ENTRYPOINT(m_hVulkanDll, vkGetPhysicalDeviceFeatures);
	GET_DLL_ENTRYPOINT(m_hVulkanDll, vkGetPhysicalDeviceFormatProperties);
	GET_DLL_ENTRYPOINT(m_hVulkanDll, vkGetPhysicalDeviceImageFormatProperties);
	GET_DLL_ENTRYPOINT(m_hVulkanDll, vkGetPhysicalDeviceProperties);
	GET_DLL_ENTRYPOINT(m_hVulkanDll, vkGetPhysicalDeviceQueueFamilyProperties);
	GET_DLL_ENTRYPOINT(m_hVulkanDll, vkGetPhysicalDeviceMemoryProperties);
	GET_DLL_ENTRYPOINT(m_hVulkanDll, vkGetInstanceProcAddr);
	GET_DLL_ENTRYPOINT(m_hVulkanDll, vkGetDeviceProcAddr);
	GET_DLL_ENTRYPOINT(m_hVulkanDll, vkCreateDevice);
	GET_DLL_ENTRYPOINT(m_hVulkanDll, vkDestroyDevice);
	GET_DLL_ENTRYPOINT(m_hVulkanDll, vkEnumerateInstanceExtensionProperties);
	GET_DLL_ENTRYPOINT(m_hVulkanDll, vkEnumerateDeviceExtensionProperties);
	GET_DLL_ENTRYPOINT(m_hVulkanDll, vkEnumerateInstanceLayerProperties);
	GET_DLL_ENTRYPOINT(m_hVulkanDll, vkEnumerateDeviceLayerProperties);
	GET_DLL_ENTRYPOINT(m_hVulkanDll, vkGetDeviceQueue);
	GET_DLL_ENTRYPOINT(m_hVulkanDll, vkQueueSubmit);
	GET_DLL_ENTRYPOINT(m_hVulkanDll, vkQueueWaitIdle);
	GET_DLL_ENTRYPOINT(m_hVulkanDll, vkDeviceWaitIdle);
	GET_DLL_ENTRYPOINT(m_hVulkanDll, vkAllocateMemory);
	GET_DLL_ENTRYPOINT(m_hVulkanDll, vkFreeMemory);
	GET_DLL_ENTRYPOINT(m_hVulkanDll, vkMapMemory);
	GET_DLL_ENTRYPOINT(m_hVulkanDll, vkUnmapMemory);
	GET_DLL_ENTRYPOINT(m_hVulkanDll, vkFlushMappedMemoryRanges);
	GET_DLL_ENTRYPOINT(m_hVulkanDll, vkInvalidateMappedMemoryRanges);
	GET_DLL_ENTRYPOINT(m_hVulkanDll, vkGetDeviceMemoryCommitment);
	GET_DLL_ENTRYPOINT(m_hVulkanDll, vkBindBufferMemory);
	GET_DLL_ENTRYPOINT(m_hVulkanDll, vkBindImageMemory);
	GET_DLL_ENTRYPOINT(m_hVulkanDll, vkGetBufferMemoryRequirements);
	GET_DLL_ENTRYPOINT(m_hVulkanDll, vkGetImageMemoryRequirements);
	GET_DLL_ENTRYPOINT(m_hVulkanDll, vkGetImageSparseMemoryRequirements);
	GET_DLL_ENTRYPOINT(m_hVulkanDll, vkGetPhysicalDeviceSparseImageFormatProperties);
	GET_DLL_ENTRYPOINT(m_hVulkanDll, vkQueueBindSparse);
	GET_DLL_ENTRYPOINT(m_hVulkanDll, vkCreateFence);
	GET_DLL_ENTRYPOINT(m_hVulkanDll, vkDestroyFence);
	GET_DLL_ENTRYPOINT(m_hVulkanDll, vkResetFences);
	GET_DLL_ENTRYPOINT(m_hVulkanDll, vkGetFenceStatus);
	GET_DLL_ENTRYPOINT(m_hVulkanDll, vkWaitForFences);
	GET_DLL_ENTRYPOINT(m_hVulkanDll, vkCreateSemaphore);
	GET_DLL_ENTRYPOINT(m_hVulkanDll, vkDestroySemaphore);
	GET_DLL_ENTRYPOINT(m_hVulkanDll, vkCreateEvent);
	GET_DLL_ENTRYPOINT(m_hVulkanDll, vkDestroyEvent);
	GET_DLL_ENTRYPOINT(m_hVulkanDll, vkGetEventStatus);
	GET_DLL_ENTRYPOINT(m_hVulkanDll, vkSetEvent);
	GET_DLL_ENTRYPOINT(m_hVulkanDll, vkResetEvent);
	GET_DLL_ENTRYPOINT(m_hVulkanDll, vkCreateQueryPool);
	GET_DLL_ENTRYPOINT(m_hVulkanDll, vkDestroyQueryPool);
	GET_DLL_ENTRYPOINT(m_hVulkanDll, vkGetQueryPoolResults);
	GET_DLL_ENTRYPOINT(m_hVulkanDll, vkCreateBuffer);
	GET_DLL_ENTRYPOINT(m_hVulkanDll, vkDestroyBuffer);
	GET_DLL_ENTRYPOINT(m_hVulkanDll, vkCreateBufferView);
	GET_DLL_ENTRYPOINT(m_hVulkanDll, vkDestroyBufferView);
	GET_DLL_ENTRYPOINT(m_hVulkanDll, vkCreateImage);
	GET_DLL_ENTRYPOINT(m_hVulkanDll, vkDestroyImage);
	GET_DLL_ENTRYPOINT(m_hVulkanDll, vkGetImageSubresourceLayout);
	GET_DLL_ENTRYPOINT(m_hVulkanDll, vkCreateImageView);
	GET_DLL_ENTRYPOINT(m_hVulkanDll, vkDestroyImageView);
	GET_DLL_ENTRYPOINT(m_hVulkanDll, vkCreateShaderModule);
	GET_DLL_ENTRYPOINT(m_hVulkanDll, vkDestroyShaderModule);
	GET_DLL_ENTRYPOINT(m_hVulkanDll, vkCreatePipelineCache);
	GET_DLL_ENTRYPOINT(m_hVulkanDll, vkDestroyPipelineCache);
	GET_DLL_ENTRYPOINT(m_hVulkanDll, vkGetPipelineCacheData);
	GET_DLL_ENTRYPOINT(m_hVulkanDll, vkMergePipelineCaches);
	GET_DLL_ENTRYPOINT(m_hVulkanDll, vkCreateGraphicsPipelines);
	GET_DLL_ENTRYPOINT(m_hVulkanDll, vkCreateComputePipelines);
	GET_DLL_ENTRYPOINT(m_hVulkanDll, vkDestroyPipeline);
	GET_DLL_ENTRYPOINT(m_hVulkanDll, vkCreatePipelineLayout);
	GET_DLL_ENTRYPOINT(m_hVulkanDll, vkDestroyPipelineLayout);
	GET_DLL_ENTRYPOINT(m_hVulkanDll, vkCreateSampler);
	GET_DLL_ENTRYPOINT(m_hVulkanDll, vkDestroySampler);
	GET_DLL_ENTRYPOINT(m_hVulkanDll, vkCreateDescriptorSetLayout);
	GET_DLL_ENTRYPOINT(m_hVulkanDll, vkDestroyDescriptorSetLayout);
	GET_DLL_ENTRYPOINT(m_hVulkanDll, vkCreateDescriptorPool);
	GET_DLL_ENTRYPOINT(m_hVulkanDll, vkDestroyDescriptorPool);
	GET_DLL_ENTRYPOINT(m_hVulkanDll, vkResetDescriptorPool);
	GET_DLL_ENTRYPOINT(m_hVulkanDll, vkAllocateDescriptorSets);
	GET_DLL_ENTRYPOINT(m_hVulkanDll, vkFreeDescriptorSets);
	GET_DLL_ENTRYPOINT(m_hVulkanDll, vkUpdateDescriptorSets);
	GET_DLL_ENTRYPOINT(m_hVulkanDll, vkCreateFramebuffer);
	GET_DLL_ENTRYPOINT(m_hVulkanDll, vkDestroyFramebuffer);
	GET_DLL_ENTRYPOINT(m_hVulkanDll, vkCreateRenderPass);
	GET_DLL_ENTRYPOINT(m_hVulkanDll, vkDestroyRenderPass);
	GET_DLL_ENTRYPOINT(m_hVulkanDll, vkGetRenderAreaGranularity);
	GET_DLL_ENTRYPOINT(m_hVulkanDll, vkCreateCommandPool);
	GET_DLL_ENTRYPOINT(m_hVulkanDll, vkDestroyCommandPool);
	GET_DLL_ENTRYPOINT(m_hVulkanDll, vkResetCommandPool);
	GET_DLL_ENTRYPOINT(m_hVulkanDll, vkAllocateCommandBuffers);
	GET_DLL_ENTRYPOINT(m_hVulkanDll, vkFreeCommandBuffers);
	GET_DLL_ENTRYPOINT(m_hVulkanDll, vkBeginCommandBuffer);
	GET_DLL_ENTRYPOINT(m_hVulkanDll, vkEndCommandBuffer);
	GET_DLL_ENTRYPOINT(m_hVulkanDll, vkResetCommandBuffer);
	GET_DLL_ENTRYPOINT(m_hVulkanDll, vkCmdBindPipeline);
	GET_DLL_ENTRYPOINT(m_hVulkanDll, vkCmdSetViewport);
	GET_DLL_ENTRYPOINT(m_hVulkanDll, vkCmdSetScissor);
	GET_DLL_ENTRYPOINT(m_hVulkanDll, vkCmdSetLineWidth);
	GET_DLL_ENTRYPOINT(m_hVulkanDll, vkCmdSetDepthBias);
	GET_DLL_ENTRYPOINT(m_hVulkanDll, vkCmdSetBlendConstants);
	GET_DLL_ENTRYPOINT(m_hVulkanDll, vkCmdSetDepthBounds);
	GET_DLL_ENTRYPOINT(m_hVulkanDll, vkCmdSetStencilCompareMask);
	GET_DLL_ENTRYPOINT(m_hVulkanDll, vkCmdSetStencilWriteMask);
	GET_DLL_ENTRYPOINT(m_hVulkanDll, vkCmdSetStencilReference);
	GET_DLL_ENTRYPOINT(m_hVulkanDll, vkCmdBindDescriptorSets);
	GET_DLL_ENTRYPOINT(m_hVulkanDll, vkCmdBindIndexBuffer);
	GET_DLL_ENTRYPOINT(m_hVulkanDll, vkCmdBindVertexBuffers);
	GET_DLL_ENTRYPOINT(m_hVulkanDll, vkCmdDraw);
	GET_DLL_ENTRYPOINT(m_hVulkanDll, vkCmdDrawIndexed);
	GET_DLL_ENTRYPOINT(m_hVulkanDll, vkCmdDrawIndirect);
	GET_DLL_ENTRYPOINT(m_hVulkanDll, vkCmdDrawIndexedIndirect);
	GET_DLL_ENTRYPOINT(m_hVulkanDll, vkCmdDispatch);
	GET_DLL_ENTRYPOINT(m_hVulkanDll, vkCmdDispatchIndirect);
	GET_DLL_ENTRYPOINT(m_hVulkanDll, vkCmdCopyBuffer);
	GET_DLL_ENTRYPOINT(m_hVulkanDll, vkCmdCopyImage);
	GET_DLL_ENTRYPOINT(m_hVulkanDll, vkCmdBlitImage);
	GET_DLL_ENTRYPOINT(m_hVulkanDll, vkCmdCopyBufferToImage);
	GET_DLL_ENTRYPOINT(m_hVulkanDll, vkCmdCopyImageToBuffer);
	GET_DLL_ENTRYPOINT(m_hVulkanDll, vkCmdUpdateBuffer);
	GET_DLL_ENTRYPOINT(m_hVulkanDll, vkCmdFillBuffer);
	GET_DLL_ENTRYPOINT(m_hVulkanDll, vkCmdClearColorImage);
	GET_DLL_ENTRYPOINT(m_hVulkanDll, vkCmdClearDepthStencilImage);
	GET_DLL_ENTRYPOINT(m_hVulkanDll, vkCmdClearAttachments);
	GET_DLL_ENTRYPOINT(m_hVulkanDll, vkCmdResolveImage);
	GET_DLL_ENTRYPOINT(m_hVulkanDll, vkCmdSetEvent);
	GET_DLL_ENTRYPOINT(m_hVulkanDll, vkCmdResetEvent);
	GET_DLL_ENTRYPOINT(m_hVulkanDll, vkCmdWaitEvents);
	GET_DLL_ENTRYPOINT(m_hVulkanDll, vkCmdPipelineBarrier);
	GET_DLL_ENTRYPOINT(m_hVulkanDll, vkCmdBeginQuery);
	GET_DLL_ENTRYPOINT(m_hVulkanDll, vkCmdEndQuery);
	GET_DLL_ENTRYPOINT(m_hVulkanDll, vkCmdResetQueryPool);
	GET_DLL_ENTRYPOINT(m_hVulkanDll, vkCmdWriteTimestamp);
	GET_DLL_ENTRYPOINT(m_hVulkanDll, vkCmdCopyQueryPoolResults);
	GET_DLL_ENTRYPOINT(m_hVulkanDll, vkCmdPushConstants);
	GET_DLL_ENTRYPOINT(m_hVulkanDll, vkCmdBeginRenderPass);
	GET_DLL_ENTRYPOINT(m_hVulkanDll, vkCmdNextSubpass);
	GET_DLL_ENTRYPOINT(m_hVulkanDll, vkCmdEndRenderPass);
	GET_DLL_ENTRYPOINT(m_hVulkanDll, vkCmdExecuteCommands);

	GET_DLL_ENTRYPOINT(m_hVulkanDll, vkGetPhysicalDeviceSurfaceSupportKHR);
	GET_DLL_ENTRYPOINT(m_hVulkanDll, vkGetPhysicalDeviceSurfaceCapabilitiesKHR);
	GET_DLL_ENTRYPOINT(m_hVulkanDll, vkGetPhysicalDeviceSurfaceFormatsKHR);
	GET_DLL_ENTRYPOINT(m_hVulkanDll, vkGetPhysicalDeviceSurfacePresentModesKHR);
    GET_DLL_ENTRYPOINT(m_hVulkanDll, vkDestroySurfaceKHR);

#ifdef WIN32
    GET_DLL_ENTRYPOINT(m_hVulkanDll, vkCreateWin32SurfaceKHR);
#else 
    GET_DLL_ENTRYPOINT(m_hVulkanDll, vkCreateXlibSurfaceKHR);
#endif
	return AMF_OK;
}

AMF_RESULT VulkanImportTable::LoadInstanceFunctionsTableExt(VkInstance instance, bool bDebug)
{
    if(bDebug)
    {
    	GET_INSTANCE_ENTRYPOINT(instance, vkCreateDebugReportCallbackEXT);
	    GET_INSTANCE_ENTRYPOINT(instance, vkDebugReportMessageEXT);
	    GET_INSTANCE_ENTRYPOINT(instance, vkDestroyDebugReportCallbackEXT);
    }
	return AMF_OK;
}

//-------------------------------------------------------------------------------------------------
AMF_RESULT VulkanImportTable::LoadDeviceFunctionsTableExt(VkDevice device)
{
	GET_DEVICE_ENTRYPOINT(device, vkCreateSwapchainKHR);
	GET_DEVICE_ENTRYPOINT(device, vkDestroySwapchainKHR);
	GET_DEVICE_ENTRYPOINT(device, vkGetSwapchainImagesKHR);
	GET_DEVICE_ENTRYPOINT(device, vkAcquireNextImageKHR);
	GET_DEVICE_ENTRYPOINT(device, vkQueuePresentKHR);
	return AMF_OK;
}

#undef GET_DEVICE_ENTRYPOINT
#undef GET_INSTANCE_ENTRYPOINT
#undef GET_INSTANCE_ENTRYPOINT_NORETURN