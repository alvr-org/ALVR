#pragma once

#include <memory>
#include <mutex>
#include <stdint.h>
#include "threadtools.h"
#include "packet_types.h"

// This header cannot include openvr_api.h. It would lead to conflicts with openvr_driver.h
// included where this header is also needed, so just forward declare the required stuff here.
namespace vr {
	class IVRChaperoneSetup;

	struct HmdVector2_t;
	struct HmdMatrix34_t;
}

const int ALVR_STANDING_CHAPERONE_POINT_COUNT = 64;

/// <summary>
/// Takes care of setting and hiding SteamVR Chaperone, along with standing/sitting position
/// and floor height. Starts a utility SteamVR client to do the actual changes.
/// </summary>
class ChaperoneUpdater : public CThread {
public:
	ChaperoneUpdater();
	~ChaperoneUpdater();

	void ResetData(uint64_t timestamp, uint32_t pointCount);
	void SetTransform(const TrackingVector3& position, const TrackingQuat& rotation, const TrackingVector2& playAreaSize);
	void SetSegment(uint32_t segmentIndex, const TrackingVector3 *points);

	/// <summary>
	/// Generates a circle for a standing Chaperone setup.
	/// </summary>
	/// <param name="scale">Radius of the generated circle</param>
	void GenerateStandingChaperone(float scale = 0.25f);

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