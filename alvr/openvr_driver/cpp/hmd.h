#pragma once

extern "C" {
#include "alvr_streamer.h"
}
#include "openvr_driver.h"
#include "tracked_devices.h"
#include <map>

struct SwapchainData {
    uint32_t pid;
    vr::SharedTextureHandle_t texture_handles[3];
};

class Hmd : public TrackedDevice, vr::IVRDisplayComponent, vr::IVRDriverDirectModeComponent {
  public:
    AlvrVideoConfig video_config;
    AlvrViewsConfig views_config;
    std::vector<AlvrLayer> current_layers; // reset after every Present()

    std::map<vr::SharedTextureHandle_t, uint64_t> texture_ids;
    // Note: each swapchain is repeated 3 times, one for each handle it contains
    std::map<vr::SharedTextureHandle_t, SwapchainData> swapchains;

    // TrackedDevice
    virtual vr::EVRInitError Activate(uint32_t object_id) override;
    virtual void *GetComponent(const char *component_name_and_version) override;

    // IVRDisplayComponent
    virtual void
    GetWindowBounds(int32_t *x, int32_t *y, uint32_t *width, uint32_t *height) override;
    virtual bool IsDisplayOnDesktop() override { return false; }
    virtual bool IsDisplayRealDisplay() override {
#ifdef _WIN32
        return false;
#else
        return true;
#endif
    }
    virtual void GetRecommendedRenderTargetSize(uint32_t *width, uint32_t *height) override;
    virtual void GetEyeOutputViewport(
        vr::EVREye eye, uint32_t *x, uint32_t *y, uint32_t *width, uint32_t *height) override;
    virtual void
    GetProjectionRaw(vr::EVREye eye, float *left, float *right, float *top, float *bottom) override;
    virtual vr::DistortionCoordinates_t
    ComputeDistortion(vr::EVREye eye, float u, float v) override;

    // IVRDriverDirectModeComponent (unused on Linux)
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

    Hmd();

    void update_video_config(AlvrVideoConfig config);
    void update_views_config(AlvrViewsConfig config);
};