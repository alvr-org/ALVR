#pragma once

#include "ALVR-common/packet_types.h"
#include "TrackedDevice.h"
#include "openvr_driver_wrap.h"
#include <map>

class Controller : public TrackedDevice {
public:
    Controller(uint64_t deviceID, vr::EVRSkeletalTrackingLevel skeletonLevel);
    virtual ~Controller() {};
    void RegisterButton(uint64_t id);
    void SetButton(uint64_t id, FfiButtonValue value);
    /// Whether this is one of the separate hand tracking devices rather than a controller, which
    /// decides which input profile's components it publishes.
    bool isHandTracker() const;
    bool OnPoseUpdate(uint64_t targetTimestampNs, float predictionS, FfiHandData handData);

private:
    static const int SKELETON_BONE_COUNT = 31;
    static const int ANIMATION_FRAME_COUNT = 15;

    std::map<uint64_t, vr::VRInputComponentHandle_t> m_buttonHandles;

    vr::VRInputComponentHandle_t m_compHaptic;
    vr::VRInputComponentHandle_t m_compSkeleton = vr::k_ulInvalidInputComponentHandle;
    // /pose/tip on the hand tracking devices: where the profile's render model, the pointer ray and
    // the poke cursor are drawn. Left invalid on controllers, whose profiles define their own tip.
    vr::VRInputComponentHandle_t m_compTip = vr::k_ulInvalidInputComponentHandle;
    vr::EVRSkeletalTrackingLevel m_skeletonLevel;

    uint64_t m_poseTargetTimestampNs;

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

    vr::VRInputComponentHandle_t getHapticComponent();
    void GetBoneTransform(bool withController, vr::VRBoneTransform_t outBoneTransform[]);

    // TrackedDevice
    bool activate() final;
    void* get_component([[maybe_unused]] const char* component_name_and_version) final {
        return nullptr;
    }
};
