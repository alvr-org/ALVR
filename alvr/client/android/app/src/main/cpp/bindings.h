#pragma once

enum class DeviceType {
    OCULUS_QUEST,
    OCULUS_QUEST_2,
    UNKNOWN,
};

struct EyeFov {
    float left;
    float right;
    float top;
    float bottom;
};

struct OnCreateResult {
    int surfaceTextureHandle;
    int webViewSurfaceHandle;
};

struct OnResumeResult {
    DeviceType deviceType;
    int recommendedEyeWidth;
    int recommendedEyeHeight;
    EyeFov leftEyeFov;
    float *refreshRates;
    int refreshRatesCount;
    float defaultRefreshRate;
};

struct OnStreamStartParams {
    int eyeWidth;
    int eyeHeight;
    EyeFov leftEyeFov;
    bool foveationEnabled;
    float foveationStrength;
    float foveationShape;
    float foveationVerticalOffset;
    bool enableGameAudio;
    bool enableMicrophone;
    float refreshRate;
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
    unsigned long long clientTime;
    unsigned long long FrameIndex;
    double predictedDisplayTime;
    TrackingQuat HeadPose_Pose_Orientation;
    TrackingVector3 HeadPose_Pose_Position;

    static const unsigned int MAX_CONTROLLERS = 2;


    struct Controller {
        static const unsigned int FLAG_CONTROLLER_ENABLE = (1 << 0);
        static const unsigned int FLAG_CONTROLLER_LEFTHAND = (1
                << 1); // 0: Left hand, 1: Right hand
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

        unsigned char batteryPercentRemaining;

        // Tracking info of controller. (float * 19 = 76 bytes)
        TrackingQuat orientation;
        TrackingVector3 position;
        TrackingVector3 angularVelocity;
        TrackingVector3 linearVelocity;
        TrackingVector3 angularAcceleration;
        TrackingVector3 linearAcceleration;

        // Tracking info of hand. A3
        TrackingQuat boneRotations[19];
        //TrackingQuat boneRotationsBase[alvrHandBone_MaxSkinnable];
        TrackingVector3 bonePositionsBase[19];
        TrackingQuat boneRootOrientation;
        TrackingVector3 boneRootPosition;
        unsigned int inputStateStatus;
        float fingerPinchStrengths[4];
        unsigned int handFingerConfidences;
    } controller[2];
};

struct MicAudioFrame {
    short *buffer;
    long long size;
};

struct GuardianData {
    TrackingQuat standingPosRotation;
    TrackingVector3 standingPosPosition;
    TrackingVector2 playAreaSize;
    unsigned int totalPointCount;
    TrackingVector3 *points;
};

// Note: JNI object are obscured behind void* to avoid problems when binding to Rust

extern "C" OnCreateResult onCreate(void *env, void *activity, void *assetManager);

extern "C" OnResumeResult onResume(void *env, void *surface);

extern "C" void onStreamStart(OnStreamStartParams params);

extern "C" void render(bool streaming, long long renderedFrameIndex);

extern "C" TrackingInfo getTrackingInfo();

extern "C" void enqueueAudio(unsigned char *buf, int len);

extern "C" MicAudioFrame getMicData();

extern "C" void
onHapticsFeedback(unsigned long long startTime, float amplitude, float duration, float frequency,
                  int hand);

extern "C" GuardianData getGuardianInfo();

extern "C" void onStreamStop();

extern "C" void onPause();

extern "C" void onDestroy(void *v_env);