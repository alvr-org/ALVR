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

        unsigned char recenterCount;

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
// Report packet loss/error from client to server.
struct PacketErrorReport {
    unsigned int type; // ALVR_PACKET_TYPE_PACKET_ERROR_REPORT
    unsigned int lostFrameType;
    unsigned int fromPacketCounter;
    unsigned int toPacketCounter;
};
// Send haptics feedback from server to client.
struct HapticsFeedback {
    unsigned int type;            // ALVR_PACKET_TYPE_HAPTICS
    unsigned long long startTime; // Elapsed time from now when start haptics. In microseconds.
    float amplitude;
    float duration;
    float frequency;
    unsigned char hand; // 0:Right, 1:Left
};

struct OnCreateResult {
    int streamSurfaceHandle;
    int loadingSurfaceHandle;
};

enum class DeviceType {
    OCULUS_GO,
    OCULUS_QUEST,
    OCULUS_QUEST_2,
    UNKNOWN,
};

struct OnResumeResult {
    DeviceType deviceType;
    int recommendedEyeWidth;
    int recommendedEyeHeight;
    float *refreshRates;
    int refreshRatesCount;
};

struct GuardianData {
    bool shouldSync;
    float position[3];
    float rotation[4]; // convention: x, y, z, w
    float areaWidth;
    float areaHeight;
    float (*perimeterPoints)[3];
    unsigned int perimeterPointsCount;
};

struct StreamConfig {
    unsigned int eyeWidth;
    unsigned int eyeHeight;
    float refreshRate;
    bool enableFoveation;
    float foveationCenterSizeX;
    float foveationCenterSizeY;
    float foveationCenterShiftX;
    float foveationCenterShiftY;
    float foveationEdgeRatioX;
    float foveationEdgeRatioY;
    int trackingSpaceType;
    bool extraLatencyMode;
};

extern "C" void decoderInput(long long frameIndex);
extern "C" void decoderOutput(long long frameIndex);

extern "C" OnCreateResult onCreate(void *env, void *activity, void *assetManager);
extern "C" void destroyNative(void *env);
extern "C" void renderNative(long long renderedFrameIndex);
extern "C" void renderLoadingNative();
extern "C" void onTrackingNative(bool clientsidePrediction);
extern "C" OnResumeResult onResumeNative(void *surface, bool darkMode);
extern "C" void setStreamConfig(StreamConfig config);
extern "C" void onStreamStartNative();
extern "C" void onPauseNative();
extern "C" void onHapticsFeedbackNative(
    long long startTime, float amplitude, float duration, float frequency, unsigned char hand);
extern "C" void onBatteryChangedNative(int battery, int plugged);
extern "C" GuardianData getGuardianData();

extern "C" void
initializeSocket(void *env, void *instance, void *nalClass, unsigned int codec, bool enableFEC);
extern "C" void legacyReceive(const unsigned char *packet, unsigned int packetSize);
extern "C" void sendTimeSync();
extern "C" unsigned char isConnectedNative();
extern "C" void closeSocket(void *env);

extern "C" void (*inputSend)(TrackingInfo data);
extern "C" void (*timeSyncSend)(TimeSync data);
extern "C" void (*videoErrorReportSend)();
extern "C" void (*viewsConfigSend)(EyeFov fov[2], float ipd_m);
extern "C" void (*batterySend)(unsigned long long device_path, float gauge_value, bool is_plugged);
extern "C" unsigned long long (*pathStringToHash)(const char *path);