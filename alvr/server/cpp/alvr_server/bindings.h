#pragma once

struct FfiFov {
    float left;
    float right;
    float up;
    float down;
};

struct FfiQuat {
    float x;
    float y;
    float z;
    float w;
};

struct FfiHandSkeleton {
    float joint_positions[26][3];
    FfiQuat joint_rotations[26];
};

struct FfiDeviceMotion {
    unsigned long long device_id;
    float prediction_s;
    int is_tracked;
    FfiQuat orientation;
    float position[3];
    float linear_velocity[3];
    float angular_velocity[3];
};

enum FfiOpenvrPropertyType {
    Bool,
    Float,
    Int32,
    Uint64,
    Vector3,
    Double,
    String,
};

union FfiOpenvrPropertyValue {
    unsigned int bool_;
    float float_;
    int int32;
    unsigned long long uint64;
    float vector3[3];
    double double_;
    char string[256];
};

struct FfiOpenvrProperty {
    unsigned int key;
    FfiOpenvrPropertyType type;
    FfiOpenvrPropertyValue value;
};

struct FfiViewsConfig {
    FfiFov fov[2];
    float ipd_m;
};

enum FfiButtonType {
    BUTTON_TYPE_BINARY,
    BUTTON_TYPE_SCALAR,
};

struct FfiButtonValue {
    FfiButtonType type;
    union {
        unsigned int binary;
        float scalar;
    };
};

struct FfiDynamicEncoderParams {
    unsigned int updated;
    unsigned long long bitrate_bps;
    float framerate;
};

extern "C" const unsigned char *FRAME_RENDER_VS_CSO_PTR;
extern "C" unsigned int FRAME_RENDER_VS_CSO_LEN;
extern "C" const unsigned char *FRAME_RENDER_PS_CSO_PTR;
extern "C" unsigned int FRAME_RENDER_PS_CSO_LEN;
extern "C" const unsigned char *QUAD_SHADER_CSO_PTR;
extern "C" unsigned int QUAD_SHADER_CSO_LEN;
extern "C" const unsigned char *COMPRESS_AXIS_ALIGNED_CSO_PTR;
extern "C" unsigned int COMPRESS_AXIS_ALIGNED_CSO_LEN;
extern "C" const unsigned char *COLOR_CORRECTION_CSO_PTR;
extern "C" unsigned int COLOR_CORRECTION_CSO_LEN;

extern "C" const unsigned char *QUAD_SHADER_COMP_SPV_PTR;
extern "C" unsigned int QUAD_SHADER_COMP_SPV_LEN;
extern "C" const unsigned char *COLOR_SHADER_COMP_SPV_PTR;
extern "C" unsigned int COLOR_SHADER_COMP_SPV_LEN;
extern "C" const unsigned char *FFR_SHADER_COMP_SPV_PTR;
extern "C" unsigned int FFR_SHADER_COMP_SPV_LEN;
extern "C" const unsigned char *RGBTOYUV420_SHADER_COMP_SPV_PTR;
extern "C" unsigned int RGBTOYUV420_SHADER_COMP_SPV_LEN;

extern "C" const char *g_sessionPath;
extern "C" const char *g_driverRootDir;

extern "C" void (*LogError)(const char *stringPtr);
extern "C" void (*LogWarn)(const char *stringPtr);
extern "C" void (*LogInfo)(const char *stringPtr);
extern "C" void (*LogDebug)(const char *stringPtr);
extern "C" void (*LogPeriodically)(const char *tag, const char *stringPtr);
extern "C" void (*DriverReadyIdle)(bool setDefaultChaprone);
extern "C" void (*SetVideoConfigNals)(const unsigned char *configBuffer, int len, int codec);
extern "C" void (*VideoSend)(unsigned long long target_timestamp_ns,
                             unsigned char *buf,
                             int len,
                             bool is_idr);
extern "C" void (*HapticsSend)(unsigned long long path,
                               float duration_s,
                               float frequency,
                               float amplitude);
extern "C" void (*ShutdownRuntime)();
extern "C" unsigned long long (*PathStringToHash)(const char *path);
extern "C" void (*ReportPresent)(unsigned long long timestamp_ns, unsigned long long offset_ns);
extern "C" void (*ReportComposed)(unsigned long long timestamp_ns, unsigned long long offset_ns);
extern "C" FfiDynamicEncoderParams (*GetDynamicEncoderParams)();
extern "C" unsigned long long (*GetSerialNumber)(unsigned long long device_id, char *outString);
extern "C" void (*SetOpenvrProps)(unsigned long long device_id);
extern "C" void (*RegisterButtons)(unsigned long long device_id);
extern "C" void (*WaitForVSync)();

extern "C" void *CppEntryPoint(const char *pInterfaceName, int *pReturnCode);
extern "C" void InitializeStreaming();
extern "C" void DeinitializeStreaming();
extern "C" void SendVSync();
extern "C" void RequestIDR();
extern "C" void SetTracking(unsigned long long target_timestamp_ns,
                            const FfiDeviceMotion *device_motions,
                            int motions_count,
                            const FfiHandSkeleton *left_hand,
                            const FfiHandSkeleton *right_hand);
extern "C" void VideoErrorReportReceive();
extern "C" void ShutdownSteamvr();

extern "C" void SetOpenvrProperty(unsigned long long device_id, FfiOpenvrProperty prop);
extern "C" void RegisterButton(unsigned long long button_id);
extern "C" void SetViewsConfig(FfiViewsConfig config);
extern "C" void SetBattery(unsigned long long device_id, float gauge_value, bool is_plugged);
extern "C" void SetButton(unsigned long long button_id, FfiButtonValue value);

extern "C" void InitOpenvrClient();
extern "C" void ShutdownOpenvrClient();
extern "C" void SetChaperoneArea(float area_width, float area_height);

extern "C" void CaptureFrame();

// NalParsing.cpp
void ParseFrameNals(
    int codec, unsigned char *buf, int len, unsigned long long target_timestamp_ns, bool is_idr);

// CrashHandler.cpp
void HookCrashHandler();
