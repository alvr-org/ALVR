#pragma once

enum class DeviceType {
    OCULUS_QUEST,
    OCULUS_QUEST_2,
    UNKNOWN,
};

struct OnCreateResult {
    DeviceType deviceType;
    int recommendedEyeWidth;
    int recommendedEyeHeight;
    float *refreshRates;
    int refreshRatesCount;
    int streamSurfaceHandle;
    int loadingSurfaceHandle;
};

extern "C" void decoderInput(long long frameIndex);
extern "C" void decoderOutput(long long frameIndex);

extern "C" OnCreateResult onCreate(void *env, void *activity, void *assetManager);
extern "C" void destroyNative(void *env);
extern "C" void renderNative(long long renderedFrameIndex);
extern "C" void renderLoadingNative();
extern "C" void onTrackingNative();
extern "C" void onResumeNative(void *env, void *surface);
extern "C" void onStreamStartNative(int width, int height, int refreshRate, unsigned char streamMic,
                                    int foveationMode, float foveationStrength,
                                    float foveationShape, float foveationVerticalOffset,
                                    int trackingSpaceType);
extern "C" void onPauseNative();
extern "C" void
onHapticsFeedbackNative(long long startTime, float amplitude, float duration,
                        float frequency, unsigned char hand);
extern "C" void onGuardianSyncAckNative(long long timestamp);
extern "C" void onGuardianSegmentAckNative(long long timestamp, int segmentIndex);
extern "C" void onBatteryChangedNative(int battery);

extern "C" void
initializeSocket(void *env, void *instance, void *nalClass, const char *ip, unsigned int codec,
                 unsigned int bufferSize);
extern "C" unsigned char isConnectedNative();
extern "C" void runSocketLoopIter();
extern "C" void sendNative(long long nativeBuffer, int bufferLength);
extern "C" void closeSocket(void *env);