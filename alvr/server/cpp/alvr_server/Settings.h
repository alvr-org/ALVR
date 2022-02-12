#pragma once

#include <string>
#include "ALVR-common/packet_types.h"

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

	uint64_t m_universeId;

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
	uint32_t m_renderWidth;
	uint32_t m_renderHeight;
	int32_t m_recommendedTargetWidth;
	int32_t m_recommendedTargetHeight;


	EyeFov m_eyeFov[2];
	float m_flSecondsFromVsyncToPhotons;
	float m_flIPD;

	bool m_enableFoveatedRendering;
	float m_foveationCenterSizeX;
	float m_foveationCenterSizeY;
	float m_foveationCenterShiftX;
	float m_foveationCenterShiftY;
	float m_foveationEdgeRatioX;
	float m_foveationEdgeRatioY;

	bool m_enableColorCorrection;
	float m_brightness;
	float m_contrast;
	float m_saturation;
	float m_gamma;
	float m_sharpening;

	int m_codec;
	uint64_t mEncodeBitrateMBs;
	bool m_enableAdaptiveBitrate;
	uint64_t m_adaptiveBitrateMaximum;
	uint64_t m_adaptiveBitrateTarget;
	bool m_adaptiveBitrateUseFrametime;
	uint64_t m_adaptiveBitrateTargetMaximum;
	int32_t m_adaptiveBitrateTargetOffset;
	uint64_t m_adaptiveBitrateThreshold;
	uint64_t m_adaptiveBitrateUpRate;
	uint64_t m_adaptiveBitrateDownRate;
	float m_adaptiveBitrateLightLoadThreshold;
	bool m_use10bitEncoder;

	// Controller configs
	std::string m_controllerTrackingSystemName;
	std::string m_controllerManufacturerName;
	std::string m_controllerModelNumber;
	std::string m_controllerRenderModelNameLeft;
	std::string m_controllerRenderModelNameRight;
	std::string m_controllerSerialNumber;
	std::string m_controllerTypeLeft;
	std::string m_controllerTypeRight;
	std::string mControllerRegisteredDeviceType;
	std::string m_controllerInputProfilePath;
	bool m_disableController;
	
	double m_controllerPoseOffset = 0;
	bool m_serversidePrediction;
	float m_linearVelocityCutoff;
	float m_angularVelocityCutoff;

	float m_OffsetPos[3];
	bool m_EnableOffsetPos;

	double m_leftControllerPositionOffset[3];
	double m_leftControllerRotationOffset[3];

	float m_hapticsIntensity;
	float m_hapticsAmplitudeCurve;
	float m_hapticsMinDuration;
	float m_hapticsLowDurationAmplitudeMultiplier;
	float m_hapticsLowDurationRange;

	int32_t m_causePacketLoss;

	int32_t m_trackingFrameOffset;

	bool m_force3DOF;

	bool m_aggressiveKeyframeResend;

	// They are not in config json and set by "SetConfig" command.
	bool m_captureLayerDDSTrigger = false;
	bool m_captureComposedDDSTrigger = false;
	
	int m_controllerMode = 0;

	bool m_TrackingRefOnly = false;

	bool m_enableViveTrackerProxy = false;

	bool m_useHeadsetTrackingSystem = false;
	
	bool m_enableFec;
};
