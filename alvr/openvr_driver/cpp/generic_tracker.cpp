#include "generic_tracker.h"

GenericTracker::GenericTracker(uint64_t device_path, const char *serial_number)
    : TrackedDevice(device_path) {
    vr::VRServerDriverHost()->TrackedDeviceAdded(
        serial_number, vr::TrackedDeviceClass_GenericTracker, this);
}

vr::EVRInitError GenericTracker::Activate(uint32_t id) {
    TrackedDevice::Activate(id);

    set_static_properties(this->device_path, this->prop_container);

    return vr::VRInitError_None;
}
