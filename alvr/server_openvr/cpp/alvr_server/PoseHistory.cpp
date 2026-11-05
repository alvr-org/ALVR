#include "PoseHistory.h"
#include "Logger.h"
#include "Utils.h"
#include "include/openvr_math.h"
#include <mutex>
#include <optional>

void PoseHistory::OnPoseUpdated(uint64_t targetTimestampNs, FfiDeviceMotion motion) {
    // Put pose history buffer
    TrackingHistoryFrame history;
    history.targetTimestampNs = targetTimestampNs;
    history.motion = motion;

    HmdMatrix_QuatToMat(
        motion.pose.orientation.w,
        motion.pose.orientation.x,
        motion.pose.orientation.y,
        motion.pose.orientation.z,
        &history.poseMatrix
    );
    for (int i = 0; i < 3; i++) {
        history.poseMatrix.m[i][3] = motion.pose.position[i];
    }

    std::unique_lock<std::mutex> lock(m_mutex);
    if (!m_transformIdentity) {
        vr::HmdMatrix34_t pose = vrmath::matMul33(m_transform, history.poseMatrix);
        for (int i = 0; i < 3; i++) {
            pose.m[i][3] = m_transform.m[i][3];
            for (int k = 0; k < 3; k++) {
                pose.m[i][3] += m_transform.m[i][k] * motion.pose.position[k];
            }
        }
        history.poseMatrix = pose;
    }

    if (m_poseBuffer.size() == 0) {
        m_poseBuffer.push_back(history);
    } else {
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

std::optional<PoseHistory::TrackingHistoryFrame>
PoseHistory::GetBestPoseMatch(const vr::HmdMatrix34_t& pose) const {
    std::unique_lock<std::mutex> lock(m_mutex);
    float minDiff = 100000;
    auto minIt = m_poseBuffer.begin();
    for (auto it = m_poseBuffer.begin(); it != m_poseBuffer.end(); ++it) {
        // Rotation and translation both: comparing rotation alone makes every sample of a head
        // that moves without turning tie, so the frame resolves to whichever pose wins the tie
        // instead of the one it was rendered from, and the head position reported with the frame
        // runs ahead of the image by an amount that varies frame to frame.
        float distance = 0;
        for (int i = 0; i < 3; i++) {
            for (int j = 0; j < 4; j++) {
                distance += pow(it->poseMatrix.m[i][j] - pose.m[i][j], 2);
            }
        }
        // `>=` rather than `>` so a tie keeps the newest sample. The buffer is iterated oldest
        // to newest, so a head that is not moving makes every entry tie and the oldest would win,
        // resolving the frame to a pose up to the buffer's whole length old. That inflates the
        // reported game time to about a second, which trips the prediction clamp and the frame is
        // dropped: video then only advances while the view is moving. A real headset is saved
        // from this by sensor noise, not by anything here.
        if (minDiff >= distance) {
            minIt = it;
            minDiff = distance;
        }
    }
    if (minIt != m_poseBuffer.end()) {
        return *minIt;
    }

    Debug("PoseHistory::GetBestPoseMatch: No pose matched.");
    return {};
}

std::optional<PoseHistory::TrackingHistoryFrame> PoseHistory::GetPoseAt(uint64_t timestampNs
) const {
    std::unique_lock<std::mutex> lock(m_mutex);
    for (auto it = m_poseBuffer.rbegin(), end = m_poseBuffer.rend(); it != end; ++it) {
        if (it->targetTimestampNs == timestampNs)
            return *it;
    }

    Debug("PoseHistory::GetPoseAt: No pose matched.");
    return {};
}

void PoseHistory::SetTransform(const vr::HmdMatrix34_t& transform) {
    std::unique_lock<std::mutex> lock(m_mutex);
    m_transform = transform;

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
