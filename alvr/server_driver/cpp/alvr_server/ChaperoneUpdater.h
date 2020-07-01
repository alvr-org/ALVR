#pragma once

#include <memory>
#include <mutex>
#include <stdint.h>
#include "threadtools.h"
#include "packet_types.h"

namespace vr {
	class IVRChaperoneSetup;

	struct HmdVector2_t;
	struct HmdMatrix34_t;
}

const int ALVR_STANDING_CHAPERONE_POINT_COUNT = 64;

class ChaperoneUpdater : public CThread {
public:
	ChaperoneUpdater();
	~ChaperoneUpdater();

	void ResetData(uint64_t timestamp, uint32_t pointCount);
	void SetTransform(const TrackingVector3& position, const TrackingQuat& rotation, const TrackingVector2& playAreaSize);
	void SetSegment(uint32_t segmentIndex, const TrackingVector3 *points);
	void GenerateStandingGuardian(float scale = 0.25f);

	bool MaybeCommitData();

	virtual void Run();

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

	bool m_Exiting = false;

	CThreadEvent m_ChaperoneDataReady;
	std::mutex m_ChaperoneDataMtx;
};