#pragma once

#include "alvr_streamer.h"
#include "openvr_driver.h"
#include "tracked_devices.h"
#include <map>

const uint64_t LEFT_HAND_PATH = alvr_path_string_to_hash("/user/hand/left");
const uint64_t RIGHT_HAND_PATH = alvr_path_string_to_hash("/user/hand/right");

const uint64_t OCULUS_CONTROLLER_PROFILE_PATH =
    alvr_path_string_to_hash("/interaction_profiles/oculus/touch_controller");
const uint64_t INDEX_CONTROLLER_PROFILE_PATH =
    alvr_path_string_to_hash("/interaction_profiles/valve/index_controller");
const uint64_t VIVE_CONTROLLER_PROFILE_PATH =
    alvr_path_string_to_hash("/interaction_profiles/htc/vive_controller");

class Controller : public TrackedDevice {
  public:
    uint64_t profile_path;
    vr::ETrackedControllerRole role = vr::TrackedControllerRole_Invalid;
    vr::PropertyContainerHandle_t haptics_container;

    virtual vr::EVRInitError Activate(uint32_t object_id) override;
    Controller(uint64_t device_path, uint64_t profile_path, const char *serial_number);

    void try_update_button(AlvrButtonInput input);
    void update_hand_skeleton(AlvrMotionData data[25], uint64_t timestamp_ns);
};