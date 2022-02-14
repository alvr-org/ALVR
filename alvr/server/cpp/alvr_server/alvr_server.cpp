#ifdef _WIN32
#include "platform/win32/CEncoder.h"
#include <windows.h>
#elif __APPLE__
#include "platform/macos/CEncoder.h"
#else
#include "platform/linux/CEncoder.h"
#endif
#include "ClientConnection.h"
#include "Logger.h"
#include "OvrController.h"
#include "OvrHMD.h"
#include "Paths.h"
#include "Settings.h"
#include "Statistics.h"
#include "TrackedDevice.h"
#include "bindings.h"
#include "driverlog.h"
#include "openvr_driver.h"
#include <cstring>
#include <map>
#include <optional>

static void load_debug_privilege(void) {
#ifdef _WIN32
    const DWORD flags = TOKEN_ADJUST_PRIVILEGES | TOKEN_QUERY;
    TOKEN_PRIVILEGES tp;
    HANDLE token;
    LUID val;

    if (!OpenProcessToken(GetCurrentProcess(), flags, &token)) {
        return;
    }

    if (!!LookupPrivilegeValue(NULL, SE_DEBUG_NAME, &val)) {
        tp.PrivilegeCount = 1;
        tp.Privileges[0].Luid = val;
        tp.Privileges[0].Attributes = SE_PRIVILEGE_ENABLED;

        AdjustTokenPrivileges(token, false, &tp, sizeof(tp), NULL, NULL);
    }

    if (!!LookupPrivilegeValue(NULL, SE_INC_BASE_PRIORITY_NAME, &val)) {
        tp.PrivilegeCount = 1;
        tp.Privileges[0].Luid = val;
        tp.Privileges[0].Attributes = SE_PRIVILEGE_ENABLED;

        if (!AdjustTokenPrivileges(token, false, &tp, sizeof(tp), NULL, NULL)) {
            Warn("[GPU PRIO FIX] Could not set privilege to increase GPU priority\n");
        }
    }

    Debug("[GPU PRIO FIX] Succeeded to set some sort of priority.\n");

    CloseHandle(token);
#endif
}

class DriverProvider : public vr::IServerTrackedDeviceProvider {
  public:
    std::shared_ptr<OvrHmd> hmd;
    std::shared_ptr<OvrController> left_controller, right_controller;
    // std::vector<OvrViveTrackerProxy> generic_trackers;

    std::map<uint64_t, TrackedDevice *> tracked_devices;

    virtual vr::EVRInitError Init(vr::IVRDriverContext *pContext) override {
        VR_INIT_SERVER_DRIVER_CONTEXT(pContext);
        InitDriverLog(vr::VRDriverLog());

        this->hmd = std::make_shared<OvrHmd>();
        this->left_controller = this->hmd->m_leftController;
        this->right_controller = this->hmd->m_rightController;

        this->tracked_devices.insert({HEAD_PATH, (TrackedDevice *)&*this->hmd});
        if (this->left_controller && this->right_controller) {
            this->tracked_devices.insert(
                {LEFT_HAND_PATH, (TrackedDevice *)&*this->left_controller});
            this->tracked_devices.insert(
                {RIGHT_HAND_PATH, (TrackedDevice *)&*this->right_controller});
        }

        return vr::VRInitError_None;
    }
    virtual void Cleanup() override {
        this->left_controller.reset();
        this->right_controller.reset();
        this->hmd.reset();

        CleanupDriverLog();

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
                vr::VREvent_HapticVibration_t haptics_info = event.data.hapticVibration;

                auto duration = haptics_info.fDurationSeconds;
                auto amplitude = haptics_info.fAmplitude;

                if (duration < Settings::Instance().m_hapticsMinDuration * 0.5)
                    duration = Settings::Instance().m_hapticsMinDuration * 0.5;

                amplitude =
                    pow(amplitude *
                            ((Settings::Instance().m_hapticsLowDurationAmplitudeMultiplier - 1) *
                                 Settings::Instance().m_hapticsMinDuration *
                                 Settings::Instance().m_hapticsLowDurationRange /
                                 (pow(Settings::Instance().m_hapticsMinDuration *
                                          Settings::Instance().m_hapticsLowDurationRange,
                                      2) *
                                      0.25 /
                                      (duration -
                                       0.5 * Settings::Instance().m_hapticsMinDuration *
                                           (1 - Settings::Instance().m_hapticsLowDurationRange)) +
                                  (duration -
                                   0.5 * Settings::Instance().m_hapticsMinDuration *
                                       (1 - Settings::Instance().m_hapticsLowDurationRange))) +
                             1),
                        1 - Settings::Instance().m_hapticsAmplitudeCurve);
                duration =
                    pow(Settings::Instance().m_hapticsMinDuration, 2) * 0.25 / duration + duration;

                if (this->left_controller &&
                    haptics_info.containerHandle == this->left_controller->prop_container) {
                    HapticsSend(
                        LEFT_CONTROLLER_HAPTIC_PATH, duration, haptics_info.fFrequency, amplitude);
                } else if (this->right_controller &&
                           haptics_info.containerHandle == this->right_controller->prop_container) {
                    HapticsSend(
                        RIGHT_CONTROLLER_HAPTIC_PATH, duration, haptics_info.fFrequency, amplitude);
                }
            }
        }
    }
    virtual bool ShouldBlockStandbyMode() override { return false; }
    virtual void EnterStandby() override {}
    virtual void LeaveStandby() override {}
} g_driver_provider;

// bindigs for Rust

const unsigned char *FRAME_RENDER_VS_CSO_PTR;
unsigned int FRAME_RENDER_VS_CSO_LEN;
const unsigned char *FRAME_RENDER_PS_CSO_PTR;
unsigned int FRAME_RENDER_PS_CSO_LEN;
const unsigned char *QUAD_SHADER_CSO_PTR;
unsigned int QUAD_SHADER_CSO_LEN;
const unsigned char *COMPRESS_AXIS_ALIGNED_CSO_PTR;
unsigned int COMPRESS_AXIS_ALIGNED_CSO_LEN;
const unsigned char *COLOR_CORRECTION_CSO_PTR;
unsigned int COLOR_CORRECTION_CSO_LEN;

const char *g_sessionPath;
const char *g_driverRootDir;

void (*LogError)(const char *stringPtr);
void (*LogWarn)(const char *stringPtr);
void (*LogInfo)(const char *stringPtr);
void (*LogDebug)(const char *stringPtr);
void (*DriverReadyIdle)(bool setDefaultChaprone);
void (*VideoSend)(VideoFrame header, unsigned char *buf, int len);
void (*HapticsSend)(unsigned long long path, float duration_s, float frequency, float amplitude);
void (*TimeSyncSend)(TimeSync packet);
void (*ShutdownRuntime)();
unsigned long long (*PathStringToHash)(const char *path);

void *CppEntryPoint(const char *interface_name, int *return_code) {
    // Initialize path constants
    init_paths();

    Settings::Instance().Load();

    load_debug_privilege();

    if (std::string(interface_name) == vr::IServerTrackedDeviceProvider_Version) {
        *return_code = vr::VRInitError_None;
        return &g_driver_provider;
    } else {
        *return_code = vr::VRInitError_Init_InterfaceNotFound;
        return nullptr;
    }
}

void InitializeStreaming() {
    // set correct client ip
    Settings::Instance().Load();

    if (g_driver_provider.hmd) {
        g_driver_provider.hmd->StartStreaming();
    }
}

void DeinitializeStreaming() {
    // nothing to do
}

void RequestIDR() {
    if (g_driver_provider.hmd && g_driver_provider.hmd->m_encoder) {
        g_driver_provider.hmd->m_encoder->InsertIDR();
    }
}

void InputReceive(TrackingInfo data) {
    if (g_driver_provider.hmd && g_driver_provider.hmd->m_Listener) {
        g_driver_provider.hmd->m_Listener->ProcessTrackingInfo(data);
    }
}
void TimeSyncReceive(TimeSync data) {
    if (g_driver_provider.hmd && g_driver_provider.hmd->m_Listener) {
        g_driver_provider.hmd->m_Listener->ProcessTimeSync(data);
    }
}
void VideoErrorReportReceive() {
    if (g_driver_provider.hmd && g_driver_provider.hmd->m_Listener) {
        g_driver_provider.hmd->m_Listener->OnFecFailure();
    }
}

void ShutdownSteamvr() {
    if (g_driver_provider.hmd) {
        vr::VRServerDriverHost()->VendorSpecificEvent(
            g_driver_provider.hmd->object_id, vr::VREvent_DriverRequestedQuit, {}, 0);
    }
}

void SetOpenvrProperty(unsigned long long top_level_path, OpenvrProperty prop) {
    auto device_it = g_driver_provider.tracked_devices.find(top_level_path);

    if (device_it != g_driver_provider.tracked_devices.end()) {
        device_it->second->set_prop(prop);
    }
}

void SetViewsConfig(ViewsConfigData config) {
    if (g_driver_provider.hmd) {
        g_driver_provider.hmd->SetViewsConfig(config);
    }
}

void SetBattery(unsigned long long top_level_path, float gauge_value, bool is_plugged) {
    auto device_it = g_driver_provider.tracked_devices.find(top_level_path);

    if (device_it != g_driver_provider.tracked_devices.end()) {
        vr::VRProperties()->SetBoolProperty(
            device_it->second->prop_container, vr::Prop_DeviceBatteryPercentage_Float, gauge_value);
        vr::VRProperties()->SetBoolProperty(
            device_it->second->prop_container, vr::Prop_DeviceIsCharging_Bool, is_plugged);
    }

    if (g_driver_provider.hmd && g_driver_provider.hmd->m_Listener) {
        auto stats = g_driver_provider.hmd->m_Listener->GetStatistics();

        if (top_level_path == HEAD_PATH) {
            stats->m_hmdBattery = gauge_value;
            stats->m_hmdPlugged = is_plugged;
        } else if (top_level_path == LEFT_HAND_PATH) {
            stats->m_leftControllerBattery = gauge_value;
        } else if (top_level_path == RIGHT_HAND_PATH) {
            stats->m_rightControllerBattery = gauge_value;
        }
    }
}