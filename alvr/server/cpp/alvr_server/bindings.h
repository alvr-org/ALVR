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
    float jointPositions[31][3];
    FfiQuat jointRotations[31];
};

struct FfiDeviceMotion {
    unsigned long long deviceID;
    FfiQuat orientation;
    float position[3];
    float linearVelocity[3];
    float angularVelocity[3];
};

struct FfiBodyTracker {
    unsigned int trackerID;
    FfiQuat orientation;
    float position[3];
    unsigned int tracking;
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
extern "C" void (*VideoSend)(unsigned long long targetTimestampNs,
                             unsigned char *buf,
                             int len,
                             bool isIdr);
extern "C" void (*HapticsSend)(unsigned long long path,
                               float duration_s,
                               float frequency,
                               float amplitude);
extern "C" void (*ShutdownRuntime)();
extern "C" unsigned long long (*PathStringToHash)(const char *path);
extern "C" void (*ReportPresent)(unsigned long long timestamp_ns, unsigned long long offset_ns);
extern "C" void (*ReportComposed)(unsigned long long timestamp_ns, unsigned long long offset_ns);
extern "C" FfiDynamicEncoderParams (*GetDynamicEncoderParams)();
extern "C" unsigned long long (*GetSerialNumber)(unsigned long long deviceID, char *outString);
extern "C" void (*SetOpenvrProps)(unsigned long long deviceID);
extern "C" void (*RegisterButtons)(unsigned long long deviceID);
extern "C" void (*WaitForVSync)();

extern "C" void *CppEntryPoint(const char *pInterfaceName, int *pReturnCode);
extern "C" void InitializeStreaming();
extern "C" void DeinitializeStreaming();
extern "C" void SendVSync();
extern "C" void RequestIDR();
extern "C" void SetTracking(unsigned long long targetTimestampNs,
                            float controllerPoseTimeOffsetS,
                            const FfiDeviceMotion *deviceMotions,
                            int motionsCount,
                            const FfiHandSkeleton *leftHand,
                            const FfiHandSkeleton *rightHand,
                            unsigned int controllersTracked,
                            const FfiBodyTracker *bodyTrackers,
                            int bodyTrackersCount);
extern "C" void VideoErrorReportReceive();
extern "C" void ShutdownSteamvr();

extern "C" void SetOpenvrProperty(unsigned long long deviceID, FfiOpenvrProperty prop);
extern "C" void RegisterButton(unsigned long long buttonID);
extern "C" void SetViewsConfig(FfiViewsConfig config);
extern "C" void SetBattery(unsigned long long deviceID, float gauge_value, bool is_plugged);
extern "C" void SetButton(unsigned long long buttonID, FfiButtonValue value);

extern "C" void InitOpenvrClient();
extern "C" void ShutdownOpenvrClient();
extern "C" void SetChaperoneArea(float areaWidth, float areaHeight);

extern "C" void CaptureFrame();

// NalParsing.cpp
void ParseFrameNals(
    int codec, unsigned char *buf, int len, unsigned long long targetTimestampNs, bool isIdr);

// CrashHandler.cpp
void HookCrashHandler();
