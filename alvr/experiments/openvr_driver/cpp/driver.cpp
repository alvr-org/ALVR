#include "bindings.h"
#include "tracked_devices.h"
#include <optional>
#include <string>
#include <vector>

const vr::HmdMatrix34_t MATRIX_IDENTITY = {
    {{1.0, 0.0, 0.0, 0.0}, {0.0, 1.0, 0.0, 0.0}, {0.0, 0.0, 1.0, 0.0}}};

class DriverProvider : vr::IServerTrackedDeviceProvider {
  public:
    std::optional<Hmd> hmd;
    std::optional<Controller> left_controller, right_controller;
    std::vector<GenericTracker> generic_trackers;

    // hmd, controllers and trackers in the order specified by the server
    std::vector<TrackedDevice *> tracked_devices;

    virtual vr::EVRInitError Init(vr::IVRDriverContext *pContext) override {
        VR_INIT_SERVER_DRIVER_CONTEXT(pContext);

        auto config = get_initialization_config();

        for (uint64_t idx = 0; idx < config.tracked_devices_count; idx++) {
            TrackedDevice *device;

            if (config.tracked_device_classes[idx] == vr::TrackedDeviceClass_HMD) {
                this->hmd = Hmd(idx, config.presentation, config.config);
                device = &*this->hmd;
            } else if (config.tracked_device_classes[idx] == vr::TrackedDeviceClass_Controller &&
                       config.controller_role[idx] == vr::TrackedControllerRole_LeftHand) {
                this->left_controller = Controller(idx, vr::TrackedControllerRole_LeftHand);
                device = &*this->left_controller;
            } else if (config.tracked_device_classes[idx] == vr::TrackedDeviceClass_Controller &&
                       config.controller_role[idx] == vr::TrackedControllerRole_RightHand) {
                this->right_controller = Controller(idx, vr::TrackedControllerRole_RightHand);
                device = &*this->right_controller;
            } else if (config.tracked_device_classes[idx] ==
                       vr::TrackedDeviceClass_GenericTracker) {
                this->generic_trackers.push_back(GenericTracker(idx));
                device = &this->generic_trackers[this->generic_trackers.size() - 1];
            } else {
                exit(1);
            }

            vr::VRServerDriverHost()->TrackedDeviceAdded(config.tracked_device_serial_numbers[idx],
                                                         config.tracked_device_classes[idx],
                                                         device);
            this->tracked_devices.push_back(device);
        }

        if (!spawn_sse_receiver_loop()) {
            return vr::VRInitError_IPC_ServerInitFailed;
        }

        return vr::VRInitError_None;
    }
    virtual void Cleanup() override {
        stop_sse_receiver();
        VR_CLEANUP_SERVER_DRIVER_CONTEXT();
    }
    virtual const char *const *GetInterfaceVersions() override { return vr::k_InterfaceVersions; }
    virtual const char *GetTrackedDeviceDriverVersion() {
        return vr::ITrackedDeviceServerDriver_Version;
    }
    virtual void RunFrame() override {
        vr::VREvent_t event;
        while (vr::VRServerDriverHost()->PollNextEvent(&event, sizeof(vr::VREvent_t))) {
            if (event.eventType == vr::VREvent_Input_HapticVibration) {
                uint64_t device_index = 0;
                for (auto device : this->tracked_devices) {
                    if (event.data.hapticVibration.containerHandle == device->haptics_container) {
                        device_index = device->device_index;
                        break;
                    }
                }

                send_haptics(device_index, event.data.hapticVibration);
            }
        }
    }
    virtual bool ShouldBlockStandbyMode() override { return false; }
    virtual void EnterStandby() override {}
    virtual void LeaveStandby() override {}

    DriverProvider() {}
} g_driver_provider;

void log(std::string message) {
    auto message_string = std::string("[ALVR] ") + message + "\n";
    vr::VRDriverLog()->Log(message_string.c_str());
}
void log(const char *message) { log(std::string(message)); }

void *entry_point(const char *interface_name, int *return_code) {
    if (std::string(interface_name) == vr::IServerTrackedDeviceProvider_Version) {
        *return_code = vr::VRInitError_None;
        return &g_driver_provider;
    } else {
        *return_code = vr::VRInitError_Init_InterfaceNotFound;
        return nullptr;
    }
}

vr::PropertyContainerHandle_t container(uint64_t device_index) {
    return g_driver_provider.tracked_devices[device_index]->prop_container;
}

void handle_prop_error(vr::ETrackedPropertyError error) {
    if (error != vr::TrackedProp_Success) {
        log(std::string("Error setting property: ") +
            vr::VRPropertiesRaw()->GetPropErrorNameFromEnum(error));
    }
}
void set_bool_property(uint64_t device_index, vr::ETrackedDeviceProperty prop, bool value) {
    handle_prop_error(vr::VRProperties()->SetBoolProperty(container(device_index), prop, value));
}
void set_float_property(uint64_t device_index, vr::ETrackedDeviceProperty prop, float value) {
    handle_prop_error(vr::VRProperties()->SetFloatProperty(container(device_index), prop, value));
}
void set_int32_property(uint64_t device_index, vr::ETrackedDeviceProperty prop, int32_t value) {
    handle_prop_error(vr::VRProperties()->SetInt32Property(container(device_index), prop, value));
}
void set_uint64_property(uint64_t device_index, vr::ETrackedDeviceProperty prop, uint64_t value) {
    handle_prop_error(vr::VRProperties()->SetUint64Property(container(device_index), prop, value));
}
void set_vec3_property(uint64_t device_index,
                       vr::ETrackedDeviceProperty prop,
                       const vr::HmdVector3_t &value) {
    handle_prop_error(vr::VRProperties()->SetVec3Property(container(device_index), prop, value));
}
void set_double_property(uint64_t device_index, vr::ETrackedDeviceProperty prop, double value) {
    handle_prop_error(vr::VRProperties()->SetDoubleProperty(container(device_index), prop, value));
}
void set_string_property(uint64_t device_index,
                         vr::ETrackedDeviceProperty prop,
                         const char *value) {
    handle_prop_error(vr::VRProperties()->SetStringProperty(container(device_index), prop, value));
}

void handle_input_error(vr::EVRInputError error) {
    if (error != vr::VRInputError_None) {
        log(std::string("Error setting input: code=") + std::to_string(error));
    }
}
vr::VRInputComponentHandle_t create_boolean_component(uint64_t device_index, const char *path) {
    vr::VRInputComponentHandle_t handle;
    handle_input_error(
        vr::VRDriverInput()->CreateBooleanComponent(container(device_index), path, &handle));
    return handle;
}
void update_boolean_component(vr::VRInputComponentHandle_t component, bool value) {
    handle_input_error(vr::VRDriverInput()->UpdateBooleanComponent(component, value, 0));
}
vr::VRInputComponentHandle_t
create_scalar_component(uint64_t device_index, const char *path, vr::EVRScalarUnits units) {
    vr::VRInputComponentHandle_t handle;
    handle_input_error(vr::VRDriverInput()->CreateScalarComponent(
        container(device_index), path, &handle, vr::VRScalarType_Absolute, units));
    return handle;
}
void update_scalar_component(vr::VRInputComponentHandle_t component, float value) {
    handle_input_error(vr::VRDriverInput()->UpdateScalarComponent(component, value, 0));
}

void update_config(DriverConfigUpdate config) {
    auto object_id = g_driver_provider.hmd->object_id;

    g_driver_provider.hmd->config = config;

    vr::VRServerDriverHost()->SetRecommendedRenderTargetSize(
        object_id, config.preferred_view_width, config.preferred_view_height);

    auto left_transform = MATRIX_IDENTITY;
    left_transform.m[0][3] = -config.ipd_m / 2.0;
    auto right_transform = MATRIX_IDENTITY;
    right_transform.m[0][3] = -config.ipd_m / 2.0;
    vr::VRServerDriverHost()->SetDisplayEyeToHead(object_id, left_transform, right_transform);

    vr::VRServerDriverHost()->SetDisplayProjectionRaw(object_id, config.fov[0], config.fov[1]);

    // todo: check if this is still needed
    vr::VRServerDriverHost()->VendorSpecificEvent(
        object_id, vr::VREvent_LensDistortionChanged, {}, 0);
}

void set_tracking_data(const vr::DriverPose_t *poses, uint32_t count) {
    for (uint32_t idx = 0; idx < count; idx++) {
        auto tracked_device = g_driver_provider.tracked_devices[idx];

        tracked_device->pose = poses[idx];

        vr::VRServerDriverHost()->TrackedDevicePoseUpdated(
            tracked_device->object_id, poses[idx], sizeof(vr::DriverPose_t));
    }
}

void vendor_event(vr::EVREventType event_type) {
    vr::VRServerDriverHost()->VendorSpecificEvent(0, event_type, {}, 0);
}

void restart() { vr::VRServerDriverHost()->RequestRestart("Restarting for ALVR", "", "", ""); }

// Fuctions provided by Rust
bool (*spawn_sse_receiver_loop)();
void (*stop_sse_receiver)();
InitializationConfig (*get_initialization_config)();
void (*set_extra_properties)(uint64_t);
void (*set_button_layout)(uint64_t);
void (*send_haptics)(uint64_t, vr::VREvent_HapticVibration_t);
SwapchainData (*create_swapchain)(uint32_t, vr::IVRDriverDirectModeComponent::SwapTextureSetDesc_t);
void (*destroy_swapchain)(uint64_t);
uint32_t (*next_swapchain_index)(uint64_t);
void (*present)(const Layer *, uint32_t);
