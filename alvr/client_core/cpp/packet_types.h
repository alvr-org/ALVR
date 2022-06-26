#ifndef ALVRCLIENT_PACKETTYPES_H
#define ALVRCLIENT_PACKETTYPES_H
#include <stdint.h>
#include <assert.h>
#include "reedsolomon/rs.h"
#include "bindings.h"

enum ALVR_CODEC {
	ALVR_CODEC_H264 = 0,
	ALVR_CODEC_H265 = 1,
};

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
