#include "Settings.h"
#include "Logger.h"
#include "openvr-utils\ipctools.h"
#include "resource.h"
#define PICOJSON_USE_INT64
#include <picojson.h>

extern uint64_t gDriverTestMode;

Settings Settings::mInstance;

Settings::Settings()
	: mEnableOffsetPos(false)
	, mLoaded(false)
{
	mOffsetPos[0] = 0.0f;
	mOffsetPos[1] = 0.0f;
	mOffsetPos[2] = 0.0f;
}


Settings::~Settings()
{
	if (mDebugLog) {
		CloseLog();
	}
}

void Settings::Load()
{
	try {
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
			FatalLog(L"Error on parsing json: %hs", err.c_str());
			return;
		}

		mSerialNumber = v.get(k_pch_Settings_SerialNumber_String).get<std::string>();
		mTrackingSystemName = v.get(k_pch_Settings_TrackingSystemName_String).get<std::string>();
		mModelNumber = v.get(k_pch_Settings_ModelNumber_String).get<std::string>();
		mManufacturerName = v.get(k_pch_Settings_ManufacturerName_String).get<std::string>();
		mRenderModelName = v.get(k_pch_Settings_RenderModelName_String).get<std::string>();
		mRegisteredDeviceType = v.get(k_pch_Settings_RegisteredDeviceType_String).get<std::string>();

		mRenderWidth = (int32_t)v.get(k_pch_Settings_RenderWidth_Int32).get<int64_t>();
		mRenderHeight = (int32_t)v.get(k_pch_Settings_RenderHeight_Int32).get<int64_t>();

		picojson::array& eyeFov = v.get(k_pch_Settings_EyeFov).get<picojson::array>();
		for (int eye = 0; eye < 2; eye++) {
			mEyeFov[eye].left = static_cast<float>(eyeFov[eye * 4 + 0].get<double>());
			mEyeFov[eye].right = static_cast<float>(eyeFov[eye * 4 + 1].get<double>());
			mEyeFov[eye].top = static_cast<float>(eyeFov[eye * 4 + 2].get<double>());
			mEyeFov[eye].bottom = static_cast<float>(eyeFov[eye * 4 + 3].get<double>());
		}

		mEnableSound = v.get(k_pch_Settings_EnableSound_Bool).get<bool>();
		mSoundDevice = v.get(k_pch_Settings_SoundDevice_String).get<std::string>();

		mSecondsFromVsyncToPhotons = (float)v.get(k_pch_Settings_SecondsFromVsyncToPhotons_Float).get<double>();

		mIPD = (float)v.get(k_pch_Settings_IPD_Float).get<double>();

		mClientRecvBufferSize = (uint32_t)v.get(k_pch_Settings_ClientRecvBufferSize_Int32).get<int64_t>();
		mFrameQueueSize = (uint32_t)v.get(k_pch_Settings_FrameQueueSize_Int32).get<int64_t>();

		mForce60HZ = v.get(k_pch_Settings_Force60HZ_Bool).get<bool>();

		mAdapterIndex = (int32_t)v.get(k_pch_Settings_AdapterIndex_Int32).get<int64_t>();

		mCodec = (int32_t)v.get(k_pch_Settings_Codec_Int32).get<int64_t>();
		mRefreshRate = (int)v.get(k_pch_Settings_RefreshRate_Int32).get<int64_t>();
		mEncodeBitrate = Bitrate::fromMiBits((int)v.get(k_pch_Settings_EncodeBitrateInMBits_Int32).get<int64_t>());

		if (v.get(k_pch_Settings_DisableThrottling_Bool).get<bool>()) {
			// No throttling
			mThrottlingBitrate = Bitrate::fromBits(0);
		}
		else {
			// Audio stream: 48kHz * 16bits * 2ch
			Bitrate audioBitrate = Bitrate::fromMiBits(2);
			// +20% for mergin
			mThrottlingBitrate = Bitrate::fromBits(mEncodeBitrate.toBits() * 12 / 10 + audioBitrate.toBits());
		}

		mDebugOutputDir = v.get(k_pch_Settings_DebugOutputDir).get<std::string>();

		// Listener Parameters
		mHost = v.get(k_pch_Settings_ListenHost_String).get<std::string>();
		mPort = (int)v.get(k_pch_Settings_ListenPort_Int32).get<int64_t>();

		mSendingTimeslotUs = (uint64_t)v.get(k_pch_Settings_SendingTimeslotUs_Int32).get<int64_t>();
		mLimitTimeslotPackets = (uint64_t)v.get(k_pch_Settings_LimitTimeslotPackets_Int32).get<int64_t>();

		mControlHost = v.get(k_pch_Settings_ControlListenHost_String).get<std::string>();
		mControlPort = (int)v.get(k_pch_Settings_ControlListenPort_Int32).get<int64_t>();

		mAutoConnectHost = v.get(k_pch_Settings_AutoConnectHost_String).get<std::string>();
		mAutoConnectPort = (int)v.get(k_pch_Settings_AutoConnectPort_Int32).get<int64_t>();

		mDebugLog = v.get(k_pch_Settings_DebugLog_Bool).get<bool>();
		mDebugFrameIndex = v.get(k_pch_Settings_DebugFrameIndex_Bool).get<bool>();
		mDebugFrameOutput = v.get(k_pch_Settings_DebugFrameOutput_Bool).get<bool>();
		mDebugCaptureOutput = v.get(k_pch_Settings_DebugCaptureOutput_Bool).get<bool>();
		mUseKeyedMutex = v.get(k_pch_Settings_UseKeyedMutex_Bool).get<bool>();

		mControllerTrackingSystemName = v.get(k_pch_Settings_ControllerTrackingSystemName_String).get<std::string>();
		mControllerManufacturerName = v.get(k_pch_Settings_ControllerManufacturerName_String).get<std::string>();
		mControllerModelNumber = v.get(k_pch_Settings_ControllerModelNumber_String).get<std::string>();
		mControllerRenderModelNameLeft = v.get(k_pch_Settings_ControllerRenderModelNameLeft_String).get<std::string>();
		mControllerRenderModelNameRight = v.get(k_pch_Settings_ControllerRenderModelNameRight_String).get<std::string>();
		mControllerSerialNumber = v.get(k_pch_Settings_ControllerSerialNumber_String).get<std::string>();
		mControllerType = v.get(k_pch_Settings_ControllerType_String).get<std::string>();
		mControllerRegisteredDeviceType = v.get(k_pch_Settings_ControllerRegisteredDeviceType_String).get<std::string>();
		mControllerLegacyInputProfile = v.get(k_pch_Settings_ControllerLegacyInputProfile_String).get<std::string>();
		mControllerInputProfilePath = v.get(k_pch_Settings_ControllerInputProfilePath_String).get<std::string>();

		mEnableController = v.get(k_pch_Settings_EnableController_Bool).get<bool>();
		mControllerTriggerMode = (int32_t)v.get(k_pch_Settings_ControllerTriggerMode_Int32).get<int64_t>();
		mControllerTrackpadClickMode = (int32_t)v.get(k_pch_Settings_ControllerTrackpadClickMode_Int32).get<int64_t>();
		mControllerTrackpadTouchMode = (int32_t)v.get(k_pch_Settings_ControllerTrackpadTouchMode_Int32).get<int64_t>();
		mControllerBackMode = (int32_t)v.get(k_pch_Settings_ControllerBackMode_Int32).get<int64_t>();
		mControllerRecenterButton = (int32_t)v.get(k_pch_Settings_ControllerRecenterButton_Int32).get<int64_t>();

		mUseTrackingReference = v.get(k_pch_Settings_UseTrackingReference_Bool).get<bool>();

		mEnableOffsetPos = v.get(k_pch_Settings_EnableOffsetPos_Bool).get<bool>();
		mOffsetPos[0] = (float)v.get(k_pch_Settings_OffsetPosX_Float).get<double>();
		mOffsetPos[1] = (float)v.get(k_pch_Settings_OffsetPosY_Float).get<double>();
		mOffsetPos[2] = (float)v.get(k_pch_Settings_OffsetPosZ_Float).get<double>();

		mTrackingFrameOffset = (int32_t)v.get(k_pch_Settings_TrackingFrameOffset_Int32).get<int64_t>();

		if (mDebugLog) {
			OpenLog((mDebugOutputDir + "\\" + LOG_FILE).c_str());
		}

		Log(L"Config JSON: %hs", json.c_str());
		Log(L"Serial Number: %hs", mSerialNumber.c_str());
		Log(L"Model Number: %hs", mModelNumber.c_str());
		Log(L"Render Target: %d %d", mRenderWidth, mRenderHeight);
		Log(L"Seconds from Vsync to Photons: %f", mSecondsFromVsyncToPhotons);
		Log(L"Refresh Rate: %d", mRefreshRate);
		Log(L"IPD: %f", mIPD);

		Log(L"debugOptions: Log:%d FrameIndex:%d FrameOutput:%d CaptureOutput:%d UseKeyedMutex:%d"
			, mDebugLog, mDebugFrameIndex, mDebugFrameOutput, mDebugCaptureOutput, mUseKeyedMutex);
		Log(L"EncoderOptions: %hs", m_EncoderOptions.c_str());

		mLoaded = true;
	}
	catch (std::exception &e) {
		FatalLog(L"Exception on parsing json: %hs", e.what());
	}
}
