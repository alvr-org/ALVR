#pragma once

#include "alvr_streamer.h"
#include "openvr_driver.h"
#include "tracked_device.h"
#include <map>

class Controller : public TrackedDevice {
  public:
    uint64_t profile_path;
    vr::ETrackedControllerRole role = vr::TrackedControllerRole_Invalid;
    vr::PropertyContainerHandle_t haptics_container;

    virtual vr::EVRInitError Activate(uint32_t object_id) override;
    Controller(uint64_t device_path, uint64_t profile_path);

    void try_update_button(AlvrButtonInput input);
    void update_hand_skeleton(AlvrMotionData data[25], uint64_t timestamp_ns);
};