#include "hmd.h"
#include <cmath>

const vr::HmdMatrix34_t MATRIX_IDENTITY = {
    {{1.0, 0.0, 0.0, 0.0}, {0.0, 1.0, 0.0, 0.0}, {0.0, 0.0, 1.0, 0.0}}};

vr::HmdRect2_t fov_to_projection(AlvrFov fov) {
    auto proj_bounds = vr::HmdRect2_t{};
    proj_bounds.vTopLeft.v[0] = tanf(fov.left);
    proj_bounds.vTopLeft.v[0] = tanf(fov.top);
    proj_bounds.vTopLeft.v[0] = -tanf(fov.right);
    proj_bounds.vTopLeft.v[0] = -tanf(fov.bottom);
}

Hmd::Hmd(const char *serial_number) : TrackedDevice(HEAD_PATH) {
    // Initialize variables with dummy values. They will be updated later

    this->video_config = AlvrVideoConfig{};
    this->video_config.preferred_view_width = 500;
    this->video_config.preferred_view_height = 500;

    auto dummy_fov = AlvrFov{-1.0, 1.0, 1.0, -1.0};

    this->views_config = AlvrViewsConfig{};
    this->views_config.ipd_m = 0.063;
    this->views_config.fov[0] = dummy_fov;
    this->views_config.fov[1] = dummy_fov;

    vr::VRServerDriverHost()->TrackedDeviceAdded(serial_number, vr::TrackedDeviceClass_HMD, this);
}

vr::EVRInitError Hmd::Activate(uint32_t id) {
    TrackedDevice::Activate(id);

    set_static_properties(this->device_path, this->prop_container);

    return vr::VRInitError_None;
};

void *Hmd::GetComponent(const char *component_name_and_version) {
    auto name_and_vers = std::string(component_name_and_version);
    if (name_and_vers == vr::IVRDisplayComponent_Version) {
        return this;
    }

#ifdef _WIN32
    if (name_and_vers == vr::IVRDriverDirectModeComponent_Version) {
        return this;
    }
#endif

    return nullptr;
}

void Hmd::GetWindowBounds(int32_t *x, int32_t *y, uint32_t *width, uint32_t *height) {
    *x = 0;
    *y = 0;
    *width = this->video_config.preferred_view_width * 2;
    *height = this->video_config.preferred_view_height;
}

void Hmd::GetRecommendedRenderTargetSize(uint32_t *width, uint32_t *height) {
    *width = this->video_config.preferred_view_width;
    *height = this->video_config.preferred_view_height;
}

void Hmd::GetEyeOutputViewport(
    vr::EVREye eye, uint32_t *x, uint32_t *y, uint32_t *width, uint32_t *height) {
    *x = (eye == vr::Eye_Left ? 0 : this->video_config.preferred_view_width);
    *y = 0;
    *width = this->video_config.preferred_view_width;
    *height = this->video_config.preferred_view_height;
}

void Hmd::GetProjectionRaw(vr::EVREye eye, float *left, float *right, float *top, float *bottom) {
    auto proj = fov_to_projection(this->views_config.fov[eye]);
    *left = proj.vTopLeft.v[0];
    *right = proj.vBottomRight.v[0];
    *top = proj.vTopLeft.v[1];
    *bottom = proj.vBottomRight.v[1];
}

vr::DistortionCoordinates_t Hmd::ComputeDistortion(vr::EVREye, float u, float v) {
    return {{u, v}, {u, v}, {u, v}};
}

void Hmd::CreateSwapTextureSet(uint32_t pid,
                               const SwapTextureSetDesc_t *swap_texture_set_desc,
                               SwapTextureSet_t *swap_texture_set) {
    vr::SharedTextureHandle_t *texture_handles;
    uint64_t id = alvr_create_swapchain(3,
                                        swap_texture_set_desc->nWidth,
                                        swap_texture_set_desc->nHeight,
                                        swap_texture_set_desc->nFormat,
                                        swap_texture_set_desc->nSampleCount,
                                        true,
                                        (void *)texture_handles);

    auto swapchain = SwapchainData{};
    swapchain.id = id;
    swapchain.pid = pid;
    swapchain.texture_handles[0] = texture_handles[0];
    swapchain.texture_handles[1] = texture_handles[1];
    swapchain.texture_handles[2] = texture_handles[2];

    this->swapchains[texture_handles[0]] = swapchain;
    this->swapchains[texture_handles[1]] = swapchain;
    this->swapchains[texture_handles[2]] = swapchain;
}

void Hmd::DestroySwapTextureSet(vr::SharedTextureHandle_t shared_texture_handle) {
    auto maybe_entry = this->swapchains.find(shared_texture_handle);

    if (maybe_entry != this->swapchains.end()) {
        auto [_, swapchain] = *maybe_entry;

        alvr_destroy_swapchain(swapchain.id);

        for (auto handle : swapchain.texture_handles) {
            this->swapchains.erase(handle);
        }
    }
}

void Hmd::DestroyAllSwapTextureSets(uint32_t pid) {
    // Note: this->swapchains is drained by DestroySwapTextureSet
    auto swapchains_copy = this->swapchains;
    for (auto &[handle, swapchain] : swapchains_copy) {
        if (swapchain.pid == pid) {
            this->DestroySwapTextureSet(handle);
        }
    }
}

void Hmd::GetNextSwapTextureSetIndex(vr::SharedTextureHandle_t shared_texture_handles[2],
                                     uint32_t (*indices)[2]) {
    for (int idx = 0; idx < 2; idx++) {
        auto swapchain = this->swapchains.at(shared_texture_handles[idx]);
        (*indices)[idx] = alvr_swapchain_get_next_index(swapchain.id);
    }
}

void Hmd::SubmitLayer(const SubmitLayerPerEye_t (&eye)[2]) {
    auto layer = AlvrLayer{};
    for (int idx = 0; idx < 2; idx++) {
        layer.views[idx].swapchain_id = this->swapchains.at(eye[idx].hTexture).id;
        layer.views[idx].fov = this->views_config.fov[idx];
        layer.views[idx].rect_offset.x = eye[idx].bounds.uMin;
        layer.views[idx].rect_offset.y = eye[idx].bounds.vMin;
        layer.views[idx].rect_size.x = eye[idx].bounds.uMax - eye[idx].bounds.uMin;
        layer.views[idx].rect_size.y = eye[idx].bounds.vMax - eye[idx].bounds.vMin;
    }
    this->current_layers.push_back(layer);
}

void Hmd::Present(vr::SharedTextureHandle_t sync_texture) {
    // todo: acquire lock on sync_texture

    // This call will block until the server finished rendering
    alvr_present_layers(&this->current_layers[0], (uint64_t)this->current_layers.size());

    this->current_layers.clear();
}

void Hmd::PostPresent() {
    alvr_wait_for_vsync(100); // timeout ms
    vr::VRServerDriverHost()->VsyncEvent(0.0);
}

void Hmd::GetFrameTiming(vr::DriverDirectMode_FrameTiming *frame_timing) {
    frame_timing->m_nNumFramePresents = 1;
    frame_timing->m_nNumMisPresented = 0;
    frame_timing->m_nNumDroppedFrames = 0;
}

void Hmd::update_video_config(AlvrVideoConfig config) {
    this->video_config = config;

    vr::VRServerDriverHost()->SetRecommendedRenderTargetSize(
        this->object_id, config.preferred_view_width, config.preferred_view_height);
}

void Hmd::update_views_config(AlvrViewsConfig config) {
    this->views_config = config;

    auto left_transform = MATRIX_IDENTITY;
    left_transform.m[0][3] = -config.ipd_m / 2.0;
    auto right_transform = MATRIX_IDENTITY;
    right_transform.m[0][3] = config.ipd_m / 2.0;
    vr::VRServerDriverHost()->SetDisplayEyeToHead(object_id, left_transform, right_transform);

    auto left_proj = fov_to_projection(config.fov[0]);
    auto right_proj = fov_to_projection(config.fov[1]);

    vr::VRServerDriverHost()->SetDisplayProjectionRaw(object_id, left_proj, right_proj);

    // todo: check if this is still needed
    vr::VRServerDriverHost()->VendorSpecificEvent(
        object_id, vr::VREvent_LensDistortionChanged, {}, 0);
}