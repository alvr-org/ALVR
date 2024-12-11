#pragma once

#include "TrackedDevice.h"
#include "bindings.h"
#include "openvr_driver_wrap.h"

class FakeViveTracker : public TrackedDevice {
public:
    FakeViveTracker(uint64_t deviceID);
    void OnPoseUpdated(uint64_t targetTimestampNs, const FfiDeviceMotion* motion);

private:
    // TrackedDevice
    bool activate() final;
    void* get_component(const char*) final { return nullptr; }
};
