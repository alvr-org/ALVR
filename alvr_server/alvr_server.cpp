//===================== Copyright (c) Valve Corporation. All Rights Reserved. ======================
//
// Example OpenVR driver for demonstrating IVRVirtualDisplay interface.
//
//==================================================================================================

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
			Log(L"[GPU PRIO FIX] Could not set privilege to increase GPU priority");
		}
	}

	Log(L"[GPU PRIO FIX] Succeeded to set some sort of priority.");

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

private:
	std::shared_ptr<OvrHmd> m_pRemoteHmd; 
	std::shared_ptr<ClientConnection> m_Listener; 
	std::shared_ptr<IPCMutex> m_mutex; 
};

vr::EVRInitError CServerDriver_DisplayRedirect::Init( vr::IVRDriverContext *pContext )
{
	VR_INIT_SERVER_DRIVER_CONTEXT( pContext );
	InitDriverLog(vr::VRDriverLog());
	

	m_mutex = std::make_shared<IPCMutex>(APP_MUTEX_NAME, true);
	if (m_mutex->AlreadyExist()) {
		// Duplicate driver installation.
		FatalLog("ALVR Server driver is installed on multiple locations. This causes some issues.\r\n"
			"Please check the installed driver list on About tab and uninstall old drivers.");
		return vr::VRInitError_Driver_Failed;
	}

	//load settings from mapped file
	Settings::Instance().Load();

	//create listener
	m_Listener = std::make_shared<ClientConnection>();

	//init listener
	if (!m_Listener->Startup())
	{
		return vr::VRInitError_Driver_Failed;
	}

	//create new virtuall hmd
	m_pRemoteHmd = std::make_shared<OvrHmd>(m_Listener);

	
	if (Settings::Instance().IsLoaded()) {
		// Launcher is running. Enable driver.Settings access mem mapped file
		m_pRemoteHmd->Enable();
	}



	return vr::VRInitError_None;
}

void CServerDriver_DisplayRedirect::Cleanup()
{
	m_Listener.reset();
	m_pRemoteHmd.reset();
	m_mutex.reset();

	CleanupDriverLog();

	VR_CLEANUP_SERVER_DRIVER_CONTEXT();
}

void CServerDriver_DisplayRedirect::RunFrame()
{
}

CServerDriver_DisplayRedirect g_serverDriverDisplayRedirect;

//-----------------------------------------------------------------------------
// Purpose: Entry point for vrserver when loading drivers.
//-----------------------------------------------------------------------------
extern "C" __declspec( dllexport )
void *HmdDriverFactory( const char *pInterfaceName, int *pReturnCode )
{
	//init logger
	InitCrashHandler();

	load_debug_privilege();

	Log(L"HmdDriverFactory %hs (%hs)", pInterfaceName, vr::IServerTrackedDeviceProvider_Version);
	if ( 0 == strcmp( vr::IServerTrackedDeviceProvider_Version, pInterfaceName ) )
	{
		Log(L"HmdDriverFactory server return");
		return &g_serverDriverDisplayRedirect;
	}

	if( pReturnCode )
		*pReturnCode = vr::VRInitError_Init_InterfaceNotFound;

	return NULL;
}

BOOL WINAPI DllMain(HINSTANCE hInstance, DWORD dwReason, LPVOID lpReserved)
{
	switch (dwReason) {
	case DLL_PROCESS_ATTACH:
		g_hInstance = hInstance;
	}

	return TRUE;
}