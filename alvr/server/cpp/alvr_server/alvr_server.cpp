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
#include "openvr_driver.h"
#include "ClientConnection.h"
#include "OvrHMD.h"
#include "driverlog.h"
#include "Settings.h"
#include "Logger.h"


static void load_debug_privilege(void)
{
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
#endif
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
	virtual void RunFrame() override;
	virtual bool ShouldBlockStandbyMode() override { return false; }
	virtual void EnterStandby() override {}
	virtual void LeaveStandby() override {}

	std::shared_ptr<OvrHmd> m_pRemoteHmd;
};

vr::EVRInitError CServerDriver_DisplayRedirect::Init( vr::IVRDriverContext *pContext )
{
	VR_INIT_SERVER_DRIVER_CONTEXT( pContext );
	InitDriverLog(vr::VRDriverLog());

	//create new virtuall hmd
	m_pRemoteHmd = std::make_shared<OvrHmd>();

	return vr::VRInitError_None;
}

void CServerDriver_DisplayRedirect::Cleanup()
{
	m_pRemoteHmd.reset();

	CleanupDriverLog();

	VR_CLEANUP_SERVER_DRIVER_CONTEXT();
}

void CServerDriver_DisplayRedirect::RunFrame()
{
}

CServerDriver_DisplayRedirect g_serverDriverDisplayRedirect;


#ifdef _WIN32
HINSTANCE g_hInstance;

BOOL WINAPI DllMain(HINSTANCE hInstance, DWORD dwReason, LPVOID lpReserved)
{
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
void (*DriverReadyIdle)(bool setDefaultChaprone);
void (*VideoSend)(VideoFrame header, unsigned char *buf, int len);
void (*HapticsSend)(HapticsFeedback packet);
void (*TimeSyncSend)(TimeSync packet);
void (*ShutdownRuntime)();

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
	// set correct client ip
	Settings::Instance().Load();

	if (g_serverDriverDisplayRedirect.m_pRemoteHmd)
		g_serverDriverDisplayRedirect.m_pRemoteHmd->StartStreaming();
}

void DeinitializeStreaming() {
	if (g_serverDriverDisplayRedirect.m_pRemoteHmd)
		g_serverDriverDisplayRedirect.m_pRemoteHmd->StopStreaming();
}

void RequestIDR() {
	if (g_serverDriverDisplayRedirect.m_pRemoteHmd)
		g_serverDriverDisplayRedirect.m_pRemoteHmd->RequestIDR();
}

void InputReceive(TrackingInfo data) {
 	if (g_serverDriverDisplayRedirect.m_pRemoteHmd
 		&& g_serverDriverDisplayRedirect.m_pRemoteHmd->m_Listener)
 	{
 		g_serverDriverDisplayRedirect.m_pRemoteHmd->m_Listener->ProcessTrackingInfo(data);
 	}
 }
 void TimeSyncReceive(TimeSync data) {
 	if (g_serverDriverDisplayRedirect.m_pRemoteHmd
 		&& g_serverDriverDisplayRedirect.m_pRemoteHmd->m_Listener)
 	{
 		g_serverDriverDisplayRedirect.m_pRemoteHmd->m_Listener->ProcessTimeSync(data);
 	}
 }
 void VideoErrorReportReceive() {
 	if (g_serverDriverDisplayRedirect.m_pRemoteHmd
 		&& g_serverDriverDisplayRedirect.m_pRemoteHmd->m_Listener)
 	{
 		g_serverDriverDisplayRedirect.m_pRemoteHmd->m_Listener->ProcessVideoError();
 	}
 }

void ShutdownSteamvr() {
	if (g_serverDriverDisplayRedirect.m_pRemoteHmd)
		g_serverDriverDisplayRedirect.m_pRemoteHmd->OnShutdown();
}

void SetOpenvrProperty(unsigned long long topLevelPath, OpenvrProperty prop) {
	// todo: select correct device
	auto prop_container = g_serverDriverDisplayRedirect.m_pRemoteHmd->m_ulPropertyContainer;

	auto key = (vr::ETrackedDeviceProperty)prop.key;

	vr::ETrackedPropertyError result;

	if (prop.type == OpenvrPropertyType::Bool) {
		result = vr::VRProperties()->SetBoolProperty(prop_container, key, prop.value.bool_);
	} else if (prop.type == OpenvrPropertyType::Float) {
		result = vr::VRProperties()->SetFloatProperty(prop_container, key, prop.value.float_);
	} else if (prop.type == OpenvrPropertyType::Int32) {
		result = vr::VRProperties()->SetInt32Property(prop_container, key, prop.value.int32);
	} else if (prop.type == OpenvrPropertyType::Uint64) {
		result =
			vr::VRProperties()->SetUint64Property(prop_container, key, prop.value.uint64);
	} else if (prop.type == OpenvrPropertyType::Vector3) {
		auto vec3 = vr::HmdVector3_t{};
		vec3.v[0] = prop.value.vector3[0];
		vec3.v[1] = prop.value.vector3[1];
		vec3.v[2] = prop.value.vector3[2];
		result = vr::VRProperties()->SetVec3Property(prop_container, key, vec3);
	} else if (prop.type == OpenvrPropertyType::Double) {
		result =
			vr::VRProperties()->SetDoubleProperty(prop_container, key, prop.value.double_);
	} else if (prop.type == OpenvrPropertyType::String) {
		result =
			vr::VRProperties()->SetStringProperty(prop_container, key, prop.value.string);
	} else {
		Error("Unreachable");
		result = vr::TrackedProp_Success;
	}

  	if (result != vr::TrackedProp_Success) {
		Error(
			"Error setting property %d: %s",
			key,
			vr::VRPropertiesRaw()->GetPropErrorNameFromEnum(result));
	}

	auto event_data = vr::VREvent_Data_t{};
	event_data.property.container = prop_container;
	event_data.property.prop = key;
	vr::VRServerDriverHost()->VendorSpecificEvent(
		g_serverDriverDisplayRedirect.m_pRemoteHmd->m_unObjectId,
		vr::VREvent_PropertyChanged,
		event_data,
		0.);
}