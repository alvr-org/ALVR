#ifdef _WIN32
#include "platform/win32/CEncoder.h"
#include <windows.h>
#elif __APPLE__
#include "platform/macos/CEncoder.h"
#else
#include "platform/linux/CEncoder.h"
#endif
#include "Controller.h"
#include "HMD.h"
#include "Logger.h"
#include "Paths.h"
#include "PoseHistory.h"
#include "Settings.h"
#include "TrackedDevice.h"
#include "bindings.h"
#include "driverlog.h"
#include "openvr_driver.h"
#include <algorithm>
#include <cmath>
#include <cstring>
#include <map>
#include <optional>

void _SetChaperoneArea(float areaWidth, float areaHeight);
#ifdef __linux__
vr::HmdMatrix34_t GetRawZeroPose();
#endif

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
    std::unique_ptr<Hmd> hmd;
    std::unique_ptr<Controller> left_controller, right_controller;
    // std::vector<ViveTrackerProxy> generic_trackers;
    bool shutdown_called = false;

    std::map<uint64_t, TrackedDevice *> tracked_devices;

    virtual vr::EVRInitError Init(vr::IVRDriverContext *pContext) override {
        VR_INIT_SERVER_DRIVER_CONTEXT(pContext);
        InitDriverLog(vr::VRDriverLog());

        this->hmd = std::make_unique<Hmd>();
        this->tracked_devices.insert({HEAD_ID, (TrackedDevice *)this->hmd.get()});
        if (vr::VRServerDriverHost()->TrackedDeviceAdded(this->hmd->get_serial_number().c_str(),
                                                         this->hmd->GetDeviceClass(),
                                                         this->hmd.get())) {
        } else {
            Warn("Failed to register HMD device");
        }

        if (Settings::Instance().m_enableControllers) {
            this->left_controller = std::make_unique<Controller>(LEFT_HAND_ID);
            this->right_controller = std::make_unique<Controller>(RIGHT_HAND_ID);

            this->tracked_devices.insert(
                {LEFT_HAND_ID, (TrackedDevice *)this->left_controller.get()});
            this->tracked_devices.insert(
                {RIGHT_HAND_ID, (TrackedDevice *)this->right_controller.get()});

            if (!vr::VRServerDriverHost()->TrackedDeviceAdded(
                    this->left_controller->get_serial_number().c_str(),
                    this->left_controller->getControllerDeviceClass(),
                    this->left_controller.get())) {
                Warn("Failed to register left controller");
            }
            if (!vr::VRServerDriverHost()->TrackedDeviceAdded(
                    this->right_controller->get_serial_number().c_str(),
                    this->right_controller->getControllerDeviceClass(),
                    this->right_controller.get())) {
                Warn("Failed to register right controller");
            }
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
                vr::VREvent_HapticVibration_t haptics = event.data.hapticVibration;

                uint64_t id = 0;
                if (this->left_controller &&
                    haptics.containerHandle == this->left_controller->prop_container) {
                    id = LEFT_HAND_ID;
                } else if (this->right_controller &&
                           haptics.containerHandle == this->right_controller->prop_container) {
                    id = RIGHT_HAND_ID;
                }

                HapticsSend(id, haptics.fDurationSeconds, haptics.fFrequency, haptics.fAmplitude);
            }
#ifdef __linux__
            else if (event.eventType == vr::VREvent_ChaperoneUniverseHasChanged) {
                if (hmd && hmd->m_poseHistory) {
                    InitOpenvrClient();
                    hmd->m_poseHistory->SetTransformUpdating();
                    hmd->m_poseHistory->SetTransform(GetRawZeroPose());
                    ShutdownOpenvrClient();
                }
            }
#endif
        }
        if(vr::VRServerDriverHost()->IsExiting() && !shutdown_called) {
            shutdown_called = true;
            ShutdownRuntime();
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

const unsigned char *QUAD_SHADER_COMP_SPV_PTR;
unsigned int QUAD_SHADER_COMP_SPV_LEN;
const unsigned char *COLOR_SHADER_COMP_SPV_PTR;
unsigned int COLOR_SHADER_COMP_SPV_LEN;
const unsigned char *FFR_SHADER_COMP_SPV_PTR;
unsigned int FFR_SHADER_COMP_SPV_LEN;
const unsigned char *RGBTOYUV420_SHADER_COMP_SPV_PTR;
unsigned int RGBTOYUV420_SHADER_COMP_SPV_LEN;

const char *g_sessionPath;
const char *g_driverRootDir;

void (*LogError)(const char *stringPtr);
void (*LogWarn)(const char *stringPtr);
void (*LogInfo)(const char *stringPtr);
void (*LogDebug)(const char *stringPtr);
void (*LogPeriodically)(const char *tag, const char *stringPtr);
void (*DriverReadyIdle)(bool setDefaultChaprone);
void (*SetVideoConfigNals)(const unsigned char *configBuffer, int len, int codec);
void (*VideoSend)(unsigned long long targetTimestampNs, unsigned char *buf, int len, bool isIdr);
void (*HapticsSend)(unsigned long long path, float duration_s, float frequency, float amplitude);
void (*ShutdownRuntime)();
unsigned long long (*PathStringToHash)(const char *path);
void (*ReportPresent)(unsigned long long timestamp_ns, unsigned long long offset_ns);
void (*ReportComposed)(unsigned long long timestamp_ns, unsigned long long offset_ns);
FfiDynamicEncoderParams (*GetDynamicEncoderParams)();
unsigned long long (*GetSerialNumber)(unsigned long long deviceID, char *outString);
void (*SetOpenvrProps)(unsigned long long deviceID);
void (*RegisterButtons)(unsigned long long deviceID);
void (*WaitForVSync)();

void *CppEntryPoint(const char *interface_name, int *return_code) {
    HookCrashHandler();

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
    Settings::Instance().Load();

    if (g_driver_provider.hmd) {
        g_driver_provider.hmd->StartStreaming();
    }
}

void DeinitializeStreaming() {
    if (g_driver_provider.hmd) {
        g_driver_provider.hmd->StopStreaming();
    }
}

void SendVSync() { vr::VRServerDriverHost()->VsyncEvent(0.0); }

void RequestIDR() {
    if (g_driver_provider.hmd && g_driver_provider.hmd->m_encoder) {
        g_driver_provider.hmd->m_encoder->InsertIDR();
    }
}

void SetTracking(unsigned long long targetTimestampNs,
                 float controllerPoseTimeOffsetS,
                 const FfiDeviceMotion *deviceMotions,
                 int motionsCount,
                 const FfiHandSkeleton *leftHand,
                 const FfiHandSkeleton *rightHand,
                 unsigned int controllersTracked) {
    for (int i = 0; i < motionsCount; i++) {
        if (deviceMotions[i].deviceID == HEAD_ID && g_driver_provider.hmd) {
            g_driver_provider.hmd->OnPoseUpdated(targetTimestampNs, deviceMotions[i]);
        } else {
            if (g_driver_provider.left_controller && deviceMotions[i].deviceID == LEFT_HAND_ID) {
                g_driver_provider.left_controller->onPoseUpdate(
                    controllerPoseTimeOffsetS, deviceMotions[i], leftHand, controllersTracked);
            } else if (g_driver_provider.right_controller &&
                       deviceMotions[i].deviceID == RIGHT_HAND_ID) {
                g_driver_provider.right_controller->onPoseUpdate(
                    controllerPoseTimeOffsetS, deviceMotions[i], rightHand, controllersTracked);
            }
        }
    }
}

void VideoErrorReportReceive() {
    if (g_driver_provider.hmd) {
        g_driver_provider.hmd->m_encoder->OnPacketLoss();
    }
}

void ShutdownSteamvr() {
    if (g_driver_provider.hmd) {
        vr::VRServerDriverHost()->VendorSpecificEvent(
            g_driver_provider.hmd->object_id, vr::VREvent_DriverRequestedQuit, {}, 0);
    }
}

void SetOpenvrProperty(unsigned long long deviceID, FfiOpenvrProperty prop) {
    auto device_it = g_driver_provider.tracked_devices.find(deviceID);

    if (device_it != g_driver_provider.tracked_devices.end()) {
        device_it->second->set_prop(prop);
    }
}

void RegisterButton(unsigned long long buttonID) {
    if (g_driver_provider.left_controller &&
        LEFT_CONTROLLER_BUTTON_MAPPING.find(buttonID) != LEFT_CONTROLLER_BUTTON_MAPPING.end()) {
        g_driver_provider.left_controller->RegisterButton(buttonID);
    } else if (g_driver_provider.right_controller &&
               RIGHT_CONTROLLER_BUTTON_MAPPING.find(buttonID) !=
                   RIGHT_CONTROLLER_BUTTON_MAPPING.end()) {
        g_driver_provider.right_controller->RegisterButton(buttonID);
    }
}

void SetViewsConfig(FfiViewsConfig config) {
    if (g_driver_provider.hmd) {
        g_driver_provider.hmd->SetViewsConfig(config);
    }
}

void SetBattery(unsigned long long deviceID, float gauge_value, bool is_plugged) {
    auto device_it = g_driver_provider.tracked_devices.find(deviceID);

    if (device_it != g_driver_provider.tracked_devices.end()) {
        vr::VRProperties()->SetFloatProperty(
            device_it->second->prop_container, vr::Prop_DeviceBatteryPercentage_Float, gauge_value);
        vr::VRProperties()->SetBoolProperty(
            device_it->second->prop_container, vr::Prop_DeviceIsCharging_Bool, is_plugged);
    }
}

void SetButton(unsigned long long buttonID, FfiButtonValue value) {
    if (g_driver_provider.left_controller &&
        LEFT_CONTROLLER_BUTTON_MAPPING.find(buttonID) != LEFT_CONTROLLER_BUTTON_MAPPING.end()) {
        g_driver_provider.left_controller->SetButton(buttonID, value);
    } else if (g_driver_provider.right_controller &&
               RIGHT_CONTROLLER_BUTTON_MAPPING.find(buttonID) !=
                   RIGHT_CONTROLLER_BUTTON_MAPPING.end()) {
        g_driver_provider.right_controller->SetButton(buttonID, value);
    }
}

void SetChaperoneArea(float areaWidth, float areaHeight) {
    _SetChaperoneArea(areaWidth, areaHeight);

#ifdef __linux__
    if (g_driver_provider.hmd && g_driver_provider.hmd->m_poseHistory) {
        g_driver_provider.hmd->m_poseHistory->SetTransformUpdating();
        g_driver_provider.hmd->m_poseHistory->SetTransform(GetRawZeroPose());
    }
#endif
}

void CaptureFrame() {
#ifndef __APPLE__
    if (g_driver_provider.hmd && g_driver_provider.hmd->m_encoder) {
        g_driver_provider.hmd->m_encoder->CaptureFrame();
    }
#endif
}
