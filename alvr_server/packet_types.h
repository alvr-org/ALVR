#ifndef ALVRCLIENT_PACKETTYPES_H
#define ALVRCLIENT_PACKETTYPES_H
#include <stdint.h>

enum ALVR_PACKET_TYPE {
	ALVR_PACKET_TYPE_HELLO_MESSAGE = 1,
	ALVR_PACKET_TYPE_CONNECTION_MESSAGE = 2,
	ALVR_PACKET_TYPE_RECOVER_CONNECTION = 3,
	ALVR_PACKET_TYPE_BROADCAST_REQUEST_MESSAGE = 4,
	ALVR_PACKET_TYPE_STREAM_CONTROL_MESSAGE = 5,
	ALVR_PACKET_TYPE_TRACKING_INFO = 6,
	ALVR_PACKET_TYPE_TIME_SYNC = 7,
	ALVR_PACKET_TYPE_CHANGE_SETTINGS = 8,
	ALVR_PACKET_TYPE_VIDEO_FRAME_START = 9,
	ALVR_PACKET_TYPE_VIDEO_FRAME = 10,
	ALVR_PACKET_TYPE_AUDIO_FRAME_START = 11,
	ALVR_PACKET_TYPE_AUDIO_FRAME = 12,
	ALVR_PACKET_TYPE_PACKET_ERROR_REPORT = 13,
};

enum {
	ALVR_PROTOCOL_VERSION = 16
};

enum ALVR_LOST_FRAME_TYPE {
	ALVR_LOST_FRAME_TYPE_P = 0,
	ALVR_LOST_FRAME_TYPE_IDR = 1,
	ALVR_LOST_FRAME_TYPE_AUDIO = 2,
};

#pragma pack(push, 1)
// hello message
struct HelloMessage {
	uint32_t type; // 1
	uint32_t version; // ALVR_PROTOCOL_VERSION
	char deviceName[32]; // null-terminated
	uint32_t refreshRate; // 60 or 72
};
struct ConnectionMessage {
	uint32_t type; // 2
	uint32_t version; // ALVR_PROTOCOL_VERSION
	uint32_t videoWidth; // in pixels
	uint32_t videoHeight; // in pixels
	uint32_t bufferSize; // in bytes
};
struct RecoverConnection {
	uint32_t type; // 3
};
struct BroadcastRequestMessage {
	uint32_t type; // 4
};
struct StreamControlMessage {
	uint32_t type; // 5
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
struct TrackingInfo {
	uint32_t type; // 6

	static const int FLAG_OTHER_TRACKING_SOURCE = (1 << 0); // Other_Tracking_Source_Position has valid value (For ARCore)
	static const int FLAG_CONTROLLER_ENABLE = (1 << 8);
	static const int FLAG_CONTROLLER_LEFTHAND = (1 << 9); // 0: Left hand, 1: Right hand
	static const int FLAG_CONTROLLER_OCULUSGO = (1 << 10); // 0: Gear VR, 1: Oculus Go
	static const int FLAG_CONTROLLER_TRACKPAD_TOUCH = (1 << 11); // 0: Not touched, 1: Touched
	static const int FLAG_CONTROLLER_BACK = (1 << 12);
	static const int FLAG_CONTROLLER_VOLUME_UP = (1 << 13);
	static const int FLAG_CONTROLLER_VOLUME_DOWN = (1 << 14);
	uint32_t flags;

	uint64_t clientTime;
	uint64_t FrameIndex;
	double predictedDisplayTime;
	TrackingQuat HeadPose_Pose_Orientation;
	TrackingVector3 HeadPose_Pose_Position;

	TrackingVector3 Other_Tracking_Source_Position;
	TrackingQuat Other_Tracking_Source_Orientation;

	static const int CONTROLLER_BUTTON_TRIGGER_CLICK = 0x00000001;
	static const int CONTROLLER_BUTTON_TRACKPAD_CLICK = 0x00100000;
	static const int CONTROLLER_BUTTON_BACK = 0x00200000;
	uint32_t controllerButtons;

	struct {
		float x;
		float y;
	} controllerTrackpadPosition;

	uint8_t	controllerBatteryPercentRemaining;
	uint8_t	controllerRecenterCount;

	// Tracking info of controller. (float * 19 = 76 bytes)
	TrackingQuat controller_Pose_Orientation;
	TrackingVector3 controller_Pose_Position;
	TrackingVector3 controller_AngularVelocity;
	TrackingVector3 controller_LinearVelocity;
	TrackingVector3 controller_AngularAcceleration;
	TrackingVector3 controller_LinearAcceleration;
};
// Client >----(mode 0)----> Server
// Client <----(mode 1)----< Server
// Client >----(mode 2)----> Server
struct TimeSync {
	uint32_t type; // 7
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
};
struct ChangeSettings {
	uint32_t type; // 8
	uint32_t enableTestMode;
	uint32_t suspend;
};
struct VideoFrameStart {
	uint32_t type; // 9
	uint32_t packetCounter;
	uint64_t presentationTime;
	uint64_t frameIndex;
	uint32_t frameByteSize;
	// char frameBuffer[];
};
struct VideoFrame {
	uint32_t type; // 10
	uint32_t packetCounter;
	// char frameBuffer[];
};
struct AudioFrameStart {
	uint32_t type; // 11
	uint32_t packetCounter;
	uint64_t presentationTime;
	uint32_t frameByteSize;
	// char frameBuffer[];
};
struct AudioFrame {
	uint32_t type; // 12
	uint32_t packetCounter;
	// char frameBuffer[];
};
// Report packet loss/error from client to server.
struct PacketErrorReport {
	uint32_t type; // ALVR_PACKET_TYPE_PACKET_ERROR_REPORT
	uint32_t lostFrameType;
	uint32_t fromPacketCounter;
	uint32_t toPacketCounter;
};
#pragma pack(pop)

#endif //ALVRCLIENT_PACKETTYPES_H
