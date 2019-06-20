#pragma once
#include <openvr_driver.h>
#include "Utils.h"
#include "Logger.h"
#include "packet_types.h"
#include "FreePIE.h"
#include "OpenVRController.h"

class RecenterManager
{
public:
	RecenterManager();

	bool HasValidTrackingInfo();

	bool IsRecentering();

	void BeginRecenter();

	void EndRecenter();

	void OnPoseUpdated(const TrackingInfo &info, Listener *listener);

	vr::HmdQuaternion_t GetRecenteredHMD();

	TrackingVector3 GetRecenteredPositionHMD();

	std::string GetFreePIEMessage();

private:
	void UpdateOtherTrackingSource(const TrackingInfo &info);

	void UpdateControllerState(const TrackingInfo& info);

	int mControllerDetected;
	std::shared_ptr<OpenVRController> mRemoteController[2];

	std::shared_ptr<FreePIE> mFreePIE;

	bool mHasValidTrackingInfo;
	bool mRecentering;
	uint64_t mRecenterStartTimestamp;
	double mCenterPitch;
	vr::HmdQuaternion_t mFixedOrientationHMD;
	TrackingVector3 mFixedPositionHMD;
	vr::HmdQuaternion_t mFixedOrientationController[2];
	TrackingVector3 mFixedPositionController[2];

	TrackingVector3 mBasePosition;
	TrackingVector3 mRotatedBasePosition;
	double mRotationDiff;
	uint64_t mRotationDiffLastInitialized;

	static const int RECENTER_DURATION = 400 * 1000;
};
