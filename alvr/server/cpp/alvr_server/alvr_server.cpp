//===================== Copyright (c) Valve Corporation. All Rights Reserved. ======================
//
// Example OpenVR driver for demonstrating IVRVirtualDisplay interface.
//
//==================================================================================================

#include "bindings.h"

#include <windows.h>
#include "openvr_driver.h"
#include "sharedstate.h"
#include "ClientConnection.h"
#include "OvrHMD.h"
#include "driverlog.h"

HINSTANCE g_hInstance;


static void load_debug_privilege(void)
{
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

		AdjustTokenPrivileges(token, false, &tp, sizeof(tp), NULL,
			NULL);
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
}

//-----------------------------------------------------------------------------
// Purpose: Server interface implementation.
//-----------------------------------------------------------------------------
class CServerDriver_DisplayRedirect : public vr::IServerTrackedDeviceProvider
{
public:
	CServerDriver_DisplayRedirect()
		: m_pRemoteHmd( NULL )
	{}

	virtual vr::EVRInitError Init( vr::IVRDriverContext *pContext ) override;
	virtual void Cleanup() override;
	virtual const char * const *GetInterfaceVersions() override
		{ return vr::k_InterfaceVersions;  }
	virtual const char *GetTrackedDeviceDriverVersion()
		{ return vr::ITrackedDeviceServerDriver_Version; }
	virtual void RunFrame();
	virtual bool ShouldBlockStandbyMode() override { return false; }
	virtual void EnterStandby() override {}
	virtual void LeaveStandby() override {}

	std::shared_ptr<OvrHmd> m_pRemoteHmd;
	std::shared_ptr<IPCMutex> m_mutex; 
};

vr::EVRInitError CServerDriver_DisplayRedirect::Init( vr::IVRDriverContext *pContext )
{
	VR_INIT_SERVER_DRIVER_CONTEXT( pContext );
	InitDriverLog(vr::VRDriverLog());
	

	m_mutex = std::make_shared<IPCMutex>(APP_MUTEX_NAME, true);
	if (m_mutex->AlreadyExist()) {
		// Duplicate driver installation.
		Error("ALVR Server driver is installed on multiple locations. This causes some issues.\n"
			"Please check the installed driver list on About tab and uninstall old drivers.\n");
		return vr::VRInitError_Driver_Failed;
	}

	//create new virtuall hmd
	m_pRemoteHmd = std::make_shared<OvrHmd>();

	// Launcher is running. Enable driver.
	m_pRemoteHmd->Enable();

	return vr::VRInitError_None;
}

void CServerDriver_DisplayRedirect::Cleanup()
{
	m_pRemoteHmd.reset();
	m_mutex.reset();

	CleanupDriverLog();

	VR_CLEANUP_SERVER_DRIVER_CONTEXT();
}

void CServerDriver_DisplayRedirect::RunFrame()
{
}

CServerDriver_DisplayRedirect g_serverDriverDisplayRedirect;


BOOL WINAPI DllMain(HINSTANCE hInstance, DWORD dwReason, LPVOID lpReserved)
{
	switch (dwReason) {
	case DLL_PROCESS_ATTACH:
		g_hInstance = hInstance;
	}

	return TRUE;
}

// bindigs for Rust

const uint8_t *FRAME_RENDER_VS_CSO_PTR;
uint32_t FRAME_RENDER_VS_CSO_LEN;
const uint8_t *FRAME_RENDER_PS_CSO_PTR;
uint32_t FRAME_RENDER_PS_CSO_LEN;
const uint8_t *QUAD_SHADER_CSO_PTR;
uint32_t QUAD_SHADER_CSO_LEN;
const uint8_t *COMPRESS_SLICES_CSO_PTR;
uint32_t COMPRESS_SLICES_CSO_LEN;
const uint8_t *COLOR_CORRECTION_CSO_PTR;
uint32_t COLOR_CORRECTION_CSO_LEN;

const char *g_alvrDir;

void (*LogError)(const char *stringPtr);
void (*LogWarn)(const char *stringPtr);
void (*LogInfo)(const char *stringPtr);
void (*LogDebug)(const char *stringPtr);
void (*MaybeLaunchWebServer)();
void (*MaybeKillWebServer)();

void *CppEntryPoint(const char *pInterfaceName, int *pReturnCode)
{
	Settings::Instance().Load();

	load_debug_privilege();

	Debug("HmdDriverFactory %hs (%hs)\n", pInterfaceName, vr::IServerTrackedDeviceProvider_Version);
	if (0 == strcmp(vr::IServerTrackedDeviceProvider_Version, pInterfaceName))
	{
		Debug("HmdDriverFactory server return\n");
		return &g_serverDriverDisplayRedirect;
	}

	if (pReturnCode)
		*pReturnCode = vr::VRInitError_Init_InterfaceNotFound;

	return NULL;
}

void InitializeStreaming() {
	g_serverDriverDisplayRedirect.m_pRemoteHmd->StartStreaming();
}

void DeinitializeStreaming() {
	g_serverDriverDisplayRedirect.m_pRemoteHmd->StopStreaming();
}