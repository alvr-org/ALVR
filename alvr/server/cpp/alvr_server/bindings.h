#pragma once

struct EyeFov {
    float left = 49.;
    float right = 45.;
    float top = 50.;
    float bottom = 48.;
};

struct TrackingQuat {
    float x;
    float y;
    float z;
    float w;
};
struct TrackingVector3 {
    float x;
    float y;
    float z;
};
struct TrackingVector2 {
    float x;
    float y;
};
struct TrackingInfo {
    unsigned long long targetTimestampNs;
    TrackingQuat HeadPose_Pose_Orientation;
    TrackingVector3 HeadPose_Pose_Position;

    unsigned char mounted;

    static const unsigned int MAX_CONTROLLERS = 2;
    struct Controller {
        bool enabled;
        bool isHand;
        unsigned long long buttons;

        struct {
            float x;
            float y;
        } trackpadPosition;

        float triggerValue;
        float gripValue;

        // Tracking info of controller. (float * 19 = 76 bytes)
        TrackingQuat orientation;
        TrackingVector3 position;
        TrackingVector3 angularVelocity;
        TrackingVector3 linearVelocity;

        // Tracking info of hand. A3
        TrackingQuat boneRotations[19];
        // TrackingQuat boneRotationsBase[alvrHandBone_MaxSkinnable];
        TrackingVector3 bonePositionsBase[19];
        TrackingQuat boneRootOrientation;
        TrackingVector3 boneRootPosition;
        unsigned int handFingerConfidences;
    } controller[2];
};

struct ClientStats {
    unsigned long long targetTimestampNs;
    unsigned long long videoDecodeNs;
    unsigned long long renderingNs;
    unsigned long long vsyncQueueNs;
    unsigned long long totalPipelineLatencyNs;
};
struct VideoFrame {
    unsigned int type; // ALVR_PACKET_TYPE_VIDEO_FRAME
    unsigned int packetCounter;
    unsigned long long trackingFrameIndex;
    // FEC decoder needs some value for identify video frame number to detect new frame.
    // trackingFrameIndex becomes sometimes same value as previous video frame (in case of low
    // tracking rate).
    unsigned long long videoFrameIndex;
    unsigned long long sentTime;
    unsigned int frameByteSize;
    unsigned int fecIndex;
    unsigned short fecPercentage;
    // char frameBuffer[];
};
enum OpenvrPropertyType {
    Bool,
    Float,
    Int32,
    Uint64,
    Vector3,
    Double,
    String,
};

union OpenvrPropertyValue {
    bool bool_;
    float float_;
    int int32;
    unsigned long long uint64;
    float vector3[3];
    double double_;
    char string[64];
};

struct OpenvrProperty {
    unsigned int key;
    OpenvrPropertyType type;
    OpenvrPropertyValue value;
};

struct ViewsConfigData {
    EyeFov fov[2];
    float ipd_m;
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

extern "C" const char *g_sessionPath;
extern "C" const char *g_driverRootDir;

extern "C" void (*LogError)(const char *stringPtr);
extern "C" void (*LogWarn)(const char *stringPtr);
extern "C" void (*LogInfo)(const char *stringPtr);
extern "C" void (*LogDebug)(const char *stringPtr);
extern "C" void (*DriverReadyIdle)(bool setDefaultChaprone);
extern "C" void (*VideoSend)(VideoFrame header, unsigned char *buf, int len);
extern "C" void (*HapticsSend)(unsigned long long path,
                               float duration_s,
                               float frequency,
                               float amplitude);
extern "C" void (*ShutdownRuntime)();
extern "C" unsigned long long (*PathStringToHash)(const char *path);
extern "C" void (*ReportPresent)(unsigned long long timestamp_ns);
extern "C" void (*ReportComposed)(unsigned long long timestamp_ns);
extern "C" void (*ReportEncoded)(unsigned long long timestamp_ns);
extern "C" void (*ReportFecFailure)(int percentage);
extern "C" float (*GetTotalLatencyS)();

extern "C" void *CppEntryPoint(const char *pInterfaceName, int *pReturnCode);
extern "C" void InitializeStreaming();
extern "C" void DeinitializeStreaming();
extern "C" void RequestIDR();
extern "C" void SetChaperone(float areaWidth, float areaHeight);
extern "C" void InputReceive(TrackingInfo data);
extern "C" void ReportNetworkLatency(unsigned long long latencyUs);
extern "C" unsigned long long GetGameFrameIntervalNs();
extern "C" void VideoErrorReportReceive();
extern "C" void ShutdownSteamvr();

extern "C" void SetOpenvrProperty(unsigned long long topLevelPath, OpenvrProperty prop);
extern "C" void SetViewsConfig(ViewsConfigData config);
extern "C" void SetBattery(unsigned long long topLevelPath, float gauge_value, bool is_plugged);