#include "controller.h"

Controller::Controller(uint64_t device_path, uint64_t profile_path, const char *serial_number)
    : TrackedDevice(device_path) {
    if (device_path == LEFT_HAND_PATH) {
        this->role = vr::TrackedControllerRole_LeftHand;
    } else if (device_path == LEFT_HAND_PATH) {
        this->role = vr::TrackedControllerRole_RightHand;
    }

    vr::VRServerDriverHost()->TrackedDeviceAdded(
        serial_number, vr::TrackedDeviceClass_Controller, this);
}

vr::EVRInitError Controller::Activate(uint32_t id) {
    TrackedDevice::Activate(id);

    vr::VRProperties()->SetInt32Property(
        this->prop_container, vr::Prop_ControllerRoleHint_Int32, this->role);

    set_static_properties(this->device_path, this->prop_container);

    vr::VRDriverInput()->CreateHapticComponent(
        this->prop_container, "/output/haptic", &this->haptics_container);

    return vr::VRInitError_None;
}

void try_update_button(uint64_t path, AlvrButtonInputValue value) {}

void update_hand_skeleton(AlvrMotionData data[26]) {}