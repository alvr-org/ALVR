#ifndef ALVRCLIENT_PACKETTYPES_H
#define ALVRCLIENT_PACKETTYPES_H
#include <stdint.h>
#include <assert.h>
#include "reedsolomon/rs.h"
#include "../alvr_server/bindings.h"

enum ALVR_CODEC {
	ALVR_CODEC_H264 = 0,
	ALVR_CODEC_H265 = 1,
};

enum ALVR_RATE_CONTROL_METHOD {
	ALVR_CBR = 0,
	ALVR_VBR = 1,
};

enum ALVR_ENTROPY_CODING {
	ALVR_CABAC = 0,
	ALVR_CAVLC = 1,
};

enum ALVR_ENCODER_QUALITY_PRESET {
	ALVR_QUALITY = 0,
	ALVR_BALANCED = 1,
	ALVR_SPEED = 2
};

enum ALVR_INPUT {
	ALVR_INPUT_SYSTEM_CLICK,
	ALVR_INPUT_APPLICATION_MENU_CLICK,
	ALVR_INPUT_GRIP_CLICK,
	ALVR_INPUT_GRIP_VALUE,
	ALVR_INPUT_GRIP_TOUCH,
	ALVR_INPUT_A_CLICK,
	ALVR_INPUT_A_TOUCH,
	ALVR_INPUT_B_CLICK,
	ALVR_INPUT_B_TOUCH,
	ALVR_INPUT_X_CLICK,
	ALVR_INPUT_X_TOUCH,
	ALVR_INPUT_Y_CLICK,
	ALVR_INPUT_Y_TOUCH,
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
	ALVR_INPUT_THUMB_REST_TOUCH,

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
#define ALVR_BUTTON_FLAG(input) (1ULL << input)


static const int ALVR_MAX_VIDEO_BUFFER_SIZE = 1400;

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
