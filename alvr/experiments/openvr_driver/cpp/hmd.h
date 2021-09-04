#pragma once

#include "bindings.h"
#include "tracked_device.h"

class Hmd : public TrackedDevice, vr::IVRDisplayComponent, vr::IVRDriverDirectModeComponent {
    bool presentation;
    DriverConfigUpdate config;

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
    virtual void CreateSwapTextureSet(
        uint32_t pid,
        const vr::IVRDriverDirectModeComponent::SwapTextureSetDesc_t *swap_texture_set_desc,
        vr::IVRDriverDirectModeComponent::SwapTextureSet_t *swap_texture_set) override {}
    virtual void DestroySwapTextureSet(vr::SharedTextureHandle_t shared_texture_handle) override {}
    virtual void DestroyAllSwapTextureSets(uint32_t pid) override {}
    virtual void GetNextSwapTextureSetIndex(vr::SharedTextureHandle_t shared_texture_handles[2],
                                            uint32_t (*indices)[2]) override {}
    virtual void
    SubmitLayer(const vr::IVRDriverDirectModeComponent::SubmitLayerPerEye_t (&eye)[2]) override {}
    virtual void Present(vr::SharedTextureHandle_t syncTexture) override {}
    virtual void PostPresent() override {}
    virtual void GetFrameTiming(vr::DriverDirectMode_FrameTiming *pFrameTiming) override {}

  public:
    Hmd(uint64_t device_index, bool presentation, DriverConfigUpdate config)
        : TrackedDevice(device_index), presentation(presentation), config(config) {}
};
