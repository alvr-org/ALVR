#pragma once

#include <algorithm>
#include <cassert>
#include <cstdint>
#include <cstring>
#include <fcntl.h>
#include <functional>
#include <iostream>
#include <optional>
#include <stdexcept>
#include <string_view>
#include <vector>

#include <sys/stat.h>
#include <sys/sysmacros.h>
#include <unistd.h>

#define VULKAN_HPP_NO_CONSTRUCTORS
#include <vulkan/vulkan.h>
#include <vulkan/vulkan.hpp>
#include <vulkan/vulkan_core.h>
#include <vulkan/vulkan_enums.hpp>
#include <vulkan/vulkan_handles.hpp>
#include <vulkan/vulkan_structs.hpp>

#include "utils.hpp"

#include "alvr_server/Logger.h"

extern "C" {
#include <libavutil/log.h>
}

namespace alvr {

enum class Vendor { Amd, Intel, Nvidia };

class VkContext {
public:
    vk::Instance instance;
    vk::PhysicalDevice physDev;
    vk::Device dev;

    vk::DispatchLoaderDynamic dispatch;

    struct Meta {
        Vendor vendor;

        std::vector<char const*> instExtensions;
        std::vector<char const*> devExtensions;

        vk::PhysicalDeviceVulkan12Features feats12;
        vk::PhysicalDeviceFeatures2 feats;

        u32 queueFamily;
        u32 queueIndex;
    } meta;

private:
    vk::Queue queue;

    void sharedInit() {
        auto devProps = physDev.getProperties2();

        if (devProps.properties.vendorID == 0x1002)
            meta.vendor = Vendor::Amd;
        else if (devProps.properties.vendorID == 0x8086)
            meta.vendor = Vendor::Intel;
        else if (devProps.properties.vendorID == 0x10de)
            meta.vendor = Vendor::Nvidia;

        dispatch = { instance, vkGetInstanceProcAddr };
    }

public:
    VkContext(std::vector<u8> deviceUUID) {
        std::vector<std::string_view> wantedInstExts = {
            VK_KHR_GET_PHYSICAL_DEVICE_PROPERTIES_2_EXTENSION_NAME,
        };

        auto availInstExts = vk::enumerateInstanceExtensionProperties();
        std::vector<const char*> acquiredInstExts;
        for (auto wantedName : wantedInstExts) {
            auto it = std::find_if(availInstExts.begin(), availInstExts.end(), [&](auto& ext) {
                return wantedName == ext.extensionName;
            });
            if (it != availInstExts.end())
                acquiredInstExts.push_back(wantedName.data());
            else
                assert(false);
        }

        vk::ApplicationInfo appInfo {
            .pApplicationName = "ALVR",
            .apiVersion = VK_API_VERSION_1_2,
        };
        vk::InstanceCreateInfo instanceCI {
            .pApplicationInfo = &appInfo,
            .enabledExtensionCount = static_cast<uint32_t>(acquiredInstExts.size()),
            .ppEnabledExtensionNames = acquiredInstExts.data(),
        };
        instance = vk::createInstance(instanceCI);
        auto physDevs = instance.enumeratePhysicalDevices();

        physDev = physDevs[0];
        //    for (auto dev : physDevs) {
        //        vk::PhysicalDeviceVulkan11Properties props11{};
        //        vk::PhysicalDeviceProperties2 props {
        //            .pNext = &props11,
        //        };
        //        dev.getProperties2(&props);

        // assert(deviceUUID.size() ==  VK_UUID_SIZE);
        //        if (memcmp(props11.deviceUUID, deviceUUID.data(), VK_UUID_SIZE) == 0) {
        //            physDev = dev;
        //            break;
        //        }
        //    }
        //    if (!physDev && !physDevs.empty()) {
        //        Warn("Falling back to first physical device");
        //        physDev = physDevs[0];
        //    }
        //    if (!physDev) {
        //        throw std::runtime_error("Failed to find vulkan device");
        //    }

        auto queueFamilyProps = physDev.getQueueFamilyProperties();

        std::optional<u32> wantedQueueFamily;

        for (u32 i = 0; i < queueFamilyProps.size(); ++i) {
            auto& props = queueFamilyProps[i];
            bool isGraphics = static_cast<bool>(props.queueFlags & vk::QueueFlagBits::eGraphics);
            bool isCompute = static_cast<bool>(props.queueFlags & vk::QueueFlagBits::eCompute);

            if (isCompute && (!wantedQueueFamily.has_value() || !isGraphics)) {
                wantedQueueFamily = i;
            }
        }
        meta.queueFamily = wantedQueueFamily.value();
        meta.queueIndex = 0;

        std::vector<std::string_view> wantedExts = {
            VK_KHR_EXTERNAL_MEMORY_FD_EXTENSION_NAME,
            VK_KHR_EXTERNAL_SEMAPHORE_FD_EXTENSION_NAME,
            VK_EXT_EXTERNAL_MEMORY_DMA_BUF_EXTENSION_NAME,
            VK_EXT_IMAGE_DRM_FORMAT_MODIFIER_EXTENSION_NAME,
            VK_KHR_EXTERNAL_SEMAPHORE_FD_EXTENSION_NAME,
            VK_EXT_EXTERNAL_MEMORY_HOST_EXTENSION_NAME,
            VK_KHR_PUSH_DESCRIPTOR_EXTENSION_NAME,
            VK_KHR_SAMPLER_YCBCR_CONVERSION_EXTENSION_NAME,
            VK_EXT_PHYSICAL_DEVICE_DRM_EXTENSION_NAME,
            VK_EXT_CALIBRATED_TIMESTAMPS_EXTENSION_NAME,
            VK_KHR_COPY_COMMANDS_2_EXTENSION_NAME,
        };

        auto availExts = physDev.enumerateDeviceExtensionProperties();

        std::vector<char const*> acquiredExts;
        for (auto name : wantedExts) {
            auto it = std::find_if(availExts.begin(), availExts.end(), [&](auto& other) {
                return name == (const char*)other.extensionName;
            });
            if (it != availExts.end())
                acquiredExts.push_back(name.data());
        }

        f32 queuePrio = 1.f;
        vk::DeviceQueueCreateInfo queueCI {
            .queueFamilyIndex = meta.queueFamily,
            .queueCount = 1,
            .pQueuePriorities = &queuePrio,
        };

        meta.feats12 = vk::PhysicalDeviceVulkan12Features {
            .timelineSemaphore = 1,
        };

        meta.feats = vk::PhysicalDeviceFeatures2 {
            .pNext = &meta.feats12,
            .features = {
                .robustBufferAccess = true,
                .samplerAnisotropy = true,
            },
        };
        vk::DeviceCreateInfo devCI {
            .pNext = &meta.feats,
            .queueCreateInfoCount = 1,
            .pQueueCreateInfos = &queueCI,
            .enabledExtensionCount = static_cast<u32>(acquiredExts.size()),
            .ppEnabledExtensionNames = acquiredExts.data(),
        };
        dev = physDev.createDevice(devCI);

        meta.devExtensions = acquiredExts;

        queue = dev.getQueue(wantedQueueFamily.value(), meta.queueIndex);

        sharedInit();
    }

    void useQueue(std::function<void(vk::Queue&)> fn) { fn(queue); }

    void destroy() {
        dev.destroy();
        instance.destroy();
    }
};

}
