#include "chaperone.h"
#include <algorithm>
#include <array>
#include <mutex>
#include <vector>
#ifndef __APPLE__
#include "openvr.h"
#endif

using namespace std;

void set_chaperone(AlvrVec2 bounds_rect) {
#ifndef __APPLE__
    const vr::HmdMatrix34_t MATRIX_IDENTITY = {
        {{1.0, 0.0, 0.0, 0.0}, {0.0, 1.0, 0.0, 0.0}, {0.0, 0.0, 1.0, 0.0}}};
    const uint32_t perimeterPointsCount = 4;

    static float perimeterPoints[perimeterPointsCount][2];
    perimeterPoints[0][0] = -bounds_rect.x / 2.0f;
    perimeterPoints[0][1] = -bounds_rect.y / 2.0f;
    perimeterPoints[1][0] = -bounds_rect.x / 2.0f;
    perimeterPoints[1][1] = bounds_rect.y / 2.0f;
    perimeterPoints[2][0] = bounds_rect.x / 2.0f;
    perimeterPoints[2][1] = bounds_rect.y / 2.0f;
    perimeterPoints[3][0] = bounds_rect.x / 2.0f;
    perimeterPoints[3][1] = -bounds_rect.y / 2.0f;

    vr::EVRInitError error;
    vr::VR_Init(&error, vr::VRApplication_Utility);

    if (error != vr::VRInitError_None) {
        auto message =
            std::string("Failed to init OpenVR client to update Chaperone boundary! Error: ") +
            to_string(error);
        alvr_error(message.c_str());
        return;
    }

    vr::VRChaperoneSetup()->RoomSetupStarting();

    vr::VRChaperoneSetup()->SetWorkingPerimeter(
        reinterpret_cast<vr::HmdVector2_t *>(perimeterPoints), perimeterPointsCount);
    vr::VRChaperoneSetup()->SetWorkingStandingZeroPoseToRawTrackingPose(&MATRIX_IDENTITY);
    vr::VRChaperoneSetup()->SetWorkingSeatedZeroPoseToRawTrackingPose(&MATRIX_IDENTITY);
    vr::VRChaperoneSetup()->SetWorkingPlayAreaSize(bounds_rect.x, bounds_rect.y);
    vr::VRChaperoneSetup()->CommitWorkingCopy(vr::EChaperoneConfigFile_Live);

    // Hide SteamVR Chaperone
    vr::VRSettings()->SetFloat(
        vr::k_pch_CollisionBounds_Section, vr::k_pch_CollisionBounds_FadeDistance_Float, 0.0f);

    vr::VR_Shutdown();
#endif
}