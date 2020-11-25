#pragma once

#include <stdint.h>

// bindings for Rust:

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
extern "C" void (*MaybeKillWebServer)();

extern "C" void *CppEntryPoint(const char *pInterfaceName, int *pReturnCode);
extern "C" void InitializeStreaming();
extern "C" void DeinitializeStreaming();