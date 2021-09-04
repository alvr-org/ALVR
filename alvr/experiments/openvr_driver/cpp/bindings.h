#pragma once

#include "openvr_driver.h"
#include <stdint.h>

struct Fov {
    float left;
    float right;
    float top;
    float bottom;
};

struct DriverConfigUpdate {
    uint32_t preferred_view_width;
    uint32_t preferred_view_height;
    Fov fov[2];
    float ipd_m;
    float fps;
};

struct TrackerConfig {
    unsigned int type_and_index; // 0: left, 1: right, >1: generic - 2
};

struct InitializationConfig {
    const char tracked_device_serial_numbers[10][20];
    vr::ETrackedDeviceClass tracked_device_classes[10];
    vr::ETrackedControllerRole controller_role[10];
    uint64_t tracked_devices_count;
    bool presentation;
    DriverConfigUpdate config;
};

struct MotionData {
    bool connected;
    double position[3];
    vr::HmdQuaternion_t orientation;
    double linear_velocity[3];
    double angular_velocity[3];
    bool has_linear_velocity;
    bool has_angular_velocity;
};

// Fuctions provided by Rust
extern "C" bool (*spawn_sse_receiver_loop)();
extern "C" InitializationConfig (*get_initialization_config)();
extern "C" void (*set_extra_properties)(uint64_t device_index);

// This is our only way of logging. OpenVR does not have severity levels
extern "C" void log(const char *message);

extern "C" void
set_bool_property(uint64_t device_index, vr::ETrackedDeviceProperty prop, bool value);
extern "C" void
set_float_property(uint64_t device_index, vr::ETrackedDeviceProperty prop, float value);
extern "C" void
set_int32_property(uint64_t device_index, vr::ETrackedDeviceProperty prop, int32_t value);
extern "C" void
set_uint64_property(uint64_t device_index, vr::ETrackedDeviceProperty prop, uint64_t value);
extern "C" void set_vec3_property(uint64_t device_index,
                                  vr::ETrackedDeviceProperty prop,
                                  const vr::HmdVector3_t &value);
extern "C" void
set_double_property(uint64_t device_index, vr::ETrackedDeviceProperty prop, double value);
extern "C" void
set_string_property(uint64_t device_index, vr::ETrackedDeviceProperty prop, const char *value);

extern "C" void *entry_point(const char *interface_name, int *return_code);
