#pragma once

#include <openvr_driver.h>
#include "common-utils.h"
#include "Utils.h"

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
static const char * const k_pch_Settings_FrameQueueSize_Int32 = "frameQueueSize";

static const char * const k_pch_Settings_EnableSound_Bool = "enableSound";
static const char * const k_pch_Settings_SoundDevice_String = "soundDevice";

static const char * const k_pch_Settings_Codec_Int32 = "codec";
static const char * const k_pch_Settings_EncoderOptions_String = "nvencOptions";
static const char * const k_pch_Settings_EncodeFPS_Int32 = "encodeFPS";
static const char * const k_pch_Settings_EncodeBitrateInMBits_Int32 = "encodeBitrateInMBits";
static const char * const k_pch_Settings_DebugLog_Bool = "debugLog";
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

static const char * const k_pch_Settings_ControllerTrackingSystemName_String = "controllerTrackingSystemName";
static const char * const k_pch_Settings_ControllerManufacturerName_String = "controllerManufacturerName";
static const char * const k_pch_Settings_ControllerModelNumber_String = "controllerModelNumber";
static const char * const k_pch_Settings_ControllerRenderModelName_String = "controllerRenderModelName";
static const char * const k_pch_Settings_ControllerSerialNumber_String = "controllerSerialNumber";
static const char * const k_pch_Settings_EnableController_Bool = "enableController";
static const char * const k_pch_Settings_ControllerTriggerMode_Int32 = "controllerTriggerMode";
static const char * const k_pch_Settings_ControllerTrackpadClickMode_Int32 = "controllerTrackpadClickMode";
static const char * const k_pch_Settings_ControllerTrackpadTouchMode_Int32 = "controllerTrackpadTouchMode";
static const char * const k_pch_Settings_ControllerBackMode_Int32 = "controllerBackMode";
static const char * const k_pch_Settings_ControllerRecenterButton_Int32 = "controllerRecenterButton";

static const char * const k_pch_Settings_UseTrackingReference_Bool = "useTrackingReference";

static const char * const k_pch_Settings_EnableOffsetPos_Bool = "enableOffsetPos";
static const char * const k_pch_Settings_OffsetPosX_Float = "offsetPosX";
static const char * const k_pch_Settings_OffsetPosY_Float = "offsetPosY";
static const char * const k_pch_Settings_OffsetPosZ_Float = "offsetPosZ";

static const char * const k_pch_Settings_TrackingFrameOffset_Int32 = "trackingFrameOffset";
//
// Constants
//
static const char * const LOG_FILE = "driver.log";

static const char * const DEBUG_VIDEO_CAPTURE_OUTPUT_NAME = "capture.h264";
static const wchar_t * const DEBUG_AUDIO_CAPTURE_OUTPUT_NAME = L"capture.wav";

class Settings
{
	static Settings m_Instance;
	bool m_loaded;

	Settings();
	virtual ~Settings();

public:
	void Load();
	static Settings &Instance() {
		return m_Instance;
	}

	bool IsLoaded() {
		return m_loaded;
	}

	std::string GetVideoOutput() {
		return m_DebugOutputDir + "\\" + DEBUG_VIDEO_CAPTURE_OUTPUT_NAME;
	}
	std::wstring GetAudioOutput() {
		return ToWstring(m_DebugOutputDir) + L"\\" + DEBUG_AUDIO_CAPTURE_OUTPUT_NAME;
	}

	std::string m_DebugOutputDir;

	std::string m_sSerialNumber;
	std::string m_sModelNumber;

	int32_t m_nAdapterIndex;

	int32_t m_renderWidth;
	int32_t m_renderHeight;
	float m_flSecondsFromVsyncToPhotons;
	float m_flDisplayFrequency;
	float m_flIPD;

	bool m_enableSound;
	std::string m_soundDevice;

	int m_codec;
	std::string m_EncoderOptions;
	int m_encodeFPS;
	int m_encodeBitrateInMBits;

	std::string m_Host;
	int m_Port;
	std::string m_ControlHost;
	int m_ControlPort;

	bool m_DebugLog;
	bool m_DebugFrameIndex;
	bool m_DebugFrameOutput;
	bool m_DebugCaptureOutput;
	bool m_UseKeyedMutex;


	uint64_t m_SendingTimeslotUs;
	uint64_t m_LimitTimeslotPackets;

	uint32_t m_clientRecvBufferSize;

	uint32_t m_frameQueueSize;

	// Controller configs
	std::string m_controllerTrackingSystemName;
	std::string m_controllerManufacturerName;
	std::string m_controllerModelNumber;
	std::string m_controllerRenderModelName;
	std::string m_controllerSerialNumber;
	bool m_enableController;
	int32_t m_controllerTriggerMode;
	int32_t m_controllerTrackpadClickMode;
	int32_t m_controllerTrackpadTouchMode;
	int32_t m_controllerBackMode;
	int32_t m_controllerRecenterButton;

	float m_OffsetPos[3];
	bool m_EnableOffsetPos;

	int32_t m_causePacketLoss;

	bool m_useTrackingReference;

	int32_t m_trackingFrameOffset;

	// They are not in config json and set by "SetConfig" command.
	bool m_captureLayerDDSTrigger = false;
	bool m_captureComposedDDSTrigger = false;
};

