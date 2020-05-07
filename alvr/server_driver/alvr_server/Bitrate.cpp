#include "Bitrate.h"


Bitrate Bitrate::fromBits(uint64_t rateInBits) {
	return Bitrate(rateInBits);
}

Bitrate Bitrate::fromKiBits(uint64_t rateInKiBits) {
	return Bitrate(rateInKiBits * 1000);
}

Bitrate Bitrate::fromMiBits(uint64_t rateInMiBits) {
	return Bitrate(rateInMiBits * 1000000);
}

uint64_t Bitrate::toBits() {
	return rateInBits;
}
uint64_t Bitrate::toKiBits() {
	return rateInBits / 1000;
}
uint64_t Bitrate::toMiBits() {
	return rateInBits / 1000000;
}
uint64_t Bitrate::toBytes() {
	return rateInBits / 8;
}
uint64_t Bitrate::toKiBytes() {
	return rateInBits / 8000;
}
uint64_t Bitrate::toMiBytes() {
	return rateInBits / 8000000;
}

Bitrate::Bitrate() : rateInBits(0)
{
}

Bitrate::Bitrate(const Bitrate &a) {
	rateInBits = a.rateInBits;
}

Bitrate &Bitrate::operator=(const Bitrate &a) {
	rateInBits = a.rateInBits;
	return *this;
}

Bitrate::Bitrate(uint64_t rateInBits) : rateInBits(rateInBits) {
}