#pragma once

// bindings for Rust:

extern "C" __declspec(dllexport) const char *g_alvrDir;

extern "C" __declspec(dllexport) void (*LogError)(const char *stringPtr);
extern "C" __declspec(dllexport) void (*LogWarn)(const char *stringPtr);
extern "C" __declspec(dllexport) void (*LogInfo)(const char *stringPtr);
extern "C" __declspec(dllexport) void (*LogDebug)(const char *stringPtr);
extern "C" __declspec(dllexport) void (*MaybeKillWebServer)();

extern "C" __declspec(dllexport) void *CppEntryPoint(const char *pInterfaceName, int *pReturnCode);