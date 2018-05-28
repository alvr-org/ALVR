#include "Settings.h"
#include "Logger.h"


extern std::string g_DebugOutputDir;

extern uint64_t g_DriverTestMode;

//
// Settings
//
static const char * const k_pch_Settings_Section = "driver_alvr_server";
static const char * const k_pch_Settings_SerialNumber_String = "serialNumber";
static const char * const k_pch_Settings_ModelNumber_String = "modelNumber";
static const char * const k_pch_Settings_RenderWidth_Int32 = "renderWidth";
static const char * const k_pch_Settings_RenderHeight_Int32 = "renderHeight";
static const char * const k_pch_Settings_IPD_Float = "IPD";
static const char * const k_pch_Settings_SecondsFromVsyncToPhotons_Float = "secondsFromVsyncToPhotons";
static const char * const k_pch_Settings_DisplayFrequency_Float = "displayFrequency";
static const char * const k_pch_Settings_ClientRecvBufferSize_Int32 = "clientRecvBufferSize";

static const char * const k_pch_Settings_EncoderOptions_String = "nvencOptions";
static const char * const k_pch_Settings_DebugLog_Bool = "debugLog";
static const char * const k_pch_Settings_DebugTimestamp_Bool = "debugTimestamp";
static const char * const k_pch_Settings_DebugFrameIndex_Bool = "debugFrameIndex";
static const char * const k_pch_Settings_DebugFrameOutput_Bool = "debugFrameOutput";
static const char * const k_pch_Settings_DebugCaptureOutput_Bool = "debugCaptureOutput";
static const char * const k_pch_Settings_UseKeyedMutex_Bool = "useKeyedMutex";
static const char * const k_pch_Settings_DebugOutputDir = "debugOutputDir";
static const char * const k_pch_Settings_ListenHost_String = "listenHost";
static const char * const k_pch_Settings_ListenPort_Int32 = "listenPort";
static const char * const k_pch_Settings_ControlListenHost_String = "controlListenHost";
static const char * const k_pch_Settings_ControlListenPort_Int32 = "controlListenPort";

static const char * const k_pch_Settings_AdapterIndex_Int32 = "adapterIndex";

static const char * const k_pch_Settings_SendingTimeslotUs_Int32 = "sendingTimeslotUs";
static const char * const k_pch_Settings_LimitTimeslotPackets_Int32 = "limitTimeslotPackets";

//
// Constants
//
static const char * const LOG_FILE = "driver.log";

Settings Settings::m_Instance;

Settings::Settings()
{
}


Settings::~Settings()
{
}

void Settings::Load()
{
	char buf[10240];

	vr::VRSettings()->GetString(k_pch_Settings_Section, k_pch_Settings_SerialNumber_String, buf, sizeof(buf));
	m_sSerialNumber = buf;

	vr::VRSettings()->GetString(k_pch_Settings_Section, k_pch_Settings_ModelNumber_String, buf, sizeof(buf));
	m_sModelNumber = buf;

	m_renderWidth = vr::VRSettings()->GetInt32(k_pch_Settings_Section, k_pch_Settings_RenderWidth_Int32);
	m_renderHeight = vr::VRSettings()->GetInt32(k_pch_Settings_Section, k_pch_Settings_RenderHeight_Int32);
	m_flSecondsFromVsyncToPhotons = vr::VRSettings()->GetFloat(k_pch_Settings_Section, k_pch_Settings_SecondsFromVsyncToPhotons_Float);
	m_flDisplayFrequency = vr::VRSettings()->GetFloat(k_pch_Settings_Section, k_pch_Settings_DisplayFrequency_Float);

	m_clientRecvBufferSize = vr::VRSettings()->GetInt32(k_pch_Settings_Section, k_pch_Settings_ClientRecvBufferSize_Int32);

	m_nAdapterIndex = vr::VRSettings()->GetInt32(k_pch_Settings_Section, k_pch_Settings_AdapterIndex_Int32);

	vr::VRSettings()->GetString(k_pch_Settings_Section, k_pch_Settings_EncoderOptions_String, buf, sizeof(buf));
	m_EncoderOptions = buf;
	vr::VRSettings()->GetString(k_pch_Settings_Section, k_pch_Settings_DebugOutputDir, buf, sizeof(buf));
	g_DebugOutputDir = buf;
	
	// Listener Parameters
	vr::VRSettings()->GetString(k_pch_Settings_Section, k_pch_Settings_ListenHost_String, buf, sizeof(buf));
	m_Host = buf;
	m_Port = vr::VRSettings()->GetInt32(k_pch_Settings_Section, k_pch_Settings_ListenPort_Int32);


	m_SendingTimeslotUs = vr::VRSettings()->GetInt32(k_pch_Settings_Section, k_pch_Settings_SendingTimeslotUs_Int32);
	m_LimitTimeslotPackets = vr::VRSettings()->GetInt32(k_pch_Settings_Section, k_pch_Settings_LimitTimeslotPackets_Int32);

	vr::VRSettings()->GetString(k_pch_Settings_Section, k_pch_Settings_ControlListenHost_String, buf, sizeof(buf));
	m_ControlHost = buf;
	m_ControlPort = vr::VRSettings()->GetInt32(k_pch_Settings_Section, k_pch_Settings_ControlListenPort_Int32);

	m_DebugLog = vr::VRSettings()->GetBool(k_pch_Settings_Section, k_pch_Settings_DebugLog_Bool);
	m_DebugTimestamp = vr::VRSettings()->GetBool(k_pch_Settings_Section, k_pch_Settings_DebugTimestamp_Bool);
	m_DebugFrameIndex = vr::VRSettings()->GetBool(k_pch_Settings_Section, k_pch_Settings_DebugFrameIndex_Bool);
	m_DebugFrameOutput = vr::VRSettings()->GetBool(k_pch_Settings_Section, k_pch_Settings_DebugFrameOutput_Bool);
	m_DebugCaptureOutput = vr::VRSettings()->GetBool(k_pch_Settings_Section, k_pch_Settings_DebugCaptureOutput_Bool);
	m_UseKeyedMutex = vr::VRSettings()->GetBool(k_pch_Settings_Section, k_pch_Settings_UseKeyedMutex_Bool);
	
	
	m_flIPD = vr::VRSettings()->GetFloat(k_pch_Settings_Section, k_pch_Settings_IPD_Float);

	if (Settings::Instance().m_DebugLog) {
		OpenLog((g_DebugOutputDir + "\\" + LOG_FILE).c_str());
	}
	
	Log("driver_null: Serial Number: %s", m_sSerialNumber.c_str());
	Log("driver_null: Model Number: %s", m_sModelNumber.c_str());
	Log("driver_null: Render Target: %d %d", m_renderWidth, m_renderHeight);
	Log("driver_null: Seconds from Vsync to Photons: %f", m_flSecondsFromVsyncToPhotons);
	Log("driver_null: Display Frequency: %f", m_flDisplayFrequency);
	Log("driver_null: IPD: %f", m_flIPD);

	Log("driver_null: EncoderOptions: %s%s", m_EncoderOptions.c_str(), m_EncoderOptions.size() == sizeof(buf) - 1 ? " (Maybe truncated)" : "");

}
