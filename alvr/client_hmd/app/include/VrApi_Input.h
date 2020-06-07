/************************************************************************************

Filename    :   VrApi_Input.h
Content     :   Input API
Created     :   Feb 9, 2016
Authors     :   Jonathan E. Wright
Language    :   C99

Copyright   :   Copyright (c) Facebook Technologies, LLC and its affiliates. All rights reserved.

*************************************************************************************/
#ifndef OVR_VrApi_Input_h
#define OVR_VrApi_Input_h

#include <stddef.h>
#include <stdint.h>
#include "VrApi_Config.h"
#include "VrApi_Types.h"

#if defined(__cplusplus)
extern "C" {
#endif

/// Describes button input types.
/// For the Gear VR Controller and headset, only the following ovrButton types are reported to the
/// application:
///
/// ovrButton_Back, ovrButton_A, ovrButton_Enter
///
/// ovrButton_Home, ovrButton_VolUp, ovrButtonVolDown and ovrButton_Back are system buttons that are
/// never reported to applications. ovrButton_Back button has system-level handling for long
/// presses, but application-level handling for short-presses. Since a short-press is determined by
/// the time interval between down and up events, the ovrButton_Back flag is only set when the back
/// button comes up in less than the short-press time (0.25 seconds). The ovrButton_Back flag always
/// signals a short press and will only remain set for a single frame.
typedef enum ovrButton_ {
    ovrButton_A = 0x00000001, // Set for trigger pulled on the Gear VR and Go Controllers
    ovrButton_B = 0x00000002,
    ovrButton_RThumb = 0x00000004,
    ovrButton_RShoulder = 0x00000008,

    ovrButton_X = 0x00000100,
    ovrButton_Y = 0x00000200,
    ovrButton_LThumb = 0x00000400,
    ovrButton_LShoulder = 0x00000800,

    ovrButton_Up = 0x00010000,
    ovrButton_Down = 0x00020000,
    ovrButton_Left = 0x00040000,
    ovrButton_Right = 0x00080000,
    ovrButton_Enter = 0x00100000, //< Set for touchpad click on the Go Controller, menu
                                  // button on Left Quest Controller
    ovrButton_Back = 0x00200000, //< Back button on the Go Controller (only set when
                                 // a short press comes up)
        ovrButton_GripTrigger = 0x04000000, //< grip trigger engaged
        ovrButton_Trigger = 0x20000000, //< Index Trigger engaged
        ovrButton_Joystick = 0x80000000, //< Click of the Joystick

    ovrButton_EnumSize = 0x7fffffff
} ovrButton;

/// Describes touch input types.
/// These values map to capacitive touch values and derived pose states
typedef enum ovrTouch_ {
    ovrTouch_A = 0x00000001, //< The A button has a finger resting on it.
    ovrTouch_B = 0x00000002, //< The B button has a finger resting on it.
    ovrTouch_X = 0x00000004, //< The X button has a finger resting on it.
    ovrTouch_Y = 0x00000008, //< The Y button has a finger resting on it.
    ovrTouch_TrackPad = 0x00000010, //< The TrackPad has a finger resting on it.
    ovrTouch_Joystick = 0x00000020, //< The Joystick has a finger resting on it.
    ovrTouch_IndexTrigger = 0x00000040, //< The Index Trigger has a finger resting on it.
    ovrTouch_ThumbUp =
        0x00000100, //< None of A, B, X, Y, or Joystick has a finger/thumb in proximity to it
    ovrTouch_IndexPointing = 0x00000200, //< The finger is sufficiently far away from the trigger to
                                         // not be considered in proximity to it.
    ovrTouch_BaseState =
        0x00000300, //< No buttons touched or in proximity.  finger pointing and thumb up.
    ovrTouch_LThumb = 0x00000400, //< The Left controller Joystick has a finger/thumb resting on it.
    ovrTouch_RThumb =
        0x00000800, //< The Right controller Joystick has a finger/thumb resting on it.
        ovrTouch_EnumSize
} ovrTouch;

/// Specifies which controller is connected; multiple can be connected at once.
typedef enum ovrControllerType_ {
    ovrControllerType_None = 0,
    ovrControllerType_Reserved0 = (1 << 0), //< LTouch in CAPI
    ovrControllerType_Reserved1 = (1 << 1), //< RTouch in CAPI
    ovrControllerType_TrackedRemote = (1 << 2),
        ovrControllerType_Gamepad = (1 << 4), // Deprecated, will be removed in a future release
    ovrControllerType_Hand = (1 << 5),

        ovrControllerType_EnumSize = 0x7fffffff
} ovrControllerType;

typedef uint32_t ovrDeviceID;

typedef enum ovrDeviceIdType_ { ovrDeviceIdType_Invalid = 0x7fffffff } ovrDeviceIdType;

/// This header starts all ovrInputCapabilities structures. It should only hold fields
/// that are common to all input controllers.
typedef struct ovrInputCapabilityHeader_ {
    ovrControllerType Type;

    /// A unique ID for the input device
    ovrDeviceID DeviceID;
} ovrInputCapabilityHeader;

/// Specifies capabilites of a controller
/// Note that left and right hand are non-exclusive (a two-handed controller could set both)
typedef enum ovrControllerCapabilities_ {
    ovrControllerCaps_HasOrientationTracking = 0x00000001,
    ovrControllerCaps_HasPositionTracking = 0x00000002,
    ovrControllerCaps_LeftHand = 0x00000004, //< Controller is configured for left hand
    ovrControllerCaps_RightHand = 0x00000008, //< Controller is configured for right hand

    ovrControllerCaps_ModelOculusGo = 0x00000010, //< Controller for Oculus Go devices

        ovrControllerCaps_HasAnalogIndexTrigger =
        0x00000040, //< Controller has an analog index trigger vs. a binary one
    ovrControllerCaps_HasAnalogGripTrigger =
        0x00000080, //< Controller has an analog grip trigger vs. a binary one
        ovrControllerCaps_HasSimpleHapticVibration =
        0x00000200, //< Controller supports simple haptic vibration
    ovrControllerCaps_HasBufferedHapticVibration =
        0x00000400, //< Controller supports buffered haptic vibration

    ovrControllerCaps_ModelGearVR = 0x00000800, //< Controller is the Gear VR Controller

    ovrControllerCaps_HasTrackpad = 0x00001000, //< Controller has a trackpad

    ovrControllerCaps_HasJoystick = 0x00002000, //< Controller has a joystick.
    ovrControllerCaps_ModelOculusTouch = 0x00004000, //< Oculus Touch Controller For Oculus Quest

    
    ovrControllerCaps_EnumSize = 0x7fffffff
} ovrControllerCapabilties;

//-----------------------------------------------------------------
// Tracked Remote Capabilities
//-----------------------------------------------------------------

/// Details about the Oculus Remote input device.
typedef struct ovrInputTrackedRemoteCapabilities_ {
    ovrInputCapabilityHeader Header;

    /// Mask of controller capabilities described by ovrControllerCapabilities
    uint32_t ControllerCapabilities;

    /// Mask of button capabilities described by ovrButton
    uint32_t ButtonCapabilities;

    /// Maximum coordinates of the Trackpad, bottom right exclusive
    /// For a 300x200 Trackpad, return 299x199
    uint16_t TrackpadMaxX;
    uint16_t TrackpadMaxY;

    /// Size of the Trackpad in mm (millimeters)
    float TrackpadSizeX;
    float TrackpadSizeY;

    /// added in API version 1.1.13.0
    /// Maximum submittable samples for the haptics buffer
    uint32_t HapticSamplesMax;
    /// length in milliseconds of a sample in the haptics buffer.
    uint32_t HapticSampleDurationMS;
    /// added in API version 1.1.15.0
    uint32_t TouchCapabilities;
    uint32_t Reserved4;
    uint32_t Reserved5;
} ovrInputTrackedRemoteCapabilities;

/// Capabilities for an XBox style game pad
OVR_VRAPI_DEPRECATED(typedef struct ovrInputGamepadCapabilities_ {
    ovrInputCapabilityHeader Header;

    /// Mask of controller capabilities described by ovrControllerCapabilities
    uint32_t ControllerCapabilities;

    /// Mask of button capabilities described by ovrButton
    uint32_t ButtonCapabilities;

    // Reserved for future use.
    uint64_t Reserved[20];
} ovrInputGamepadCapabilities);


/// The buffer data for playing haptics
typedef struct ovrHapticBuffer_ {
    /// Start time of the buffer
    double BufferTime;

    /// Number of samples in the buffer;
    uint32_t NumSamples;

    // True if this is the end of the buffers being sent
    bool Terminated;

    uint8_t* HapticBuffer;
} ovrHapticBuffer;

/// This header starts all ovrInputState structures. It should only hold fields
/// that are common to all input controllers.
typedef struct ovrInputStateHeader_ {
    /// Type type of controller
    ovrControllerType ControllerType;

    /// System time when the controller state was last updated.
    double TimeInSeconds;
} ovrInputStateHeader;

/// ovrInputStateTrackedRemote describes the complete input state for the
/// orientation-tracked remote. The TrackpadPosition coordinates returned
/// for the GearVR Controller are in raw, absolute units.
typedef struct ovrInputStateTrackedRemote_ {
    ovrInputStateHeader Header;

    /// Values for buttons described by ovrButton.
    uint32_t Buttons;

    /// Finger contact status for trackpad
    /// true = finger is on trackpad, false = finger is off trackpad
    uint32_t TrackpadStatus;

    /// X and Y coordinates of the Trackpad
    ovrVector2f TrackpadPosition;

    /// The percentage of max battery charge remaining.
    uint8_t BatteryPercentRemaining;
    /// Increments every time the remote is recentered. If this changes, the application may need
    /// to adjust its arm model accordingly.
    uint8_t RecenterCount;
    /// Reserved for future use.
    uint16_t Reserved;

    /// added in API version 1.1.13.0
    // Analog values from 0.0 - 1.0 of the pull of the triggers
    float IndexTrigger;
    float GripTrigger;

    /// added in API version 1.1.15.0
    uint32_t Touches;
    uint32_t Reserved5a;
    // Analog values from -1.0 - 1.0
    // The value is set to 0.0 on Joystick, if the magnitude of the vector is < 0.1f
    ovrVector2f Joystick;
    // JoystickNoDeadZone does change the raw values of the data.
    ovrVector2f JoystickNoDeadZone;

} ovrInputStateTrackedRemote;


/// ovrInputStateGamepad describes the input state gamepad input devices
OVR_VRAPI_DEPRECATED(typedef struct ovrInputStateGamepad_ {
    ovrInputStateHeader Header;

    /// Values for buttons described by ovrButton.
    uint32_t Buttons;

    // Analog value from 0.0 - 1.0 of the pull of the Left Trigger
    float LeftTrigger;
    // Analog value from 0.0 - 1.0 of the pull of the Right Trigger
    float RightTrigger;

    /// X and Y coordinates of the Left Joystick, -1.0 - 1.0
    ovrVector2f LeftJoystick;
    /// X and Y coordinates of the Right Joystick, -1.0 - 1.0
    ovrVector2f RightJoystick;

    // Reserved for future use.
    uint64_t Reserved[20];
} ovrInputStateGamepad);


//-----------------------------------------------------------------
// Hand tracking
//-----------------------------------------------------------------

/// Specifies left or right handedness.
typedef enum ovrHandedness_ {
    VRAPI_HAND_UNKNOWN = 0,
    VRAPI_HAND_LEFT = 1,
    VRAPI_HAND_RIGHT = 2
} ovrHandedness;

//-----------------------------------------------------------------
// Hand capabilities
typedef enum ovrHandCapabilities_ {
    ovrHandCaps_LeftHand = (1 << 0), // if set, this is the left hand
    ovrHandCaps_RightHand = (1 << 1), // if set, this is the right hand
    ovrHandCaps_EnumSize = 0x7fffffff
} ovrHandCapabilities;

typedef enum ovrHandStateCapabilities_ {
    ovrHandStateCaps_PinchIndex = (1 << 0), // if set, index finger pinch is supported
    ovrHandStateCaps_PinchMiddle = (1 << 1), // if set, middle finger pinch is supported
    ovrHandStateCaps_PinchRing = (1 << 2), // if set, ring finger pinch is supported
    ovrHandStateCaps_PinchPinky = (1 << 3), // if set, pinky finger pinch is supported
    ovrHandStateCaps_EnumSize = 0x7fffffff
} ovrHandStateCapabilities;

typedef struct ovrInputHandCapabilities_ {
    ovrInputCapabilityHeader Header;

    // Mask of hand capabilities described by ovrHandCapabilities
    uint32_t HandCapabilities;

    // Mask of hand state capabilities described by ovrInputHandStateCapabilities
    uint32_t StateCapabilities;
} ovrInputHandCapabilities;

typedef enum ovrHandTrackingStatus_ {
    ovrHandTrackingStatus_Untracked = 0, // not tracked
    ovrHandTrackingStatus_Tracked = 1, // tracked
    ovrHandTrackingStatus_EnumSize = 0x7fffffff
} ovrHandTrackingStatus;

//-----------------------------------------------------------------
// Hand state

typedef enum ovrHandFingers_ {
    ovrHandFinger_Thumb = 0,
    ovrHandFinger_Index = 1,
    ovrHandFinger_Middle = 2,
    ovrHandFinger_Ring = 3,
    ovrHandFinger_Pinky = 4,
    ovrHandFinger_Max,
    ovrHandFinger_EnumSize = 0x7fffffff
} ovrHandFingers;

typedef enum ovrHandPinchStrength_ {
    ovrHandPinchStrength_Index = 0, // hand is in the index finger pinch state
    ovrHandPinchStrength_Middle = 1, // hand is in the middle finger pinch state
    ovrHandPinchStrength_Ring = 2, // hand is in the ring finger pinch state
    ovrHandPinchStrength_Pinky = 3, // hand is in the pinky finger pinch state
    ovrHandPinchStrength_Max = 4,
    ovrHandPinchStrength_EnumSize = 0x7fffffff
} ovrHandPinchStrength;

typedef int16_t ovrVertexIndex;

typedef enum ovrHandBone_ {
    ovrHandBone_Invalid = -1,
    ovrHandBone_WristRoot = 0, // root frame of the hand, where the wrist is located
    ovrHandBone_ForearmStub = 1, // frame for user's forearm
    ovrHandBone_Thumb0 = 2, // thumb trapezium bone
    ovrHandBone_Thumb1 = 3, // thumb metacarpal bone
    ovrHandBone_Thumb2 = 4, // thumb proximal phalange bone
    ovrHandBone_Thumb3 = 5, // thumb distal phalange bone
    ovrHandBone_Index1 = 6, // index proximal phalange bone
    ovrHandBone_Index2 = 7, // index intermediate phalange bone
    ovrHandBone_Index3 = 8, // index distal phalange bone
    ovrHandBone_Middle1 = 9, // middle proximal phalange bone
    ovrHandBone_Middle2 = 10, // middle intermediate phalange bone
    ovrHandBone_Middle3 = 11, // middle distal phalange bone
    ovrHandBone_Ring1 = 12, // ring proximal phalange bone
    ovrHandBone_Ring2 = 13, // ring intermediate phalange bone
    ovrHandBone_Ring3 = 14, // ring distal phalange bone
    ovrHandBone_Pinky0 = 15, // pinky metacarpal bone
    ovrHandBone_Pinky1 = 16, // pinky proximal phalange bone
    ovrHandBone_Pinky2 = 17, // pinky intermediate phalange bone
    ovrHandBone_Pinky3 = 18, // pinky distal phalange bone
        ovrHandBone_MaxSkinnable = 19,

    // Bone tips are position only. They are not used for skinning but useful for hit-testing.
    // NOTE: ovrHandBone_ThumbTip == ovrHandBone_MaxSkinnable since the extended tips need to be
    // contiguous
    ovrHandBone_ThumbTip = ovrHandBone_MaxSkinnable + 0, // tip of the thumb
    ovrHandBone_IndexTip = ovrHandBone_MaxSkinnable + 1, // tip of the index finger
    ovrHandBone_MiddleTip = ovrHandBone_MaxSkinnable + 2, // tip of the middle finger
    ovrHandBone_RingTip = ovrHandBone_MaxSkinnable + 3, // tip of the ring finger
    ovrHandBone_PinkyTip = ovrHandBone_MaxSkinnable + 4, // tip of the pinky
    ovrHandBone_Max = ovrHandBone_MaxSkinnable + 5,
    ovrHandBone_EnumSize = 0x7fff
} ovrHandBone;
typedef int16_t ovrHandBoneIndex;

typedef enum ovrConfidence_ {
    ovrConfidence_LOW = 0x00000000,
    ovrConfidence_HIGH = 0x3f800000
} ovrConfidence;

/// Unified version struct
typedef enum ovrHandVersion_ {
    ovrHandVersion_1 = 0xdf000001, /// Current

    
    ovrHandVersion_EnumSize = 0x7fffffff
} ovrHandVersion;

// ovrBoneCapsule
//    _---_
//  -"     "-
// /         \
// |----A----|
// |    |    |
// |    |    |
// |    |-r->|
// |    |    |
// |    |    |
// |----B----|
// \         /
//  -.     .-
//    '---'
typedef struct ovrBoneCapsule_ {
    // Index of the bone this capsule is on.
    ovrHandBoneIndex BoneIndex;
    // Points at either end of the cylinder inscribed in the capsule. Also the center points for
    // spheres at either end of the capsule. Points A and B in the diagram above.
    ovrVector3f Points[2];
    // The radius of the capsule cylinder and of the half-sphere caps on the ends of the capsule.
    float Radius;
} ovrBoneCapsule;

typedef enum ovrHandConstants_ {
    ovrHand_MaxVertices = 3000,
    ovrHand_MaxIndices = ovrHand_MaxVertices * 6,
    ovrHand_MaxFingers = ovrHandFinger_Max,
    ovrHand_MaxPinchStrengths = ovrHandPinchStrength_Max,
    ovrHand_MaxSkinnableBones = ovrHandBone_MaxSkinnable,
    ovrHand_MaxBones = ovrHandBone_Max,
    ovrHand_MaxCapsules = 19,
        ovrHand_EnumSize = 0x7fffffff
} ovrHandConstants;

typedef enum ovrInputStateHandStatus_ {
    ovrInputStateHandStatus_PointerValid =
        (1 << 1), // if this is set the PointerPose and PinchStrength contain valid data, otherwise
                  // they should not be used.
    ovrInputStateHandStatus_IndexPinching =
        (1 << 2), // if this is set the pinch gesture for that finger is on
    ovrInputStateHandStatus_MiddlePinching =
        (1 << 3), // if this is set the pinch gesture for that finger is on
    ovrInputStateHandStatus_RingPinching =
        (1 << 4), // if this is set the pinch gesture for that finger is on
    ovrInputStateHandStatus_PinkyPinching =
        (1 << 5), // if this is set the pinch gesture for that finger is on
    ovrInputStateHandStatus_SystemGestureProcessing =
        (1 << 6), // if this is set the hand is currently processing a system gesture
    ovrInputStateHandStatus_DominantHand =
        (1 << 7), // if this is set the hand is considered the dominant hand
    ovrInputStateHandStatus_MenuPressed =
        (1 << 8), // if this is set the hand performed the system gesture as the non-dominant hand
    ovrInputStateHandStatus_EnumSize = 0x7fffffff
} ovrInputStateHandStatus;

// Pass this structure to vrapi_GetCurrentInputState() with a device id for a hand to get the
// current, second-order state of the hand.
typedef struct ovrInputStateHand_ {
    ovrInputStateHeader Header;

    // For each pinch type, indicates how far the fingers are into that pinch state. Range 0.0
    // to 1.0, where 1.0 is fully pinching. Indexable via the ovrHandPinchStrength enums.
    float PinchStrength[ovrHandPinchStrength_Max];

    // World space position and orientation of the pointer attached to the hand. This describes
    // a pointing ray useful for UI interactions.
    // Note that the pointer pose is not valid unless the ovrInputStateHandStatus_PointerValid flag
    // is set in the InputStateStatus field.
    ovrPosef PointerPose;

    // Status flags for this hand's input state. Mask of ovrInputStateHandStatus flags.
    uint32_t InputStateStatus;
} ovrInputStateHand;

//-----------------------------------------------------------------
// Hand pose

// Header for all hand pose structures.
typedef struct ovrHandPoseHeader_ {
    // The version number of the Pose structure.
    // When requesting a pose with vrapi_GetHandPose this MUST be set to the proper version.
    // If this is not set to a known version, or if the version it is set to is no longer
    // supported for the current SDK, ovr_GetHand* functions will return ovrError_InvalidParameter.
    ovrHandVersion Version;

    /// Reserved for later use
    double Reserved;
} ovrHandPoseHeader;

// Pass this structure to vrapi_GetHandPose() to get the pose of the hand at a particular time.
typedef struct ovrHandPose_ {
    ovrHandPoseHeader Header;

    // Status of tracking for this pose. This is not a bit field, but an exclusive state.
    ovrHandTrackingStatus Status;

    // Root pose of the hand in world space. Not to be confused with the root bone's transform.
    // The root bone can still be offset from this by the skeleton's rest pose.
    ovrPosef RootPose;

    // Current rotation of each bone.
    ovrQuatf BoneRotations[ovrHandBone_Max];

    // Time stamp for the pose that was requested in global system time.
    double RequestedTimeStamp;

    // Time stamp of the captured sample that the pose was extrapolated from.
    double SampleTimeStamp;

    // Tracking confidence.
    // This is the amount of confidence that the system has that the entire hand pose is correct.
    ovrConfidence HandConfidence;

    // Scale of the hand relative to the original hand model. This value may change at any time
    // based on the size of the hand being tracked. The default is 1.0.
    float HandScale;

    // Per-finger tracking confidence.
    // This is the amount of confidence the system has that the individual finger poses are correct.
    ovrConfidence FingerConfidences[ovrHandFinger_Max];
} ovrHandPose;


OVR_VRAPI_EXPORT ovrResult vrapi_GetHandPose(
    ovrMobile* ovr,
    const ovrDeviceID deviceID,
    const double absTimeInSeconds,
    ovrHandPoseHeader* header);

//-----------------------------------------------------------------
// Hand skeleton

// Header for all mesh structures.
typedef struct ovrHandSkeletonHeader_ {
    // The version number of the skeleton structure.
    ovrHandVersion Version;
} ovrHandSkeletonHeader;

typedef struct ovrHandSkeleton_V1_ {
    // Version of the mesh structure.
    ovrHandSkeletonHeader Header;

    // The number of bones in this skeleton.
    uint32_t NumBones;

    // The number of capsules on this skeleton.
    uint32_t NumCapsules;

    // reserved for future use
    uint32_t Reserved[5];

    // An array of count NumBones transforms for each bone in local (parent) space.
    ovrPosef BonePoses[ovrHand_MaxBones];

    // An array of count NumBones indicating the parent bone index for each bone.
    ovrHandBoneIndex BoneParentIndices[ovrHand_MaxBones];

    // An array of count NumCapsules ovrHandCapsules. Note that the number of capsules
    // is not necessarily the same as the number of bones.
    ovrBoneCapsule Capsules[ovrHand_MaxCapsules];
} ovrHandSkeleton;


OVR_VRAPI_EXPORT ovrResult vrapi_GetHandSkeleton(
    ovrMobile* ovr,
    const ovrHandedness handedness,
    ovrHandSkeletonHeader* header);

//-----------------------------------------------------------------
// Hand mesh

// Header for all mesh structures.
typedef struct ovrHandMeshHeader_ {
    // The version number of the mesh structure.
    ovrHandVersion Version;
} ovrHandMeshHeader;

typedef struct ovrHandMesh_V1_ {
    // All mesh structures will start with this header and the version.
    ovrHandMeshHeader Header;

    // Number of unique vertices in the mesh.
    uint32_t NumVertices;
    // Number of unique indices in the mesh.
    uint32_t NumIndices;

    // Reserved for future use
    uint32_t Reserved[13];

    // An array of count NumVertices positions for each vertex.
    ovrVector3f VertexPositions[ovrHand_MaxVertices];
    // An array of count NumIndices of vertex indices specifying triangles that make up the mesh.
    ovrVertexIndex Indices[ovrHand_MaxIndices];
    // An array of count NumVertices of normals for each vertex.
    ovrVector3f VertexNormals[ovrHand_MaxVertices];
    // An array of count NumVertices of texture coordinates for each vertex.
    ovrVector2f VertexUV0[ovrHand_MaxVertices];
    // An array of count NumVertices of blend indices for each of the bones that each vertex is
    // weighted to. Always valid. An index of < 0 means no blend weight.
    ovrVector4s BlendIndices[ovrHand_MaxVertices];
    // An array of count NumVertices of weights for each of the bones affecting each vertex.
    ovrVector4f BlendWeights[ovrHand_MaxVertices];
} ovrHandMesh;

OVR_VRAPI_EXPORT ovrResult
vrapi_GetHandMesh(ovrMobile* ovr, const ovrHandedness handedness, ovrHandMeshHeader* header);


/// Enumerates the input devices connected to the system
/// Start with index=0 and counting up. Stop when ovrResult is < 0
///
/// Input: ovrMobile, device index, and a capabilities header
/// The capabilities header does not need to have any fields set before calling.
/// Output: capabilitiesHeader with information for that enumeration index
OVR_VRAPI_EXPORT ovrResult vrapi_EnumerateInputDevices(
    ovrMobile* ovr,
    const uint32_t index,
    ovrInputCapabilityHeader* capsHeader);

/// Returns the capabilities of the input device for the corresponding device ID
///
/// Input: ovr, pointer to a capabilities structure
/// Output: capabilities will be filled with information for the deviceID
/// Example:
///     The Type field of the capabilitiesHeader must be set when calling this function.
///     Normally the capabilitiesHeader is obtained from the vrapi_EnumerateInputDevices API
///     The Type field in the header should match the structure type that is passed.
///
///         ovrInputCapabilityHeader capsHeader;
///         if ( vrapi_EnumerateInputDevices( ovr, deviceIndex, &capsHeader ) >= 0 ) {
///             if ( capsHeader.Type == ovrDeviceType_TrackedRemote ) {
///                 ovrInputTrackedRemoteCapabilities remoteCaps;
///                 remoteCaps.Header = capsHeader;
///                 vrapi_GetInputDeviceCapabilities( ovr, &remoteCaps.Header );
OVR_VRAPI_EXPORT ovrResult
vrapi_GetInputDeviceCapabilities(ovrMobile* ovr, ovrInputCapabilityHeader* capsHeader);

/// Sets the vibration level of a haptic device.
/// there should only be one call to vrapi_SetHapticVibrationSimple or
/// vrapi_SetHapticVibrationBuffer per frame
///  additional calls of either will return ovrError_InvalidOperation and have undefined behavior
/// Input: ovr, deviceID, intensity: 0.0 - 1.0
OVR_VRAPI_EXPORT ovrResult
vrapi_SetHapticVibrationSimple(ovrMobile* ovr, const ovrDeviceID deviceID, const float intensity);

/// Fills the haptic vibration buffer of a haptic device
/// there should only be one call to vrapi_SetHapticVibrationSimple or
/// vrapi_SetHapticVibrationBuffer per frame
///  additional calls of either will return ovrError_InvalidOperation and have undefined behavior
/// Input: ovr, deviceID, pointer to a hapticBuffer with filled in data.
OVR_VRAPI_EXPORT ovrResult vrapi_SetHapticVibrationBuffer(
    ovrMobile* ovr,
    const ovrDeviceID deviceID,
    const ovrHapticBuffer* hapticBuffer);


/// Returns the current input state for controllers, without positional tracking info.
///
/// Input: ovr, deviceID, pointer to a capabilities structure (with Type field set)
/// Output: Upon return the inputState structure will be set to the device's current input state
/// Example:
///     The Type field of the passed ovrInputStateHeader must be set to the type that
///     corresponds to the type of structure being passed.
///     The pointer to the ovrInputStateHeader should be a pointer to a Header field in
///     structure matching the value of the Type field.
///
///     ovrInputStateTrackedRemote state;
///     state.Header.Type = ovrControllerType_TrackedRemote;
///     if ( vrapi_GetCurrentInputState( ovr, remoteDeviceID, &state.Header ) >= 0 ) {
OVR_VRAPI_EXPORT ovrResult vrapi_GetCurrentInputState(
    ovrMobile* ovr,
    const ovrDeviceID deviceID,
    ovrInputStateHeader* inputState);


/// Returns the predicted input state based on the specified absolute system time
/// in seconds. Pass absTime value of 0.0 to request the most recent sensor reading.
/// Input: ovr, device ID, prediction time
/// Output: ovrTracking structure containing the device's predicted tracking state.
OVR_VRAPI_EXPORT ovrResult vrapi_GetInputTrackingState(
    ovrMobile* ovr,
    const ovrDeviceID deviceID,
    const double absTimeInSeconds,
    ovrTracking* tracking);

/// Can be called from any thread while in VR mode. Recenters the tracked remote to the current yaw
/// of the headset. Input: ovr, device ID Output: None
OVR_VRAPI_DEPRECATED(
    OVR_VRAPI_EXPORT void vrapi_RecenterInputPose(ovrMobile* ovr, const ovrDeviceID deviceID));

/// Enable or disable emulation for the GearVR Controller.
/// Emulation is false by default.
/// If emulationOn == true, then the back button and touch events on the GearVR Controller will be
/// sent through the Android dispatchKeyEvent and dispatchTouchEvent path as if they were from the
/// headset back button and touchpad. Applications that are intentionally enumerating the controller
/// will likely want to turn emulation off in order to differentiate between controller and headset
/// input events.
OVR_VRAPI_EXPORT ovrResult vrapi_SetRemoteEmulation(ovrMobile* ovr, const bool emulationOn);

#if defined(__cplusplus)
} // extern "C"
#endif

#endif // OVR_VrApi_Input_h
