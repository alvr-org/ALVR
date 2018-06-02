#pragma once
#include <openvr_driver.h>
#include "Utils.h"
#include "Logger.h"
#include "packet_types.h"

class RecenterManager
{
public:
	RecenterManager()
		: m_recentering(false)
		, m_recenterStartTimestamp(0)
		, m_centerPitch(0.0)
	{
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

	void OnPoseUpdated(TrackingInfo &info) {
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
	}

	vr::HmdQuaternion_t GetRecentered(const TrackingQuat &orientation) {
		vr::HmdQuaternion_t fixedOrientation = MultiplyPitchQuaternion(
			-m_centerPitch
			, orientation.x
			, orientation.y
			, orientation.z
			, orientation.w);

		Log("GetRecentered: Old=(%f,%f,%f,%f) New=(%f,%f,%f,%f) pitch=%f-%f %f"
			, orientation.x, orientation.y
			, orientation.z, orientation.w
			, fixedOrientation.x, fixedOrientation.y
			, fixedOrientation.z, fixedOrientation.w
			, PitchFromQuaternion(
				orientation.x
				, orientation.y
				, orientation.z
				, orientation.w)
			, PitchFromQuaternion(
				fixedOrientation.x
				, fixedOrientation.y
				, fixedOrientation.z
				, fixedOrientation.w)
			, m_centerPitch);
		return fixedOrientation;
	}

	TrackingVector3 GetRecenteredVector(const TrackingVector3 &position) {
		TrackingVector3 fixedPosition = RotateVectorQuaternion(position, m_centerPitch);
		Log("GetRecenteredVector: Old=(%f,%f,%f) New=(%f,%f,%f) pitch=%f %f %f"
			, position.x, position.y
			, position.z
			, fixedPosition.x, fixedPosition.y
			, fixedPosition.z
			, m_centerPitch
			, sqrt(position.x * position.x + position.z * position.z)
			, sqrt(fixedPosition.x * fixedPosition.x + fixedPosition.z * fixedPosition.z));
		return fixedPosition;
	}
private:
	bool m_recentering;
	uint64_t m_recenterStartTimestamp;
	double m_centerPitch;

	static const int RECENTER_DURATION = 400 * 1000;
};
