#pragma once

#include <stdint.h>

// Rust to C++:

extern "C" const uint8_t *FRAME_RENDER_VS_CSO_PTR;
extern "C" uint32_t FRAME_RENDER_VS_CSO_LEN;
extern "C" const uint8_t *FRAME_RENDER_PS_CSO_PTR;
extern "C" uint32_t FRAME_RENDER_PS_CSO_LEN;
extern "C" const uint8_t *QUAD_SHADER_CSO_PTR;
extern "C" uint32_t QUAD_SHADER_CSO_LEN;
extern "C" const uint8_t *COMPRESS_SLICES_CSO_PTR;
extern "C" uint32_t COMPRESS_SLICES_CSO_LEN;
extern "C" const uint8_t *COLOR_CORRECTION_CSO_PTR;
extern "C" uint32_t COLOR_CORRECTION_CSO_LEN;

extern "C" const char *g_alvrDir;

extern "C" void (*LogError)(const char *);
extern "C" void (*LogWarn)(const char *);
extern "C" void (*LogInfo)(const char *);
extern "C" void (*LogDebug)(const char *);
extern "C" void (*SendVideo)(uint64_t, uint8_t *, int, uint64_t);
extern "C" void (*SendAudio)(uint64_t, uint8_t *, int, uint64_t);
extern "C" void (*SendHapticsFeedback)(uint64_t, float, float, float, uint8_t);
extern "C" void (*ReportEncodeLatency)(uint64_t);
extern "C" void (*ShutdownRuntime)();

// C++ to Rust:

struct TrackingQuat
{
    float x;
    float y;
    float z;
    float w;
};
struct TrackingVector3
{
    float x;
    float y;
    float z;
};
struct TrackingVector2
{
    float x;
    float y;
};
struct TrackingInfo
{
    uint64_t clientTime;
    uint64_t FrameIndex;
    double predictedDisplayTime;
    TrackingQuat HeadPose_Pose_Orientation;
    TrackingVector3 HeadPose_Pose_Position;

    TrackingVector3 Other_Tracking_Source_Position;
    TrackingQuat Other_Tracking_Source_Orientation;

    static const uint32_t MAX_CONTROLLERS = 2;

    struct Controller
    {
        static const uint32_t FLAG_CONTROLLER_ENABLE = (1 << 0);
        static const uint32_t FLAG_CONTROLLER_LEFTHAND = (1 << 1); // 0: Left hand, 1: Right hand
        static const uint32_t FLAG_CONTROLLER_GEARVR = (1 << 2);
        static const uint32_t FLAG_CONTROLLER_OCULUS_GO = (1 << 3);
        static const uint32_t FLAG_CONTROLLER_OCULUS_QUEST = (1 << 4);
        static const uint32_t FLAG_CONTROLLER_OCULUS_HAND = (1 << 5);
        uint32_t flags;
        uint64_t buttons;

        struct
        {
            float x;
            float y;
        } trackpadPosition;

        float triggerValue;
        float gripValue;

        uint8_t batteryPercentRemaining;
        uint8_t recenterCount;

        // Tracking info of controller. (float * 19 = 76 bytes)
        TrackingQuat orientation;
        TrackingVector3 position;
        TrackingVector3 angularVelocity;
        TrackingVector3 linearVelocity;
        TrackingVector3 angularAcceleration;
        TrackingVector3 linearAcceleration;

        // Tracking info of hand. A3
        TrackingQuat boneRotations[19];
        //TrackingQuat boneRotationsBase[alvrHandBone_MaxSkinnable];
        TrackingVector3 bonePositionsBase[19];
        TrackingQuat boneRootOrientation;
        TrackingVector3 boneRootPosition;
        uint32_t inputStateStatus;
        float fingerPinchStrengths[4];
        uint32_t handFingerConfidences;
    } controller[2];
};

extern "C" void *CppEntryPoint(const char *, int *);
extern "C" void InitalizeStreaming();
extern "C" void UpdatePose(TrackingInfo);
extern "C" void HandlePacketLoss();
extern "C" void PlayMicAudio(uint8_t *, int);
extern "C" void UpdateChaperone(
    TrackingVector3 standingPosPosition,
	TrackingQuat standingPosRotation,
	TrackingVector2 playAreaSize,
	TrackingVector3 *points,
	int count);

extern "C" void ShutdownSteamvr();