#include "bindings.h"
#include "controller.h"
#include "generic_tracker.h"
#include "hmd.h"
#include "tracked_device.h"

#include <optional>
#include <string>
#include <vector>

class DriverProvider : vr::IServerTrackedDeviceProvider {
    virtual vr::EVRInitError Init(vr::IVRDriverContext *pContext) override {
        VR_INIT_SERVER_DRIVER_CONTEXT(pContext);

        if (!spawn_sse_receiver_loop()) {
            return vr::VRInitError_IPC_ServerInitFailed;
        }

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
                continue;
            }

            vr::VRServerDriverHost()->TrackedDeviceAdded(config.tracked_device_serial_numbers[idx],
                                                         config.tracked_device_classes[idx],
                                                         device);
            this->tracked_devices.push_back(device);
        }

        return vr::VRInitError_None;
    }
    virtual void Cleanup() override { VR_CLEANUP_SERVER_DRIVER_CONTEXT(); }
    virtual const char *const *GetInterfaceVersions() override { return vr::k_InterfaceVersions; }
    virtual const char *GetTrackedDeviceDriverVersion() {
        return vr::ITrackedDeviceServerDriver_Version;
    }
    virtual void RunFrame() override {}
    virtual bool ShouldBlockStandbyMode() override { return false; }
    virtual void EnterStandby() override {}
    virtual void LeaveStandby() override {}

  public:
    std::optional<Hmd> hmd;
    std::optional<Controller> left_controller, right_controller;
    std::vector<GenericTracker> generic_trackers;

    // hmd, controllers and trackers in the order specified by the server
    std::vector<TrackedDevice *> tracked_devices;

    DriverProvider() {}
} g_driver_provider;

void *entry_point(const char *interface_name, int *return_code) {
    if (std::string(interface_name) == vr::IServerTrackedDeviceProvider_Version) {
        *return_code = vr::VRInitError_None;
        return &g_driver_provider;
    } else {
        *return_code = vr::VRInitError_Init_InterfaceNotFound;
        return nullptr;
    }
}

void log(const char *message) {
    auto message_string = std::string("[ALVR] ") + message;
    vr::VRDriverLog()->Log(message_string.c_str());
}

void handle_prop_error(vr::ETrackedPropertyError error) {
    log(vr::VRPropertiesRaw()->GetPropErrorNameFromEnum(error));
}
void set_bool_property(uint64_t device_index, vr::ETrackedDeviceProperty prop, bool value) {
    auto container = g_driver_provider.tracked_devices[device_index]->get_prop_container();
    handle_prop_error(vr::VRProperties()->SetBoolProperty(container, prop, value));
}
void set_float_property(uint64_t device_index, vr::ETrackedDeviceProperty prop, float value) {
    auto container = g_driver_provider.tracked_devices[device_index]->get_prop_container();
    handle_prop_error(vr::VRProperties()->SetFloatProperty(container, prop, value));
}
void set_int32_property(uint64_t device_index, vr::ETrackedDeviceProperty prop, int32_t value) {
    auto container = g_driver_provider.tracked_devices[device_index]->get_prop_container();
    handle_prop_error(vr::VRProperties()->SetInt32Property(container, prop, value));
}
void set_uint64_property(uint64_t device_index, vr::ETrackedDeviceProperty prop, uint64_t value) {
    auto container = g_driver_provider.tracked_devices[device_index]->get_prop_container();
    handle_prop_error(vr::VRProperties()->SetUint64Property(container, prop, value));
}
void set_vec3_property(uint64_t device_index,
                       vr::ETrackedDeviceProperty prop,
                       const vr::HmdVector3_t &value) {
    auto container = g_driver_provider.tracked_devices[device_index]->get_prop_container();
    handle_prop_error(vr::VRProperties()->SetVec3Property(container, prop, value));
}
void set_double_property(uint64_t device_index, vr::ETrackedDeviceProperty prop, double value) {
    auto container = g_driver_provider.tracked_devices[device_index]->get_prop_container();
    handle_prop_error(vr::VRProperties()->SetDoubleProperty(container, prop, value));
}
void set_string_property(uint64_t device_index,
                         vr::ETrackedDeviceProperty prop,
                         const char *value) {
    auto container = g_driver_provider.tracked_devices[device_index]->get_prop_container();
    handle_prop_error(vr::VRProperties()->SetStringProperty(container, prop, value));
}

void set_motion_data(MotionData *data, size_t count, double time_offset_s) {
    for (size_t idx = 0; idx < count; idx++) {
        g_driver_provider.tracked_devices[idx]->set_motion(data[idx], time_offset_s);
    }
}

// Fuctions provided by Rust
bool (*spawn_sse_receiver_loop)();
InitializationConfig (*get_initialization_config)();
void (*set_extra_properties)(uint64_t);
SwapchainData (*create_swapchain)(uint32_t, vr::IVRDriverDirectModeComponent::SwapTextureSetDesc_t);
void (*destroy_swapchain)(uint64_t);
uint32_t (*next_swapchain_index)(uint64_t);
void (*present)(const Layer *, uint32_t);