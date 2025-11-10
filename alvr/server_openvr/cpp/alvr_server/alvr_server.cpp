#ifdef _WIN32
#include "platform/win32/CEncoder.h"
#include <windows.h>
#elif __APPLE__
#include "platform/macos/CEncoder.h"
#else
#include "platform/linux/CEncoder.h"
#endif
#include "Controller.h"
#include "FakeViveTracker.h"
#include "HMD.h"
#include "Logger.h"
#include "Paths.h"
#include "PoseHistory.h"
#include "Settings.h"
#include "TrackedDevice.h"
#include "bindings.h"
#include "driverlog.h"
#include "openvr_driver_wrap.h"
#include <algorithm>
#include <cmath>
#include <cstring>
#include <map>
#include <optional>

#ifdef __linux__
#include "include/openvr_math.h"
std::unique_ptr<vr::HmdMatrix34_t> GetInvZeroPose();

std::unique_ptr<vr::HmdMatrix34_t> GetRawZeroPose() {
    auto invZeroPose = GetInvZeroPose();
    if (invZeroPose == nullptr) {
        return nullptr;
    }
    return std::make_unique<vr::HmdMatrix34_t>(vrmath::matInv33(*invZeroPose));
}

bool IsOpenvrClientReady();
#endif
void _SetChaperoneArea(float areaWidth, float areaHeight);

vr::EVREventType VendorEvent_ALVRDriverResync
    = (vr::EVREventType)(vr::VREvent_VendorSpecific_Reserved_Start + ((vr::EVREventType)0xC0));

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
    bool early_hmd_initialization = false;

    std::unique_ptr<Hmd> hmd;
    std::unique_ptr<Controller> left_controller, right_controller;
    std::unique_ptr<Controller> left_hand_tracker, right_hand_tracker;
    std::vector<std::unique_ptr<FakeViveTracker>> generic_trackers;
    bool devices_initialized = false;
    bool shutdown_called = false;

    std::map<uint64_t, TrackedDevice*> tracked_devices;

    virtual vr::EVRInitError Init(vr::IVRDriverContext* pContext) override {
        Debug("DriverProvider::Init");

        VR_INIT_SERVER_DRIVER_CONTEXT(pContext);
        InitDriverLog(vr::VRDriverLog());

        if (this->early_hmd_initialization) {
            auto hmd = new Hmd();
            // Note: we disable awaiting for Acivate() call. That will only be called after
            // IServerTrackedDeviceProvider::Init() (this function) returns.
            hmd->register_device(false);
            this->hmd = std::unique_ptr<Hmd>(hmd);
            this->tracked_devices.insert({ HEAD_ID, this->hmd.get() });
        }

        return vr::VRInitError_None;
    }
    virtual void Cleanup() override {
        Debug("DriverProvider::Cleanup");

        this->left_hand_tracker.reset();
        this->right_hand_tracker.reset();
        this->left_controller.reset();
        this->right_controller.reset();
        this->hmd.reset();
        // this->generic_trackers.clear();

        CleanupDriverLog();

        VR_CLEANUP_SERVER_DRIVER_CONTEXT();
    }
    virtual const char* const* GetInterfaceVersions() override { return vr::k_InterfaceVersions; }
    virtual const char* GetTrackedDeviceDriverVersion() {
        return vr::ITrackedDeviceServerDriver_Version;
    }
    virtual void RunFrame() override {
        vr::VREvent_t event;
        while (vr::VRServerDriverHost()->PollNextEvent(&event, sizeof(vr::VREvent_t))) {
            if (event.eventType == vr::VREvent_Input_HapticVibration) {
                Debug("DriverProvider: Received HapticVibration event");

                vr::VREvent_HapticVibration_t haptics = event.data.hapticVibration;

                uint64_t id = 0;
                if (this->left_controller
                    && haptics.containerHandle == this->left_controller->prop_container) {
                    id = HAND_LEFT_ID;
                } else if (this->right_controller
                           && haptics.containerHandle == this->right_controller->prop_container) {
                    id = HAND_RIGHT_ID;
                }

                HapticsSend(id, haptics.fDurationSeconds, haptics.fFrequency, haptics.fAmplitude);
            }
#ifdef __linux__
            else if (event.eventType == vr::VREvent_ChaperoneUniverseHasChanged
                     || event.eventType == vr::VREvent_ChaperoneRoomSetupFinished
                     || event.eventType == vr::VREvent_ChaperoneFlushCache
                     || event.eventType == vr::VREvent_ChaperoneSettingsHaveChanged
                     || event.eventType == vr::VREvent_SeatedZeroPoseReset
                     || event.eventType == vr::VREvent_StandingZeroPoseReset
                     || event.eventType == vr::VREvent_SceneApplicationChanged
                     || event.eventType == VendorEvent_ALVRDriverResync) {
                if (hmd && hmd->m_poseHistory) {
                    auto rawZeroPose = GetRawZeroPose();
                    if (rawZeroPose != nullptr) {
                        hmd->m_poseHistory->SetTransform(*rawZeroPose);
                    }
                }
            }
#endif
        }
        if (vr::VRServerDriverHost()->IsExiting() && !shutdown_called) {
            Debug("DriverProvider: Received shutdown event");

            shutdown_called = true;
            ShutdownRuntime();
        }
    }
    virtual bool ShouldBlockStandbyMode() override { return false; }
    virtual void EnterStandby() override { Debug("DriverProvider::EnterStandby"); }
    virtual void LeaveStandby() override { Debug("DriverProvider::LeaveStandby"); }
} g_driver_provider;

// bindigs for Rust

const unsigned char* FRAME_RENDER_VS_CSO_PTR;
unsigned int FRAME_RENDER_VS_CSO_LEN;
const unsigned char* FRAME_RENDER_PS_CSO_PTR;
unsigned int FRAME_RENDER_PS_CSO_LEN;
const unsigned char* QUAD_SHADER_CSO_PTR;
unsigned int QUAD_SHADER_CSO_LEN;
const unsigned char* COMPRESS_AXIS_ALIGNED_CSO_PTR;
unsigned int COMPRESS_AXIS_ALIGNED_CSO_LEN;
const unsigned char* COLOR_CORRECTION_CSO_PTR;
unsigned int COLOR_CORRECTION_CSO_LEN;
const unsigned char* RGBTOYUV420_CSO_PTR;
unsigned int RGBTOYUV420_CSO_LEN;

const unsigned char* QUAD_SHADER_COMP_SPV_PTR;
unsigned int QUAD_SHADER_COMP_SPV_LEN;
const unsigned char* COLOR_SHADER_COMP_SPV_PTR;
unsigned int COLOR_SHADER_COMP_SPV_LEN;
const unsigned char* FFR_SHADER_COMP_SPV_PTR;
unsigned int FFR_SHADER_COMP_SPV_LEN;
const unsigned char* RGBTOYUV420_SHADER_COMP_SPV_PTR;
unsigned int RGBTOYUV420_SHADER_COMP_SPV_LEN;

const char* g_sessionPath;
const char* g_driverRootDir;

void (*LogError)(const char* stringPtr);
void (*LogWarn)(const char* stringPtr);
void (*LogInfo)(const char* stringPtr);
void (*LogDebug)(const char* stringPtr);
void (*LogEncoder)(const char* stringPtr);
void (*LogPeriodically)(const char* tag, const char* stringPtr);
void (*DriverReadyIdle)(bool setDefaultChaprone);
void (*SetVideoConfigNals)(const unsigned char* configBuffer, int len, int codec);
void (*VideoSend)(unsigned long long targetTimestampNs, unsigned char* buf, int len, bool isIdr);
void (*HapticsSend)(unsigned long long path, float duration_s, float frequency, float amplitude);
void (*ShutdownRuntime)();
unsigned long long (*PathStringToHash)(const char* path);
void (*ReportPresent)(unsigned long long timestamp_ns, unsigned long long offset_ns);
void (*ReportComposed)(unsigned long long timestamp_ns, unsigned long long offset_ns);
FfiDynamicEncoderParams (*GetDynamicEncoderParams)();
unsigned long long (*GetSerialNumber)(unsigned long long deviceID, char* outString);
void (*SetOpenvrProps)(void* instancePtr, unsigned long long deviceID);
void (*RegisterButtons)(void* instancePtr, unsigned long long deviceID);
void (*WaitForVSync)();

void CppInit(bool earlyHmdInitialization) {
    g_driver_provider.early_hmd_initialization = earlyHmdInitialization;

    HookCrashHandler();

    // Initialize path constants
    init_paths();

    Settings::Instance().Load();

    load_debug_privilege();
}

void* CppOpenvrEntryPoint(const char* interface_name, int* return_code) {
    if (std::string(interface_name) == vr::IServerTrackedDeviceProvider_Version) {
        *return_code = vr::VRInitError_None;
        return &g_driver_provider;
    } else {
        *return_code = vr::VRInitError_Init_InterfaceNotFound;
        return nullptr;
    }
}

bool InitializeStreaming() {
    Settings::Instance().Load();

    if (!g_driver_provider.devices_initialized) {
        if (!g_driver_provider.early_hmd_initialization) {
            auto hmd = new Hmd();
            if (!hmd->register_device(false)) {
                Error("Failed to register HMD");
                return false;
            }
            g_driver_provider.hmd = std::unique_ptr<Hmd>(hmd);
            g_driver_provider.tracked_devices.insert({ HEAD_ID, g_driver_provider.hmd.get() });
        }

        // Note: for controllers, hands and trackers don't bail out if registration fails
        if (Settings::Instance().m_enableControllers) {
            auto controllerSkeletonLevel = Settings::Instance().m_useSeparateHandTrackers
                ? vr::VRSkeletalTracking_Estimated
                : vr::VRSkeletalTracking_Partial;

            auto left_controller = new Controller(HAND_LEFT_ID, controllerSkeletonLevel);
            if (left_controller->register_device(true)) {
                g_driver_provider.left_controller = std::unique_ptr<Controller>(left_controller);
                g_driver_provider.tracked_devices.insert(
                    { HAND_LEFT_ID, g_driver_provider.left_controller.get() }
                );
            }

            auto right_controller = new Controller(HAND_RIGHT_ID, controllerSkeletonLevel);
            if (right_controller->register_device(true)) {
                g_driver_provider.right_controller = std::unique_ptr<Controller>(right_controller);
                g_driver_provider.tracked_devices.insert(
                    { HAND_RIGHT_ID, g_driver_provider.right_controller.get() }
                );
            }

            if (Settings::Instance().m_useSeparateHandTrackers) {
                auto left_hand_tracker
                    = new Controller(HAND_TRACKER_LEFT_ID, vr::VRSkeletalTracking_Full);
                if (left_hand_tracker->register_device(true)) {
                    g_driver_provider.left_hand_tracker
                        = std::unique_ptr<Controller>(left_hand_tracker);
                    g_driver_provider.tracked_devices.insert(
                        { HAND_TRACKER_LEFT_ID, g_driver_provider.left_hand_tracker.get() }
                    );
                }

                auto right_hand_tracker
                    = new Controller(HAND_TRACKER_RIGHT_ID, vr::VRSkeletalTracking_Full);
                if (right_hand_tracker->register_device(true)) {
                    g_driver_provider.right_hand_tracker
                        = std::unique_ptr<Controller>(right_hand_tracker);
                    g_driver_provider.tracked_devices.insert(
                        { HAND_TRACKER_RIGHT_ID, g_driver_provider.right_hand_tracker.get() }
                    );
                }
            }
        }

        if (Settings::Instance().m_enableBodyTrackingFakeVive) {
            auto chestTracker = std::make_unique<FakeViveTracker>(BODY_CHEST_ID);
            if (chestTracker->register_device(true)) {
                g_driver_provider.tracked_devices.insert({ BODY_CHEST_ID, chestTracker.get() });
                g_driver_provider.generic_trackers.push_back(std::move(chestTracker));
            }

            auto waistTracker = std::make_unique<FakeViveTracker>(BODY_HIPS_ID);
            if (waistTracker->register_device(true)) {
                g_driver_provider.tracked_devices.insert({ BODY_HIPS_ID, waistTracker.get() });
                g_driver_provider.generic_trackers.push_back(std::move(waistTracker));
            }

            auto leftElbowTracker = std::make_unique<FakeViveTracker>(BODY_LEFT_ELBOW_ID);
            if (leftElbowTracker->register_device(true)) {
                g_driver_provider.tracked_devices.insert({ BODY_LEFT_ELBOW_ID,
                                                           leftElbowTracker.get() });
                g_driver_provider.generic_trackers.push_back(std::move(leftElbowTracker));
            }

            auto rightElbowTracker = std::make_unique<FakeViveTracker>(BODY_RIGHT_ELBOW_ID);
            if (rightElbowTracker->register_device(true)) {
                g_driver_provider.tracked_devices.insert({ BODY_RIGHT_ELBOW_ID,
                                                           rightElbowTracker.get() });
                g_driver_provider.generic_trackers.push_back(std::move(rightElbowTracker));
            }

            if (Settings::Instance().m_bodyTrackingHasLegs) {
                auto leftKneeTracker = std::make_unique<FakeViveTracker>(BODY_LEFT_KNEE_ID);
                if (leftKneeTracker->register_device(true)) {
                    g_driver_provider.tracked_devices.insert({ BODY_LEFT_KNEE_ID,
                                                               leftKneeTracker.get() });
                    g_driver_provider.generic_trackers.push_back(std::move(leftKneeTracker));
                }

                auto leftFootTracker = std::make_unique<FakeViveTracker>(BODY_LEFT_FOOT_ID);
                if (leftFootTracker->register_device(true)) {
                    g_driver_provider.tracked_devices.insert({ BODY_LEFT_FOOT_ID,
                                                               leftFootTracker.get() });
                    g_driver_provider.generic_trackers.push_back(std::move(leftFootTracker));
                }

                auto rightKneeTracker = std::make_unique<FakeViveTracker>(BODY_RIGHT_KNEE_ID);
                if (rightKneeTracker->register_device(true)) {
                    g_driver_provider.tracked_devices.insert({ BODY_RIGHT_KNEE_ID,
                                                               rightKneeTracker.get() });
                    g_driver_provider.generic_trackers.push_back(std::move(rightKneeTracker));
                }

                auto rightFootTracker = std::make_unique<FakeViveTracker>(BODY_RIGHT_FOOT_ID);
                if (rightFootTracker->register_device(true)) {
                    g_driver_provider.tracked_devices.insert({ BODY_RIGHT_FOOT_ID,
                                                               rightFootTracker.get() });
                    g_driver_provider.generic_trackers.push_back(std::move(rightFootTracker));
                }
            }
        }

        g_driver_provider.devices_initialized = true;
    }

    if (g_driver_provider.hmd) {
        g_driver_provider.hmd->StartStreaming();
    }

    return true;
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

void SetTracking(
    unsigned long long targetTimestampNs,
    float controllerPoseTimeOffsetS,
    FfiDeviceMotion headMotion,
    FfiHandData leftHandData,
    FfiHandData rightHandData,
    const FfiDeviceMotion* bodyTrackerMotions,
    int bodyTrackerMotionCount
) {
    if (g_driver_provider.hmd) {
        g_driver_provider.hmd->OnPoseUpdated(targetTimestampNs, headMotion);
    }

    if (g_driver_provider.left_hand_tracker) {
        g_driver_provider.left_hand_tracker->OnPoseUpdate(
            targetTimestampNs, controllerPoseTimeOffsetS, leftHandData
        );
    }

    if (g_driver_provider.left_controller) {
        g_driver_provider.left_controller->OnPoseUpdate(
            targetTimestampNs, controllerPoseTimeOffsetS, leftHandData
        );
    }

    if (g_driver_provider.right_hand_tracker) {
        g_driver_provider.right_hand_tracker->OnPoseUpdate(
            targetTimestampNs, controllerPoseTimeOffsetS, rightHandData
        );
    }

    if (g_driver_provider.right_controller) {
        g_driver_provider.right_controller->OnPoseUpdate(
            targetTimestampNs, controllerPoseTimeOffsetS, rightHandData
        );
    }

    if (Settings::Instance().m_enableBodyTrackingFakeVive) {
        std::map<uint64_t, FfiDeviceMotion> motionsMap;
        for (int i = 0; i < bodyTrackerMotionCount; i++) {
            auto m = bodyTrackerMotions[i];
            motionsMap.insert({ m.deviceID, m });
        }

        for (auto id : BODY_IDS) {
            auto it = g_driver_provider.tracked_devices.find(id);
            if (it != g_driver_provider.tracked_devices.end()) {
                auto* maybeTracker = (FakeViveTracker*)it->second;
                auto res = motionsMap.find(id);
                auto* maybeMotion = res != motionsMap.end() ? &res->second : nullptr;

                maybeTracker->OnPoseUpdated(targetTimestampNs, maybeMotion);
            }
        }
    }
}

void RequestDriverResync() {
    if (g_driver_provider.hmd) {
        vr::VRServerDriverHost()->VendorSpecificEvent(
            g_driver_provider.hmd->object_id, VendorEvent_ALVRDriverResync, {}, 0
        );
    }
}

void ShutdownSteamvr() {
    if (g_driver_provider.hmd) {
        vr::VRServerDriverHost()->VendorSpecificEvent(
            g_driver_provider.hmd->object_id, vr::VREvent_DriverRequestedQuit, {}, 0
        );
    }
}

void SetOpenvrProperty(void* instancePtr, FfiOpenvrProperty prop) {
    ((TrackedDevice*)instancePtr)->set_prop(prop);
}

void SetOpenvrPropByDeviceID(unsigned long long deviceID, FfiOpenvrProperty prop) {
    auto device_it = g_driver_provider.tracked_devices.find(deviceID);

    if (device_it != g_driver_provider.tracked_devices.end()) {
        device_it->second->set_prop(prop);
    }
}

void RegisterButton(void* instancePtr, unsigned long long buttonID) {
    // Todo: move RegisterButton to generic TrackedDevice interface
    ((Controller*)instancePtr)->RegisterButton(buttonID);
}

void SetLocalViewParams(const FfiViewParams params[2]) {
    if (g_driver_provider.hmd) {
        g_driver_provider.hmd->SetViewParams(params);
    }
}

void SetBattery(unsigned long long deviceID, float gauge_value, bool is_plugged) {
    auto device_it = g_driver_provider.tracked_devices.find(deviceID);

    if (device_it != g_driver_provider.tracked_devices.end()) {
        vr::VRProperties()->SetFloatProperty(
            device_it->second->prop_container, vr::Prop_DeviceBatteryPercentage_Float, gauge_value
        );
        vr::VRProperties()->SetBoolProperty(
            device_it->second->prop_container, vr::Prop_DeviceIsCharging_Bool, is_plugged
        );
    }
}

void SetButton(unsigned long long buttonID, FfiButtonValue value) {
    if (LEFT_CONTROLLER_BUTTON_MAPPING.find(buttonID) != LEFT_CONTROLLER_BUTTON_MAPPING.end()) {
        if (g_driver_provider.left_controller) {
            g_driver_provider.left_controller->SetButton(buttonID, value);
        }
        if (g_driver_provider.left_hand_tracker) {
            g_driver_provider.left_hand_tracker->SetButton(buttonID, value);
        }
    } else if (RIGHT_CONTROLLER_BUTTON_MAPPING.find(buttonID)
               != RIGHT_CONTROLLER_BUTTON_MAPPING.end()) {
        if (g_driver_provider.right_controller) {
            g_driver_provider.right_controller->SetButton(buttonID, value);
        }
        if (g_driver_provider.right_hand_tracker) {
            g_driver_provider.right_hand_tracker->SetButton(buttonID, value);
        }
    }
}

void SetProximityState(bool headset_is_worn) {
    if (g_driver_provider.hmd) {
        g_driver_provider.hmd->SetProximityState(headset_is_worn);
    }
}

void SetChaperoneArea(float areaWidth, float areaHeight) {
    _SetChaperoneArea(areaWidth, areaHeight);
}

void CaptureFrame() {
#ifndef __APPLE__
    if (g_driver_provider.hmd && g_driver_provider.hmd->m_encoder) {
        g_driver_provider.hmd->m_encoder->CaptureFrame();
    }
#endif
}
