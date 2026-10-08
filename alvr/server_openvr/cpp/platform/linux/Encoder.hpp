#pragma once

#include <fstream>
#include <memory>
#include <stdexcept>
#include <type_traits>

#include "EncodePipeline.h"
#include "Renderer.hpp"

#include "alvr_server/IDRScheduler.h"
#include "alvr_server/Settings.h"
#include "alvr_server/bindings.h"
#include <unistd.h>

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

        auto const offset = static_cast<u32>(info.specData.size());
        info.specData.insert(info.specData.end(), arr.begin(), arr.end());
        info.specs.push_back({
            .constantID = index,
            .offset = offset,
            .size = Size,
        });
        ++index;
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

// The encoder-aligned parameters are computed on the Rust side from the
// negotiated config. The center shifts are not specialization constants: they
// can move every frame and reach the shader as push constants.
inline auto makeFoveation(Settings const& settings) {
    auto const& params = settings.m_foveatedEncoding;

    vk::Extent2D outSize {
        .width = params.encodedViewResolution[0] * 2,
        .height = params.encodedViewResolution[1],
    };

    // The order of this needs to be kept in sync with the shader!
    // clang-format off
    auto info = makeSpecs(
        params.viewRatio[0],
        params.viewRatio[1],
        params.centerSize[0],
        params.centerSize[1],
        params.edgeRatio[0],
        params.edgeRatio[1]);
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
    std::unique_ptr<VkFrame> frame;
    bool encoderMissingLogged = false;
    IDRScheduler idrScheduler;

    // Whether the current pass chain ends in a shader carrying the warp fold,
    // which is ffr.comp or quad.comp. A colour-correction-only chain does not,
    // and warping there would move the frame's stamp while leaving the pixels
    // unwarped, which is worse than not warping at all.
    bool warpCapableChain = false;

public:
    // TODO: How are we supposed to match the physical device with direct mode?
    Encoder()
        : vkCtx(std::vector<u8> {}) { }

    void createImages(render::RendererCreateInfo rendererCI) {
        if (renderer.hasValue()) {
            renderer.get().destroy(vkCtx);
            renderer.reset();
        }

        auto const& settings = Settings_Instance();

        vk::Extent2D inExtent {
            .width = rendererCI.inputEyeExtent.width * 2,
            .height = rendererCI.inputEyeExtent.height,
        };

        std::vector<render::PipelineCreateInfo> pipeCIs;

        if (settings->m_enableColorCorrection)
            pipeCIs.push_back(makeColorCorrection(*settings, inExtent));

        // NOTE: This needs to be last as it needs to render into the output image
        if (settings->m_enableFoveatedEncoding) {
            auto [info, newExtent] = makeFoveation(*settings);
            rendererCI.outputExtent = newExtent;

            pipeCIs.push_back(info);
        }

        if (pipeCIs.empty()) {
            pipeCIs.push_back({
                .shaderData = std::vector(
                    QUAD_SHADER_COMP_SPV_PTR, QUAD_SHADER_COMP_SPV_PTR + QUAD_SHADER_COMP_SPV_LEN
                ),
            });
            warpCapableChain = true;
        } else {
            warpCapableChain = settings->m_enableFoveatedEncoding;
        }

        outExtent = rendererCI.outputExtent;
        renderer.emplace(vkCtx, rendererCI, pipeCIs);
    }

    void initEncoding() {
        auto const& settings = Settings_Instance();

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

        auto out = renderer.get().getOutput();

        // TODO: Fix Nvidia

        frame = std::make_unique<VkFrame>(
            vkCtx, out.image.image, out.imageCI, out.size, out.image.memory, out.drm
        );

        encoder
            = EncodePipeline::Create(vkCtx, devicePath, *frame, outExtent.width, outExtent.height);
        if (!encoder) {
            throw std::runtime_error("no usable encoder for the current settings");
        }
        encoderMissingLogged = false;

        idrScheduler.OnStreamStart();
    }

    // Tears down the encode pipeline so the next createImages + initEncoding
    // rebuilds from current settings. The pipeline references the frame, so
    // destroy it first. Only call this from the thread that runs present().
    void shutdown() {
        // Drain the queue before releasing the frame. Nothing on the VAAPI
        // path leaves Vulkan work in flight, but the Nvidia path signals the
        // frame's semaphore from a queue submission and will need this when
        // it comes back.
        vkCtx.dev.waitIdle();

        encoder.reset();
        frame.reset();
        if (renderer.hasValue()) {
            renderer.get().destroy(vkCtx);
            renderer.reset();
        }
        encoderMissingLogged = false;
    }

    // waitFds are sync_file fds the render submission must wait on before
    // sampling the eye textures; ownership transfers here on every path.
    void present(
        u32 leftIdx,
        u32 rightIdx,
        u64 targetTimestampNs,
        render::WarpParams const& warp,
        int const waitFds[2]
    ) {
        if (!encoder) {
            for (int e = 0; e < 2; ++e) {
                if (waitFds[e] >= 0) {
                    close(waitFds[e]);
                }
            }
            // Say it once instead of at frame rate.
            if (!encoderMissingLogged) {
                Error("Encoder not initialized, skipping frames until it is rebuilt.\n");
                encoderMissingLogged = true;
            }
            return;
        }
        renderer.get().warpParams = warp;
        ReportPresent(targetTimestampNs, 0);
        updateFoveationCenters(targetTimestampNs);
        renderer.get().render(vkCtx, leftIdx, rightIdx, waitFds);
        ReportComposed(targetTimestampNs, 0);

        encoder->SetParams(GetDynamicEncoderParams());
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

    // True when engaging the reprojection actually warps pixels with the
    // current pass chain. Also false while the encoder has not been built.
    // Non-const because Optional::hasValue() is not const-qualified.
    bool warpCapable() { return renderer.hasValue() && warpCapableChain; }

private:
    // Eye-tracked centers when the client supplies gaze, the negotiated static
    // ones otherwise. Looked up and reported under the timestamp the frame is
    // sent with, which after a warp is the newest pose's, so the client
    // decodes the frame with the same centers it was encoded with.
    void updateFoveationCenters(u64 targetTimestampNs) {
        auto const& settings = Settings_Instance();
        if (!settings->m_enableFoveatedEncoding) {
            return;
        }

        auto const gazeCenters = GetEyeTrackedFoveationCenters(targetTimestampNs);
        auto const& centers = gazeCenters.valid ? gazeCenters.centerShifts
                                                : settings->m_foveatedEncoding.centerShifts;

        auto& shifts = renderer.get().foveationCenterShifts;
        shifts.left[0] = centers[0][0];
        shifts.left[1] = centers[0][1];
        shifts.right[0] = centers[1][0];
        shifts.right[1] = centers[1][1];

        ReportEncoderFoveationCenters(
            targetTimestampNs, shifts.left[0], shifts.left[1], shifts.right[0], shifts.right[1]
        );
    }

public:
    ~Encoder() {
        // The device dies in this body, but members are destroyed after the
        // body runs. Release the pipeline and the frame first so ~VkFrame
        // does not destroy its semaphore on a dead device.
        encoder.reset();
        frame.reset();

        if (renderer.hasValue()) {
            renderer.get().destroy(vkCtx);
            renderer.reset();
        }

        vkCtx.destroy();
    }
};

}
