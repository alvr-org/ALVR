#include "alvr_streamer.h"
#include "bindings.h"
#include "chaperone.h"
#include "controller.h"
#include "generic_tracker.h"
#include "hmd.h"
#include "paths.h"
#include "tracked_device.h"
#include <map>
#include <optional>
#include <string>
#include <thread>
#include <vector>

void rendering_statistics(float *render_ms, float *idle_ms, float *wait_ms) {
    vr::Compositor_FrameTiming timing[2];
    timing[0].m_nSize = sizeof(vr::Compositor_FrameTiming);
    vr::VRServerDriverHost()->GetFrameTimings(&timing[0], 2);

    *render_ms = timing[0].m_flPreSubmitGpuMs + timing[0].m_flPostSubmitGpuMs +
                 timing[0].m_flTotalRenderGpuMs + timing[0].m_flCompositorRenderGpuMs +
                 timing[0].m_flCompositorRenderCpuMs;
    *idle_ms = timing[0].m_flCompositorIdleCpuMs;
    *wait_ms = timing[0].m_flClientFrameIntervalMs + timing[0].m_flPresentCallCpuMs +
               timing[0].m_flWaitForPresentCpuMs + timing[0].m_flSubmitFrameMs;
}

class DriverProvider : vr::IServerTrackedDeviceProvider {
  public:
    Hmd hmd;
    std::optional<Controller> left_controller, right_controller;
    std::vector<GenericTracker> generic_trackers;

    std::map<uint64_t, TrackedDevice *> tracked_devices;

    std::optional<std::thread> event_thread;
    bool running = false;

    DriverProvider() {
        // Make sure paths are initialized before instantiating hmd
        init_paths();

        this->hmd = Hmd();
    }

    void event_loop() {
        // set_chaperone({1.0, 1.0});

        while (this->running) {
            auto event = alvr_read_event(100); // ms

            if (event.tag == ALVR_EVENT_DEVICE_CONNECTED) {
                auto profile = event.device_connected;

                auto device_it = this->tracked_devices.find(profile.top_level_path);
                if (device_it == this->tracked_devices.end()) {
                    if (profile.top_level_path == HEAD_PATH) {
                        // unreachable
                    } else if (profile.top_level_path == LEFT_HAND_PATH) {
                        this->left_controller =
                            Controller(LEFT_HAND_PATH, profile.interaction_profile);
                        this->tracked_devices.insert({LEFT_HAND_PATH, &*this->left_controller});
                    } else if (profile.top_level_path == RIGHT_HAND_PATH) {
                        this->right_controller =
                            Controller(RIGHT_HAND_PATH, profile.interaction_profile);
                        this->tracked_devices.insert({RIGHT_HAND_PATH, &*this->right_controller});
                    } else {
                        this->generic_trackers.push_back(GenericTracker(profile.top_level_path));
                        this->tracked_devices.insert(
                            {profile.top_level_path,
                             &this->generic_trackers[this->generic_trackers.size() - 1]});
                    }
                } else {
                    vr::VRServerDriverHost()->VendorSpecificEvent(
                        device_it->second->object_id, vr::VREvent_WirelessReconnect, {}, 0);
                }
            } else if (event.tag == ALVR_EVENT_DEVICE_DISCONNECTED) {
                alvr_popup_error("device disconnected event");
                auto device_it = this->tracked_devices.find(event.device_disconnected);
                if (device_it != this->tracked_devices.end()) {
                    vr::VRServerDriverHost()->VendorSpecificEvent(
                        device_it->second->object_id, vr::VREvent_WirelessDisconnect, {}, 0);
                    device_it->second->clear_pose();
                }
            } else if (event.tag == ALVR_EVENT_OPENVR_PROPERTY) {
                alvr_popup_error("prop changed event");
                auto device_it = this->tracked_devices.find(event.openvr_property.top_level_path);
                if (device_it != this->tracked_devices.end()) {
                    device_it->second->set_prop(event.openvr_property.prop);
                }
            } else if (event.tag == ALVR_EVENT_VIDEO_CONFIG) {
                this->hmd.update_video_config(event.video_config);
            } else if (event.tag == ALVR_EVENT_VIEWS_CONFIG) {
                this->hmd.update_views_config(event.views_config);
            } else if (event.tag == ALVR_EVENT_DEVICE_POSE) {
                auto device_it = this->tracked_devices.find(event.device_pose.top_level_path);
                if (device_it != this->tracked_devices.end()) {
                    device_it->second->update_pose(event.device_pose.data,
                                                   event.device_pose.timestamp_ns);
                }
            } else if (event.tag == ALVR_EVENT_BUTTON) {
                this->left_controller->try_update_button(event.button);
                this->right_controller->try_update_button(event.button);
            } else if (event.tag == ALVR_EVENT_HAND_SKELETON) {
                if (event.hand_skeleton.hand_type == ALVR_HAND_TYPE_LEFT) {
                    this->left_controller->update_hand_skeleton(event.hand_skeleton.joints,
                                                                event.hand_skeleton.timestamp_ns);
                } else {
                    this->right_controller->update_hand_skeleton(event.hand_skeleton.joints,
                                                                 event.hand_skeleton.timestamp_ns);
                }
            } else if (event.tag == ALVR_EVENT_BATTERY) {
                auto device_it = this->tracked_devices.find(event.battery.top_level_path);
                if (device_it != this->tracked_devices.end()) {
                    vr::VRProperties()->SetFloatProperty(device_it->second->object_id,
                                                         vr::Prop_DeviceBatteryPercentage_Float,
                                                         event.battery.value);
                }
            } else if (event.tag == ALVR_EVENT_BOUNDS) {
                set_chaperone(event.bounds);
            } else if (event.tag == ALVR_EVENT_RESTART) {
                // Note: Currently unused. The launcher is in charge of restarting SteamVR to set
                // the correct driver path
                vr::VRServerDriverHost()->RequestRestart(
                    "ALVR requested SteamVR restart", "", "", "");
            } else if (event.tag == ALVR_EVENT_SHUTDOWN) {
                vr::VRServerDriverHost()->VendorSpecificEvent(
                    0, vr::VREvent_DriverRequestedQuit, {}, 0);
            }
        }
    }

    virtual vr::EVRInitError Init(vr::IVRDriverContext *context) override {
        VR_INIT_SERVER_DRIVER_CONTEXT(context);

        auto graphics_context = AlvrGraphicsContext{};
#ifdef _WIN32
        graphics_context.vk_get_device_proc_addr = nullptr;
#else
        // todo: initialize from vulkan layer
#endif

        if (alvr_initialize(graphics_context, rendering_statistics)) {
            this->tracked_devices.insert({HEAD_PATH, &this->hmd});

            char hmd_serial_number[64];
            alvr_get_serial_number(HEAD_PATH, hmd_serial_number, 64);

            // If there is another HMD connected this call will fail. ALVR will continue using the
            // Hmd instance, but its data will remain unused.
            vr::VRServerDriverHost()->TrackedDeviceAdded(
                hmd_serial_number, vr::TrackedDeviceClass_HMD, &this->hmd);

            this->running = true;
            this->event_thread = std::thread(&DriverProvider::event_loop, this);

            return vr::VRInitError_None;
        } else {
            return vr::VRInitError_Driver_Failed;
        }
    }

    virtual void Cleanup() override {
        running = false;
        if (event_thread) {
            event_thread->join();
        }

        alvr_shutdown();

        VR_CLEANUP_SERVER_DRIVER_CONTEXT();
    }

    virtual const char *const *GetInterfaceVersions() override { return vr::k_InterfaceVersions; }

    virtual const char *GetTrackedDeviceDriverVersion() {
        return vr::ITrackedDeviceServerDriver_Version;
    }

    virtual void RunFrame() override {
        // vr::VRServerDriverHost()->VsyncEvent(0.016);

        // vr::VREvent_t event;
        // while (vr::VRServerDriverHost()->PollNextEvent(&event, sizeof(vr::VREvent_t))) {
        //     if (event.eventType == vr::VREvent_Input_HapticVibration) {
        //         vr::VREvent_HapticVibration_t haptics_info = event.data.hapticVibration;

        //         if (this->left_controller &&
        //             haptics_info.containerHandle == this->left_controller->haptics_container) {
        //             alvr_send_haptics(LEFT_HAND_PATH,
        //                               haptics_info.fDurationSeconds,
        //                               haptics_info.fFrequency,
        //                               haptics_info.fAmplitude);
        //         } else if (this->right_controller &&
        //                    haptics_info.containerHandle ==
        //                        this->right_controller->haptics_container) {
        //             alvr_send_haptics(RIGHT_HAND_PATH,
        //                               haptics_info.fDurationSeconds,
        //                               haptics_info.fFrequency,
        //                               haptics_info.fAmplitude);
        //         }
        //     }
        // }
    }

    virtual bool ShouldBlockStandbyMode() override { return false; }

    virtual void EnterStandby() override {}

    virtual void LeaveStandby() override {}
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