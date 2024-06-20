#pragma once

#include <fstream>
#include <memory>
#include <type_traits>

#include "EncodePipeline.h"
#include "Renderer.hpp"

#include "alvr_server/IDRScheduler.h"
#include "alvr_server/Settings.h"
#include "alvr_server/bindings.h"

namespace alvr {

// TODO: Move to source file
template <typename... T>
concept floats = (std::is_same_v<float, T> && ...);

template <typename... T>
auto makeSpecs(T... args)
    requires floats<T...>
{
    render::PipelineCreateInfo info {};

    constexpr auto Size = sizeof(float);
    u32 index = 0;

    // NOTE: This only works with little endian (but what pcvr target is big endian anyway)
    auto put = [&](float data) {
        std::array<u8, Size> arr;
        memcpy(arr.data(), reinterpret_cast<u8*>(&data), arr.size());

        info.specData.insert(info.specData.end(), arr.begin(), arr.end());
        info.specs.push_back({
            .constantID = index,
            .offset = static_cast<uint32_t>(index * Size),
            .size = Size,
        });
    };

    (put(args), ...);

    return info;
}

inline auto makeColorCorrection(Settings const& settings, vk::Extent2D extent) {
    // The order of this needs to be kept in sync with the shader!
    // clang-format off
    auto info = makeSpecs(
        (float)extent.width,
        (float)extent.height,
        settings.m_brightness,
        settings.m_contrast + 1.f,
        settings.m_saturation + 1.f,
        settings.m_gamma,
        settings.m_sharpening);
    // clang-format on

    info.shaderData = std::vector(
        COLOR_SHADER_COMP_SPV_PTR, COLOR_SHADER_COMP_SPV_PTR + COLOR_SHADER_COMP_SPV_LEN
    );

    return info;
}

inline auto makeFoveation(Settings const& settings, vk::Extent2D extent) {
    float targetEyeWidth = (float)extent.width / 2;
    float targetEyeHeight = extent.height;

    float centerSizeX = settings.m_foveationCenterSizeX;
    float centerSizeY = settings.m_foveationCenterSizeY;
    float centerShiftX = settings.m_foveationCenterShiftX;
    float centerShiftY = settings.m_foveationCenterShiftY;
    float edgeRatioX = settings.m_foveationEdgeRatioX;
    float edgeRatioY = settings.m_foveationEdgeRatioY;

    float edgeSizeX = targetEyeWidth - centerSizeX * targetEyeWidth;
    float edgeSizeY = targetEyeHeight - centerSizeY * targetEyeHeight;

    float centerSizeXAligned
        = 1. - ceil(edgeSizeX / (edgeRatioX * 2.)) * (edgeRatioX * 2.) / targetEyeWidth;
    float centerSizeYAligned
        = 1. - ceil(edgeSizeY / (edgeRatioY * 2.)) * (edgeRatioY * 2.) / targetEyeHeight;

    float edgeSizeXAligned = targetEyeWidth - centerSizeXAligned * targetEyeWidth;
    float edgeSizeYAligned = targetEyeHeight - centerSizeYAligned * targetEyeHeight;

    float centerShiftXAligned = ceil(centerShiftX * edgeSizeXAligned / (edgeRatioX * 2.))
        * (edgeRatioX * 2.) / edgeSizeXAligned;
    float centerShiftYAligned = ceil(centerShiftY * edgeSizeYAligned / (edgeRatioY * 2.))
        * (edgeRatioY * 2.) / edgeSizeYAligned;

    float foveationScaleX = (centerSizeXAligned + (1. - centerSizeXAligned) / edgeRatioX);
    float foveationScaleY = (centerSizeYAligned + (1. - centerSizeYAligned) / edgeRatioY);

    float optimizedEyeWidth = foveationScaleX * targetEyeWidth;
    float optimizedEyeHeight = foveationScaleY * targetEyeHeight;

    // round the frame dimensions to a number of pixel multiple of 32 for the encoder
    auto optimizedEyeWidthAligned = (uint32_t)ceil(optimizedEyeWidth / 32.f) * 32;
    auto optimizedEyeHeightAligned = (uint32_t)ceil(optimizedEyeHeight / 32.f) * 32;

    float eyeWidthRatioAligned = optimizedEyeWidth / optimizedEyeWidthAligned;
    float eyeHeightRatioAligned = optimizedEyeHeight / optimizedEyeHeightAligned;

    vk::Extent2D outSize {
        .width = optimizedEyeWidthAligned * 2,
        .height = optimizedEyeHeightAligned,
    };

    // The order of this needs to be kept in sync with the shader!
    // clang-format off
    auto info = makeSpecs(
        eyeWidthRatioAligned,
        eyeHeightRatioAligned,
        centerSizeXAligned,
        centerSizeYAligned,
        centerShiftXAligned,
        centerShiftYAligned,
        edgeRatioX,
        edgeRatioY);
    // clang-format on

    info.shaderData
        = std::vector(FFR_SHADER_COMP_SPV_PTR, FFR_SHADER_COMP_SPV_PTR + FFR_SHADER_COMP_SPV_LEN);

    return std::tuple(info, outSize);
}

class Encoder {
    vk::Extent2D outExtent;

    VkContext vkCtx;

    Optional<render::Renderer> renderer;

    std::unique_ptr<EncodePipeline> encoder;
    IDRScheduler idrScheduler;

public:
    // TODO: How are we supposed to match the physical device with direct mode?
    Encoder()
        : vkCtx(std::vector<u8> {}) { }

    void createImages(render::RendererCreateInfo rendererCI) {
        if (renderer.hasValue()) {
            renderer.get().destroy(vkCtx);
        }

        auto const& settings = Settings::Instance();

        vk::Extent2D inExtent {
            .width = rendererCI.inputEyeExtent.width * 2,
            .height = rendererCI.inputEyeExtent.height,
        };

        std::vector<render::PipelineCreateInfo> pipeCIs;

        if (settings.m_enableColorCorrection)
            pipeCIs.push_back(makeColorCorrection(settings, inExtent));

        // NOTE: This needs to be last as it needs to render into the output image
        if (settings.m_enableFoveatedEncoding) {
            auto [info, newExtent] = makeFoveation(settings, rendererCI.outputExtent);
            rendererCI.outputExtent = newExtent;

            pipeCIs.push_back(info);
        }

        if (pipeCIs.empty()) {
            pipeCIs.push_back({
                .shaderData = std::vector(
                    QUAD_SHADER_COMP_SPV_PTR, QUAD_SHADER_COMP_SPV_PTR + QUAD_SHADER_COMP_SPV_LEN
                ),
            });
        }

        outExtent = rendererCI.outputExtent;
        renderer.emplace(vkCtx, rendererCI, pipeCIs);
    }

    void initEncoding() {
        auto const& settings = Settings::Instance();

        VkPhysicalDeviceDrmPropertiesEXT drmProps = {};
        drmProps.sType = VK_STRUCTURE_TYPE_PHYSICAL_DEVICE_DRM_PROPERTIES_EXT;

        VkPhysicalDeviceProperties2 deviceProps = {};
        deviceProps.sType = VK_STRUCTURE_TYPE_PHYSICAL_DEVICE_PROPERTIES_2;
        deviceProps.pNext = &drmProps;
        vkGetPhysicalDeviceProperties2(vkCtx.physDev, &deviceProps);

        std::string devicePath;
        for (int i = 128; i < 136; ++i) {
            auto path = "/dev/dri/renderD" + std::to_string(i);
            int fd = open(path.c_str(), O_RDONLY);
            if (fd == -1) {
                continue;
            }
            struct stat s = {};
            int ret = fstat(fd, &s);
            close(fd);
            if (ret != 0) {
                continue;
            }
            dev_t primaryDev = makedev(drmProps.primaryMajor, drmProps.primaryMinor);
            dev_t renderDev = makedev(drmProps.renderMajor, drmProps.renderMinor);
            if (primaryDev == s.st_rdev || renderDev == s.st_rdev) {
                devicePath = path;
                break;
            }
        }
        if (devicePath.empty()) {
            devicePath = "/dev/dri/renderD128";
        }
        Info("Using device path %s", devicePath.c_str());

        // av_log_set_level(AV_LOG_DEBUG);

        // TODO: Fix this memory leakage

        // TODO: The EncodePipeline should store this on it's own
        auto& avHwCtx = *new alvr::HWContext(vkCtx);

        auto out = renderer.get().getOutput();

        // TODO: Fix Nvidia

        // auto framCtx = new alvr::VkFrameCtx(aCtx, *(vk::ImageCreateInfo*)&out.imageCI);

        auto frame = new alvr::VkFrame(
            vkCtx, out.image.image, out.imageCI, out.size, out.image.memory, out.drm
        );

        encoder
            = EncodePipeline::Create(vkCtx, devicePath, *frame, outExtent.width, outExtent.height);

        idrScheduler.OnStreamStart();
    }

    void present(u32 leftIdx, u32 rightIdx, u64 targetTimestampNs) {
        ReportPresent(targetTimestampNs, 0);
        renderer.get().render(vkCtx, leftIdx, rightIdx);
        ReportComposed(targetTimestampNs, 0);

        encoder->PushFrame(0, idrScheduler.CheckIDRInsertion());

        alvr::FramePacket framePacket;
        if (!encoder->GetEncoded(framePacket)) {
            assert(false);
        }

        ParseFrameNals(
            encoder->GetCodec(),
            framePacket.data,
            framePacket.size,
            targetTimestampNs,
            framePacket.isIDR
        );
    }

    void requestIdr() { idrScheduler.InsertIDR(); }

    ~Encoder() {
        if (renderer.hasValue()) {
            renderer.get().destroy(vkCtx);
        }

        vkCtx.destroy();
    }
};

}
