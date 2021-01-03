#include "bindings.h"
#include "Logger.h"
#include "packet_types.h"
#include <vector>
#include <openvr.h>

using namespace std;

void SetChaperone(const float transform[12], float areaWidth, float areaHeight,
				  float (*perimeterPoints)[2], unsigned int perimeterPointsCount)
{
	if (perimeterPointsCount == 0)
	{
		areaWidth = 2.0f;
		areaHeight = 2.0f;

		static float standingPerimeterPointsBuffer[4][2];
		standingPerimeterPointsBuffer[0][0] = -1.0f;
		standingPerimeterPointsBuffer[0][1] = -1.0f;
		standingPerimeterPointsBuffer[1][0] = -1.0f;
		standingPerimeterPointsBuffer[1][1] = 1.0f;
		standingPerimeterPointsBuffer[2][0] = 1.0f;
		standingPerimeterPointsBuffer[2][1] = 1.0f;
		standingPerimeterPointsBuffer[3][0] = 1.0f;
		standingPerimeterPointsBuffer[3][1] = -1.0f;

		perimeterPoints = standingPerimeterPointsBuffer;
		perimeterPointsCount = 4;
	}

	vr::EVRInitError error;
	vr::VR_Init(&error, vr::VRApplication_Utility);

	if (error != vr::VRInitError_None)
	{
		Warn("Failed to init OpenVR client to update Chaperone boundary! Error: %d", error);
		return;
	}

	vr::VRChaperoneSetup()->RoomSetupStarting();

	vr::VRChaperoneSetup()->SetWorkingPerimeter(reinterpret_cast<vr::HmdVector2_t *>(perimeterPoints), perimeterPointsCount);
	vr::VRChaperoneSetup()->SetWorkingStandingZeroPoseToRawTrackingPose(reinterpret_cast<vr::HmdMatrix34_t *>(&transform));
	vr::VRChaperoneSetup()->SetWorkingSeatedZeroPoseToRawTrackingPose(reinterpret_cast<vr::HmdMatrix34_t *>(&transform));
	vr::VRChaperoneSetup()->SetWorkingPlayAreaSize(areaWidth, areaHeight);
	vr::VRChaperoneSetup()->CommitWorkingCopy(vr::EChaperoneConfigFile_Live);

	// Hide SteamVR Chaperone
	vr::VRSettings()->SetFloat(vr::k_pch_CollisionBounds_Section, vr::k_pch_CollisionBounds_FadeDistance_Float, 0.0f);

	vr::VR_Shutdown();
}

void SetDefaultChaperone()
{
	float transform[12] = {1, 0, 0, 0,
						   0, 1, 0, 1.5,
						   0, 0, 1, 0};

	SetChaperone(transform, 0, 0, nullptr, 0);
}