#ifndef ALVRCLIENT_NAL_H
#define ALVRCLIENT_NAL_H

#include <jni.h>
#include <list>
#include "utils.h"
#include "fec.h"


class NALParser {
public:
    NALParser(bool enableFEC);

    void setCodec(int codec);
    bool processPacket(VideoFrame *packet, int packetSize, bool &fecFailure);

    bool fecFailure();
private:
    int findVPSSPS(const std::byte *frameBuffer, int frameByteSize);

    bool m_enableFEC;

    FECQueue m_queue;

    int m_codec = 1;
};
#endif //ALVRCLIENT_NAL_H
