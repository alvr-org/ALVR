#ifndef ALVRCLIENT_PACKETTYPES_H
#define ALVRCLIENT_PACKETTYPES_H
#include <stdint.h>
#include <assert.h>
#include "reedsolomon/rs.h"

// Maximum UDP packet size (payload size in bytes)
static const int ALVR_MAX_PACKET_SIZE = 1400;
static const int ALVR_REFRESH_RATE_LIST_SIZE = 4;

// Maximum UDP packet size
static const int MAX_PACKET_UDP_PACKET_SIZE = 2000;

static const char *ALVR_HELLO_PACKET_SIGNATURE = "ALVR";

// Guardian syncing constants
static const int ALVR_GUARDIAN_SEGMENT_SIZE = 100;
static const double ALVR_GUARDIAN_RESEND_CD_SEC = 1.0;

static const char *ALVR_VERSION = ALVR_CLIENT_VERSION; // defined through app build.gradle

enum ALVR_PACKET_TYPE {
	ALVR_PACKET_TYPE_HELLO_MESSAGE = 1,
	ALVR_PACKET_TYPE_CONNECTION_MESSAGE = 2,
	ALVR_PACKET_TYPE_RECOVER_CONNECTION = 3,
	ALVR_PACKET_TYPE_BROADCAST_REQUEST_MESSAGE = 4,
	ALVR_PACKET_TYPE_STREAM_CONTROL_MESSAGE = 5,
	ALVR_PACKET_TYPE_TRACKING_INFO = 6,
	ALVR_PACKET_TYPE_TIME_SYNC = 7,
	ALVR_PACKET_TYPE_CHANGE_SETTINGS = 8,
	ALVR_PACKET_TYPE_VIDEO_FRAME = 9,
	ALVR_PACKET_TYPE_AUDIO_FRAME_START = 10,
	ALVR_PACKET_TYPE_AUDIO_FRAME = 11,
	ALVR_PACKET_TYPE_PACKET_ERROR_REPORT = 12,
	ALVR_PACKET_TYPE_HAPTICS = 13,
    ALVR_PACKET_TYPE_MIC_AUDIO = 14,
	ALVR_PACKET_TYPE_GUARDIAN_SYNC_START = 15,
	ALVR_PACKET_TYPE_GUARDIAN_SYNC_ACK = 16,
	ALVR_PACKET_TYPE_GUARDIAN_SEGMENT_DATA = 17,
	ALVR_PACKET_TYPE_GUARDIAN_SEGMENT_ACK = 18,
};

enum ALVR_CODEC {
	ALVR_CODEC_H264 = 0,
	ALVR_CODEC_H265 = 1,
};

enum ALVR_LOST_FRAME_TYPE {
	ALVR_LOST_FRAME_TYPE_VIDEO = 0,
	ALVR_LOST_FRAME_TYPE_AUDIO = 1,
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

	ALVR_INPUT_MAX = ALVR_INPUT_TRACKPAD_TOUCH,
	ALVR_INPUT_COUNT = ALVR_INPUT_MAX + 1
};
enum ALVR_HAND {
    alvrHandBone_Invalid						= -1,
    alvrHandBone_WristRoot 					= 0,	// root frame of the hand, where the wrist is located
    alvrHandBone_ForearmStub					= 1,	// frame for user's forearm
    alvrHandBone_Thumb0						= 2,	// thumb trapezium bone
    alvrHandBone_Thumb1						= 3,	// thumb metacarpal bone
    alvrHandBone_Thumb2						= 4,	// thumb proximal phalange bone
    alvrHandBone_Thumb3						= 5,	// thumb distal phalange bone
    alvrHandBone_Index1						= 6,	// index proximal phalange bone
    alvrHandBone_Index2						= 7,	// index intermediate phalange bone
    alvrHandBone_Index3						= 8,	// index distal phalange bone
    alvrHandBone_Middle1						= 9,	// middle proximal phalange bone
    alvrHandBone_Middle2						= 10,	// middle intermediate phalange bone
    alvrHandBone_Middle3						= 11,	// middle distal phalange bone
    alvrHandBone_Ring1						= 12,	// ring proximal phalange bone
    alvrHandBone_Ring2						= 13,	// ring intermediate phalange bone
    alvrHandBone_Ring3						= 14,	// ring distal phalange bone
    alvrHandBone_Pinky0						= 15,	// pinky metacarpal bone
    alvrHandBone_Pinky1						= 16,	// pinky proximal phalange bone
    alvrHandBone_Pinky2						= 17,	// pinky intermediate phalange bone
    alvrHandBone_Pinky3						= 18,	// pinky distal phalange bone
    alvrHandBone_MaxSkinnable				= 19,
};
enum ALVR_FINGER_PINCH {
    alvrFingerPinch_Index                   = 0,
    alvrFingerPinch_Middle                  = 1,
    alvrFingerPinch_Ring                    = 2,
    alvrFingerPinch_Pinky                   = 3,
    alvrFingerPinch_MaxPinches              = 4,
};
enum ALVR_HAND_CONFIDENCE {
    alvrThumbConfidence_High                  = (1 << 0),
    alvrIndexConfidence_High                  = (1 << 1),
    alvrMiddleConfidence_High                  = (1 << 2),
    alvrRingConfidence_High                  = (1 << 3),
    alvrPinkyConfidence_High                  = (1 << 4),
    alvrHandConfidence_High                 = (1 << 5),
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
// hello message
struct HelloMessage {
	uint32_t type; // ALVR_PACKET_TYPE_HELLO_MESSAGE
	char signature[4]; // Ascii string "ALVR". NOT null-terminated.
	char version[32];  // ALVR client semver version (null terminated)

	char deviceName[32]; // null-terminated

	uint16_t refreshRate;

	uint32_t renderWidth;
	uint32_t renderHeight;

	// FOV of left and right eyes.
	struct EyeFov eyeFov[2];

	float ipd;
};
struct ConnectionMessage {
	uint32_t type; // ALVR_PACKET_TYPE_CONNECTION_MESSAGE
	uint32_t codec; // enum ALVR_CODEC
	uint32_t videoWidth; // in pixels
	uint32_t videoHeight; // in pixels
	uint32_t bufferSize; // in bytes
	uint32_t frameQueueSize;
	uint8_t refreshRate;
	bool streamMic;
	uint8_t foveationMode;
    float foveationStrength;
    float foveationShape;
    float foveationVerticalOffset;
	char webGuiUrl[32];
};
struct RecoverConnection {
	uint32_t type; // ALVR_PACKET_TYPE_RECOVER_CONNECTION
};
struct BroadcastRequestMessage {
	uint32_t type; // ALVR_PACKET_TYPE_BROADCAST_REQUEST_MESSAGE
};
struct StreamControlMessage {
	uint32_t type; // ALVR_PACKET_TYPE_STREAM_CONTROL_MESSAGE
	uint32_t mode; // 1=Start stream, 2=Stop stream
};
struct TrackingQuat {
	float x;
	float y;
	float z;
	float w;
};
struct TrackingVector3 {
	float x;
	float y;
	float z;
};
struct TrackingVector2 {
	float x;
	float y;
};
struct TrackingInfo {
	uint32_t type; // ALVR_PACKET_TYPE_TRACKING_INFO
	static const uint32_t FLAG_OTHER_TRACKING_SOURCE = (1 << 0); // Other_Tracking_Source_Position has valid value (For ARCore)
	uint32_t flags;

	uint64_t clientTime;
	uint64_t FrameIndex;
	double predictedDisplayTime;
	TrackingQuat HeadPose_Pose_Orientation;
	TrackingVector3 HeadPose_Pose_Position;

	TrackingVector3 Other_Tracking_Source_Position;
	TrackingQuat Other_Tracking_Source_Orientation;

	static const uint32_t MAX_CONTROLLERS = 2;


	struct Controller {
		static const uint32_t FLAG_CONTROLLER_ENABLE         = (1 << 0);
		static const uint32_t FLAG_CONTROLLER_LEFTHAND       = (1 << 1); // 0: Left hand, 1: Right hand
		static const uint32_t FLAG_CONTROLLER_GEARVR         = (1 << 2);
		static const uint32_t FLAG_CONTROLLER_OCULUS_GO      = (1 << 3);
		static const uint32_t FLAG_CONTROLLER_OCULUS_QUEST   = (1 << 4);
        static const uint32_t FLAG_CONTROLLER_OCULUS_HAND    = (1 << 5);
		uint32_t flags;
		uint64_t buttons;

		struct {
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
        TrackingQuat boneRotations[alvrHandBone_MaxSkinnable];
        //TrackingQuat boneRotationsBase[alvrHandBone_MaxSkinnable];
        TrackingVector3 bonePositionsBase[alvrHandBone_MaxSkinnable];
        TrackingQuat boneRootOrientation;
        TrackingVector3 boneRootPosition;
        uint32_t inputStateStatus;
        float fingerPinchStrengths[alvrFingerPinch_MaxPinches];
        uint32_t handFingerConfidences;
	} controller[2];
};
// Client >----(mode 0)----> Server
// Client <----(mode 1)----< Server
// Client >----(mode 2)----> Server
struct TimeSync {
	uint32_t type; // ALVR_PACKET_TYPE_TIME_SYNC
	uint32_t mode; // 0,1,2
	uint64_t sequence;
	uint64_t serverTime;
	uint64_t clientTime;

	// Following value are filled by client only when mode=0.
	uint64_t packetsLostTotal;
	uint64_t packetsLostInSecond;

	uint32_t averageTotalLatency;
	uint32_t maxTotalLatency;
	uint32_t minTotalLatency;

	uint32_t averageTransportLatency;
	uint32_t maxTransportLatency;
	uint32_t minTransportLatency;

	uint32_t averageDecodeLatency;
	uint32_t maxDecodeLatency;
	uint32_t minDecodeLatency;

	uint32_t fecFailure;
	uint64_t fecFailureInSecond;
	uint64_t fecFailureTotal;

	uint32_t fps;
};
struct ChangeSettings {
	uint32_t type; // 8
	uint64_t debugFlags;
	uint32_t suspend;
	uint32_t frameQueueSize;
};
struct VideoFrame {
	uint32_t type; // ALVR_PACKET_TYPE_VIDEO_FRAME
	uint32_t packetCounter;
	uint64_t trackingFrameIndex;
	// FEC decoder needs some value for identify video frame number to detect new frame.
	// trackingFrameIndex becomes sometimes same value as previous video frame (in case of low tracking rate).
	uint64_t videoFrameIndex;
	uint64_t sentTime;
	uint32_t frameByteSize;
	uint32_t fecIndex;
	uint16_t fecPercentage;
	// char frameBuffer[];
};
struct AudioFrameStart {
	uint32_t type; // ALVR_PACKET_TYPE_AUDIO_FRAME_START
	uint32_t packetCounter;
	uint64_t presentationTime;
	uint32_t frameByteSize;
	// char frameBuffer[];
};
struct AudioFrame {
	uint32_t type; // ALVR_PACKET_TYPE_AUDIO_FRAME
	uint32_t packetCounter;
	// char frameBuffer[];
};
struct MicAudioFrame {
    uint32_t type; // ALVR_PACKET_TYPE_AUDIO_FRAME
    size_t outputBufferNumElements;
    size_t completeSize;
    int packetIndex;
    int16_t micBuffer[100];
};
// Report packet loss/error from client to server.
struct PacketErrorReport {
	uint32_t type; // ALVR_PACKET_TYPE_PACKET_ERROR_REPORT
	uint32_t lostFrameType;
	uint32_t fromPacketCounter;
	uint32_t toPacketCounter;
};
// Send haptics feedback from server to client.
struct HapticsFeedback {
	uint32_t type; // ALVR_PACKET_TYPE_HAPTICS
	uint64_t startTime; // Elapsed time from now when start haptics. In microseconds.
	float amplitude;
	float duration;
	float frequency;
	uint8_t hand; // 0:Right, 1:Left
};
// Send Oculus guardian data
struct GuardianSyncStart {
	uint32_t type; // ALVR_PACKET_TYPE_GUARDIAN_SYNC_START
	uint64_t timestamp;
	TrackingQuat standingPosRotation;
	TrackingVector3 standingPosPosition;
	TrackingVector2 playAreaSize;
	uint32_t totalPointCount;
};
struct GuardianSyncStartAck {
	uint32_t type; // ALVR_PACKET_TYPE_GUARDIAN_SYNC_ACK
	uint64_t timestamp;
};
struct GuardianSegmentData {
	uint32_t type; // ALVR_PACKET_TYPE_GUARDIAN_SEGMENT_DATA
	uint64_t timestamp;
	uint32_t segmentIndex;
	TrackingVector3 points[ALVR_GUARDIAN_SEGMENT_SIZE];
};
struct GuardianSegmentAck {
	uint32_t type; // ALVR_PACKET_TYPE_GUARDIAN_SEGMENT_ACK
	uint64_t timestamp;
	uint32_t segmentIndex;
};
#pragma pack(pop)

static const int ALVR_MAX_VIDEO_BUFFER_SIZE = ALVR_MAX_PACKET_SIZE - sizeof(VideoFrame);

static const int ALVR_FEC_SHARDS_MAX = 20;

inline int CalculateParityShards(int dataShards, int fecPercentage) {
	int totalParityShards = (dataShards * fecPercentage + 99) / 100;
	return totalParityShards;
}

// Calculate how many packet is needed for make signal shard.
inline int CalculateFECShardPackets(int len, int fecPercentage) {
	// This reed solomon implementation accept only 255 shards.
	// Normally, we use ALVR_MAX_VIDEO_BUFFER_SIZE as block_size and single packet becomes single shard.
	// If we need more than maxDataShards packets, we need to combine multiple packet to make single shrad.
	// NOTE: Moonlight seems to use only 255 shards for video frame.
	int maxDataShards = ((ALVR_FEC_SHARDS_MAX - 2) * 100 + 99 + fecPercentage) / (100 + fecPercentage);
	int minBlockSize = (len + maxDataShards - 1) / maxDataShards;
	int shardPackets = (minBlockSize + ALVR_MAX_VIDEO_BUFFER_SIZE - 1) / ALVR_MAX_VIDEO_BUFFER_SIZE;
	assert(maxDataShards + CalculateParityShards(maxDataShards, fecPercentage) <= ALVR_FEC_SHARDS_MAX);
	return shardPackets;
}

#endif //ALVRCLIENT_PACKETTYPES_H
