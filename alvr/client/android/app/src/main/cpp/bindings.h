#pragma once

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
    bool streamMic;
    bool enableFoveation;
    float foveationStrength;
    float foveationShape;
    float foveationVerticalOffset;
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
extern "C" void
onHapticsFeedbackNative(long long startTime, float amplitude, float duration,
                        float frequency, unsigned char hand);
extern "C" void onBatteryChangedNative(int battery);
extern "C" GuardianData getGuardianData();

extern "C" void
initializeSocket(void *env, void *instance, void *nalClass, const char *ip, unsigned int codec,
                 unsigned int bufferSize, bool enableFEC);
extern "C" unsigned char isConnectedNative();
extern "C" void runSocketLoopIter();
extern "C" void sendNative(long long nativeBuffer, int bufferLength);
extern "C" void closeSocket(void *env);