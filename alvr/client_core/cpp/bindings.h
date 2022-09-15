#pragma once

struct EyeInput {
    float orientation[4]; // x, y, z, w
    float position[3];
    float fovLeft;
    float fovRight;
    float fovTop;
    float fovBottom;
};

struct VideoFrame {
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

struct StreamConfigInput {
    unsigned int eyeWidth;
    unsigned int eyeHeight;
    bool enableFoveation;
    float foveationCenterSizeX;
    float foveationCenterSizeY;
    float foveationCenterShiftX;
    float foveationCenterShiftY;
    float foveationEdgeRatioX;
    float foveationEdgeRatioY;
};

extern "C" const unsigned char *LOBBY_ROOM_GLTF_PTR;
extern "C" unsigned int LOBBY_ROOM_GLTF_LEN;
extern "C" const unsigned char *LOBBY_ROOM_BIN_PTR;
extern "C" unsigned int LOBBY_ROOM_BIN_LEN;

extern "C" void initNative();
extern "C" void destroyNative();
extern "C" void prepareLoadingRoom(int eyeWidth,
                                   int eyeHeight,
                                   bool darkMode,
                                   const int *swapchainTextures[2],
                                   int swapchainLength);
extern "C" void setStreamConfig(StreamConfigInput config);
extern "C" void streamStartNative(const int *swapchainTextures[2], int swapchainLength);
extern "C" void updateLoadingTexuture(const unsigned char *data);
extern "C" void renderLoadingNative(const EyeInput eyeInputs[2], const int swapchainIndices[2]);
extern "C" void destroyRenderers();
extern "C" void renderNative(const int swapchainIndices[2], void *streamHardwareBuffer);

extern "C" void initializeSocket(unsigned int codec, bool enableFEC);
extern "C" void legacyReceive(const unsigned char *packet, unsigned int packetSize);
extern "C" bool isConnectedNative();
extern "C" void closeSocket();

extern "C" unsigned long long (*pathStringToHash)(const char *path);

extern "C" void (*videoErrorReportSend)();
extern "C" void (*createDecoder)(const char *csd_0, int length);
extern "C" void (*pushNal)(const char *buffer, int length, unsigned long long frameIndex);