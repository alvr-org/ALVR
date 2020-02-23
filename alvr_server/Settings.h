#pragma once

#include <openvr_driver.h>
#include "common-utils.h"
#include "Bitrate.h"
#include "Utils.h"
#include "FFR.h"

//
// Settings
//
static const char * const k_pch_Settings_Section = "driver_alvr_server";
static const char * const k_pch_Settings_SerialNumber_String = "serialNumber";
static const char * const k_pch_Settings_TrackingSystemName_String = "trackingSystemName";
static const char * const k_pch_Settings_ModelNumber_String = "modelNumber";
static const char * const k_pch_Settings_DriverVersion_String = "driverVersion";
static const char * const k_pch_Settings_ManufacturerName_String = "manufacturerName";
static const char * const k_pch_Settings_RenderModelName_String = "renderModelName";
static const char * const k_pch_Settings_RegisteredDeviceType_String = "registeredDeviceType";

static const char * const k_pch_Settings_RefreshRate_Int32 = "refreshRate";
static const char * const k_pch_Settings_RenderWidth_Int32 = "renderWidth";
static const char * const k_pch_Settings_RenderHeight_Int32 = "renderHeight";
static const char * const k_pch_Settings_RecommendedRenderWidth_Int32 = "recommendedRenderWidth";
static const char * const k_pch_Settings_RecommendedRenderHeight_Int32 = "recommendedRenderHeight";
static const char * const k_pch_Settings_EyeFov = "eyeFov";
static const char * const k_pch_Settings_IPD_Float = "IPD";
static const char * const k_pch_Settings_SecondsFromVsyncToPhotons_Float = "secondsFromVsyncToPhotons";
static const char * const k_pch_Settings_ClientRecvBufferSize_Int32 = "clientRecvBufferSize";
static const char * const k_pch_Settings_FrameQueueSize_Int32 = "frameQueueSize";

static const char * const k_pch_Settings_Force60HZ_Bool = "force60HZ";

static const char * const k_pch_Settings_Force3DOF_Bool = "force3DOF";
static const char* const k_pch_Settings_Nv12_Bool = "nv12";

static const char * const k_pch_Settings_AggressiveKeyframeResend_Bool = "aggressiveKeyframeResend";

static const char * const k_pch_Settings_EnableSound_Bool = "enableSound";
static const char * const k_pch_Settings_SoundDevice_String = "soundDevice";
static const char * const k_pch_Settings_StreamMic_Bool = "streamMic";

static const char * const k_pch_Settings_Codec_Int32 = "codec";
static const char * const k_pch_Settings_EncoderOptions_String = "nvencOptions";
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
static const char * const k_pch_Settings_AutoConnectHost_String = "autoConnectHost";
static const char * const k_pch_Settings_AutoConnectPort_Int32 = "autoConnectPort";
static const char * const k_pch_Settings_DisableThrottling_Bool = "disableThrottling";

static const char * const k_pch_Settings_AdapterIndex_Int32 = "adapterIndex";

static const char * const k_pch_Settings_SendingTimeslotUs_Int32 = "sendingTimeslotUs";
static const char * const k_pch_Settings_LimitTimeslotPackets_Int32 = "limitTimeslotPackets";

static const char * const k_pch_Settings_ControllerTrackingSystemName_String = "controllerTrackingSystemName";
static const char * const k_pch_Settings_ControllerManufacturerName_String = "controllerManufacturerName";
static const char * const k_pch_Settings_ControllerModelNumber_String = "controllerModelNumber";
static const char * const k_pch_Settings_ControllerRenderModelNameLeft_String = "controllerRenderModelNameLeft";
static const char * const k_pch_Settings_ControllerRenderModelNameRight_String = "controllerRenderModelNameRight";
static const char * const k_pch_Settings_ControllerSerialNumber_String = "controllerSerialNumber";
static const char * const k_pch_Settings_ControllerType_String = "controllerType";
static const char * const k_pch_Settings_ControllerRegisteredDeviceType_String = "controllerRegisteredDeviceType";
static const char * const k_pch_Settings_ControllerInputProfilePath_String = "controllerInputProfilePath";


static const char * const k_pch_Settings_DisableController_Bool = "disableController";
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

static const char* const k_pch_Settings_leftControllerPositionOffsetX_Float = "leftControllerPositionOffsetX";
static const char* const k_pch_Settings_leftControllerPositionOffsetY_Float = "leftControllerPositionOffsetY";
static const char* const k_pch_Settings_leftControllerPositionOffsetZ_Float = "leftControllerPositionOffsetZ";
static const char* const k_pch_Settings_leftControllerPitchOffset_Float = "leftControllerPitchOffset";
static const char* const k_pch_Settings_leftControllerYawOffset_Float = "leftControllerYawOffset";
static const char* const k_pch_Settings_leftControllerRollOffset_Float = "leftControllerRollOffset";

static const char * const k_pch_Settings_controllerPoseOffset_Float = "controllerPoseOffset";

static const char * const k_pch_Settings_foveationMode_Int32 = "foveationMode";
static const char * const k_pch_Settings_foveationStrength_Float = "foveationStrength";
static const char * const k_pch_Settings_foveationShape_Float = "foveationShape";
static const char * const k_pch_Settings_foveationVerticalOffset_Float = "foveationVerticalOffset";

static const char* const k_pch_Settings_EnableColorCorrection_Bool = "enableColorCorrection";
static const char* const k_pch_Settings_Brightness_Float = "brightness";
static const char* const k_pch_Settings_Contrast_Float = "contrast";
static const char* const k_pch_Settings_Saturation_Float = "saturation";
static const char* const k_pch_Settings_Gamma_Float = "gamma";
static const char* const k_pch_Settings_Sharpening_Float = "sharpening";


static const char * const k_pch_Settings_TrackingFrameOffset_Int32 = "trackingFrameOffset";

static const char* const k_pch_Settings_ControllerMode_Int32 = "controllerMode";

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

	std::string mSerialNumber;
	std::string mTrackingSystemName;
	std::string mModelNumber;
	std::string mDriverVersion;
	std::string mManufacturerName;
	std::string mRenderModelName;
	std::string mRegisteredDeviceType;

	int32_t m_nAdapterIndex;

	uint64_t m_DriverTestMode = 0;

	int m_refreshRate;
	int32_t m_renderWidth;
	int32_t m_renderHeight;
	int32_t m_recommendedTargetWidth;
	int32_t m_recommendedTargetHeight;


	EyeFov m_eyeFov[2];
	float m_flSecondsFromVsyncToPhotons;
	float m_flIPD;

	FOVEATION_MODE m_foveationMode;
	float m_foveationStrength;
	float m_foveationShape;
	float m_foveationVerticalOffset;

	bool m_enableColorCorrection;
	float m_brightness;
	float m_contrast;
	float m_saturation;
	float m_gamma;
	float m_sharpening;

	bool m_enableSound;
	std::string m_soundDevice;

	bool m_streamMic;

	int m_codec;
	std::string m_EncoderOptions;
	Bitrate mEncodeBitrate;

	std::string m_Host;
	int m_Port;
	std::string m_ControlHost;
	int m_ControlPort;
	std::string m_AutoConnectHost;
	int m_AutoConnectPort;
	Bitrate mThrottlingBitrate;

	bool m_DebugLog;
	bool m_DebugFrameIndex;
	bool m_DebugFrameOutput;
	bool m_DebugCaptureOutput;
	bool m_UseKeyedMutex;


	uint64_t m_SendingTimeslotUs;
	uint64_t m_LimitTimeslotPackets;

	uint32_t m_clientRecvBufferSize;

	uint32_t m_frameQueueSize;

	bool m_force60HZ;

	bool m_nv12;

	// Controller configs
	std::string m_controllerTrackingSystemName;
	std::string m_controllerManufacturerName;
	std::string m_controllerModelNumber;
	std::string m_controllerRenderModelNameLeft;
	std::string m_controllerRenderModelNameRight;
	std::string m_controllerSerialNumber;
	std::string m_controllerType;
	std::string mControllerRegisteredDeviceType;
	std::string m_controllerInputProfilePath;
	bool m_disableController;
	int32_t m_controllerTriggerMode;
	int32_t m_controllerTrackpadClickMode;
	int32_t m_controllerTrackpadTouchMode;
	int32_t m_controllerBackMode;
	int32_t m_controllerRecenterButton;

	double m_controllerPoseOffset = 0;

	float m_OffsetPos[3];
	bool m_EnableOffsetPos;

	double m_leftControllerPositionOffset[3];
	double m_leftControllerRotationOffset[3];

	int32_t m_causePacketLoss;

	bool m_useTrackingReference;

	int32_t m_trackingFrameOffset;

	bool m_force3DOF;

	bool m_aggressiveKeyframeResend;

	// They are not in config json and set by "SetConfig" command.
	bool m_captureLayerDDSTrigger = false;
	bool m_captureComposedDDSTrigger = false;
	
	int m_controllerMode = 0;
};

