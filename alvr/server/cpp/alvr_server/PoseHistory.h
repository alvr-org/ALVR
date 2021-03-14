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
		TrackingInfo info;
		vr::HmdMatrix34_t rotationMatrix;
	};

	void OnPoseUpdated(const TrackingInfo &info);

	std::optional<TrackingHistoryFrame> GetBestPoseMatch(const vr::HmdMatrix34_t &pose) const;
	std::optional<TrackingHistoryFrame> GetPoseAt(uint64_t client_timestamp_us) const;

private:
	mutable std::mutex m_mutex;
	std::list<TrackingHistoryFrame> m_poseBuffer;
};
