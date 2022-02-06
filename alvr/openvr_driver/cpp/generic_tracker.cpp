#include "generic_tracker.h"

GenericTracker::GenericTracker(uint64_t device_path) : TrackedDevice(device_path) {
    char serial_number[64];
    alvr_get_serial_number(device_path, serial_number, 64);
    vr::VRServerDriverHost()->TrackedDeviceAdded(
        serial_number, vr::TrackedDeviceClass_GenericTracker, this);
}

vr::EVRInitError GenericTracker::Activate(uint32_t id) {
    this->object_id = id;
    this->prop_container = vr::VRProperties()->TrackedDeviceToPropertyContainer(id);

    TrackedDevice::set_static_props();

    return vr::VRInitError_None;
}
