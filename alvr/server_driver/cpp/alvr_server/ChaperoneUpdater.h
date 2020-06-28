#pragma once

#include <memory>
#include <stdint.h>
#include "packet_types.h"

namespace vr {
	class IVRChaperoneSetup;

	struct HmdVector2_t;
	struct HmdMatrix34_t;
}

class ChaperoneUpdater {
public:
	ChaperoneUpdater();
	~ChaperoneUpdater();

	void ResetData(uint64_t timestamp, uint32_t pointCount);
	void SetTransform(const TrackingVector3& position, const TrackingQuat& rotation, const TrackingVector2& playAreaSize);
	void SetSegment(uint32_t segmentIndex, const TrackingVector3 *points);

	bool MaybeCommitData();

	uint64_t GetDataTimestamp();
	uint32_t GetTotalPointCount();
	uint32_t GetSegmentCount();
private:
	vr::HmdVector2_t *m_ChaperonePoints = nullptr;
	std::shared_ptr<vr::HmdMatrix34_t> m_Transform;
	TrackingVector2 m_PlayArea;

	uint64_t m_Timestamp = 0;
	uint32_t m_TotalPointCount = 0;
	uint32_t m_SegmentCount = 0;
	bool m_CommitDone = true;
};