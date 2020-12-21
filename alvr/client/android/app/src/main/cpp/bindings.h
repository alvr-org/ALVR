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

struct ConnectionMessage {
    const char *ip;
    unsigned int codec; // enum ALVR_CODEC
    bool realtimeDecoder;
    unsigned int videoWidth; // in pixels
    unsigned int videoHeight; // in pixels
    unsigned int bufferSize; // in bytes
    unsigned int frameQueueSize;
    unsigned char refreshRate;
    bool streamMic;
    unsigned char foveationMode;
    float foveationStrength;
    float foveationShape;
    float foveationVerticalOffset;
    unsigned int trackingSpace; // ALVR_TRACKING_SPACE
    const char *webGuiUrl;
};

extern "C" void decoderInput(long long frameIndex);
extern "C" void decoderOutput(long long frameIndex);

extern "C" OnCreateResult onCreate(void *env, void *activity, void *assetManager);
extern "C" void destroyNative(void *env);
extern "C" void renderNative(long long renderedFrameIndex);
extern "C" void renderLoadingNative();
extern "C" void onTrackingNative(void *env, void *udpReceiverThread);
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

extern "C" void connectSocket(void *env, ConnectionMessage message);
extern "C" void
initializeSocket(void *env, void *instance, int helloPort, int port, void *deviceName,
                 void *broadcastAddrList, int refreshRate, int renderWidth, int renderHeight);
extern "C" void closeSocket();
extern "C" void runLoop(void *env, void *instance);
extern "C" void interruptNative();
extern "C" unsigned char isConnectedNative();
extern "C" void *getServerAddress(void *env);
extern "C" int getServerPort();
extern "C" void sendNative(long long nativeBuffer, int bufferLength);
extern "C" void setSinkPreparedNative(unsigned char prepared);
extern "C" void disconnectSocket(void *env);