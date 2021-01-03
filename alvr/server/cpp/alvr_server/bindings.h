#pragma once

#include <stdint.h>

extern "C" const uint8_t *FRAME_RENDER_VS_CSO_PTR;
extern "C" uint32_t FRAME_RENDER_VS_CSO_LEN;
extern "C" const uint8_t *FRAME_RENDER_PS_CSO_PTR;
extern "C" uint32_t FRAME_RENDER_PS_CSO_LEN;
extern "C" const uint8_t *QUAD_SHADER_CSO_PTR;
extern "C" uint32_t QUAD_SHADER_CSO_LEN;
extern "C" const uint8_t *COMPRESS_SLICES_CSO_PTR;
extern "C" uint32_t COMPRESS_SLICES_CSO_LEN;
extern "C" const uint8_t *COLOR_CORRECTION_CSO_PTR;
extern "C" uint32_t COLOR_CORRECTION_CSO_LEN;

extern "C" const char *g_alvrDir;

extern "C" void (*LogError)(const char *stringPtr);
extern "C" void (*LogWarn)(const char *stringPtr);
extern "C" void (*LogInfo)(const char *stringPtr);
extern "C" void (*LogDebug)(const char *stringPtr);
extern "C" void (*DriverReadyIdle)();
extern "C" void (*ShutdownRuntime)();

extern "C" void *CppEntryPoint(const char *pInterfaceName, int *pReturnCode);
extern "C" void InitializeStreaming();
extern "C" void DeinitializeStreaming();
extern "C" void RequestIDR();
extern "C" void SetChaperone(const float transform[12], float areaWidth, float areaHeight,
                             float (*perimeterPoints)[2], unsigned int perimeterPointsCount);
extern "C" void SetDefaultChaperone();
extern "C" void ShutdownSteamvr();