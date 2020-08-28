#include "Settings.h"
#include "Logger.h"
#include "ipctools.h"
#include "resource.h"
#define PICOJSON_USE_INT64
#include <picojson.h>
#include <string>
#include <fstream>
#include <streambuf>

using namespace std;

Settings Settings::m_Instance;

Settings::Settings()
{
	m_OffsetPos[0] = 0.0f;
	m_OffsetPos[1] = 0.0f;
	m_OffsetPos[2] = 0.0f;
}

Settings::~Settings()
{
}

void Settings::Load()
{
	try
	{
		auto sessionFile = std::ifstream(g_alvrDir + "/session.json"s);

		auto json = std::string(
			std::istreambuf_iterator<char>(sessionFile),
			std::istreambuf_iterator<char>());

		picojson::value v;
		std::string err = picojson::parse(v, json);
		if (!err.empty())
		{
			Error("Error on parsing json: %hs", err.c_str());
			return;
		}

		auto openvrConfig = v.get("openvrConfig");

		mSerialNumber = openvrConfig.get("headsetSerialNumber").get<std::string>().c_str();
		mTrackingSystemName = openvrConfig.get("headsetTrackingSystemName").get<std::string>();
		mModelNumber = openvrConfig.get("headsetModelNumber").get<std::string>();
		mDriverVersion = openvrConfig.get("headsetDriverVersion").get<std::string>();
		mManufacturerName = openvrConfig.get("headsetManufacturerName").get<std::string>();
		mRenderModelName = openvrConfig.get("headsetRenderModelName").get<std::string>();
		mRegisteredDeviceType = openvrConfig.get("headsetRegisteredDeviceType").get<std::string>();

		auto eyeResolution = openvrConfig.get("eyeResolution").get<picojson::array>();
		m_renderWidth = eyeResolution[0].get<int64_t>() * 2;
		m_renderHeight = eyeResolution[1].get<int64_t>();

		auto targetEyeResolution = openvrConfig.get("targetEyeResolution").get<picojson::array>();
		m_recommendedTargetWidth = targetEyeResolution[0].get<int64_t>() * 2;
		m_recommendedTargetHeight = targetEyeResolution[1].get<int64_t>();

		picojson::array &eyeFov = openvrConfig.get("fov").get<picojson::array>();
		for (int eye = 0; eye < 2; eye++)
		{
			m_eyeFov[eye].left = static_cast<float>(eyeFov[eye].get("left").get<double>());
			m_eyeFov[eye].right = static_cast<float>(eyeFov[eye].get("right").get<double>());
			m_eyeFov[eye].top = static_cast<float>(eyeFov[eye].get("top").get<double>());
			m_eyeFov[eye].bottom = static_cast<float>(eyeFov[eye].get("bottom").get<double>());
		}

		m_flSecondsFromVsyncToPhotons = (float)openvrConfig.get("secondsFromVsyncToPhotons").get<double>();

		m_flIPD = (float)openvrConfig.get("ipd").get<double>();

		m_nAdapterIndex = (int32_t)openvrConfig.get("adapterIndex").get<int64_t>();

		m_refreshRate = (int)openvrConfig.get("fps").get<int64_t>();

		m_controllerTrackingSystemName = openvrConfig.get("controllersTrackingSystemName").get<std::string>();
		m_controllerManufacturerName = openvrConfig.get("controllersManufacturerName").get<std::string>();
		m_controllerModelNumber = openvrConfig.get("controllersModelNumber").get<std::string>();
		m_controllerRenderModelNameLeft = openvrConfig.get("renderModelNameLeftcontroller").get<std::string>();
		m_controllerRenderModelNameRight = openvrConfig.get("renderModelNameRightcontroller").get<std::string>();
		m_controllerSerialNumber = openvrConfig.get("controllersSerialNumber").get<std::string>();
		m_controllerType = openvrConfig.get("controllersType").get<std::string>();
		mControllerRegisteredDeviceType = openvrConfig.get("controllersRegisteredDeviceType").get<std::string>();
		m_controllerInputProfilePath = openvrConfig.get("controllersInputProfilePath").get<std::string>();

		m_controllerMode = (int32_t)openvrConfig.get("controllersModeIdx").get<int64_t>();

		m_disableController = !openvrConfig.get("controllersEnabled").get<bool>();

		m_enableFoveatedRendering = openvrConfig.get("enableFoveatedRendering").get<bool>();
		m_foveationStrength = (float)openvrConfig.get("foveationStrength").get<double>();
		m_foveationShape = (float)openvrConfig.get("foveationShape").get<double>();
		m_foveationVerticalOffset = (float)openvrConfig.get("foveationVerticalOffset").get<double>();

		m_enableColorCorrection = openvrConfig.get("enableColorCorrection").get<bool>();
		m_brightness = (float)openvrConfig.get("brightness").get<double>();
		m_contrast = (float)openvrConfig.get("contrast").get<double>();
		m_saturation = (float)openvrConfig.get("saturation").get<double>();
		m_gamma = (float)openvrConfig.get("gamma").get<double>();
		m_sharpening = (float)openvrConfig.get("sharpening").get<double>();
	}
	catch (std::exception &e)
	{
		FatalLog("Exception on parsing json: %hs", e.what());
	}
}

void Settings::UpdateForStream(StreamSettings settings)
{
	m_enableSound = settings.gameAudio;
	m_soundDevice = settings.gameAudioDevice;
	m_streamMic = settings.microphone;

	m_keyframeResendIntervalMs = settings.keyframeResendIntervalMs;

	m_codec = settings.codec;
	mEncodeBitrate = Bitrate::fromMiBits(settings.encodeBitrateMbs);

	m_trackingFrameOffset = settings.trackingFrameOffset;
	m_controllerPoseOffset = settings.poseTimeOffset;

	m_leftControllerPositionOffset[0] = settings.positionOffsetLeft[0];
	m_leftControllerPositionOffset[1] = settings.positionOffsetLeft[1];
	m_leftControllerPositionOffset[2] = settings.positionOffsetLeft[2];

	m_leftControllerRotationOffset[0] = settings.rotationOffsetLeft[0];
	m_leftControllerRotationOffset[1] = settings.rotationOffsetLeft[1];
	m_leftControllerRotationOffset[2] = settings.rotationOffsetLeft[2];

	m_hapticsIntensity = settings.hapticsIntensity;
}
