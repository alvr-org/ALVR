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
    unsigned int type; // ALVR_PACKET_TYPE_TRACKING_INFO
    static const unsigned int FLAG_OTHER_TRACKING_SOURCE =
        (1 << 0); // Other_Tracking_Source_Position has valid value (For ARCore)
    unsigned int flags;

    unsigned long long clientTime;
    unsigned long long FrameIndex;
    double predictedDisplayTime;
    TrackingQuat HeadPose_Pose_Orientation;
    TrackingVector3 HeadPose_Pose_Position;

    TrackingVector3 Other_Tracking_Source_Position;
    TrackingQuat Other_Tracking_Source_Orientation;

    unsigned char mounted;

    static const unsigned int MAX_CONTROLLERS = 2;
    struct Controller {
        static const unsigned int FLAG_CONTROLLER_ENABLE = (1 << 0);
        static const unsigned int FLAG_CONTROLLER_LEFTHAND =
            (1 << 1); // 0: Left hand, 1: Right hand
        static const unsigned int FLAG_CONTROLLER_GEARVR = (1 << 2);
        static const unsigned int FLAG_CONTROLLER_OCULUS_GO = (1 << 3);
        static const unsigned int FLAG_CONTROLLER_OCULUS_QUEST = (1 << 4);
        static const unsigned int FLAG_CONTROLLER_OCULUS_HAND = (1 << 5);
        unsigned int flags;
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
// Client >----(mode 0)----> Server
// Client <----(mode 1)----< Server
// Client >----(mode 2)----> Server
// Client <----(mode 3)----< Server
struct TimeSync {
    unsigned int type; // ALVR_PACKET_TYPE_TIME_SYNC
    unsigned int mode; // 0,1,2,3
    unsigned long long sequence;
    unsigned long long serverTime;
    unsigned long long clientTime;

    // Following value are filled by client only when mode=0.
    unsigned long long packetsLostTotal;
    unsigned long long packetsLostInSecond;

    unsigned int averageTotalLatency;

    unsigned int averageSendLatency;

    unsigned int averageTransportLatency;

    unsigned long long averageDecodeLatency;

    unsigned int idleTime;

    unsigned int fecFailure;
    unsigned long long fecFailureInSecond;
    unsigned long long fecFailureTotal;

    float fps;

    // Following value are filled by server only when mode=1.
    unsigned int serverTotalLatency;

    // Following value are filled by server only when mode=3.
    unsigned long long trackingRecvFrameIndex;
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
extern "C" void (*TimeSyncSend)(TimeSync packet);
extern "C" void (*ShutdownRuntime)();
extern "C" unsigned long long (*PathStringToHash)(const char *path);

extern "C" void *CppEntryPoint(const char *pInterfaceName, int *pReturnCode);
extern "C" void InitializeStreaming();
extern "C" void DeinitializeStreaming();
extern "C" void RequestIDR();
extern "C" void SetChaperone(float areaWidth, float areaHeight);
extern "C" void InputReceive(TrackingInfo data);
extern "C" void TimeSyncReceive(TimeSync data);
extern "C" void VideoErrorReportReceive();
extern "C" void ShutdownSteamvr();

extern "C" void SetOpenvrProperty(unsigned long long topLevelPath, OpenvrProperty prop);
extern "C" void SetViewsConfig(ViewsConfigData config);
extern "C" void SetBattery(unsigned long long topLevelPath, float gauge_value, bool is_plugged);