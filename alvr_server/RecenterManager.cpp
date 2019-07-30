#include "RecenterManager.h"

RecenterManager::RecenterManager()
	: mHasValidTrackingInfo(false)
	, mRecentering(false)
	, mRecenterStartTimestamp(0)
	, mCenterPitch(0.0)
	, mRotationDiffLastInitialized(0)
	, mFreePIE(std::make_shared<FreePIE>())
	, mControllerDetected(false)
{
}

bool RecenterManager::HasValidTrackingInfo() {
	return mHasValidTrackingInfo;
}

bool RecenterManager::IsRecentering() {
	return mRecentering;
}

void RecenterManager::BeginRecenter() {
	mRecenterStartTimestamp = GetTimestampUs();
	mRecentering = true;
}

void RecenterManager::EndRecenter() {
	mRecentering = false;
}

void RecenterManager::OnPoseUpdated(const TrackingInfo & info, Listener * listener) {
	mHasValidTrackingInfo = true;
	if (mRecentering) {
		if (GetTimestampUs() - mRecenterStartTimestamp > RECENTER_DURATION) {
			mCenterPitch = PitchFromQuaternion(info.HeadPose_Pose_Orientation);

			Log(L"Do recentered: Cur=(%f,%f,%f,%f) pitch=%f"
				, info.HeadPose_Pose_Orientation.x
				, info.HeadPose_Pose_Orientation.y
				, info.HeadPose_Pose_Orientation.z
				, info.HeadPose_Pose_Orientation.w
				, mCenterPitch
			);

			mRecentering = false;
		}
	}

	mFixedOrientationHMD = MultiplyPitchQuaternion(
		-mCenterPitch
		, info.HeadPose_Pose_Orientation.x
		, info.HeadPose_Pose_Orientation.y
		, info.HeadPose_Pose_Orientation.z
		, info.HeadPose_Pose_Orientation.w);

	mFixedPositionHMD = RotateVectorQuaternion(info.HeadPose_Pose_Position, mCenterPitch);

	for (int i = 0; i < TrackingInfo::MAX_CONTROLLERS; i++) {
		mFixedOrientationController[i] = MultiplyPitchQuaternion(
			-mCenterPitch
			, info.controller[i].orientation.x
			, info.controller[i].orientation.y
			, info.controller[i].orientation.z
			, info.controller[i].orientation.w);

		mFixedPositionController[i] = RotateVectorQuaternion(info.controller[i].position, mCenterPitch);
	}

	if (info.flags & TrackingInfo::FLAG_OTHER_TRACKING_SOURCE) {
		UpdateOtherTrackingSource(info);
	}
	Log(L"GetRecenteredHMD: Old=(%f,%f,%f,%f) New=(%f,%f,%f,%f) pitch=%f-%f"
		, info.HeadPose_Pose_Orientation.x, info.HeadPose_Pose_Orientation.y
		, info.HeadPose_Pose_Orientation.z, info.HeadPose_Pose_Orientation.w
		, mFixedOrientationHMD.x, mFixedOrientationHMD.y
		, mFixedOrientationHMD.z, mFixedOrientationHMD.w
		, mCenterPitch
		, PitchFromQuaternion(
			mFixedOrientationHMD.x
			, mFixedOrientationHMD.y
			, mFixedOrientationHMD.z
			, mFixedOrientationHMD.w));

	double  hapticFeedback[2][3]{ { 0,0,0 },{ 0,0,0 } };
	vr::VREvent_t vrEvent;

	while (vr::VRServerDriverHost()->PollNextEvent(&vrEvent, sizeof(vrEvent)))
	{
		if (vrEvent.eventType == vr::VREvent_Input_HapticVibration)
		{
			for (int i = 0; i < 2; i++) {
				if (mRemoteController[i] && mRemoteController[i]->IsMyHapticComponent(vrEvent.data.hapticVibration.componentHandle)) {
					Log(L"Haptics %d: %f", i, vrEvent.data.hapticVibration.fAmplitude);
					// if multiple events occurred within one frame, they are ignored except for last event
					hapticFeedback[i][0] = vrEvent.data.hapticVibration.fAmplitude;
					hapticFeedback[i][1] = vrEvent.data.hapticVibration.fDurationSeconds;
					hapticFeedback[i][2] = vrEvent.data.hapticVibration.fFrequency;
				}
			}
		}
	}

	for (int i = 0; i < 2; i++) {
		if (hapticFeedback[i][0] != 0 || hapticFeedback[i][1] != 0 || hapticFeedback[i][2] != 0) {
			listener->SendHapticsFeedback(0,
				static_cast<float>(hapticFeedback[i][0]),
				static_cast<float>(hapticFeedback[i][1]),
				static_cast<float>(hapticFeedback[i][2]),
				mRemoteController[i]->GetHand() ? 1 : 0);
		}
	}

	mFreePIE->UpdateTrackingInfoByFreePIE(info, mFixedOrientationHMD, mFixedOrientationController, mFixedPositionHMD, mFixedPositionController, hapticFeedback);

	auto data = mFreePIE->GetData();

	if (data.flags & FreePIE::ALVR_FREEPIE_FLAG_OVERRIDE_HEAD_ORIENTATION) {
		mFixedOrientationHMD = EulerAngleToQuaternion(data.head_orientation);
	}
	if (data.flags & FreePIE::ALVR_FREEPIE_FLAG_OVERRIDE_CONTROLLER_ORIENTATION0) {
		for (int i = 0; i < TrackingInfo::MAX_CONTROLLERS; i++) {
			mFixedOrientationController[i] = EulerAngleToQuaternion(data.controller_orientation[i]);
		}
	}
	if (data.flags & FreePIE::ALVR_FREEPIE_FLAG_OVERRIDE_HEAD_POSITION) {
		mFixedPositionHMD.x = (float)data.head_position[0];
		mFixedPositionHMD.y = (float)data.head_position[1];
		mFixedPositionHMD.z = (float)data.head_position[2];
	}
	if (data.flags & FreePIE::ALVR_FREEPIE_FLAG_OVERRIDE_CONTROLLER_POSITION0) {
		for (int i = 0; i < TrackingInfo::MAX_CONTROLLERS; i++) {
			mFixedPositionController[i].x = (float)data.controller_position[i][0];
			mFixedPositionController[i].y = (float)data.controller_position[i][1];
			mFixedPositionController[i].z = (float)data.controller_position[i][2];
		}
	}

	if (Settings::Instance().mEnableOffsetPos) {
		mFixedPositionHMD.x += Settings::Instance().mOffsetPos[0];
		mFixedPositionHMD.y += Settings::Instance().mOffsetPos[1];
		mFixedPositionHMD.z += Settings::Instance().mOffsetPos[2];
		for (int i = 0; i < TrackingInfo::MAX_CONTROLLERS; i++) {
			mFixedPositionController[i].x += Settings::Instance().mOffsetPos[0];
			mFixedPositionController[i].y += Settings::Instance().mOffsetPos[1];
			mFixedPositionController[i].z += Settings::Instance().mOffsetPos[2];
		}
	}

	UpdateControllerState(info);
}

vr::HmdQuaternion_t RecenterManager::GetRecenteredHMD() {
	return mFixedOrientationHMD;
}

TrackingVector3 RecenterManager::GetRecenteredPositionHMD() {
	return mFixedPositionHMD;
}

std::string RecenterManager::GetFreePIEMessage() {
	return mFreePIE->GetData().message;
}

void RecenterManager::UpdateOtherTrackingSource(const TrackingInfo & info) {
	if (mRotationDiffLastInitialized == 0) {
		mBasePosition = info.Other_Tracking_Source_Position;
		mRotationDiff = 0.0;
		mRotatedBasePosition = mBasePosition;
	}
	TrackingVector3 transformed;
	double theta = mRotationDiff + mCenterPitch;
	transformed.x = (float)((info.Other_Tracking_Source_Position.x - mBasePosition.x) * cos(theta) - (info.Other_Tracking_Source_Position.z - mBasePosition.z) * sin(theta));
	transformed.y = info.Other_Tracking_Source_Position.y;
	transformed.z = (float)((info.Other_Tracking_Source_Position.x - mBasePosition.x) * sin(theta) + (info.Other_Tracking_Source_Position.z - mBasePosition.z) * cos(theta));

	transformed.x += mRotatedBasePosition.x;
	transformed.z += mRotatedBasePosition.z;

	if (GetTimestampUs() - mRotationDiffLastInitialized > 2 * 1000 * 1000) {
		double p1 = PitchFromQuaternion(info.Other_Tracking_Source_Orientation);
		double pitch_tracking = PitchFromQuaternion(info.HeadPose_Pose_Orientation);
		double diff = p1 - pitch_tracking;
		if (diff < 0) {
			diff += M_PI * 2;
		}

		mRotationDiffLastInitialized = GetTimestampUs();
		mRotationDiff = diff;
		mBasePosition = info.Other_Tracking_Source_Position;

		mRotatedBasePosition = transformed;
	}

	mFixedPositionHMD.x += transformed.x;
	mFixedPositionHMD.y += transformed.y;
	mFixedPositionHMD.z += transformed.z;

	for (int i = 0; i < TrackingInfo::MAX_CONTROLLERS; i++) {
		mFixedPositionController[i].x += transformed.x;
		mFixedPositionController[i].y += transformed.y;
		mFixedPositionController[i].z += transformed.z;
	}

	Log(L"OtherTrackingSource (diff:%f) (%f,%f,%f) (%f,%f,%f)",
		info.Other_Tracking_Source_Position.x,
		info.Other_Tracking_Source_Position.y,
		info.Other_Tracking_Source_Position.z,
		transformed.x, transformed.y, transformed.z);
}

void RecenterManager::UpdateControllerState(const TrackingInfo & info) {
	if (!Settings::Instance().mEnableController) {
		return;
	}
	auto data = mFreePIE->GetData();
	bool enableControllerButton = data.flags & FreePIE::ALVR_FREEPIE_FLAG_OVERRIDE_BUTTONS;
	mControllerDetected = data.controllers;
	bool defaultHand = (info.controller[0].flags & TrackingInfo::Controller::FLAG_CONTROLLER_LEFTHAND) != 0;

	// Add controller as specified.
	for (int i = 0; i < mControllerDetected; i++) {
		if (mRemoteController[i]) {
			// Already enabled.
			continue;
		}
		bool hand = i == 0 ? defaultHand : !mRemoteController[0]->GetHand();
		mRemoteController[i] = std::make_shared<OpenVRController>(hand, i);

		bool ret = vr::VRServerDriverHost()->TrackedDeviceAdded(
			mRemoteController[i]->GetSerialNumber().c_str(),
			vr::TrackedDeviceClass_Controller,
			mRemoteController[i].get());
		Log(L"TrackedDeviceAdded vr::TrackedDeviceClass_Controller index=%d Ret=%d SerialNumber=%hs Hand=%d"
			, i, ret, mRemoteController[i]->GetSerialNumber().c_str(), hand);
	}

	Log(L"UpdateControllerState. detected=%d hand=%d", mControllerDetected, defaultHand);

	for (int i = 0; i < mControllerDetected; i++) {
		if (mRemoteController[i]) {
			int index = mRemoteController[i]->GetHand() == defaultHand ? 0 : 1;
			Log(L"UpdateControllerState. Updating %d controller", index);
			bool recenterRequested = mRemoteController[i]->ReportControllerState(index, info,
				mFixedOrientationController[index], mFixedPositionController[index], enableControllerButton, data);
			if (recenterRequested) {
				BeginRecenter();
			}
		}
	}
}
