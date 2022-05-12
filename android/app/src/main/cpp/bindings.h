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

struct OnCreateResult {
    int streamSurfaceHandle;
    int loadingSurfaceHandle;
};

struct OnResumeResult {
    int recommendedEyeWidth;
    int recommendedEyeHeight;
    float *refreshRates;
    int refreshRatesCount;
};

struct GuardianData {
    bool shouldSync;
    float areaWidth;
    float areaHeight;
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
    bool extraLatencyMode;
};

extern "C" OnCreateResult onCreate(void *env, void *activity, void *assetManager);
extern "C" void destroyNative(void *env);
extern "C" void renderNative(long long renderedFrameIndex);
extern "C" void updateLoadingTexuture(const unsigned char *data);
extern "C" void renderLoadingNative();
extern "C" void onTrackingNative(bool clientsidePrediction);
extern "C" OnResumeResult onResumeNative(void *surface, bool darkMode);
extern "C" void setStreamConfig(StreamConfig config);
extern "C" void onStreamStartNative();
extern "C" void onPauseNative();
extern "C" void onHapticsFeedbackNative(unsigned long long path,
                                        float duration_s,
                                        float frequency,
                                        float amplitude);
extern "C" void onBatteryChangedNative(int battery, int plugged);
extern "C" GuardianData getGuardianData();

extern "C" void initializeSocket(unsigned int codec, bool enableFEC);
extern "C" void legacyReceive(const unsigned char *packet, unsigned int packetSize);
extern "C" unsigned char isConnectedNative();
extern "C" void closeSocket();

extern "C" void (*inputSend)(TrackingInfo data);
extern "C" void (*reportSubmit)(unsigned long long targetTimestampNs, unsigned long long vsyncQueueNs);
extern "C" unsigned long long (*getPredictionOffsetNs)();
extern "C" void (*videoErrorReportSend)();
extern "C" void (*viewsConfigSend)(EyeFov fov[2], float ipd_m);
extern "C" void (*batterySend)(unsigned long long device_path, float gauge_value, bool is_plugged);
extern "C" unsigned long long (*pathStringToHash)(const char *path);

extern "C" void (*pushNal)(const char *buffer, int length, unsigned long long frameIndex);