#pragma once

#include "ALVR-common/packet_types.h"
#include "openvr_driver_wrap.h"

#include <chrono>
#include <list>
#include <mutex>
#include <optional>

class PoseHistory {
public:
    struct TrackingHistoryFrame {
        uint64_t targetTimestampNs;
        FfiDeviceMotion motion;
        vr::HmdMatrix34_t rotationMatrix;
        uint64_t serverReceiveTimeNs;
    };

    void OnPoseUpdated(uint64_t targetTimestampNs, FfiDeviceMotion motion);

    std::optional<TrackingHistoryFrame> GetBestPoseMatch(const vr::HmdMatrix34_t& pose) const;
    std::optional<TrackingHistoryFrame> GetLatestPose() const;
    std::optional<TrackingHistoryFrame> GetPoseAt(uint64_t timestampNs) const;
    std::optional<TrackingHistoryFrame> GetPoseByPresentTime(uint64_t presentTimeNs) const;

    void SetTransform(const vr::HmdMatrix34_t& transform);

private:
    mutable std::mutex m_mutex;
    std::list<TrackingHistoryFrame> m_poseBuffer;
    vr::HmdMatrix34_t m_transform
        = { { { 1.0, 0.0, 0.0, 0.0 }, { 0.0, 1.0, 0.0, 0.0 }, { 0.0, 0.0, 1.0, 0.0 } } };
    bool m_transformIdentity = true;
};
