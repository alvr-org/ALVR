//===================== Copyright (c) Valve Corporation. All Rights Reserved. ======================
//
// Example OpenVR driver for demonstrating IVRVirtualDisplay interface.
//
//==================================================================================================

#include <openvr_driver.h>

#include "Logger.h"
#include "OpenVRHmd.h"

HINSTANCE gInstance;

class CServerDriver_DisplayRedirect : public vr::IServerTrackedDeviceProvider
{
public:
	CServerDriver_DisplayRedirect()
		: mHmd( NULL )
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
	std::shared_ptr<OpenVRHmd> mHmd;
	std::shared_ptr<Listener> mListener;
	std::shared_ptr<IPCMutex> mMutex;
};

vr::EVRInitError CServerDriver_DisplayRedirect::Init( vr::IVRDriverContext *pContext )
{
	VR_INIT_SERVER_DRIVER_CONTEXT( pContext );

	mMutex = std::make_shared<IPCMutex>(APP_MUTEX_NAME, true);
	if (mMutex->AlreadyExist()) {
		// Duplicate driver installation.
		FatalLog(L"ALVR Server driver is installed on multiple locations. This causes some issues.\r\n"
			"Please check the installed driver list on About tab and uninstall old drivers.");
		return vr::VRInitError_Driver_Failed;
	}

	Settings::Instance().Load();

	mListener = std::make_shared<Listener>();
	if (!mListener->Startup())
	{
		return vr::VRInitError_Driver_Failed;
	}

	mHmd = std::make_shared<OpenVRHmd>(mListener);

	if (Settings::Instance().IsLoaded()) {
		// Launcher is running. Enable driver.
		mHmd->Enable();
	}

	return vr::VRInitError_None;
}

void CServerDriver_DisplayRedirect::Cleanup()
{
	mListener.reset();
	mHmd.reset();
	mMutex.reset();

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
	InitCrashHandler();

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
		gInstance = hInstance;
	}

	return TRUE;
}