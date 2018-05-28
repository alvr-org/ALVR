#ifndef ALVRCLIENT_PACKETTYPES_H
#define ALVRCLIENT_PACKETTYPES_H
#include <stdint.h>

enum ALVR_PACKET_TYPE {
	ALVR_PACKET_TYPE_HELLO_MESSAGE = 1,
	ALVR_PACKET_TYPE_TRACKING_INFO = 2,
	ALVR_PACKET_TYPE_TIME_SYNC = 3,
	ALVR_PACKET_TYPE_CHANGE_SETTINGS = 4,
	ALVR_PACKET_TYPE_BROADCAST_REQUEST_MESSAGE = 5,
	ALVR_PACKET_TYPE_CONNECTION_MESSAGE = 6,
	ALVR_PACKET_TYPE_STREAM_CONTROL_MESSAGE = 7,
	ALVR_PACKET_TYPE_VIDEO_FRAME_START = 8,
	ALVR_PACKET_TYPE_VIDEO_FRAME = 9
};

#pragma pack(push, 1)
// hello message
struct HelloMessage {
	int type; // 1
	char deviceName[32]; // null-terminated
};
struct TrackingInfo {
	uint32_t type; // 2
	uint64_t clientTime;
	uint64_t FrameIndex;
	double predictedDisplayTime;
	struct {
		float x;
		float y;
		float z;
		float w;
	} HeadPose_Pose_Orientation;
	struct {
		float x;
		float y;
		float z;
	} HeadPose_Pose_Position;
	struct {
		float x;
		float y;
		float z;
	} HeadPose_AngularVelocity;
	struct {
		float x;
		float y;
		float z;
	} HeadPose_LinearVelocity;
	struct {
		float x;
		float y;
		float z;
	} HeadPose_AngularAcceleration;
	struct {
		float x;
		float y;
		float z;
	} HeadPose_LinearAcceleration;
	struct Matrix {
		float M[16];
	};
	struct {
		Matrix ProjectionMatrix;
		Matrix ViewMatrix;
	} Eye[2];

};
// Client >----(mode 0)----> Server
// Client <----(mode 1)----< Server
// Client >----(mode 2)----> Server
struct TimeSync {
	uint32_t type; // 3
	uint32_t mode; // 0,1,2
	uint64_t sequence;
	uint64_t serverTime;
	uint64_t clientTime;
};
struct ChangeSettings {
	uint32_t type; // 4
	uint32_t enableTestMode;
	uint32_t suspend;
};
struct BroadcastRequestMessage {
	uint32_t type; // 5
};
struct ConnectionMessage {
	uint32_t type; // 6
	uint32_t videoWidth; // in pixels
	uint32_t videoHeight; // in pixels
	uint32_t bufferSize; // in bytes
};
struct StreamControlMessage {
	uint32_t type; // 7
	uint32_t mode; // 1=Start stream, 2=Stop stream
};
struct VideoFrameStart {
	uint32_t type; // 8
	uint32_t packetCounter;
	uint64_t presentationTime;
	uint64_t frameIndex;
	uint32_t frameByteSize;
	// char frameBuffer[];
};
struct VideoFrame {
	uint32_t type; // 9
	uint32_t packetCounter;
	// char frameBuffer[];
};
#pragma pack(pop)

#endif //ALVRCLIENT_PACKETTYPES_H
