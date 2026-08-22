#pragma once

#define VULKAN_HPP_NO_CONSTRUCTORS
#include <vulkan/vulkan.h>
#include <cstddef>
#include <vulkan/vulkan.hpp>
#include <vulkan/vulkan_core.h>
#include <vulkan/vulkan_enums.hpp>
#include <vulkan/vulkan_handles.hpp>
#include <vulkan/vulkan_structs.hpp>

#include "VkContext.hpp"
#include "utils.hpp"

#include "ffmpeg_helper.h"

namespace alvr::render {

constexpr u32 ImageCount = 6;
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
        ctx.dev.free(memory);
        ctx.dev.destroy(image);
    }
};

struct Output {
    Image image;
    DrmImage drm;
    // VkSemaphore semaphore;
    VkImageCreateInfo imageCI;
    VkDeviceSize size;
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
    std::array<int, ImageCount> inputImgFds;
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
            WarpParams const& warp
        );

        void destroy(VkContext const& ctx);
    };

}

class Renderer {
    vk::Extent2D eyeExtent;
    vk::Extent2D outExtent;

    Image inputImages[ImageCount];
    Image stagingImgs[StagingImgCount];
    Output output;

    vk::QueryPool timestampPool;
    vk::CommandPool cmdPool;
    vk::CommandBuffer cmdBuf;
    vk::Sampler sampler;
    vk::DescriptorSetLayout descLayout;
    vk::Fence fence;

    std::vector<detail::RenderPipeline> pipes;

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
    void render(VkContext& vkCtx, u32 leftIdx, u32 rightIdx);

    WarpParams warpParams {};

    Output getOutput() { return output; }

    void destroy(VkContext const& ctx);
};

}
