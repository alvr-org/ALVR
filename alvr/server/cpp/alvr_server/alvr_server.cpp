//===================== Copyright (c) Valve Corporation. All Rights Reserved. ======================
//
// Example OpenVR driver for demonstrating IVRVirtualDisplay interface.
//
//==================================================================================================

#include "bindings.h"

#if _WIN32

#include <windows.h>
#include "openvr_driver.h"
#include "sharedstate.h"
#include "OvrHMD.h"
#include "driverlog.h"
#include "MicPlayer.h"
#include "ChaperoneUpdater.h"

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
			LogDriver("[GPU PRIO FIX] Could not set privilege to increase GPU priority");
		}
	}

	LogDriver("[GPU PRIO FIX] Succeeded to set some sort of priority.");

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
		FatalLog("ALVR driver is already running.");
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
std::shared_ptr<MicPlayer> g_MicPlayer;
std::shared_ptr<ChaperoneUpdater> g_ChaperoneUpdater;


BOOL WINAPI DllMain(HINSTANCE hInstance, DWORD dwReason, LPVOID lpReserved)
{
	switch (dwReason) {
	case DLL_PROCESS_ATTACH:
		g_hInstance = hInstance;
	}

	return TRUE;
}

#endif

//--------------------------------------------------------------------------

// Rust to C++:

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

void (*LogError)(const char *);
void (*LogWarn)(const char *);
void (*LogInfo)(const char *);
void (*LogDebug)(const char *);
void (*SendVideo)(uint64_t, uint8_t *, int, uint64_t);
void (*SendAudio)(uint64_t, uint8_t *, int, uint64_t);
void (*SendHapticsFeedback)(float, float, float, uint8_t);
void (*ReportEncodeLatency)(uint64_t);
void (*ShutdownRuntime)();

// C++ to Rust:

void *CppEntryPoint(const char *pInterfaceName, int *pReturnCode)
{
#if _WIN32
	Settings::Instance().Load();

	load_debug_privilege();

	LogDriver("HmdDriverFactory %hs (%hs)", pInterfaceName, vr::IServerTrackedDeviceProvider_Version);
	if (0 == strcmp(vr::IServerTrackedDeviceProvider_Version, pInterfaceName))
	{
		LogDriver("HmdDriverFactory server return");
		return &g_serverDriverDisplayRedirect;
	}

	if (pReturnCode)
		*pReturnCode = vr::VRInitError_Init_InterfaceNotFound;
#endif

	return nullptr;
}
void InitalizeStreaming(StreamSettings settings) {
	Settings::Instance().UpdateForStream(settings);
	g_serverDriverDisplayRedirect.m_pRemoteHmd->OnStreamStart();
	g_MicPlayer	= std::make_shared<MicPlayer>();
	g_ChaperoneUpdater = std::make_shared<ChaperoneUpdater>();
}
void UpdatePose(TrackingInfo info) {
	g_serverDriverDisplayRedirect.m_pRemoteHmd->OnPoseUpdated(info);
}
void HandlePacketLoss() {
	g_serverDriverDisplayRedirect.m_pRemoteHmd->OnPacketLoss();
}
void PlayMicAudio(uint8_t *data, int size) {
	if (g_MicPlayer) {
		g_MicPlayer->playAudio((char *)data, size);
	}
}
void UpdateChaperone(
	TrackingVector3 standingPosPosition,
	TrackingQuat standingPosRotation,
	TrackingVector2 playAreaSize,
	TrackingVector3 *points,
	int count)
{
	if (g_ChaperoneUpdater) {
		g_ChaperoneUpdater->ResetData(0, count);
		g_ChaperoneUpdater->SetTransform(standingPosPosition, standingPosRotation, playAreaSize);
		
		if (count == 0) {
			g_ChaperoneUpdater->GenerateStandingChaperone();
		} else {
			for (int i = 0; i < (count + 1) % 100; i++) {
				g_ChaperoneUpdater->SetSegment(i, &points[i * 100]);
			}
		}

		if (g_ChaperoneUpdater->MaybeCommitData()) {
			Info("Synced Guardian data to SteamVR Chaperone.");
		}
	}
}
extern "C" void ShutdownSteamvr() {
	g_serverDriverDisplayRedirect.m_pRemoteHmd->OnShutdown();
}