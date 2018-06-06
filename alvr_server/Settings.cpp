#include "Settings.h"
#include "Logger.h"


extern std::string g_DebugOutputDir;

extern uint64_t g_DriverTestMode;

Settings Settings::m_Instance;

Settings::Settings()
	: m_EnabledDebugPos(false)
{
	m_DebugPos[0] = 0.0f;
	m_DebugPos[1] = 0.0f;
	m_DebugPos[2] = 0.0f;
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
	m_DebugFrameIndex = vr::VRSettings()->GetBool(k_pch_Settings_Section, k_pch_Settings_DebugFrameIndex_Bool);
	m_DebugFrameOutput = vr::VRSettings()->GetBool(k_pch_Settings_Section, k_pch_Settings_DebugFrameOutput_Bool);
	m_DebugCaptureOutput = vr::VRSettings()->GetBool(k_pch_Settings_Section, k_pch_Settings_DebugCaptureOutput_Bool);
	m_UseKeyedMutex = vr::VRSettings()->GetBool(k_pch_Settings_Section, k_pch_Settings_UseKeyedMutex_Bool);
	
	
	m_flIPD = vr::VRSettings()->GetFloat(k_pch_Settings_Section, k_pch_Settings_IPD_Float);

	vr::VRSettings()->GetString(k_pch_Settings_Section, k_pch_Settings_ControllerModelNumber_String, buf, sizeof(buf));
	m_controllerModelNumber = buf;
	vr::VRSettings()->GetString(k_pch_Settings_Section, k_pch_Settings_ControllerSerialNumber_String, buf, sizeof(buf));
	m_controllerSerialNumber = buf;

	m_enableController = vr::VRSettings()->GetBool(k_pch_Settings_Section, k_pch_Settings_EnableController_Bool);
	m_controllerTriggerMode = vr::VRSettings()->GetInt32(k_pch_Settings_Section, k_pch_Settings_ControllerTriggerMode_Int32);
	m_controllerTrackpadClickMode = vr::VRSettings()->GetInt32(k_pch_Settings_Section, k_pch_Settings_ControllerTrackpadClickMode_Int32);
	m_controllerTrackpadTouchMode = vr::VRSettings()->GetInt32(k_pch_Settings_Section, k_pch_Settings_ControllerTrackpadTouchMode_Int32);
	m_controllerRecenterButton = vr::VRSettings()->GetInt32(k_pch_Settings_Section, k_pch_Settings_ControllerRecenterButton_Int32);

	if (Settings::Instance().m_DebugLog) {
		OpenLog((g_DebugOutputDir + "\\" + LOG_FILE).c_str());
	}
	
	Log("Serial Number: %s", m_sSerialNumber.c_str());
	Log("Model Number: %s", m_sModelNumber.c_str());
	Log("Render Target: %d %d", m_renderWidth, m_renderHeight);
	Log("Seconds from Vsync to Photons: %f", m_flSecondsFromVsyncToPhotons);
	Log("Display Frequency: %f", m_flDisplayFrequency);
	Log("IPD: %f", m_flIPD);

	Log("EncoderOptions: %s%s", m_EncoderOptions.c_str(), m_EncoderOptions.size() == sizeof(buf) - 1 ? " (Maybe truncated)" : "");

}
