#pragma once
#include <openvr_driver.h>
#include "Utils.h"
#include "Logger.h"
#include "packet_types.h"

class RecenterManager
{
public:
	RecenterManager()
		: m_hasValidTrackingInfo(false)
		, m_recentering(false)
		, m_recenterStartTimestamp(0)
		, m_centerPitch(0.0)
		, m_rotationDiffLastInitialized(0)
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
				m_centerPitch = PitchFromQuaternion(
					info.HeadPose_Pose_Orientation.x
					, info.HeadPose_Pose_Orientation.y
					, info.HeadPose_Pose_Orientation.z
					, info.HeadPose_Pose_Orientation.w);

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
			double p1 = PitchFromQuaternion(info.Other_Tracking_Source_Orientation);
			double pitch_tracking = PitchFromQuaternion(info.HeadPose_Pose_Orientation);
			double diff = p1 - pitch_tracking;
			if (diff < 0) {
				diff += M_PI * 2;
			}
			if (m_rotationDiffLastInitialized == 0) {
				m_basePosition = info.Other_Tracking_Source_Position;
				m_rotationDiff = 0.0;
				m_rotatedBasePosition = m_basePosition;
			}
			TrackingVector3 transformed;
			transformed.x = (info.Other_Tracking_Source_Position.x - m_basePosition.x) * cos(m_rotationDiff) - (info.Other_Tracking_Source_Position.z - m_basePosition.z) * sin(m_rotationDiff);
			transformed.x += m_rotatedBasePosition.x;
			transformed.y = info.Other_Tracking_Source_Position.y;
			transformed.z = (info.Other_Tracking_Source_Position.x - m_basePosition.x) * sin(m_rotationDiff) + (info.Other_Tracking_Source_Position.z - m_basePosition.z) * cos(m_rotationDiff);
			transformed.z += m_rotatedBasePosition.z;

			if (GetTimestampUs() - m_rotationDiffLastInitialized > 2 * 1000 * 1000) {
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

			Log("pitch=%f tracking pitch=%f (diff:%f) (%f,%f,%f) (%f,%f,%f)", p1, pitch_tracking, diff,
				info.Other_Tracking_Source_Position.x,
				info.Other_Tracking_Source_Position.y,
				info.Other_Tracking_Source_Position.z,
				transformed.x, transformed.y, transformed.z);
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
	}

	vr::HmdQuaternion_t GetRecenteredHMD() {
		return m_fixedOrientationHMD;
	}

	vr::HmdQuaternion_t GetRecenteredController() {
		return m_fixedOrientationController;
	}

	TrackingVector3 GetRecenteredPositionHMD() {
		return m_fixedPositionHMD;
	}

	TrackingVector3 GetRecenteredPositionController() {
		return m_fixedPositionController;
	}
private:
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
