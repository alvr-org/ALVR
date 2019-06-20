#pragma once

#include <openvr_driver.h>
#include "common-utils.h"
#include "Bitrate.h"
#include "Utils.h"

//
// Settings
//
static const char * const k_pch_Settings_Section = "driver_alvr_server";
static const char * const k_pch_Settings_SerialNumber_String = "serialNumber";
static const char * const k_pch_Settings_TrackingSystemName_String = "trackingSystemName";
static const char * const k_pch_Settings_ModelNumber_String = "modelNumber";
static const char * const k_pch_Settings_ManufacturerName_String = "manufacturerName";
static const char * const k_pch_Settings_RenderModelName_String = "renderModelName";
static const char * const k_pch_Settings_RegisteredDeviceType_String = "registeredDeviceType";

static const char * const k_pch_Settings_RefreshRate_Int32 = "refreshRate";
static const char * const k_pch_Settings_RenderWidth_Int32 = "renderWidth";
static const char * const k_pch_Settings_RenderHeight_Int32 = "renderHeight";
static const char * const k_pch_Settings_EyeFov = "eyeFov";
static const char * const k_pch_Settings_IPD_Float = "IPD";
static const char * const k_pch_Settings_SecondsFromVsyncToPhotons_Float = "secondsFromVsyncToPhotons";
static const char * const k_pch_Settings_ClientRecvBufferSize_Int32 = "clientRecvBufferSize";
static const char * const k_pch_Settings_FrameQueueSize_Int32 = "frameQueueSize";

static const char * const k_pch_Settings_Force60HZ_Bool = "force60HZ";

static const char * const k_pch_Settings_EnableSound_Bool = "enableSound";
static const char * const k_pch_Settings_SoundDevice_String = "soundDevice";

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
static const char * const k_pch_Settings_ControllerLegacyInputProfile_String = "controllerLegacyInputProfile";
static const char * const k_pch_Settings_ControllerInputProfilePath_String = "controllerInputProfilePath";


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
	static Settings mInstance;
	bool mLoaded;

	Settings();
	virtual ~Settings();

public:
	void Load();
	static Settings &Instance() {
		return mInstance;
	}

	bool IsLoaded() {
		return mLoaded;
	}

	std::string GetVideoOutput() {
		return mDebugOutputDir + "\\" + DEBUG_VIDEO_CAPTURE_OUTPUT_NAME;
	}
	std::wstring GetAudioOutput() {
		return ToWstring(mDebugOutputDir) + L"\\" + DEBUG_AUDIO_CAPTURE_OUTPUT_NAME;
	}

	std::string mDebugOutputDir;

	std::string mSerialNumber;
	std::string mTrackingSystemName;
	std::string mModelNumber;
	std::string mManufacturerName;
	std::string mRenderModelName;
	std::string mRegisteredDeviceType;

	int32_t mAdapterIndex;

	int mRefreshRate;
	int32_t mRenderWidth;
	int32_t mRenderHeight;
	EyeFov mEyeFov[2];
	float mSecondsFromVsyncToPhotons;
	float mIPD;

	bool mEnableSound;
	std::string mSoundDevice;

	int mCodec;
	std::string m_EncoderOptions;
	Bitrate mEncodeBitrate;

	std::string mHost;
	int mPort;
	std::string mControlHost;
	int mControlPort;
	std::string mAutoConnectHost;
	int mAutoConnectPort;
	Bitrate mThrottlingBitrate;

	bool mDebugLog;
	bool mDebugFrameIndex;
	bool mDebugFrameOutput;
	bool mDebugCaptureOutput;
	bool mUseKeyedMutex;


	uint64_t mSendingTimeslotUs;
	uint64_t mLimitTimeslotPackets;

	uint32_t mClientRecvBufferSize;

	uint32_t mFrameQueueSize;

	bool mForce60HZ;

	// Controller configs
	std::string mControllerTrackingSystemName;
	std::string mControllerManufacturerName;
	std::string mControllerModelNumber;
	std::string mControllerRenderModelNameLeft;
	std::string mControllerRenderModelNameRight;
	std::string mControllerSerialNumber;
	std::string mControllerType;
	std::string mControllerRegisteredDeviceType;
	std::string mControllerLegacyInputProfile;
	std::string mControllerInputProfilePath;
	bool mEnableController;
	int32_t mControllerTriggerMode;
	int32_t mControllerTrackpadClickMode;
	int32_t mControllerTrackpadTouchMode;
	int32_t mControllerBackMode;
	int32_t mControllerRecenterButton;

	float mOffsetPos[3];
	bool mEnableOffsetPos;

	bool mUseTrackingReference;

	int32_t mTrackingFrameOffset;

	// They are not in config json and set by "SetConfig" command.
	bool mCaptureLayerDDSTrigger = false;
	bool mCaptureComposedDDSTrigger = false;
	int32_t mCausePacketLoss = 0;
};

