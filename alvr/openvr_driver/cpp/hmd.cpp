#include "hmd.h"
#include <cmath>
#include <sstream>

const vr::HmdMatrix34_t MATRIX_IDENTITY = {
    {{1.0, 0.0, 0.0, 0.0}, {0.0, 1.0, 0.0, 0.0}, {0.0, 0.0, 1.0, 0.0}}};

vr::HmdRect2_t fov_to_projection(AlvrFov fov) {
    auto proj_bounds = vr::HmdRect2_t{};
    proj_bounds.vTopLeft.v[0] = tanf(fov.left);
    proj_bounds.vBottomRight.v[0] = tanf(fov.right);
    proj_bounds.vTopLeft.v[1] = -tanf(fov.top);
    proj_bounds.vBottomRight.v[1] = -tanf(fov.bottom);

    return proj_bounds;
}

Hmd::Hmd() : TrackedDevice(HEAD_PATH) {
    // Initialize variables with dummy values. They will be updated later

    this->video_config = AlvrVideoConfig{};
    this->video_config.preferred_view_width = 800;
    this->video_config.preferred_view_height = 900;

    auto dummy_fov = AlvrFov{-1.0, 1.0, 1.0, -1.0};

    this->views_config = AlvrViewsConfig{};
    this->views_config.ipd_m = 0.063;
    this->views_config.fov[0] = dummy_fov;
    this->views_config.fov[1] = dummy_fov;
}

vr::EVRInitError Hmd::Activate(uint32_t id) {
    this->object_id = id;
    this->prop_container = vr::VRProperties()->TrackedDeviceToPropertyContainer(id);

    vr::VRProperties()->SetStringProperty(
        this->prop_container, vr::Prop_TrackingSystemName_String, "oculus");
    vr::VRProperties()->SetStringProperty(
        this->prop_container, vr::Prop_ModelNumber_String, "Miramar");
    vr::VRProperties()->SetStringProperty(
        this->prop_container, vr::Prop_ManufacturerName_String, "Oculus");
    vr::VRProperties()->SetStringProperty(
        this->prop_container, vr::Prop_RenderModelName_String, "generic_hmd");
    vr::VRProperties()->SetStringProperty(
        this->prop_container, vr::Prop_RegisteredDeviceType_String, "oculus/1WMGH000XX0000");
    vr::VRProperties()->SetStringProperty(
        this->prop_container, vr::Prop_DriverVersion_String, "1.55.0");
    vr::VRProperties()->SetFloatProperty(this->prop_container, vr::Prop_UserIpdMeters_Float, 0.063);
    vr::VRProperties()->SetFloatProperty(
        this->prop_container, vr::Prop_UserHeadToEyeDepthMeters_Float, 0.f);
    vr::VRProperties()->SetFloatProperty(
        this->prop_container, vr::Prop_DisplayFrequency_Float, 60.0);
    vr::VRProperties()->SetFloatProperty(
        this->prop_container, vr::Prop_SecondsFromVsyncToPhotons_Float, 0.);
    // vr::VRProperties()->SetFloatProperty(m_ulPropertyContainer,
    // vr::Prop_SecondsFromVsyncToPhotons_Float,
    // Settings::Instance().m_flSecondsFromVsyncToPhotons);

    // return a constant that's not 0 (invalid) or 1 (reserved for Oculus)
    vr::VRProperties()->SetUint64Property(
        this->prop_container, vr::Prop_CurrentUniverseId_Uint64, 2);

#ifdef _WIN32
    // avoid "not fullscreen" warnings from vrmonitor
    vr::VRProperties()->SetBoolProperty(this->prop_container, vr::Prop_IsOnDesktop_Bool, false);

    // Manually send VSync events on direct mode.
    // ref:https://github.com/ValveSoftware/virtual_display/issues/1
    vr::VRProperties()->SetBoolProperty(
        this->prop_container, vr::Prop_DriverDirectModeSendsVsyncEvents_Bool, true);
#endif

    // Set battery as true
    vr::VRProperties()->SetBoolProperty(
        this->prop_container, vr::Prop_DeviceProvidesBatteryStatus_Bool, true);

    // vr::VRProperties()->SetBoolProperty(
    //     this->prop_container, vr::Prop_ContainsProximitySensor_Bool, true);
    // vr::VRDriverInput()->CreateBooleanComponent(this->prop_container, "/proximity",
    // &m_proximity);

#ifdef _WIN32
    float originalIPD =
        vr::VRSettings()->GetFloat(vr::k_pch_SteamVR_Section, vr::k_pch_SteamVR_IPD_Float);
    vr::VRSettings()->SetFloat(vr::k_pch_SteamVR_Section, vr::k_pch_SteamVR_IPD_Float, 0.63);
#endif

    // set the icons in steamvr to the default icons used for Oculus Link
    vr::VRProperties()->SetStringProperty(this->prop_container,
                                          vr::Prop_NamedIconPathDeviceOff_String,
                                          "{oculus}/icons/quest_headset_off.png");
    vr::VRProperties()->SetStringProperty(this->prop_container,
                                          vr::Prop_NamedIconPathDeviceSearching_String,
                                          "{oculus}/icons/quest_headset_searching.gif");
    vr::VRProperties()->SetStringProperty(this->prop_container,
                                          vr::Prop_NamedIconPathDeviceSearchingAlert_String,
                                          "{oculus}/icons/quest_headset_alert_searching.gif");
    vr::VRProperties()->SetStringProperty(this->prop_container,
                                          vr::Prop_NamedIconPathDeviceReady_String,
                                          "{oculus}/icons/quest_headset_ready.png");
    vr::VRProperties()->SetStringProperty(this->prop_container,
                                          vr::Prop_NamedIconPathDeviceReadyAlert_String,
                                          "{oculus}/icons/quest_headset_ready_alert.png");
    vr::VRProperties()->SetStringProperty(this->prop_container,
                                          vr::Prop_NamedIconPathDeviceStandby_String,
                                          "{oculus}/icons/quest_headset_standby.png");

    // TrackedDevice::set_static_props();

    vr::VREvent_Data_t eventData;
    eventData.ipd = {0.063};
    vr::VRServerDriverHost()->VendorSpecificEvent(id, vr::VREvent_IpdChanged, eventData, 0);

    // HMD device is always added before it connects, so disconnect it
    // vr::VRServerDriverHost()->VendorSpecificEvent(id, vr::VREvent_WirelessDisconnect, {}, 0);

    return vr::VRInitError_None;
};

void *Hmd::GetComponent(const char *component_name_and_version) {
    // NB: "this" pointer needs to be statically cast to point to the correct vtable

    auto name_and_vers = std::string(component_name_and_version);
    if (name_and_vers == vr::IVRDisplayComponent_Version) {
        return (vr::IVRDisplayComponent *)this;
    }

#ifdef _WIN32
    if (name_and_vers == vr::IVRDriverDirectModeComponent_Version) {
        return (vr::IVRDriverDirectModeComponent *)this;
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
    auto swapchain = SwapchainData{};
    swapchain.pid = pid;

    for (int idx = 0; idx < 3; idx++) {
        void *handle = nullptr; // HANDLE type
        uint64_t id = alvr_create_texture(swap_texture_set_desc->nWidth,
                                          swap_texture_set_desc->nHeight,
                                          swap_texture_set_desc->nFormat,
                                          swap_texture_set_desc->nSampleCount,
                                          true,
                                          &handle);
        // std::stringstream message;
        // message << "texture handle: " << texture_handle;
        // alvr_popup_error(message.str().c_str());

        auto texture_handle = (vr::SharedTextureHandle_t)handle;

        swapchain.texture_handles[idx] = texture_handle;
        swap_texture_set->rSharedTextureHandles[idx] = texture_handle;
        this->texture_ids[texture_handle] = id;
    }

    this->swapchains[swapchain.texture_handles[0]] = swapchain;
    this->swapchains[swapchain.texture_handles[1]] = swapchain;
    this->swapchains[swapchain.texture_handles[2]] = swapchain;
}

void Hmd::DestroySwapTextureSet(vr::SharedTextureHandle_t shared_texture_handle) {
    auto maybe_entry = this->swapchains.find(shared_texture_handle);

    if (maybe_entry != this->swapchains.end()) {
        auto [_, swapchain] = *maybe_entry;

        for (int idx = 0; idx < 3; idx++) {
            auto texture_handle = swapchain.texture_handles[idx];
            alvr_destroy_texture(this->texture_ids[texture_handle]);
            this->texture_ids.erase(texture_handle);
            this->swapchains.erase(texture_handle);
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

void Hmd::GetNextSwapTextureSetIndex(vr::SharedTextureHandle_t[2], uint32_t (*indices)[2]) {
    (*indices)[0] = ((*indices)[0] + 1) % 3;
    (*indices)[1] = ((*indices)[1] + 1) % 3;
}

void Hmd::SubmitLayer(const SubmitLayerPerEye_t (&eye)[2]) {
    auto layer = AlvrLayer{};
    for (int idx = 0; idx < 2; idx++) {
        layer.views[idx].texture_id = this->texture_ids[eye[idx].hTexture];
        layer.views[idx].fov = this->views_config.fov[idx];
        layer.views[idx].rect_offset.x = eye[idx].bounds.uMin;
        layer.views[idx].rect_offset.y = eye[idx].bounds.vMin;
        layer.views[idx].rect_size.x = eye[idx].bounds.uMax - eye[idx].bounds.uMin;
        layer.views[idx].rect_size.y = eye[idx].bounds.vMax - eye[idx].bounds.vMin;
    }
    this->current_layers.push_back(layer);
}

void Hmd::Present(vr::SharedTextureHandle_t sync_texture) {

    // This call will block until the server finished rendering
    alvr_present_layers(
        (void *)sync_texture, &this->current_layers[0], (uint64_t)this->current_layers.size(), 0);

    this->current_layers.clear();
}

void Hmd::PostPresent() {
    alvr_wait_for_vsync(100); // timeout ms
    vr::VRServerDriverHost()->VsyncEvent(0.0);
}

void Hmd::GetFrameTiming(vr::DriverDirectMode_FrameTiming *frame_timing) {
    // frame_timing->m_nNumFramePresents = 1;
    // frame_timing->m_nNumMisPresented = 0;
    // frame_timing->m_nNumDroppedFrames = 0;
}

void Hmd::update_video_config(AlvrVideoConfig config) {
    alvr_popup_error("update_video_config");
    this->video_config = config;

    vr::VRServerDriverHost()->SetRecommendedRenderTargetSize(
        this->object_id, config.preferred_view_width, config.preferred_view_height);
}

void Hmd::update_views_config(AlvrViewsConfig config) {
    alvr_popup_error("update_views_config");
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