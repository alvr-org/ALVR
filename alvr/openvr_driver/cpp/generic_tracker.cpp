#include "generic_tracker.h"

GenericTracker::GenericTracker(uint64_t device_path, const char *serial_number)
    : TrackedDevice(device_path) {
    vr::VRServerDriverHost()->TrackedDeviceAdded(
        serial_number, vr::TrackedDeviceClass_GenericTracker, this);
}

vr::EVRInitError GenericTracker::Activate(uint32_t id) {
    TrackedDevice::Activate(id);

    TrackedDevice::set_static_props();

    return vr::VRInitError_None;
}
