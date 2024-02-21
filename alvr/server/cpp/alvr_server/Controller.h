#pragma once

#include "ALVR-common/packet_types.h"
#include "TrackedDevice.h"
#include "openvr_driver.h"
#include <map>

class Controller final : public TrackedDevice {
  public:
    Controller(uint64_t device_id);

    vr::VRInputComponentHandle_t getHapticComponent();

    void RegisterButton(uint64_t id);

    void SetButton(uint64_t id, FfiButtonValue value);

    void UpdateTracking(FfiDeviceMotion motion, const FfiHandSkeleton *hand);

    void GetBoneTransform(bool withController, vr::VRBoneTransform_t outBoneTransform[]);

    vr::ETrackedDeviceClass getControllerDeviceClass();

    // ITrackedDeviceServerDriver

    virtual vr::EVRInitError Activate(vr::TrackedDeviceIndex_t object_id);

  private:
    static const int SKELETON_BONE_COUNT = 31;
    static const int ANIMATION_FRAME_COUNT = 15;

    std::map<uint64_t, vr::VRInputComponentHandle_t> m_buttonHandles;

    vr::VRInputComponentHandle_t m_compHaptic = vr::k_ulInvalidInputComponentHandle;
    vr::VRInputComponentHandle_t m_compSkeleton = vr::k_ulInvalidInputComponentHandle;

    // These variables are used for controller hand animation
    // todo: move to rust
    float m_thumbTouchAnimationProgress = 0;
    float m_indexTouchAnimationProgress = 0;
    bool m_currentThumbTouch = false;
    bool m_lastThumbTouch = false;
    bool m_currentTriggerTouch = false;
    bool m_lastTriggerTouch = false;
    float m_triggerValue = 0;
    float m_gripValue = 0;
};
