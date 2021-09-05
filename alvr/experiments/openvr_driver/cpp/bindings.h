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

struct SwapchainData {
    uint64_t id;
    uint32_t pid;
    vr::SharedTextureHandle_t texture_handles[3];
};

struct Layer {
    uint64_t swapchain_ids[2];
    Fov fov[2];
    vr::VRTextureBounds_t bounds[2];
    vr::HmdMatrix34_t poses[2];
};

// This is our only way of logging. OpenVR does not have severity levels
extern "C" void log(const char *message);

extern "C" void *entry_point(const char *interface_name, int *return_code);

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

// Functions provided by Rust
extern "C" bool (*spawn_sse_receiver_loop)();
extern "C" InitializationConfig (*get_initialization_config)();
extern "C" void (*set_extra_properties)(uint64_t device_index);
extern "C" SwapchainData (*create_swapchain)(
    uint32_t pid, vr::IVRDriverDirectModeComponent::SwapTextureSetDesc_t desc);
extern "C" void (*destroy_swapchain)(uint64_t id);
extern "C" uint32_t (*next_swapchain_index)(uint64_t id);
extern "C" void (*present)(const Layer *layers, uint32_t count);