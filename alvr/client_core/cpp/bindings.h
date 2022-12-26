#pragma once

struct VideoFrame {
    unsigned int packetCounter;
    unsigned long long trackingFrameIndex;
    // FEC decoder needs some value for identify video frame number to detect new frame.
    // trackingFrameIndex becomes sometimes same value as previous video frame (in case of low
    // tracking rate).
    unsigned long long videoFrameIndex;
    unsigned long long sentTime;
    unsigned int frameByteSize;
    unsigned int fecIndex;
    unsigned short fecPercentage;
};

// nal.h
extern "C" void initializeNalParser(int codec, bool enableFec);
extern "C" void notifyNewDecoder();
extern "C" bool processNalPacket(VideoFrame header,
                                 const unsigned char *payload,
                                 int payloadSize,
                                 bool &outHadFecFailure);
extern "C" void (*pushNal)(const char *buffer, int length, unsigned long long frameIndex);