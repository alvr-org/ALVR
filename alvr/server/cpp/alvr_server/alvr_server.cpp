#ifdef _WIN32
#include <windows.h>
#endif
#include "ClientConnection.h"
#include "Logger.h"
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
    virtual void RunFrame() override {}
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
void (*HapticsSend)(HapticsFeedback packet);
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

    if (g_driver_provider.hmd)
        g_driver_provider.hmd->StartStreaming();
}

void DeinitializeStreaming() {
    if (g_driver_provider.hmd)
        g_driver_provider.hmd->StopStreaming();
}

void RequestIDR() {
    if (g_driver_provider.hmd)
        g_driver_provider.hmd->RequestIDR();
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
        g_driver_provider.hmd->m_Listener->ProcessVideoError();
    }
}

void ShutdownSteamvr() {
    if (g_driver_provider.hmd)
        g_driver_provider.hmd->OnShutdown();
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