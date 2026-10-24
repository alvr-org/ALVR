#pragma once

#define VULKAN_HPP_NO_CONSTRUCTORS
#include <cstddef>
#include <map>
#include <vector>
#include <vulkan/vulkan.h>
#include <vulkan/vulkan.hpp>
#include <vulkan/vulkan_core.h>
#include <vulkan/vulkan_enums.hpp>
#include <vulkan/vulkan_handles.hpp>
#include <vulkan/vulkan_structs.hpp>

#include "VkContext.hpp"
#include "utils.hpp"

#include "ffmpeg_helper.h"

namespace alvr::render {

constexpr usize StagingImgCount = 2;

enum class HandleType {
    None,
    DmaBuf,
    OpaqueFd,
};

struct Image {
    vk::Image image = VK_NULL_HANDLE;
    vk::ImageLayout layout = vk::ImageLayout::eUndefined;
    vk::DeviceMemory memory = VK_NULL_HANDLE;
    vk::ImageView view = VK_NULL_HANDLE;

    void destroy(VkContext const& ctx) {
        ctx.dev.destroy(view);
        ctx.dev.destroy(image);
        ctx.dev.free(memory);
    }
};

struct Output {
    Image image;
    DrmImage drm;
    // VkSemaphore semaphore;
    VkImageCreateInfo imageCI;
    VkDeviceSize size;

    void destroy(VkContext const& ctx) {
        if (drm.fd >= 0) {
            close(drm.fd);
            drm.fd = -1;
        }
        image.destroy(ctx);
    }
};

struct PipelineCreateInfo {
    std::vector<u8> shaderData;
    std::vector<vk::SpecializationMapEntry> specs;
    std::vector<u8> specData;
};

struct RendererCreateInfo {
    vk::Format format;
    vk::Extent2D inputEyeExtent;
    vk::Extent2D outputExtent;
};

// One eye of a submitted layer: SteamVR's texture by its shared handle, the
// fd it was received with (owned by the driver, dup'd for the import), and
// the valid sub-rectangle SteamVR submitted (normalized; reversed endpoints
// flip the image).
struct LayerEye {
    u64 handle = 0;
    int fd = -1;
    float uMin = 0.f;
    float vMin = 0.f;
    float uMax = 1.f;
    float vMax = 1.f;
};

// A submitted layer. Layer 0 is the scene; later layers (SteamVR's dashboard,
// other overlays) are blended over it in order.
struct Layer {
    vk::Format format = vk::Format::eUndefined;
    vk::Extent2D extent {};
    LayerEye eyes[2];
};

struct SemaphoreWait {
    uint64_t semaphore = 0;
    uint64_t value = 0;
};

// Push-constant payload for the rotational reprojection fold in the sampling
// shaders. Field offsets must match the GLSL block (0/64/80/96). enabled == 0
// reproduces the unwarped sampling exactly, so a zero-initialized struct is a
// correct passthrough.
struct WarpParams {
    float rotation[16] = {}; // column-major mat4, rotation in the upper-left 3x3
    float leftTans[4] = {}; // tanLeft, tanRight, tanUp, tanDown
    float rightTans[4] = {};
    u32 enabled = 0;
    u32 _padding[3] = {};
};
static_assert(sizeof(WarpParams) == 112);
static_assert(offsetof(WarpParams, rotation) == 0);
static_assert(offsetof(WarpParams, leftTans) == 64);
static_assert(offsetof(WarpParams, rightTans) == 80);
static_assert(offsetof(WarpParams, enabled) == 96);

// Per-eye foveation center shifts for ffr.comp, updated every frame.
struct FoveationCenterShifts {
    float left[2] = {};
    float right[2] = {};
};
static_assert(sizeof(FoveationCenterShifts) == 16);

// The push-constant range every pass shares. The warp sits at offset 0 so
// quad.comp can declare only that prefix; ffr.comp declares the whole block
// (centers at 112/120). Shaders that declare neither ignore it.
struct PushConstants {
    WarpParams warp;
    FoveationCenterShifts centerShifts;
};
static_assert(sizeof(PushConstants) == 128);
static_assert(offsetof(PushConstants, centerShifts) == 112);

namespace detail {

    class RenderPipeline {
        vk::ShaderModule shader;
        vk::PipelineLayout pipeLayout;
        vk::Pipeline pipe;

    public:
        RenderPipeline(
            VkContext const& ctx, vk::DescriptorSetLayout& layout, PipelineCreateInfo& pipelineCI
        );

        void render(
            VkContext const& ctx,
            vk::CommandBuffer cmdBuf,
            vk::ImageView in,
            vk::ImageView out,
            vk::Extent2D outSize,
            PushConstants const& pushConstants
        );

        void destroy(VkContext const& ctx);
    };

}

class Renderer {
    vk::Extent2D eyeExtent;
    vk::Extent2D outExtent;

    Image stagingImgs[StagingImgCount];
    Output output;

    vk::QueryPool timestampPool;
    vk::CommandPool cmdPool;
    vk::CommandBuffer cmdBuf;
    vk::Sampler sampler;
    vk::DescriptorSetLayout descLayout;
    vk::Fence fence;

    std::vector<detail::RenderPipeline> pipes;

    // SteamVR's textures, imported on first use by handle and kept until
    // SteamVR destroys them (releaseTextures) or the renderer goes.
    std::map<u64, Image> textures;
    std::unordered_map<u64, vk::Semaphore> timelines;
    // layer.comp: draws one eye of a layer into the working image.
    vk::ShaderModule layerShader;
    vk::PipelineLayout layerPipeLayout;
    vk::Pipeline layerPipe;

    Image& importTexture(VkContext& vkCtx, Layer const& layer, LayerEye const& eye);
    void drawLayerEye(VkContext& vkCtx, Image& source, int eye, LayerEye const& rect, bool opaque);

    // vk::Semaphore renderFinishedSem;

public:
    Renderer(
        VkContext const& vkCtx,
        RendererCreateInfo& createInfo,
        std::vector<PipelineCreateInfo> pipeCIs
    );

    // TODO: Import output (somehow?)

    // NOTE: Use the output immediately afterwards, as this synchronizes to the end of gpu
    // operations
    // Draws layers[0] into the working image, blends the other layers over
    // it, then runs the pass chain. The caller has already waited for the
    // writers of every texture.
    void render(
        VkContext& vkCtx,
        std::vector<Layer> const& layers,
        std::vector<SemaphoreWait> const& semaphoreWaits = {}
    );

    // Frees the imports of textures SteamVR destroyed. The GPU is idle
    // between frames (render waits for its submission).
    void releaseTextures(VkContext const& vkCtx, std::vector<u64> const& handles);
    void releaseSemaphores(VkContext const& vkCtx, std::vector<u64> const& handles);

    WarpParams warpParams {};

    FoveationCenterShifts foveationCenterShifts {};

    Output getOutput() { return output; }

    void destroy(VkContext const& ctx);
};

}
