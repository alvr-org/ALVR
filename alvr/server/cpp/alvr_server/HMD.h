#pragma once

#include "ALVR-common/packet_types.h"
#include "TrackedDevice.h"
#include "openvr_driver.h"
#include <memory>
#ifdef _WIN32
#include "platform/win32/OvrDirectModeComponent.h"
#endif

class Controller;
class Controller;
class ViveTrackerProxy;

class CEncoder;
#ifdef _WIN32
class CD3DRender;
#endif
class PoseHistory;

class Hmd : public TrackedDevice, public vr::ITrackedDeviceServerDriver, vr::IVRDisplayComponent {
  public:
    Hmd();

    virtual ~Hmd();

    virtual vr::EVRInitError Activate(vr::TrackedDeviceIndex_t unObjectId);
    virtual void Deactivate();
    virtual void EnterStandby() {}
    void *GetComponent(const char *pchComponentNameAndVersion);
    virtual void DebugRequest(const char *, char *, uint32_t) {}
    virtual vr::DriverPose_t GetPose();

    void OnPoseUpdated(uint64_t targetTimestampNs, FfiDeviceMotion motion);

    void StartStreaming();

    void StopStreaming();

    void SetViewsConfig(FfiViewsConfig config);

    vr::ETrackedDeviceClass GetDeviceClass() const { return m_deviceClass; }
    bool IsTrackingRef() const { return m_deviceClass == vr::TrackedDeviceClass_TrackingReference; }
    bool IsHMD() const { return m_deviceClass == vr::TrackedDeviceClass_HMD; }

    // IVRDisplayComponent

    virtual void GetWindowBounds(int32_t *x, int32_t *y, uint32_t *width, uint32_t *height);
    virtual bool IsDisplayOnDesktop() { return false; }
    virtual bool IsDisplayRealDisplay();
    virtual void GetRecommendedRenderTargetSize(uint32_t *width, uint32_t *height);
    virtual void GetEyeOutputViewport(
        vr::EVREye eye, uint32_t *x, uint32_t *y, uint32_t *width, uint32_t *height);
    virtual void
    GetProjectionRaw(vr::EVREye eEye, float *pfLeft, float *pfRight, float *pfTop, float *pfBottom);
    virtual vr::DistortionCoordinates_t ComputeDistortion(vr::EVREye eEye, float fU, float fV);

    vr::VRInputComponentHandle_t m_proximity;

    std::shared_ptr<CEncoder> m_encoder;
    std::shared_ptr<PoseHistory> m_poseHistory;

  private:
    FfiViewsConfig views_config;

    bool m_baseComponentsInitialized;
    bool m_streamComponentsInitialized;
    vr::ETrackedDeviceClass m_deviceClass;

    vr::HmdMatrix34_t m_eyeToHeadLeft;
    vr::HmdMatrix34_t m_eyeToHeadRight;
    vr::HmdRect2_t m_eyeFoVLeft;
    vr::HmdRect2_t m_eyeFoVRight;

    std::wstring m_adapterName;

#ifdef _WIN32
    std::shared_ptr<CD3DRender> m_D3DRender;
#endif

#ifdef _WIN32
    std::shared_ptr<OvrDirectModeComponent> m_directModeComponent;
#endif

    std::shared_ptr<ViveTrackerProxy> m_viveTrackerProxy;

    vr::DriverPose_t m_pose = {};

#ifndef _WIN32
    bool m_refreshRateSet = false;
#endif
};
