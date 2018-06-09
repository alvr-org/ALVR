#include "Settings.h"
#include "Logger.h"
#include "ipctools.h"
#include "resource.h"
#define PICOJSON_USE_INT64
#include <picojson.h>

extern std::string g_DebugOutputDir;

extern uint64_t g_DriverTestMode;

Settings Settings::m_Instance;

Settings::Settings()
	: m_EnabledDebugPos(false)
	, m_loaded(false)
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
	IPCFileMapping filemapping(APP_FILEMAPPING_NAME);
	if (!filemapping.Opened()) {
		return;
	}

	char *configBuf = (char *)filemapping.Map();
	int32_t size = *(int32_t *)configBuf;

	std::string json(configBuf + sizeof(int32_t), size);

	picojson::value v;
	std::string err = picojson::parse(v, json);
	if (!err.empty()) {
		FatalLog("Error on parsing json: %s", err.c_str());
		return;
	}

	m_sSerialNumber = v.get(k_pch_Settings_SerialNumber_String).get<std::string>();
	m_sModelNumber = v.get(k_pch_Settings_ModelNumber_String).get<std::string>();

	m_renderWidth = (int32_t)v.get(k_pch_Settings_RenderWidth_Int32).get<int64_t>();
	m_renderHeight = (int32_t)v.get(k_pch_Settings_RenderHeight_Int32).get<int64_t>();

	m_flSecondsFromVsyncToPhotons = (float)v.get(k_pch_Settings_SecondsFromVsyncToPhotons_Float).get<double>();
	m_flDisplayFrequency = (float)v.get(k_pch_Settings_DisplayFrequency_Float).get<double>();

	m_flIPD = (float)v.get(k_pch_Settings_IPD_Float).get<double>();

	m_clientRecvBufferSize = (uint32_t)v.get(k_pch_Settings_ClientRecvBufferSize_Int32).get<int64_t>();

	m_nAdapterIndex = (int32_t)v.get(k_pch_Settings_AdapterIndex_Int32).get<int64_t>();

	m_EncoderOptions = v.get(k_pch_Settings_EncoderOptions_String).get<std::string>();
	g_DebugOutputDir = v.get(k_pch_Settings_DebugOutputDir).get<std::string>();
	
	// Listener Parameters
	m_Host = v.get(k_pch_Settings_ListenHost_String).get<std::string>();
	m_Port = (int)v.get(k_pch_Settings_ListenPort_Int32).get<int64_t>();

	m_SendingTimeslotUs = (uint64_t)v.get(k_pch_Settings_SendingTimeslotUs_Int32).get<int64_t>();
	m_LimitTimeslotPackets = (uint64_t)v.get(k_pch_Settings_LimitTimeslotPackets_Int32).get<int64_t>();

	m_ControlHost = v.get(k_pch_Settings_ControlListenHost_String).get<std::string>();
	m_ControlPort = (int)v.get(k_pch_Settings_ControlListenPort_Int32).get<int64_t>();

	m_DebugLog = v.get(k_pch_Settings_DebugLog_Bool).get<bool>();
	m_DebugFrameIndex = v.get(k_pch_Settings_DebugFrameIndex_Bool).get<bool>();
	m_DebugFrameOutput = v.get(k_pch_Settings_DebugFrameOutput_Bool).get<bool>();
	m_DebugCaptureOutput = v.get(k_pch_Settings_DebugCaptureOutput_Bool).get<bool>();
	m_UseKeyedMutex = v.get(k_pch_Settings_UseKeyedMutex_Bool).get<bool>();

	m_controllerModelNumber = v.get(k_pch_Settings_ControllerModelNumber_String).get<std::string>();
	m_controllerSerialNumber = v.get(k_pch_Settings_ControllerSerialNumber_String).get<std::string>();

	m_enableController = v.get(k_pch_Settings_EnableController_Bool).get<bool>();
	m_controllerTriggerMode = (int32_t)v.get(k_pch_Settings_ControllerTriggerMode_Int32).get<int64_t>();
	m_controllerTrackpadClickMode = (int32_t)v.get(k_pch_Settings_ControllerTrackpadClickMode_Int32).get<int64_t>();
	m_controllerTrackpadTouchMode = (int32_t)v.get(k_pch_Settings_ControllerTrackpadTouchMode_Int32).get<int64_t>();
	m_controllerRecenterButton = (int32_t)v.get(k_pch_Settings_ControllerRecenterButton_Int32).get<int64_t>();

	m_useTrackingReference = v.get(k_pch_Settings_UseTrackingReference_Bool).get<bool>();

	if (m_DebugLog) {
		OpenLog((g_DebugOutputDir + "\\" + LOG_FILE).c_str());
	}
	
	Log("Serial Number: %s", m_sSerialNumber.c_str());
	Log("Model Number: %s", m_sModelNumber.c_str());
	Log("Render Target: %d %d", m_renderWidth, m_renderHeight);
	Log("Seconds from Vsync to Photons: %f", m_flSecondsFromVsyncToPhotons);
	Log("Display Frequency: %f", m_flDisplayFrequency);
	Log("IPD: %f", m_flIPD);

	Log("renderWidth: %d", m_renderWidth);
	Log("renderHeight: %d", m_renderHeight);
	Log("debugOptions: Log:%d FrameIndex:%d FrameOutput:%d CaptureOutput:%d UseKeyedMutex:%d"
		, m_DebugLog, m_DebugFrameIndex, m_DebugFrameOutput, m_DebugCaptureOutput, m_UseKeyedMutex);
	Log("EncoderOptions: %s", m_EncoderOptions.c_str());

	m_loaded = true;
}
