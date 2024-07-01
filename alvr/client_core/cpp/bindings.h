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
    unsigned int fixLimitedRange;
    float encodingGamma;
};

// graphics.h
extern "C" void initGraphicsNative();
extern "C" void destroyStream();
extern "C" void streamStartNative(FfiStreamConfig config);
extern "C" void renderStreamNative(void *streamHardwareBuffer,
                                   const unsigned int swapchainIndices[2]);
