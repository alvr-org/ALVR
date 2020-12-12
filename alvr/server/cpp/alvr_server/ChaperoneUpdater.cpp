#include "ChaperoneUpdater.h"
#include "Logger.h"
#include <openvr.h>

#define _USE_MATH_DEFINES
#include <math.h>

inline void MakeTransformFromVecQuat(const TrackingVector3 &p, const TrackingQuat &q, std::shared_ptr<vr::HmdMatrix34_t> result)
{
	float sqw = q.w * q.w;
	float sqx = q.x * q.x;
	float sqy = q.y * q.y;
	float sqz = q.z * q.z;

	result->m[0][0] = (sqx - sqy - sqz + sqw);
	result->m[1][1] = (-sqx + sqy - sqz + sqw);
	result->m[2][2] = (-sqx - sqy + sqz + sqw);

	float tmp1 = q.x * q.y;
	float tmp2 = q.z * q.w;
	result->m[1][0] = 2.0f * (tmp1 + tmp2);
	result->m[0][1] = 2.0f * (tmp1 - tmp2);

	tmp1 = q.x * q.z;
	tmp2 = q.y * q.w;
	result->m[2][0] = 2.0f * (tmp1 - tmp2);
	result->m[0][2] = 2.0f * (tmp1 + tmp2);
	tmp1 = q.y * q.z;
	tmp2 = q.x * q.w;
	result->m[2][1] = 2.0f * (tmp1 + tmp2);
	result->m[1][2] = 2.0f * (tmp1 - tmp2);

	result->m[0][3] = p.x;
	result->m[1][3] = p.y;
	result->m[2][3] = p.z;
}

void commit(std::vector<vr::HmdVector2_t> &points, const vr::HmdMatrix34_t &transform, const TrackingVector2 &playArea)
{
	vr::EVRInitError error;
	vr::VR_Init(&error, vr::VRApplication_Utility);

	if (error != vr::VRInitError_None) {
		Warn("Failed to init OpenVR client to update Chaperone boundary! Error: %d", error);
		return;
	}

	vr::VRChaperoneSetup()->RoomSetupStarting();

	vr::VRChaperoneSetup()->SetWorkingPerimeter(&points[0], points.size());
	vr::VRChaperoneSetup()->SetWorkingStandingZeroPoseToRawTrackingPose(&transform);
	vr::VRChaperoneSetup()->SetWorkingSeatedZeroPoseToRawTrackingPose(&transform);
	vr::VRChaperoneSetup()->SetWorkingPlayAreaSize(playArea.x, playArea.y);
	vr::VRChaperoneSetup()->CommitWorkingCopy(vr::EChaperoneConfigFile_Live);

	// Hide SteamVR Chaperone
	vr::VRSettings()->SetFloat(vr::k_pch_CollisionBounds_Section, vr::k_pch_CollisionBounds_FadeDistance_Float, 0.0f);

	vr::VR_Shutdown();
}

ChaperoneUpdater::ChaperoneUpdater()
{
	m_Transform = std::make_shared<vr::HmdMatrix34_t>();

	this->Start();
}

ChaperoneUpdater::~ChaperoneUpdater()
{
	m_Exiting = true;
	m_ChaperoneDataReady.Set();
}

void ChaperoneUpdater::ResetData(uint64_t timestamp, uint32_t pointCount)
{
	m_ChaperoneDataReady.Reset();
	m_Timestamp = timestamp;
	m_TotalPointCount = pointCount;

	std::unique_lock<std::mutex> chapDataLock(m_ChaperoneDataMtx);
	m_ChaperonePoints.clear();
	m_ChaperonePoints.resize(pointCount);
	chapDataLock.unlock();

	m_SegmentCount = pointCount / ALVR_GUARDIAN_SEGMENT_SIZE;
	if (pointCount % ALVR_GUARDIAN_SEGMENT_SIZE > 0)
	{
		m_SegmentCount++;
	}

	m_CommitDone = false;
}

void ChaperoneUpdater::SetTransform(const TrackingVector3 &position, const TrackingQuat& rotation, const TrackingVector2& playAreaSize)
{
	MakeTransformFromVecQuat(position, rotation, m_Transform);
	m_PlayArea = playAreaSize;
}

void ChaperoneUpdater::SetSegment(uint32_t segmentIndex, const TrackingVector3* points)
{
	int actualPointCount;

	if (segmentIndex >= m_TotalPointCount / ALVR_GUARDIAN_SEGMENT_SIZE) {
		actualPointCount = m_TotalPointCount % ALVR_GUARDIAN_SEGMENT_SIZE;
	}
	else {
		actualPointCount = ALVR_GUARDIAN_SEGMENT_SIZE;
	}

	int segmentStart = segmentIndex * ALVR_GUARDIAN_SEGMENT_SIZE;

	for (int i = 0; i < actualPointCount; ++i) {
		m_ChaperonePoints[segmentStart + i].v[0] = points[i].x;
		m_ChaperonePoints[segmentStart + i].v[1] = points[i].z;
	}
}

void ChaperoneUpdater::GenerateStandingChaperone(float scale)
{
	m_ChaperoneDataReady.Reset();
	m_TotalPointCount = ALVR_STANDING_CHAPERONE_POINT_COUNT;

	std::unique_lock<std::mutex> chapDataLock(m_ChaperoneDataMtx);
	m_ChaperonePoints.clear();
	m_ChaperonePoints.resize(m_TotalPointCount);
	chapDataLock.unlock();

	for (uint32_t i = 0; i < m_TotalPointCount; ++i) {
		float x = i * 2.0f * (float)M_PI / m_TotalPointCount;
		m_ChaperonePoints[i] = { (cosf(x) * scale), (sinf(x) * scale) };
	}

	m_PlayArea = { scale, scale };
}

bool ChaperoneUpdater::MaybeCommitData()
{
	if (m_CommitDone) {
		return false;
	}

	// defer the actual commit to a separate thread
	m_ChaperoneDataReady.Set();

	m_CommitDone = true;
	return true;
}

void ChaperoneUpdater::Run()
{
	while (!m_Exiting)
	{
		m_ChaperoneDataReady.Wait();
		if (m_Exiting) {
			break;
		}

		// Make a copy so the main thread can start filling new data if needed.
		std::unique_lock<std::mutex> chapDataLock(m_ChaperoneDataMtx);
		std::vector<vr::HmdVector2_t> points(m_ChaperonePoints);
		vr::HmdMatrix34_t transform(*m_Transform);
		TrackingVector2 playArea(m_PlayArea);
		chapDataLock.unlock();

		commit(points, transform, playArea);
	}
}

uint64_t ChaperoneUpdater::GetDataTimestamp()
{
	return m_Timestamp;
}

uint32_t ChaperoneUpdater::GetTotalPointCount()
{
	return m_TotalPointCount;
}

uint32_t ChaperoneUpdater::GetSegmentCount()
{
	return m_SegmentCount;
}
