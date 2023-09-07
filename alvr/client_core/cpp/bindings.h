#pragma once

struct FfiViewInput {
    float orientation[4]; // x, y, z, w
    float position[3];
    float fovLeft;
    float fovRight;
    float fovUp;
    float fovDown;
    unsigned int swapchainIndex;
};

struct FfiStreamConfig {
    unsigned int viewWidth;
    unsigned int viewHeight;
    const unsigned int *swapchainTextures[2];
    unsigned int swapchainLength;
    unsigned int enableFoveation;
    float foveationCenterSizeX;
    float foveationCenterSizeY;
    float foveationCenterShiftX;
    float foveationCenterShiftY;
    float foveationEdgeRatioX;
    float foveationEdgeRatioY;
    unsigned int enableSrgbCorrection;
};

// gltf_model.h
extern "C" const unsigned char *LOBBY_ROOM_GLTF_PTR;
extern "C" unsigned int LOBBY_ROOM_GLTF_LEN;
extern "C" const unsigned char *LOBBY_ROOM_BIN_PTR;
extern "C" unsigned int LOBBY_ROOM_BIN_LEN;

// graphics.h
extern "C" void initGraphicsNative();
extern "C" void destroyGraphicsNative();
extern "C" void prepareLobbyRoom(int viewWidth,
                                 int viewHeight,
                                 const unsigned int *swapchainTextures[2],
                                 int swapchainLength);
extern "C" void destroyRenderers();
extern "C" void streamStartNative(FfiStreamConfig config);
extern "C" void updateLobbyHudTexture(const unsigned char *data);
extern "C" void renderLobbyNative(const FfiViewInput eyeInputs[2]);
extern "C" void renderStreamNative(void *streamHardwareBuffer,
                                   const unsigned int swapchainIndices[2]);
