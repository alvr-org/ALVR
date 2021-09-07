#pragma once

#include "bindings.h"
#include "openvr_driver.h"
#include <chrono>
#include <map>

class TrackedDevice : public vr::ITrackedDeviceServerDriver {
  public:
    uint64_t device_index;              // index dictated by the server
    vr::TrackedDeviceIndex_t object_id; // index dictated by SteamVR
    vr::PropertyContainerHandle_t prop_container;
    vr::DriverPose_t pose;
    vr::PropertyContainerHandle_t haptics_container;
    // Note: the button containers are stored on the Rust side

    virtual vr::EVRInitError Activate(uint32_t object_id) override;
    virtual void *GetComponent(const char *component_name_and_version) override { return nullptr; }
    virtual void Deactivate() override {}
    virtual void EnterStandby() override {}
    virtual void DebugRequest(const char *request,
                              char *response_buffer,
                              uint32_t response_buffer_size) override {}
    virtual vr::DriverPose_t GetPose() override { return this->pose; }

    TrackedDevice(uint64_t device_index) : device_index(device_index) {
        this->pose.result = vr::TrackingResult_Uninitialized;
    }
    virtual void activate_inner() {}
};

class Hmd : public TrackedDevice, vr::IVRDisplayComponent, vr::IVRDriverDirectModeComponent {
  public:
    bool do_presentation;
    DriverConfigUpdate config;
    std::chrono::steady_clock::time_point next_virtual_vsync;
    std::vector<Layer> current_layers; // reset after every Present()

    // map texture handles to their swapchain, which can be repeated
    std::map<vr::SharedTextureHandle_t, SwapchainData> swapchains;

    // TrackedDevice
    virtual void activate_inner() override;
    virtual void *GetComponent(const char *component_name_and_version) override;

    // IVRDisplayComponent
    virtual void
    GetWindowBounds(int32_t *x, int32_t *y, uint32_t *width, uint32_t *height) override;
    virtual bool IsDisplayOnDesktop() override { return false; }
    virtual bool IsDisplayRealDisplay() override { return true; }
    virtual void GetRecommendedRenderTargetSize(uint32_t *width, uint32_t *height) override;
    virtual void GetEyeOutputViewport(
        vr::EVREye eye, uint32_t *x, uint32_t *y, uint32_t *width, uint32_t *height) override;
    virtual void
    GetProjectionRaw(vr::EVREye eye, float *left, float *right, float *top, float *bottom) override;
    virtual vr::DistortionCoordinates_t
    ComputeDistortion(vr::EVREye eye, float u, float v) override;

    // IVRDriverDirectModeComponent
    virtual void CreateSwapTextureSet(uint32_t pid,
                                      const SwapTextureSetDesc_t *swap_texture_set_desc,
                                      SwapTextureSet_t *swap_texture_set) override;
    virtual void DestroySwapTextureSet(vr::SharedTextureHandle_t shared_texture_handle) override;
    virtual void DestroyAllSwapTextureSets(uint32_t pid) override;
    virtual void GetNextSwapTextureSetIndex(vr::SharedTextureHandle_t shared_texture_handles[2],
                                            uint32_t (*indices)[2]) override;
    virtual void SubmitLayer(const SubmitLayerPerEye_t (&eye)[2]) override;
    virtual void Present(vr::SharedTextureHandle_t sync_texture) override;
    virtual void PostPresent() override;
    virtual void GetFrameTiming(vr::DriverDirectMode_FrameTiming *frame_timing) override;

    Hmd(uint64_t device_index, bool do_presentation, DriverConfigUpdate config)
        : TrackedDevice(device_index), do_presentation(do_presentation), config(config) {}
};

class Controller : public TrackedDevice {
  public:
    vr::ETrackedControllerRole role;

    virtual void activate_inner() override;

    Controller(uint64_t device_index, vr::ETrackedControllerRole role)
        : TrackedDevice(device_index), role(role) {}
};

class GenericTracker : public TrackedDevice {
  public:
    GenericTracker(uint64_t device_index) : TrackedDevice(device_index){};
};
