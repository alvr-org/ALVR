#include "PoseHistory.h"
#include "Utils.h"
#include "Logger.h"
#include <mutex>
#include <optional>

void PoseHistory::OnPoseUpdated(uint64_t targetTimestampNs, FfiDeviceMotion motion) {
	// Put pose history buffer
	TrackingHistoryFrame history;
	history.targetTimestampNs = targetTimestampNs;
	history.motion = motion;

	HmdMatrix_QuatToMat(motion.orientation.w,
		motion.orientation.x,
		motion.orientation.y,
		motion.orientation.z,
		&history.rotationMatrix);

	std::unique_lock<std::mutex> lock(m_mutex);
	if (!m_transformIdentity) {
		vr::HmdMatrix34_t rotation = {};
		for (int i = 0; i < 3; ++i) {
			for (int j = 0; j < 3; ++j) {
				rotation.m[j][i] = 0;
				for (int k = 0; k < 3; ++k) {
					rotation.m[j][i] += history.rotationMatrix.m[k][i] * m_transform.m[j][k];
				}
			}
		}
		history.rotationMatrix = rotation;
	}

	if (m_poseBuffer.size() == 0) {
		m_poseBuffer.push_back(history);
	}
	else {
		if (m_poseBuffer.back().targetTimestampNs != targetTimestampNs) {
			// New track info
			m_poseBuffer.push_back(history);
		}
	}
        // The value should match with the client's MAXIMUM_TRACKING_FRAMES in ovr_context.cpp
	if (m_poseBuffer.size() > 120 * 3) {
		m_poseBuffer.pop_front();
	}
}

std::optional<PoseHistory::TrackingHistoryFrame> PoseHistory::GetBestPoseMatch(const vr::HmdMatrix34_t &pose) const
{
	std::unique_lock<std::mutex> lock(m_mutex);
	if (m_transformUpdating && !m_poseBuffer.empty()) {
		return m_poseBuffer.back();
	}
	float minDiff = 100000;
	auto minIt = m_poseBuffer.begin();
	for (auto it = m_poseBuffer.begin(); it != m_poseBuffer.end(); ++it) {
		float distance = 0;
		// Rotation matrix composes a part of ViewMatrix of TrackingInfo.
		// Be carefull of transpose.
		// And bottom side and right side of matrix should not be compared, because pPose does not contain that part of matrix.
		for (int i = 0; i < 3; i++) {
			for (int j = 0; j < 3; j++) {
				distance += pow(it->rotationMatrix.m[j][i] - pose.m[j][i], 2);
			}
		}
		if (minDiff > distance) {
			minIt = it;
			minDiff = distance;
		}
	}
	if (minIt != m_poseBuffer.end()) {
		return *minIt;
	}
	return {};
}

std::optional<PoseHistory::TrackingHistoryFrame> PoseHistory::GetPoseAt(uint64_t timestampNs) const
{
	std::unique_lock<std::mutex> lock(m_mutex);
	for (auto it = m_poseBuffer.rbegin(), end = m_poseBuffer.rend() ; it != end ; ++it)
	{
		if (it->targetTimestampNs == timestampNs)
			return *it;
	}
	return {};
}

void PoseHistory::SetTransformUpdating()
{
	std::unique_lock<std::mutex> lock(m_mutex);
	m_transformUpdating = true;
}

void PoseHistory::SetTransform(const vr::HmdMatrix34_t &transform)
{
	std::unique_lock<std::mutex> lock(m_mutex);
	m_transform = transform;
	m_transformUpdating = false;
	m_poseBuffer.clear();

	for (int i = 0; i < 3; ++i) {
		for (int j = 0; j < 3; ++j) {
			if (transform.m[i][j] != (i == j ? 1 : 0)) {
				m_transformIdentity = false;
				return;
			}
		}
	}
	m_transformIdentity = true;
}
