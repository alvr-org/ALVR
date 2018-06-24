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

	void OnPoseUpdated(const TrackingInfo &info) {
		m_hasValidTrackingInfo = true;
		if (m_recentering) {
			if (GetTimestampUs() - m_recenterStartTimestamp > RECENTER_DURATION) {
				m_centerPitch = PitchFromQuaternion(info.HeadPose_Pose_Orientation);

				Log("Do recentered: Cur=(%f,%f,%f,%f) pitch=%f"
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
		if (Settings::Instance().m_EnableOffsetPos) {
			m_fixedPositionHMD.x += Settings::Instance().m_OffsetPos[0];
			m_fixedPositionHMD.y += Settings::Instance().m_OffsetPos[1];
			m_fixedPositionHMD.z += Settings::Instance().m_OffsetPos[2];
		}

		m_fixedOrientationController = MultiplyPitchQuaternion(
			-m_centerPitch
			, info.controller_Pose_Orientation.x
			, info.controller_Pose_Orientation.y
			, info.controller_Pose_Orientation.z
			, info.controller_Pose_Orientation.w);

		m_fixedPositionController = RotateVectorQuaternion(info.controller_Pose_Position, m_centerPitch);
		if (Settings::Instance().m_EnableOffsetPos) {
			m_fixedPositionController.x += Settings::Instance().m_OffsetPos[0];
			m_fixedPositionController.y += Settings::Instance().m_OffsetPos[1];
			m_fixedPositionController.z += Settings::Instance().m_OffsetPos[2];
		}

		if (info.flags & TrackingInfo::FLAG_OTHER_TRACKING_SOURCE) {
			UpdateOtherTrackingSource(info);
		}
		Log("GetRecenteredHMD: Old=(%f,%f,%f,%f) New=(%f,%f,%f,%f) pitch=%f-%f"
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

		double head_position[3];
		double controller_position[3];
		uint32_t controllerOverrideButtons;
		uint32_t controllerButtons;

		Log("RecenteredController:(%f, %f, %f), %f, %f, %f, %f", m_fixedPositionController.x, m_fixedPositionController.y, m_fixedPositionController.z
		, m_fixedOrientationController.x, m_fixedOrientationController.y, m_fixedOrientationController.z, m_fixedOrientationController.w);
		m_freePIE->UpdateTrackingInfoByFreePIE(info, m_fixedOrientationHMD, head_position, m_fixedOrientationController, controller_position
			, &controllerOverrideButtons, &controllerButtons);

		UpdateControllerState(info, controllerOverrideButtons, controllerButtons);
	}

	vr::HmdQuaternion_t GetRecenteredHMD() {
		return m_fixedOrientationHMD;
	}

	TrackingVector3 GetRecenteredPositionHMD() {
		return m_fixedPositionHMD;
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
		transformed.x = (info.Other_Tracking_Source_Position.x - m_basePosition.x) * cos(theta) - (info.Other_Tracking_Source_Position.z - m_basePosition.z) * sin(theta);
		transformed.x += m_rotatedBasePosition.x;
		transformed.y = info.Other_Tracking_Source_Position.y;
		transformed.z = (info.Other_Tracking_Source_Position.x - m_basePosition.x) * sin(theta) + (info.Other_Tracking_Source_Position.z - m_basePosition.z) * cos(theta);
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

		m_fixedPositionController.x += transformed.x;
		m_fixedPositionController.y += transformed.y;
		m_fixedPositionController.z += transformed.z;

		Log("OtherTrackingSource (diff:%f) (%f,%f,%f) (%f,%f,%f)",
			info.Other_Tracking_Source_Position.x,
			info.Other_Tracking_Source_Position.y,
			info.Other_Tracking_Source_Position.z,
			transformed.x, transformed.y, transformed.z);
	}

	void UpdateControllerState(const TrackingInfo& info, uint32_t controllerOverrideButtons, uint32_t controllerButtons) {
		if (!Settings::Instance().m_enableController) {
			return;
		}
		if (!m_controllerDetected) {
			if ((info.flags & TrackingInfo::FLAG_CONTROLLER_ENABLE) || controllerOverrideButtons) {
				Log("Enabling new controller by %s", controllerOverrideButtons ? "FreePIE" : "Client");
				m_controllerDetected = true;

				// false: right hand, true: left hand
				bool handed = (info.flags & TrackingInfo::FLAG_CONTROLLER_LEFTHAND) != 0;
				m_remoteController = std::make_shared<RemoteControllerServerDriver>(handed);

				bool ret;
				ret = vr::VRServerDriverHost()->TrackedDeviceAdded(
					m_remoteController->GetSerialNumber().c_str(),
					vr::TrackedDeviceClass_Controller,
					m_remoteController.get());
				Log("TrackedDeviceAdded Ret=%d SerialNumber=%s", ret, m_remoteController->GetSerialNumber().c_str());
			}
		}
		if (m_controllerDetected) {
			Log("RecenteredController2:(%f, %f, %f), %f, %f, %f, %f", m_fixedPositionController.x, m_fixedPositionController.y, m_fixedPositionController.z
				, m_fixedOrientationController.x, m_fixedOrientationController.y, m_fixedOrientationController.z, m_fixedOrientationController.w);
			bool recenterRequested = m_remoteController->ReportControllerState(info, m_fixedOrientationController, m_fixedPositionController, controllerOverrideButtons, controllerButtons);
			if (recenterRequested) {
				BeginRecenter();
			}
		}
	}

	bool m_controllerDetected;
	std::shared_ptr<RemoteControllerServerDriver> m_remoteController;

	std::shared_ptr<FreePIE> m_freePIE;

	bool m_hasValidTrackingInfo;
	bool m_recentering;
	uint64_t m_recenterStartTimestamp;
	double m_centerPitch;
	vr::HmdQuaternion_t m_fixedOrientationHMD;
	TrackingVector3 m_fixedPositionHMD;
	vr::HmdQuaternion_t m_fixedOrientationController;
	TrackingVector3 m_fixedPositionController;

	TrackingVector3 m_basePosition;
	TrackingVector3 m_rotatedBasePosition;
	double m_rotationDiff;
	uint64_t m_rotationDiffLastInitialized;

	static const int RECENTER_DURATION = 400 * 1000;
};
