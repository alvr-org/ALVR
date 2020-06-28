#include "ChaperoneUpdater.h"
//#include "Logger.h"
#include <openvr.h>

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

ChaperoneUpdater::ChaperoneUpdater()
{
	m_Transform = std::make_shared<vr::HmdMatrix34_t>();

	this->Start();
}

ChaperoneUpdater::~ChaperoneUpdater()
{
	delete[] m_ChaperonePoints;
	m_Exiting = true;
	m_ChaperoneDataReady.Set();
}

void ChaperoneUpdater::ResetData(uint64_t timestamp, uint32_t pointCount)
{
	m_Timestamp = timestamp;
	m_TotalPointCount = pointCount;

	std::unique_lock<std::mutex> chapDataLock(m_ChaperoneDataMtx);
	delete[] m_ChaperonePoints;
	m_ChaperonePoints = new vr::HmdVector2_t[pointCount];
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

bool ChaperoneUpdater::MaybeCommitData()
{
	if (m_CommitDone) {
		return false;
	}

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

		vr::EVRInitError error;
		vr::VR_Init(&error, vr::VRApplication_Utility);

		if (error != vr::VRInitError_None) {
			//Error("Failed to init OpenVR client to update Chaperone boundary! Error: %d", error);
			// TODO: logging
			continue;
		}

		vr::VRChaperoneSetup()->RoomSetupStarting();

		std::unique_lock<std::mutex> chapDataLock(m_ChaperoneDataMtx);
		vr::VRChaperoneSetup()->SetWorkingPerimeter(m_ChaperonePoints, m_TotalPointCount);
		chapDataLock.unlock();

		vr::VRChaperoneSetup()->SetWorkingStandingZeroPoseToRawTrackingPose(m_Transform.get());
		vr::VRChaperoneSetup()->SetWorkingSeatedZeroPoseToRawTrackingPose(m_Transform.get());
		vr::VRChaperoneSetup()->SetWorkingPlayAreaSize(m_PlayArea.x, m_PlayArea.y);
		vr::VRChaperoneSetup()->CommitWorkingCopy(vr::EChaperoneConfigFile_Live);

		vr::VR_Shutdown();
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
