#include "controller.h"
#include "paths.h"

Controller::Controller(uint64_t device_path, uint64_t profile_path) : TrackedDevice(device_path) {
    if (device_path == LEFT_HAND_PATH) {
        this->role = vr::TrackedControllerRole_LeftHand;
    } else if (device_path == LEFT_HAND_PATH) {
        this->role = vr::TrackedControllerRole_RightHand;
    }

    char serial_number[64];
    alvr_get_serial_number(device_path, serial_number, 64);

    vr::VRServerDriverHost()->TrackedDeviceAdded(
        serial_number, vr::TrackedDeviceClass_Controller, this);
}

vr::EVRInitError Controller::Activate(uint32_t id) {
    this->object_id = id;
    this->prop_container = vr::VRProperties()->TrackedDeviceToPropertyContainer(id);

    vr::VRProperties()->SetInt32Property(
        this->prop_container, vr::Prop_ControllerRoleHint_Int32, this->role);

    TrackedDevice::set_static_props();

    vr::VRDriverInput()->CreateHapticComponent(
        this->prop_container, "/output/haptic", &this->haptics_container);

    return vr::VRInitError_None;
}

void Controller::try_update_button(AlvrButtonInput input) {
    // todo
}

void Controller::update_hand_skeleton(AlvrMotionData data[25], uint64_t timestamp_ns) {
    // todo
}