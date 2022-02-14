#pragma once

#include "ALVR-common/packet_types.h"
#include "TrackedDevice.h"
#include "openvr_driver.h"
#include <memory>
#ifdef _WIN32
#include "platform/win32/OvrDirectModeComponent.h"
#endif

class ClientConnection;
class VSyncThread;

class OvrController;
class OvrController;
class OvrViveTrackerProxy;

class CEncoder;
#ifdef _WIN32
class CD3DRender;
#endif
class PoseHistory;

class OvrHmd : public TrackedDevice, vr::IVRDisplayComponent {
  public:
    OvrHmd();

    virtual ~OvrHmd();

    std::string GetSerialNumber() const;

    virtual vr::EVRInitError Activate(vr::TrackedDeviceIndex_t unObjectId);
    virtual void Deactivate();
    virtual void EnterStandby() {}
    void *GetComponent(const char *pchComponentNameAndVersion);
    virtual void DebugRequest(const char *request, char *response_buffer, uint32_t size) {}
    virtual vr::DriverPose_t GetPose();

    void OnPoseUpdated(TrackingInfo info);

    void StartStreaming();

    void OnStreamStart();

    void updateController(const TrackingInfo &info);

    void SetViewsConfig(ViewsConfigData config);

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

    std::shared_ptr<ClientConnection> m_Listener;
    float m_poseTimeOffset;

    vr::VRInputComponentHandle_t m_proximity;

    std::shared_ptr<OvrController> m_leftController;
    std::shared_ptr<OvrController> m_rightController;

    std::shared_ptr<CEncoder> m_encoder;

    TrackingInfo m_TrackingInfo;
  private:
    ViewsConfigData views_config;

    bool m_baseComponentsInitialized;
    bool m_streamComponentsInitialized;
    vr::ETrackedDeviceClass m_deviceClass;

    std::wstring m_adapterName;

#ifdef _WIN32
    std::shared_ptr<CD3DRender> m_D3DRender;
#endif
    std::shared_ptr<VSyncThread> m_VSyncThread;

#ifdef _WIN32
    std::shared_ptr<OvrDirectModeComponent> m_directModeComponent;
#endif
    std::shared_ptr<PoseHistory> m_poseHistory;

    std::shared_ptr<OvrViveTrackerProxy> m_viveTrackerProxy;
};
