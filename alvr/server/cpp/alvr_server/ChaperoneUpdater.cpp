#include "ChaperoneUpdater.h"
#include "ALVR-common/packet_types.h"
#include "Logger.h"
#include "bindings.h"
#include <algorithm>
#include <array>
#include <mutex>
#include <vector>

#ifndef __APPLE__
#include <openvr.h>
#endif

using namespace std;

static std::array<float, 12> zero_to_raw;
static std::mutex chaperone_mutex;

void SetChaperone(float areaWidth, float areaHeight) {
    float transform[12] = {1, 0, 0, 0, 0, 1, 0, 0, 0, 0, 1, 0};

    float perimeterPoints[4][2];

#ifndef __APPLE__
    perimeterPoints[0][0] = -1.0f * areaWidth;
    perimeterPoints[0][1] = -1.0f * areaHeight;
    perimeterPoints[1][0] = -1.0f * areaWidth;
    perimeterPoints[1][1] = 1.0f * areaHeight;
    perimeterPoints[2][0] = 1.0f * areaWidth;
    perimeterPoints[2][1] = 1.0f * areaHeight;
    perimeterPoints[3][0] = 1.0f * areaWidth;
    perimeterPoints[3][1] = -1.0f * areaHeight;

    std::unique_lock<std::mutex> lock(chaperone_mutex);

    vr::EVRInitError error;
    vr::VR_Init(&error, vr::VRApplication_Utility);

    if (error != vr::VRInitError_None) {
        Warn("Failed to init OpenVR client to update Chaperone boundary! Error: %d", error);
        return;
    }

    vr::VRChaperoneSetup()->RoomSetupStarting();

    std::copy(transform, transform + 12, zero_to_raw.begin());

    vr::VRChaperoneSetup()->SetWorkingPerimeter(
        reinterpret_cast<vr::HmdVector2_t *>(perimeterPoints), 4);
    vr::VRChaperoneSetup()->SetWorkingStandingZeroPoseToRawTrackingPose(
        reinterpret_cast<vr::HmdMatrix34_t *>(&transform));
    vr::VRChaperoneSetup()->SetWorkingSeatedZeroPoseToRawTrackingPose(
        reinterpret_cast<vr::HmdMatrix34_t *>(&transform));
    vr::VRChaperoneSetup()->SetWorkingPlayAreaSize(areaWidth, areaHeight);
    vr::VRChaperoneSetup()->CommitWorkingCopy(vr::EChaperoneConfigFile_Live);

    // Hide SteamVR Chaperone
    vr::VRSettings()->SetFloat(
        vr::k_pch_CollisionBounds_Section, vr::k_pch_CollisionBounds_FadeDistance_Float, 0.0f);

    vr::VR_Shutdown();
#endif
}

#ifndef __APPLE__
float *ZeroToRawPose(bool force) {
    if (force) {
        std::unique_lock<std::mutex> lock(chaperone_mutex);
        vr::EVRInitError error;
        vr::VR_Init(&error, vr::VRApplication_Utility);
        vr::VRChaperoneSetup()->GetWorkingStandingZeroPoseToRawTrackingPose(
            reinterpret_cast<vr::HmdMatrix34_t *>(zero_to_raw.data()));
        vr::VR_Shutdown();
    }
    return zero_to_raw.data();
}
#endif