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
extern "C" void onBatteryChangedNative(int battery, int plugged);
extern "C" GuardianData getGuardianData();

extern "C" void
initializeSocket(void *env, void *instance, void *nalClass, unsigned int codec, bool enableFEC);
extern "C" void (*legacySend)(const unsigned char *buffer, unsigned int size);
extern "C" void legacyReceive(const unsigned char *packet, unsigned int packetSize);
extern "C" void sendTimeSync();
extern "C" unsigned char isConnectedNative();
extern "C" void closeSocket(void *env);
