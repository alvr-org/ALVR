#pragma once

#include <list>
#include <mutex>
#include <openvr_driver.h>
#include <optional>
#include "ALVR-common/packet_types.h"

class PoseHistory
{
public:
	struct TrackingHistoryFrame {
		uint64_t targetTimestampNs;
		FfiDeviceMotion motion;
		vr::HmdMatrix34_t rotationMatrix;
	};

	void OnPoseUpdated(uint64_t targetTimestampNs, FfiDeviceMotion motion);

	std::optional<TrackingHistoryFrame> GetBestPoseMatch(const vr::HmdMatrix34_t &pose) const;
	// Return the most recent pose known at the given timestamp
	std::optional<TrackingHistoryFrame> GetPoseAt(uint64_t timestampNs) const;

	void SetTransformUpdating();
	void SetTransform(const vr::HmdMatrix34_t &transform);

private:
	mutable std::mutex m_mutex;
	std::list<TrackingHistoryFrame> m_poseBuffer;
	vr::HmdMatrix34_t m_transform = {{{1.0, 0.0, 0.0, 0.0}, {0.0, 1.0, 0.0, 0.0}, {0.0, 0.0, 1.0, 0.0}}};
	bool m_transformIdentity = true;
	bool m_transformUpdating = false;
};
