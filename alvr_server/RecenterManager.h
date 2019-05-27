#pragma once
#include <openvr_driver.h>
#include "Utils.h"
#include "Logger.h"
#include "packet_types.h"
#include "FreePIE.h"
#include "RemoteController.h"

class RecenterManager
{
public:
	RecenterManager()
		: m_hasValidTrackingInfo(false)
		, m_recentering(false)
		, m_recenterStartTimestamp(0)
		, m_centerPitch(0.0)
		, m_rotationDiffLastInitialized(0)
		, m_freePIE(std::make_shared<FreePIE>())
		, m_controllerDetected(false)
	{
	}

	bool HasValidTrackingInfo() {
		return m_hasValidTrackingInfo;
	}

	bool IsRecentering() {
		return m_recentering;
	}

	void BeginRecenter() {
		m_recenterStartTimestamp = GetTimestampUs();
		m_recentering = true;
	}

	void EndRecenter() {
		m_recentering = false;
	}

	void OnPoseUpdated(const TrackingInfo &info, Listener *listener) {
		m_hasValidTrackingInfo = true;
		if (m_recentering) {
			if (GetTimestampUs() - m_recenterStartTimestamp > RECENTER_DURATION) {
				m_centerPitch = PitchFromQuaternion(info.HeadPose_Pose_Orientation);

				Log(L"Do recentered: Cur=(%f,%f,%f,%f) pitch=%f"
					, info.HeadPose_Pose_Orientation.x
					, info.HeadPose_Pose_Orientation.y
					, info.HeadPose_Pose_Orientation.z
					, info.HeadPose_Pose_Orientation.w
					, m_centerPitch
				);

				m_recentering = false;
			}
		}

		m_fixedOrientationHMD = MultiplyPitchQuaternion(
			-m_centerPitch
			, info.HeadPose_Pose_Orientation.x
			, info.HeadPose_Pose_Orientation.y
			, info.HeadPose_Pose_Orientation.z
			, info.HeadPose_Pose_Orientation.w);

		m_fixedPositionHMD = RotateVectorQuaternion(info.HeadPose_Pose_Position, m_centerPitch);

		for (int i = 0; i < TrackingInfo::MAX_CONTROLLERS; i++) {
			m_fixedOrientationController[i] = MultiplyPitchQuaternion(
				-m_centerPitch
				, info.controller[i].orientation.x
				, info.controller[i].orientation.y
				, info.controller[i].orientation.z
				, info.controller[i].orientation.w);

			m_fixedPositionController[i] = RotateVectorQuaternion(info.controller[i].position, m_centerPitch);
		}

		if (info.flags & TrackingInfo::FLAG_OTHER_TRACKING_SOURCE) {
			UpdateOtherTrackingSource(info);
		}
		Log(L"GetRecenteredHMD: Old=(%f,%f,%f,%f) New=(%f,%f,%f,%f) pitch=%f-%f"
			, info.HeadPose_Pose_Orientation.x, info.HeadPose_Pose_Orientation.y
			, info.HeadPose_Pose_Orientation.z, info.HeadPose_Pose_Orientation.w
			, m_fixedOrientationHMD.x, m_fixedOrientationHMD.y
			, m_fixedOrientationHMD.z, m_fixedOrientationHMD.w
			, m_centerPitch
			, PitchFromQuaternion(
				m_fixedOrientationHMD.x
				, m_fixedOrientationHMD.y
				, m_fixedOrientationHMD.z
				, m_fixedOrientationHMD.w));

		double  hapticFeedback[2][3]{ {0,0,0},{0,0,0} };
		vr::VREvent_t vrEvent;

		while (vr::VRServerDriverHost()->PollNextEvent(&vrEvent, sizeof(vrEvent)))
		{
			if (vrEvent.eventType == vr::VREvent_Input_HapticVibration)
			{
				for (int i = 0; i < 2; i++) {
					if (m_remoteController[i] && m_remoteController[i]->IsMyHapticComponent(vrEvent.data.hapticVibration.componentHandle)) {
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
				listener->SendHapticsFeedback(0, hapticFeedback[i][0], hapticFeedback[i][1], hapticFeedback[i][2], m_remoteController[i]->GetHand() ? 1 : 0);
			}
		}

		m_freePIE->UpdateTrackingInfoByFreePIE(info, m_fixedOrientationHMD, m_fixedOrientationController, m_fixedPositionHMD, m_fixedPositionController, hapticFeedback);

		auto data = m_freePIE->GetData();

		if (data.flags & FreePIE::ALVR_FREEPIE_FLAG_OVERRIDE_HEAD_ORIENTATION) {
			m_fixedOrientationHMD = EulerAngleToQuaternion(data.head_orientation);
		}
		if (data.flags & FreePIE::ALVR_FREEPIE_FLAG_OVERRIDE_CONTROLLER_ORIENTATION0) {
			for (int i = 0; i < TrackingInfo::MAX_CONTROLLERS; i++) {
				m_fixedOrientationController[i] = EulerAngleToQuaternion(data.controller_orientation[i]);
			}
		}
		if (data.flags & FreePIE::ALVR_FREEPIE_FLAG_OVERRIDE_HEAD_POSITION) {
			m_fixedPositionHMD.x = (float) data.head_position[0];
			m_fixedPositionHMD.y = (float) data.head_position[1];
			m_fixedPositionHMD.z = (float) data.head_position[2];
		}
		if (data.flags & FreePIE::ALVR_FREEPIE_FLAG_OVERRIDE_CONTROLLER_POSITION0) {
			for (int i = 0; i < TrackingInfo::MAX_CONTROLLERS; i++) {
				m_fixedPositionController[i].x = (float)data.controller_position[i][0];
				m_fixedPositionController[i].y = (float)data.controller_position[i][1];
				m_fixedPositionController[i].z = (float)data.controller_position[i][2];
			}
		}

		if (Settings::Instance().m_EnableOffsetPos) {
			m_fixedPositionHMD.x += Settings::Instance().m_OffsetPos[0];
			m_fixedPositionHMD.y += Settings::Instance().m_OffsetPos[1];
			m_fixedPositionHMD.z += Settings::Instance().m_OffsetPos[2];
			for (int i = 0; i < TrackingInfo::MAX_CONTROLLERS; i++) {
				m_fixedPositionController[i].x += Settings::Instance().m_OffsetPos[0];
				m_fixedPositionController[i].y += Settings::Instance().m_OffsetPos[1];
				m_fixedPositionController[i].z += Settings::Instance().m_OffsetPos[2];
			}
		}

		UpdateControllerState(info);
	}

	vr::HmdQuaternion_t GetRecenteredHMD() {
		return m_fixedOrientationHMD;
	}

	TrackingVector3 GetRecenteredPositionHMD() {
		return m_fixedPositionHMD;
	}

	std::string GetFreePIEMessage() {
		return m_freePIE->GetData().message;
	}

private:
	void UpdateOtherTrackingSource(const TrackingInfo &info) {
		if (m_rotationDiffLastInitialized == 0) {
			m_basePosition = info.Other_Tracking_Source_Position;
			m_rotationDiff = 0.0;
			m_rotatedBasePosition = m_basePosition;
		}
		TrackingVector3 transformed;
		double theta = m_rotationDiff + m_centerPitch;
		transformed.x = (float) ((info.Other_Tracking_Source_Position.x - m_basePosition.x) * cos(theta) - (info.Other_Tracking_Source_Position.z - m_basePosition.z) * sin(theta));
		transformed.y = info.Other_Tracking_Source_Position.y;
		transformed.z = (float)((info.Other_Tracking_Source_Position.x - m_basePosition.x) * sin(theta) + (info.Other_Tracking_Source_Position.z - m_basePosition.z) * cos(theta));

		transformed.x += m_rotatedBasePosition.x;
		transformed.z += m_rotatedBasePosition.z;

		if (GetTimestampUs() - m_rotationDiffLastInitialized > 2 * 1000 * 1000) {
			double p1 = PitchFromQuaternion(info.Other_Tracking_Source_Orientation);
			double pitch_tracking = PitchFromQuaternion(info.HeadPose_Pose_Orientation);
			double diff = p1 - pitch_tracking;
			if (diff < 0) {
				diff += M_PI * 2;
			}

			m_rotationDiffLastInitialized = GetTimestampUs();
			m_rotationDiff = diff;
			m_basePosition = info.Other_Tracking_Source_Position;

			m_rotatedBasePosition = transformed;
		}

		m_fixedPositionHMD.x += transformed.x;
		m_fixedPositionHMD.y += transformed.y;
		m_fixedPositionHMD.z += transformed.z;

		for (int i = 0; i < TrackingInfo::MAX_CONTROLLERS; i++) {
			m_fixedPositionController[i].x += transformed.x;
			m_fixedPositionController[i].y += transformed.y;
			m_fixedPositionController[i].z += transformed.z;
		}

		Log(L"OtherTrackingSource (diff:%f) (%f,%f,%f) (%f,%f,%f)",
			info.Other_Tracking_Source_Position.x,
			info.Other_Tracking_Source_Position.y,
			info.Other_Tracking_Source_Position.z,
			transformed.x, transformed.y, transformed.z);
	}

	void UpdateControllerState(const TrackingInfo& info) {
		if (!Settings::Instance().m_enableController) {
			return;
		}
		auto data = m_freePIE->GetData();
		bool enableControllerButton = data.flags & FreePIE::ALVR_FREEPIE_FLAG_OVERRIDE_BUTTONS;
		m_controllerDetected = data.controllers;
		bool defaultHand = (info.controller[0].flags & TrackingInfo::Controller::FLAG_CONTROLLER_LEFTHAND) != 0;

		// Add controller as specified.
		for (int i = 0; i < m_controllerDetected; i++) {
			if (m_remoteController[i]) {
				// Already enabled.
				continue;
			}
			bool hand = i == 0 ? defaultHand : !m_remoteController[0]->GetHand();
			m_remoteController[i] = std::make_shared<RemoteControllerServerDriver>(hand, i);

			bool ret = vr::VRServerDriverHost()->TrackedDeviceAdded(
				m_remoteController[i]->GetSerialNumber().c_str(),
				vr::TrackedDeviceClass_Controller,
				m_remoteController[i].get());
			Log(L"TrackedDeviceAdded vr::TrackedDeviceClass_Controller index=%d Ret=%d SerialNumber=%hs Hand=%d"
				, i, ret, m_remoteController[i]->GetSerialNumber().c_str(), hand);
		}

		Log(L"UpdateControllerState. detected=%d hand=%d", m_controllerDetected, defaultHand);

		for (int i = 0; i < m_controllerDetected; i++) {
			if (m_remoteController[i]) {
				int index = m_remoteController[i]->GetHand() == defaultHand ? 0 : 1;
				Log(L"UpdateControllerState. Updating %d controller", index);
				bool recenterRequested = m_remoteController[i]->ReportControllerState(index, info,
					m_fixedOrientationController[index], m_fixedPositionController[index], enableControllerButton, data);
				if (recenterRequested) {
					BeginRecenter();
				}
			}
		}
	}

	int m_controllerDetected;
	std::shared_ptr<RemoteControllerServerDriver> m_remoteController[2];

	std::shared_ptr<FreePIE> m_freePIE;

	bool m_hasValidTrackingInfo;
	bool m_recentering;
	uint64_t m_recenterStartTimestamp;
	double m_centerPitch;
	vr::HmdQuaternion_t m_fixedOrientationHMD;
	TrackingVector3 m_fixedPositionHMD;
	vr::HmdQuaternion_t m_fixedOrientationController[2];
	TrackingVector3 m_fixedPositionController[2];

	TrackingVector3 m_basePosition;
	TrackingVector3 m_rotatedBasePosition;
	double m_rotationDiff;
	uint64_t m_rotationDiffLastInitialized;

	static const int RECENTER_DURATION = 400 * 1000;
};
