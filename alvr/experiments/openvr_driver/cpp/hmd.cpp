
#include "hmd.h"
#include <algorithm>
#include <string>
#include <thread>

void Hmd::activate_inner() {}

void *Hmd::GetComponent(const char *component_name_and_version) {
    auto name_and_vers = std::string(component_name_and_version);
    if (name_and_vers == vr::IVRDisplayComponent_Version ||
        (name_and_vers == vr::IVRDriverDirectModeComponent_Version && this->do_presentation)) {
        return this;
    }
}

void Hmd::GetWindowBounds(int32_t *x, int32_t *y, uint32_t *width, uint32_t *height) {
    *x = 0;
    *y = 0;
    *width = this->config.preferred_view_width * 2;
    *height = this->config.preferred_view_height;
}

void Hmd::GetRecommendedRenderTargetSize(uint32_t *width, uint32_t *height) {
    *width = this->config.preferred_view_width;
    *height = this->config.preferred_view_height;
}

void Hmd::GetEyeOutputViewport(
    vr::EVREye eye, uint32_t *x, uint32_t *y, uint32_t *width, uint32_t *height) {
    *x = (eye == vr::Eye_Left ? 0 : this->config.preferred_view_width);
    *y = 0;
    *width = this->config.preferred_view_width;
    *height = this->config.preferred_view_height;
}

void Hmd::GetProjectionRaw(vr::EVREye eye, float *left, float *right, float *top, float *bottom) {
    auto fov = this->config.fov[eye];
    *left = fov.left;
    *right = fov.right;
    *top = fov.top;
    *bottom = fov.bottom;
}

vr::DistortionCoordinates_t Hmd::ComputeDistortion(vr::EVREye, float u, float v) {
    return {{u, v}, {u, v}, {u, v}};
}

void Hmd::CreateSwapTextureSet(uint32_t pid,
                               const SwapTextureSetDesc_t *swap_texture_set_desc,
                               SwapTextureSet_t *swap_texture_set) {
    auto swapchain = create_swapchain(pid, *swap_texture_set_desc);

    for (int idx = 0; idx < 3; idx++) {
        this->swapchains[swapchain.texture_handles[0]] = swapchain;
        swap_texture_set->rSharedTextureHandles[0] = swapchain.texture_handles[0];
    }
}

void Hmd::DestroySwapTextureSet(vr::SharedTextureHandle_t shared_texture_handle) {
    auto maybe_entry = this->swapchains.find(shared_texture_handle);

    if (maybe_entry != this->swapchains.end()) {
        auto [_, swapchain] = *maybe_entry;

        destroy_swapchain(swapchain.id);

        for (auto handle : swapchain.texture_handles) {
            this->swapchains.erase(handle);
        }
    }
}

void Hmd::DestroyAllSwapTextureSets(uint32_t pid) {
    auto swapchains_copy = this->swapchains;
    for (auto &[handle, swapchain] : swapchains_copy) {
        if (swapchain.pid == pid) {
            this->DestroySwapTextureSet(handle);
        }
    }
}

void Hmd::GetNextSwapTextureSetIndex(vr::SharedTextureHandle_t shared_texture_handles[2],
                                     uint32_t (*indices)[2]) {
    for (int idx; idx < 2; idx++) {
        auto swapchain = this->swapchains.at(shared_texture_handles[idx]);
        (*indices)[idx] = next_swapchain_index(swapchain.id);
    }
}

void Hmd::SubmitLayer(const SubmitLayerPerEye_t (&eye)[2]) {
    auto layer = Layer{};
    for (int idx = 0; idx < 2; idx++) {
        layer.swapchain_ids[idx] = this->swapchains.at(eye[idx].hTexture).id;
        layer.fov[idx] = this->config.fov[idx];
        layer.bounds[idx] = eye[idx].bounds;
        layer.poses[idx] = eye[idx].mHmdPose;
    }
    this->current_layers.push_back(layer);
}

void Hmd::Present(vr::SharedTextureHandle_t sync_texture) {
    // todo: acquire sync on sync_texture

    // This call will block until the server finished rendering
    present(&this->current_layers[0], this->current_layers.size());

    this->current_layers.clear();
}

void Hmd::PostPresent() {
    this->next_virtual_vsync += std::chrono::nanoseconds(int(1'000'000'000 / this->config.fps));
    std::this_thread::sleep_until(this->next_virtual_vsync);

    vr::VRServerDriverHost()->VsyncEvent(0);
}

void Hmd::GetFrameTiming(vr::DriverDirectMode_FrameTiming *frame_timing) {
    frame_timing->m_nNumFramePresents = 1;
    frame_timing->m_nNumMisPresented = 0;
    frame_timing->m_nNumDroppedFrames = 0;

    if (frame_timing->m_nReprojectionFlags & vr::VRCompositor_ReprojectionMotion_AppThrottled) {
        // todo: halve framerate
    }
}