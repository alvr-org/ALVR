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
///  @file   VulkanImportTable.h
///  @brief  Vulkan import table
///-------------------------------------------------------------------------
#pragma once

#include "../include/core/Result.h"

#define VK_NO_PROTOTYPES

#include "../include/core/VulkanAMF.h"

//#define ENABLE_VALIDATION

struct VulkanImportTable
{
	VulkanImportTable();
	~VulkanImportTable();

	AMF_RESULT LoadFunctionsTable();
	AMF_RESULT LoadInstanceFunctionsTableExt(VkInstance instance, bool bDebug);
	AMF_RESULT LoadDeviceFunctionsTableExt(VkDevice device);
	// core Vulkan

	PFN_vkCreateInstance                                    vkCreateInstance;
	PFN_vkDestroyInstance                                   vkDestroyInstance;
	PFN_vkEnumeratePhysicalDevices                          vkEnumeratePhysicalDevices;
	PFN_vkGetPhysicalDeviceFeatures                         vkGetPhysicalDeviceFeatures;
	PFN_vkGetPhysicalDeviceFormatProperties                 vkGetPhysicalDeviceFormatProperties;
	PFN_vkGetPhysicalDeviceImageFormatProperties            vkGetPhysicalDeviceImageFormatProperties;
	PFN_vkGetPhysicalDeviceProperties                       vkGetPhysicalDeviceProperties;
	PFN_vkGetPhysicalDeviceQueueFamilyProperties            vkGetPhysicalDeviceQueueFamilyProperties;
	PFN_vkGetPhysicalDeviceMemoryProperties                 vkGetPhysicalDeviceMemoryProperties;
	PFN_vkGetInstanceProcAddr                               vkGetInstanceProcAddr;
	PFN_vkGetDeviceProcAddr                                 vkGetDeviceProcAddr;
	PFN_vkCreateDevice                                      vkCreateDevice;
	PFN_vkDestroyDevice                                     vkDestroyDevice;
	PFN_vkEnumerateInstanceExtensionProperties              vkEnumerateInstanceExtensionProperties;
	PFN_vkEnumerateDeviceExtensionProperties                vkEnumerateDeviceExtensionProperties;
	PFN_vkEnumerateInstanceLayerProperties                  vkEnumerateInstanceLayerProperties;
	PFN_vkEnumerateDeviceLayerProperties                    vkEnumerateDeviceLayerProperties;
	PFN_vkGetDeviceQueue                                    vkGetDeviceQueue;
	PFN_vkQueueSubmit                                       vkQueueSubmit;
	PFN_vkQueueWaitIdle                                     vkQueueWaitIdle;
	PFN_vkDeviceWaitIdle                                    vkDeviceWaitIdle;
	PFN_vkAllocateMemory                                    vkAllocateMemory;
	PFN_vkFreeMemory                                        vkFreeMemory;
	PFN_vkMapMemory                                         vkMapMemory;
	PFN_vkUnmapMemory                                       vkUnmapMemory;
	PFN_vkFlushMappedMemoryRanges                           vkFlushMappedMemoryRanges;
	PFN_vkInvalidateMappedMemoryRanges                      vkInvalidateMappedMemoryRanges;
	PFN_vkGetDeviceMemoryCommitment                         vkGetDeviceMemoryCommitment;
	PFN_vkBindBufferMemory                                  vkBindBufferMemory;
	PFN_vkBindImageMemory                                   vkBindImageMemory;
	PFN_vkGetBufferMemoryRequirements                       vkGetBufferMemoryRequirements;
	PFN_vkGetImageMemoryRequirements                        vkGetImageMemoryRequirements;
	PFN_vkGetImageSparseMemoryRequirements                  vkGetImageSparseMemoryRequirements;
	PFN_vkGetPhysicalDeviceSparseImageFormatProperties      vkGetPhysicalDeviceSparseImageFormatProperties;
	PFN_vkQueueBindSparse                                   vkQueueBindSparse;
	PFN_vkCreateFence                                       vkCreateFence;
	PFN_vkDestroyFence                                      vkDestroyFence;
	PFN_vkResetFences                                       vkResetFences;
	PFN_vkGetFenceStatus                                    vkGetFenceStatus;
	PFN_vkWaitForFences                                     vkWaitForFences;
	PFN_vkCreateSemaphore                                   vkCreateSemaphore;
	PFN_vkDestroySemaphore                                  vkDestroySemaphore;
	PFN_vkCreateEvent                                       vkCreateEvent;
	PFN_vkDestroyEvent                                      vkDestroyEvent;
	PFN_vkGetEventStatus                                    vkGetEventStatus;
	PFN_vkSetEvent                                          vkSetEvent;
	PFN_vkResetEvent                                        vkResetEvent;
	PFN_vkCreateQueryPool                                   vkCreateQueryPool;
	PFN_vkDestroyQueryPool                                  vkDestroyQueryPool;
	PFN_vkGetQueryPoolResults                               vkGetQueryPoolResults;
	PFN_vkCreateBuffer                                      vkCreateBuffer;
	PFN_vkDestroyBuffer                                     vkDestroyBuffer;
	PFN_vkCreateBufferView                                  vkCreateBufferView;
	PFN_vkDestroyBufferView                                 vkDestroyBufferView;
	PFN_vkCreateImage                                       vkCreateImage;
	PFN_vkDestroyImage                                      vkDestroyImage;
	PFN_vkGetImageSubresourceLayout                         vkGetImageSubresourceLayout;
	PFN_vkCreateImageView                                   vkCreateImageView;
	PFN_vkDestroyImageView                                  vkDestroyImageView;
	PFN_vkCreateShaderModule                                vkCreateShaderModule;
	PFN_vkDestroyShaderModule                               vkDestroyShaderModule;
	PFN_vkCreatePipelineCache                               vkCreatePipelineCache;
	PFN_vkDestroyPipelineCache                              vkDestroyPipelineCache;
	PFN_vkGetPipelineCacheData                              vkGetPipelineCacheData;
	PFN_vkMergePipelineCaches                               vkMergePipelineCaches;
	PFN_vkCreateGraphicsPipelines                           vkCreateGraphicsPipelines;
	PFN_vkCreateComputePipelines                            vkCreateComputePipelines;
	PFN_vkDestroyPipeline                                   vkDestroyPipeline;
	PFN_vkCreatePipelineLayout                              vkCreatePipelineLayout;
	PFN_vkDestroyPipelineLayout                             vkDestroyPipelineLayout;
	PFN_vkCreateSampler                                     vkCreateSampler;
	PFN_vkDestroySampler                                    vkDestroySampler;
	PFN_vkCreateDescriptorSetLayout                         vkCreateDescriptorSetLayout;
	PFN_vkDestroyDescriptorSetLayout                        vkDestroyDescriptorSetLayout;
	PFN_vkCreateDescriptorPool                              vkCreateDescriptorPool;
	PFN_vkDestroyDescriptorPool                             vkDestroyDescriptorPool;
	PFN_vkResetDescriptorPool                               vkResetDescriptorPool;
	PFN_vkAllocateDescriptorSets                            vkAllocateDescriptorSets;
	PFN_vkFreeDescriptorSets                                vkFreeDescriptorSets;
	PFN_vkUpdateDescriptorSets                              vkUpdateDescriptorSets;
	PFN_vkCreateFramebuffer                                 vkCreateFramebuffer;
	PFN_vkDestroyFramebuffer                                vkDestroyFramebuffer;
	PFN_vkCreateRenderPass                                  vkCreateRenderPass;
	PFN_vkDestroyRenderPass                                 vkDestroyRenderPass;
	PFN_vkGetRenderAreaGranularity                          vkGetRenderAreaGranularity;
	PFN_vkCreateCommandPool                                 vkCreateCommandPool;
	PFN_vkDestroyCommandPool                                vkDestroyCommandPool;
	PFN_vkResetCommandPool                                  vkResetCommandPool;
	PFN_vkAllocateCommandBuffers                            vkAllocateCommandBuffers;
	PFN_vkFreeCommandBuffers                                vkFreeCommandBuffers;
	PFN_vkBeginCommandBuffer                                vkBeginCommandBuffer;
	PFN_vkEndCommandBuffer                                  vkEndCommandBuffer;
	PFN_vkResetCommandBuffer                                vkResetCommandBuffer;
	PFN_vkCmdBindPipeline                                   vkCmdBindPipeline;
	PFN_vkCmdSetViewport                                    vkCmdSetViewport;
	PFN_vkCmdSetScissor                                     vkCmdSetScissor;
	PFN_vkCmdSetLineWidth                                   vkCmdSetLineWidth;
	PFN_vkCmdSetDepthBias                                   vkCmdSetDepthBias;
	PFN_vkCmdSetBlendConstants                              vkCmdSetBlendConstants;
	PFN_vkCmdSetDepthBounds                                 vkCmdSetDepthBounds;
	PFN_vkCmdSetStencilCompareMask                          vkCmdSetStencilCompareMask;
	PFN_vkCmdSetStencilWriteMask                            vkCmdSetStencilWriteMask;
	PFN_vkCmdSetStencilReference                            vkCmdSetStencilReference;
	PFN_vkCmdBindDescriptorSets                             vkCmdBindDescriptorSets;
	PFN_vkCmdBindIndexBuffer                                vkCmdBindIndexBuffer;
	PFN_vkCmdBindVertexBuffers                              vkCmdBindVertexBuffers;
	PFN_vkCmdDraw                                           vkCmdDraw;
	PFN_vkCmdDrawIndexed                                    vkCmdDrawIndexed;
	PFN_vkCmdDrawIndirect                                   vkCmdDrawIndirect;
	PFN_vkCmdDrawIndexedIndirect                            vkCmdDrawIndexedIndirect;
	PFN_vkCmdDispatch                                       vkCmdDispatch;
	PFN_vkCmdDispatchIndirect                               vkCmdDispatchIndirect;
	PFN_vkCmdCopyBuffer                                     vkCmdCopyBuffer;
	PFN_vkCmdCopyImage                                      vkCmdCopyImage;
	PFN_vkCmdBlitImage                                      vkCmdBlitImage;
	PFN_vkCmdCopyBufferToImage                              vkCmdCopyBufferToImage;
	PFN_vkCmdCopyImageToBuffer                              vkCmdCopyImageToBuffer;
	PFN_vkCmdUpdateBuffer                                   vkCmdUpdateBuffer;
	PFN_vkCmdFillBuffer                                     vkCmdFillBuffer;
	PFN_vkCmdClearColorImage                                vkCmdClearColorImage;
	PFN_vkCmdClearDepthStencilImage                         vkCmdClearDepthStencilImage;
	PFN_vkCmdClearAttachments                               vkCmdClearAttachments;
	PFN_vkCmdResolveImage                                   vkCmdResolveImage;
	PFN_vkCmdSetEvent                                       vkCmdSetEvent;
	PFN_vkCmdResetEvent                                     vkCmdResetEvent;
	PFN_vkCmdWaitEvents                                     vkCmdWaitEvents;
	PFN_vkCmdPipelineBarrier                                vkCmdPipelineBarrier;
	PFN_vkCmdBeginQuery                                     vkCmdBeginQuery;
	PFN_vkCmdEndQuery                                       vkCmdEndQuery;
	PFN_vkCmdResetQueryPool                                 vkCmdResetQueryPool;
	PFN_vkCmdWriteTimestamp                                 vkCmdWriteTimestamp;
	PFN_vkCmdCopyQueryPoolResults                           vkCmdCopyQueryPoolResults;
	PFN_vkCmdPushConstants                                  vkCmdPushConstants;
	PFN_vkCmdBeginRenderPass                                vkCmdBeginRenderPass;
	PFN_vkCmdNextSubpass                                    vkCmdNextSubpass;
	PFN_vkCmdEndRenderPass                                  vkCmdEndRenderPass;
	PFN_vkCmdExecuteCommands                                vkCmdExecuteCommands;


	// public extensions
	PFN_vkDestroySurfaceKHR vkDestroySurfaceKHR;
	PFN_vkGetPhysicalDeviceSurfaceSupportKHR      vkGetPhysicalDeviceSurfaceSupportKHR;
	PFN_vkGetPhysicalDeviceSurfaceCapabilitiesKHR vkGetPhysicalDeviceSurfaceCapabilitiesKHR;
	PFN_vkGetPhysicalDeviceSurfaceFormatsKHR      vkGetPhysicalDeviceSurfaceFormatsKHR;
	PFN_vkGetPhysicalDeviceSurfacePresentModesKHR vkGetPhysicalDeviceSurfacePresentModesKHR;

	PFN_vkCreateSwapchainKHR    vkCreateSwapchainKHR;
	PFN_vkDestroySwapchainKHR   vkDestroySwapchainKHR;
	PFN_vkGetSwapchainImagesKHR vkGetSwapchainImagesKHR;
	PFN_vkAcquireNextImageKHR   vkAcquireNextImageKHR;
	PFN_vkQueuePresentKHR       vkQueuePresentKHR;

	#ifdef WIN32
	PFN_vkCreateWin32SurfaceKHR vkCreateWin32SurfaceKHR;
    #else 
    PFN_vkCreateXlibSurfaceKHR  vkCreateXlibSurfaceKHR;
	#endif

	PFN_vkCreateDebugReportCallbackEXT  vkCreateDebugReportCallbackEXT;
	PFN_vkDebugReportMessageEXT         vkDebugReportMessageEXT;
	PFN_vkDestroyDebugReportCallbackEXT vkDestroyDebugReportCallbackEXT;

	amf_handle                          m_hVulkanDll;
};
