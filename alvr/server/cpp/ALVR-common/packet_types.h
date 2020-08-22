#ifndef ALVRCLIENT_PACKETTYPES_H
#define ALVRCLIENT_PACKETTYPES_H
#include <stdint.h>
#include <assert.h>

// Maximum UDP packet size (payload size in bytes)
static const int ALVR_MAX_PACKET_SIZE = 1400;
static const int ALVR_REFRESH_RATE_LIST_SIZE = 4;

// Maximum UDP packet size
static const int MAX_PACKET_UDP_PACKET_SIZE = 2000;

static const char *ALVR_HELLO_PACKET_SIGNATURE = "ALVR";

// Guardian syncing constants
static const int ALVR_GUARDIAN_SEGMENT_SIZE = 100;

enum ALVR_CODEC {
	ALVR_CODEC_H264 = 0,
	ALVR_CODEC_H265 = 1,
};

enum ALVR_DEVICE_TYPE {
	ALVR_DEVICE_TYPE_UNKNOWN = 0,
	ALVR_DEVICE_TYPE_OCULUS_MOBILE = 1,
	ALVR_DEVICE_TYPE_DAYDREAM = 2,
	ALVR_DEVICE_TYPE_CARDBOARD = 3,
};

enum ALVR_DEVICE_SUB_TYPE {
	ALVR_DEVICE_SUBTYPE_OCULUS_MOBILE_GEARVR = 1,
	ALVR_DEVICE_SUBTYPE_OCULUS_MOBILE_GO = 2,
	ALVR_DEVICE_SUBTYPE_OCULUS_MOBILE_QUEST = 3,

	ALVR_DEVICE_SUBTYPE_DAYDREAM_GENERIC = 1,
	ALVR_DEVICE_SUBTYPE_DAYDREAM_MIRAGE_SOLO = 2,

	ALVR_DEVICE_SUBTYPE_CARDBOARD_GENERIC = 1,
};

enum ALVR_DEVICE_CAPABILITY_FLAG {
	ALVR_DEVICE_CAPABILITY_FLAG_HMD_6DOF = 1 << 0,
};

enum ALVR_CONTROLLER_CAPABILITY_FLAG {
	ALVR_CONTROLLER_CAPABILITY_FLAG_ONE_CONTROLLER = 1 << 0,
	ALVR_CONTROLLER_CAPABILITY_FLAG_TWO_CONTROLLERS = 1 << 1,
	ALVR_CONTROLLER_CAPABILITY_FLAG_6DOF = 1 << 2,
};

enum ALVR_INPUT {
	ALVR_INPUT_SYSTEM_CLICK,
	ALVR_INPUT_APPLICATION_MENU_CLICK,
	ALVR_INPUT_GRIP_CLICK,
	ALVR_INPUT_GRIP_VALUE,
	ALVR_INPUT_GRIP_TOUCH,
	ALVR_INPUT_DPAD_LEFT_CLICK,
	ALVR_INPUT_DPAD_UP_CLICK,
	ALVR_INPUT_DPAD_RIGHT_CLICK,
	ALVR_INPUT_DPAD_DOWN_CLICK,
	ALVR_INPUT_A_CLICK,
	ALVR_INPUT_A_TOUCH,
	ALVR_INPUT_B_CLICK,
	ALVR_INPUT_B_TOUCH,
	ALVR_INPUT_X_CLICK,
	ALVR_INPUT_X_TOUCH,
	ALVR_INPUT_Y_CLICK,
	ALVR_INPUT_Y_TOUCH,
	ALVR_INPUT_TRIGGER_LEFT_VALUE,
	ALVR_INPUT_TRIGGER_RIGHT_VALUE,
	ALVR_INPUT_SHOULDER_LEFT_CLICK,
	ALVR_INPUT_SHOULDER_RIGHT_CLICK,
	ALVR_INPUT_JOYSTICK_LEFT_CLICK,
	ALVR_INPUT_JOYSTICK_LEFT_X,
	ALVR_INPUT_JOYSTICK_LEFT_Y,
	ALVR_INPUT_JOYSTICK_RIGHT_CLICK,
	ALVR_INPUT_JOYSTICK_RIGHT_X,
	ALVR_INPUT_JOYSTICK_RIGHT_Y,
	ALVR_INPUT_JOYSTICK_CLICK,
	ALVR_INPUT_JOYSTICK_X,
	ALVR_INPUT_JOYSTICK_Y,
	ALVR_INPUT_JOYSTICK_TOUCH,
	ALVR_INPUT_BACK_CLICK,
	ALVR_INPUT_GUIDE_CLICK,
	ALVR_INPUT_START_CLICK,
	ALVR_INPUT_TRIGGER_CLICK,
	ALVR_INPUT_TRIGGER_VALUE,
	ALVR_INPUT_TRIGGER_TOUCH,
	ALVR_INPUT_TRACKPAD_X,
	ALVR_INPUT_TRACKPAD_Y,
	ALVR_INPUT_TRACKPAD_CLICK,
	ALVR_INPUT_TRACKPAD_TOUCH,

	ALVR_INPUT_FINGER_INDEX,
	ALVR_INPUT_FINGER_MIDDLE,
	ALVR_INPUT_FINGER_RING,
	ALVR_INPUT_FINGER_PINKY,
	ALVR_INPUT_GRIP_FORCE,
	ALVR_INPUT_TRACKPAD_FORCE,

	ALVR_INPUT_MAX = ALVR_INPUT_TRACKPAD_FORCE,
	ALVR_INPUT_COUNT = ALVR_INPUT_MAX + 1
};
enum ALVR_HAND {
	alvrHandBone_Invalid = -1,
	alvrHandBone_WristRoot = 0,	// root frame of the hand, where the wrist is located
	alvrHandBone_ForearmStub = 1,	// frame for user's forearm
	alvrHandBone_Thumb0 = 2,	// thumb trapezium bone
	alvrHandBone_Thumb1 = 3,	// thumb metacarpal bone
	alvrHandBone_Thumb2 = 4,	// thumb proximal phalange bone
	alvrHandBone_Thumb3 = 5,	// thumb distal phalange bone
	alvrHandBone_Index1 = 6,	// index proximal phalange bone
	alvrHandBone_Index2 = 7,	// index intermediate phalange bone
	alvrHandBone_Index3 = 8,	// index distal phalange bone
	alvrHandBone_Middle1 = 9,	// middle proximal phalange bone
	alvrHandBone_Middle2 = 10,	// middle intermediate phalange bone
	alvrHandBone_Middle3 = 11,	// middle distal phalange bone
	alvrHandBone_Ring1 = 12,	// ring proximal phalange bone
	alvrHandBone_Ring2 = 13,	// ring intermediate phalange bone
	alvrHandBone_Ring3 = 14,	// ring distal phalange bone
	alvrHandBone_Pinky0 = 15,	// pinky metacarpal bone
	alvrHandBone_Pinky1 = 16,	// pinky proximal phalange bone
	alvrHandBone_Pinky2 = 17,	// pinky intermediate phalange bone
	alvrHandBone_Pinky3 = 18,	// pinky distal phalange bone
	alvrHandBone_MaxSkinnable = 19,
};
enum ALVR_FINGER_PINCH {
	alvrFingerPinch_Index = 0,
	alvrFingerPinch_Middle = 1,
	alvrFingerPinch_Ring = 2,
	alvrFingerPinch_Pinky = 3,
	alvrFingerPinch_MaxPinches = 4,
};
enum ALVR_HAND_CONFIDENCE {
	alvrThumbConfidence_High = (1 << 0),
	alvrIndexConfidence_High = (1 << 1),
	alvrMiddleConfidence_High = (1 << 2),
	alvrRingConfidence_High = (1 << 3),
	alvrPinkyConfidence_High = (1 << 4),
	alvrHandConfidence_High = (1 << 5),
};
typedef enum ALVR_HAND_INPUT
{
	alvrInputStateHandStatus_PointerValid = (1 << 1),	// if this is set the PointerPose and PinchStrength contain valid data, otherwise they should not be used.
	alvrInputStateHandStatus_IndexPinching = (1 << 2),	// if this is set the pinch gesture for that finger is on
	alvrInputStateHandStatus_MiddlePinching = (1 << 3),	// if this is set the pinch gesture for that finger is on
	alvrInputStateHandStatus_RingPinching = (1 << 4),	// if this is set the pinch gesture for that finger is on
	alvrInputStateHandStatus_PinkyPinching = (1 << 5),	// if this is set the pinch gesture for that finger is on
	alvrInputStateHandStatus_SystemGestureProcessing = (1 << 6),	// if this is set the hand is currently processing a system gesture
	alvrInputStateHandStatus_EnumSize = 0x7fffffff
};
#define ALVR_BUTTON_FLAG(input) (1ULL << input)

#pragma pack(push, 1)
// Represent FOV for each eye in degree.
struct EyeFov {
	float left;
	float right;
	float top;
	float bottom;
};
#pragma pack(pop)

#endif //ALVRCLIENT_PACKETTYPES_H
