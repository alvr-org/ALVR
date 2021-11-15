//===================== Copyright (c) Valve Corporation. All Rights Reserved. ======================
//
// Example OpenVR driver for demonstrating IVRVirtualDisplay interface.
//
//==================================================================================================

#include "bindings.h"

#include <cstring>

#ifdef _WIN32
#include <windows.h>
#endif
#include "ClientConnection.h"
#include "Logger.h"
#include "OvrHMD.h"
#include "PoseHistory.h"
#include "Settings.h"
#include "driverlog.h"
#include "openvr_driver.h"

static void load_debug_privilege(void) {
#ifdef _WIN32
    const DWORD flags = TOKEN_ADJUST_PRIVILEGES | TOKEN_QUERY;
    TOKEN_PRIVILEGES tp;
    HANDLE token;
    LUID val;

    if (!OpenProcessToken(GetCurrentProcess(), flags, &token)) {
        return;
    }

    if (!!LookupPrivilegeValue(NULL, SE_DEBUG_NAME, &val)) {
        tp.PrivilegeCount = 1;
        tp.Privileges[0].Luid = val;
        tp.Privileges[0].Attributes = SE_PRIVILEGE_ENABLED;

        AdjustTokenPrivileges(token, false, &tp, sizeof(tp), NULL, NULL);
    }

    if (!!LookupPrivilegeValue(NULL, SE_INC_BASE_PRIORITY_NAME, &val)) {
        tp.PrivilegeCount = 1;
        tp.Privileges[0].Luid = val;
        tp.Privileges[0].Attributes = SE_PRIVILEGE_ENABLED;

        if (!AdjustTokenPrivileges(token, false, &tp, sizeof(tp), NULL, NULL)) {
            Warn("[GPU PRIO FIX] Could not set privilege to increase GPU priority\n");
        }
    }

    Debug("[GPU PRIO FIX] Succeeded to set some sort of priority.\n");

    CloseHandle(token);
#endif
}

#ifdef _WIN32
HINSTANCE g_hInstance;

BOOL WINAPI DllMain(HINSTANCE hInstance, DWORD dwReason, LPVOID lpReserved) {
    switch (dwReason) {
    case DLL_PROCESS_ATTACH:
        g_hInstance = hInstance;
    }

    return TRUE;
}
#endif

// bindigs for Rust

const unsigned char *FRAME_RENDER_VS_CSO_PTR;
unsigned int FRAME_RENDER_VS_CSO_LEN;
const unsigned char *FRAME_RENDER_PS_CSO_PTR;
unsigned int FRAME_RENDER_PS_CSO_LEN;
const unsigned char *QUAD_SHADER_CSO_PTR;
unsigned int QUAD_SHADER_CSO_LEN;
const unsigned char *COMPRESS_AXIS_ALIGNED_CSO_PTR;
unsigned int COMPRESS_AXIS_ALIGNED_CSO_LEN;
const unsigned char *COLOR_CORRECTION_CSO_PTR;
unsigned int COLOR_CORRECTION_CSO_LEN;

const char *g_sessionPath;
const char *g_driverRootDir;

void (*LogError)(const char *stringPtr);
void (*LogWarn)(const char *stringPtr);
void (*LogInfo)(const char *stringPtr);
void (*LogDebug)(const char *stringPtr);
void (*LegacySend)(unsigned char *buf, int len);

std::shared_ptr<OvrHmd> g_remoteHmd;

void InitializeCpp() {
    Settings::Instance().Load();

    g_remoteHmd = std::make_shared<OvrHmd>();

    g_remoteHmd->Activate(0);
}

void InitializeStreaming() {
    Settings::Instance().Load();

    g_remoteHmd->StartStreaming();
}

void DeinitializeStreaming() {
    if (g_remoteHmd) {
        g_remoteHmd->StopStreaming();
    }
}

void RequestIDR() {
    if (g_remoteHmd)
        g_remoteHmd->RequestIDR();
}

void LegacyReceive(unsigned char *buf, int len) {
    if (g_remoteHmd && g_remoteHmd->m_Listener) {
        g_remoteHmd->m_Listener->ProcessRecv(buf, len);
    }
}

void CreateSwapchain(unsigned int pid, SwapchainDesc desc, unsigned long long outHandles[3]) {
#ifdef _WIN32
    if (g_remoteHmd && g_remoteHmd->m_directModeComponent) {
        SwapTextureSet_t *pOutSwapTextureSet;
        g_remoteHmd->m_directModeComponent->CreateSwapTextureSet(
            pid, {desc.nWidth, desc.nHeight, desc.nFormat, desc.nSampleCount}, pOutSwapTextureSet);

        return pOutSwapTextureSet->rSharedTextureHandles;
    }
#endif
}

void DestroySwapchain(unsigned long long texture) {
#ifdef _WIN32
    if (g_remoteHmd && g_remoteHmd->m_directModeComponent) {
        g_remoteHmd->m_directModeComponent->DestroySwapTextureSet(texture);
    }
#endif
}

void PresentLayers(Layer (*layers)[2], int len) {
#ifdef _WIN32
    if (g_remoteHmd && g_remoteHmd->m_directModeComponent) {
        for (int i = 0; i < len; i++) {
            SubmitLayerPerEye_t layerPair[2];

            Layer l = layers[i][0];
            layerPair[0].hTexture = l.textureHandle;
            layerPair[0].bounds = {
                l.rect_offset[0],
                l.rect_offset[1],
                l.rect_size[0] - l.rect_offset[0],
                l.rect_size[1] - l.rect_offset[1],
            };
            layerPair[0].mHmdPose = {l.pose};

            l = layers[i][0];
            layerPair[1].hTexture = l.textureHandle;
            layerPair[1].bounds = {
                l.rect_offset[0],
                l.rect_offset[1],
                l.rect_size[0] - l.rect_offset[0],
                l.rect_size[1] - l.rect_offset[1],
            };
            layerPair[1].mHmdPose = {l.pose};

            g_remoteHmd->m_directModeComponent->SubmitLayer(layerPair);
        }

        g_remoteHmd->m_directModeComponent->Present(layers[0][0].textureHandle);
    }
#endif
}