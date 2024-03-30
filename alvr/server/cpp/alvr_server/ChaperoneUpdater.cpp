#include "ALVR-common/packet_types.h"
#include "Logger.h"
#include "bindings.h"
#include <mutex>

#ifndef __APPLE__
// Workaround symbol clash in openvr.h / openvr_driver.h
namespace alvr_chaperone {
#include <openvr.h>
}
using namespace alvr_chaperone;
#endif

std::mutex chaperone_mutex;

bool isOpenvrInit = false;

void InitOpenvrClient() {
#ifndef __APPLE__
    std::unique_lock<std::mutex> lock(chaperone_mutex);

    if (isOpenvrInit) {
        return;
    }

    vr::EVRInitError error;
    // Background needed for VRCompositor()->GetTrackingSpace()
    vr::VR_Init(&error, vr::VRApplication_Background);

    if (error != vr::VRInitError_None) {
        Warn("Failed to init OpenVR client! Error: %d", error);
        return;
    }
    isOpenvrInit = true;
#endif
}

void ShutdownOpenvrClient() {
#ifndef __APPLE__
    std::unique_lock<std::mutex> lock(chaperone_mutex);

    if (!isOpenvrInit) {
        return;
    }

    isOpenvrInit = false;
    vr::VR_Shutdown();
#endif
}

bool IsOpenvrClientReady() {
    return isOpenvrInit;
}

void _SetChaperoneArea(float areaWidth, float areaHeight) {
#ifndef __APPLE__
    std::unique_lock<std::mutex> lock(chaperone_mutex);

    const vr::HmdMatrix34_t MATRIX_IDENTITY = {
        {{1.0, 0.0, 0.0, 0.0}, {0.0, 1.0, 0.0, 0.0}, {0.0, 0.0, 1.0, 0.0}}};

    float perimeterPoints[4][2];

    perimeterPoints[0][0] = -1.0f * areaWidth;
    perimeterPoints[0][1] = -1.0f * areaHeight;
    perimeterPoints[1][0] = -1.0f * areaWidth;
    perimeterPoints[1][1] = 1.0f * areaHeight;
    perimeterPoints[2][0] = 1.0f * areaWidth;
    perimeterPoints[2][1] = 1.0f * areaHeight;
    perimeterPoints[3][0] = 1.0f * areaWidth;
    perimeterPoints[3][1] = -1.0f * areaHeight;

    vr::VRChaperoneSetup()->SetWorkingPerimeter(
        reinterpret_cast<vr::HmdVector2_t *>(perimeterPoints), 4);
    vr::VRChaperoneSetup()->SetWorkingStandingZeroPoseToRawTrackingPose(&MATRIX_IDENTITY);
    vr::VRChaperoneSetup()->SetWorkingSeatedZeroPoseToRawTrackingPose(&MATRIX_IDENTITY);
    vr::VRChaperoneSetup()->SetWorkingPlayAreaSize(areaWidth, areaHeight);
    vr::VRChaperoneSetup()->CommitWorkingCopy(vr::EChaperoneConfigFile_Live);

    // Hide SteamVR Chaperone
    vr::VRSettings()->SetFloat(
        vr::k_pch_CollisionBounds_Section, vr::k_pch_CollisionBounds_FadeDistance_Float, 0.0f);
#endif
}

#ifdef __linux__
vr::HmdMatrix34_t GetInvZeroPose() {
    vr::HmdMatrix34_t mat;
    // revert pulls live into working copy
    vr::VRChaperoneSetup()->RevertWorkingCopy();
    if (vr::VRCompositor()->GetTrackingSpace() == vr::TrackingUniverseStanding) {
        vr::VRChaperoneSetup()->GetWorkingStandingZeroPoseToRawTrackingPose(&mat);
    } else {
        vr::VRChaperoneSetup()->GetWorkingSeatedZeroPoseToRawTrackingPose(&mat);
    }
    return mat;
}
#endif
