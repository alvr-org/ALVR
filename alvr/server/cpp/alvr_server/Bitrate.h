#pragma once

#include <stdint.h>

class Bitrate
{
public:
	static Bitrate fromBits(uint64_t rateInBits);
	static Bitrate fromKiBits(uint64_t rateInKiBits);
	static Bitrate fromMiBits(uint64_t rateInMiBits);

	uint64_t toBits();
	uint64_t toKiBits();
	uint64_t toMiBits();
	uint64_t toBytes();
	uint64_t toKiBytes();
	uint64_t toMiBytes();

	Bitrate();
	Bitrate(const Bitrate &a);

	Bitrate &operator=(const Bitrate &a);
private:
	Bitrate(uint64_t rateInBits);
	uint64_t rateInBits;
};

