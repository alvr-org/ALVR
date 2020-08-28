#pragma once

#include <openvr_driver.h>
#include "common-utils.h"
#include "Bitrate.h"
#include "Utils.h"
#include "FFR.h"

class Settings
{
	static Settings m_Instance;

	Settings();
	virtual ~Settings();

public:
	void Load();
	void UpdateForStream(StreamSettings settings);

	static Settings &Instance()
	{
		return m_Instance;
	}

	// OpenVR config (used early)

	std::string mSerialNumber;
	std::string mTrackingSystemName;
	std::string mModelNumber;
	std::string mDriverVersion;
	std::string mManufacturerName;
	std::string mRenderModelName;
	std::string mRegisteredDeviceType;

	int32_t m_nAdapterIndex;

	int m_refreshRate;
	int32_t m_renderWidth;
	int32_t m_renderHeight;
	int32_t m_recommendedTargetWidth;
	int32_t m_recommendedTargetHeight;

	EyeFov m_eyeFov[2];
	float m_flSecondsFromVsyncToPhotons;
	float m_flIPD;

	bool m_enableFoveatedRendering;
	float m_foveationStrength;
	float m_foveationShape;
	float m_foveationVerticalOffset;

	bool m_enableColorCorrection;
	float m_brightness;
	float m_contrast;
	float m_saturation;
	float m_gamma;
	float m_sharpening;

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

	// Stream config (used after the stream starts)

	bool m_enableSound = false;
	char *m_soundDevice;

	bool m_streamMic = false;

	int m_codec = 0;
	Bitrate mEncodeBitrate = Bitrate::fromMiBits(30);

	double m_controllerPoseOffset = 0;

	float m_OffsetPos[3] = {0};

	double m_leftControllerPositionOffset[3] = {0};
	double m_leftControllerRotationOffset[3] = {0};

	float m_hapticsIntensity = 0;

	int32_t m_trackingFrameOffset = 0;

	int64_t m_keyframeResendIntervalMs = 100;

	int m_controllerMode = 0;
};
