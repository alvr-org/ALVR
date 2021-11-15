#pragma once

struct SwapchainDesc {
    unsigned int nWidth;
    unsigned int nHeight;
    unsigned int nFormat;
    unsigned int nSampleCount;
};

struct Layer {
    unsigned long long textureHandle;
    float pose[3][4];
    float rect_offset[2];
    float rect_size[2];
};

extern "C" const unsigned char *FRAME_RENDER_VS_CSO_PTR;
extern "C" unsigned int FRAME_RENDER_VS_CSO_LEN;
extern "C" const unsigned char *FRAME_RENDER_PS_CSO_PTR;
extern "C" unsigned int FRAME_RENDER_PS_CSO_LEN;
extern "C" const unsigned char *QUAD_SHADER_CSO_PTR;
extern "C" unsigned int QUAD_SHADER_CSO_LEN;
extern "C" const unsigned char *COMPRESS_AXIS_ALIGNED_CSO_PTR;
extern "C" unsigned int COMPRESS_AXIS_ALIGNED_CSO_LEN;
extern "C" const unsigned char *COLOR_CORRECTION_CSO_PTR;
extern "C" unsigned int COLOR_CORRECTION_CSO_LEN;

extern "C" const char *g_sessionPath;
extern "C" const char *g_driverRootDir;

extern "C" void (*LogError)(const char *stringPtr);
extern "C" void (*LogWarn)(const char *stringPtr);
extern "C" void (*LogInfo)(const char *stringPtr);
extern "C" void (*LogDebug)(const char *stringPtr);
extern "C" void (*LegacySend)(unsigned char *buf, int len);

extern "C" void InitializeCpp();
extern "C" void InitializeStreaming();
extern "C" void DeinitializeStreaming();
extern "C" void RequestIDR();
extern "C" void SetChaperone(const float transform[12],
                             float areaWidth,
                             float areaHeight,
                             float (*perimeterPoints)[2],
                             unsigned int perimeterPointsCount);
extern "C" void SetDefaultChaperone();
extern "C" void LegacyReceive(unsigned char *buf, int len);
extern "C" void
CreateSwapchain(unsigned int pid, SwapchainDesc desc, unsigned long long outHandles[3]);
extern "C" void
DestroySwapchain(unsigned long long texture); // the texture handle is used as swaphcain ID
extern "C" void PresentLayers(Layer (*layers)[2], int len);