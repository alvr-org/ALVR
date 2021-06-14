#ifndef ALVRCLIENT_LATENCY_COLLECTOR_H
#define ALVRCLIENT_LATENCY_COLLECTOR_H

#include <memory>
#include <vector>

class LatencyCollector {
public:
    static LatencyCollector &Instance();

    uint64_t getLatency(uint32_t i, uint32_t j);
    uint64_t getPacketsLostTotal();
    uint64_t getPacketsLostInSecond();
    uint64_t getFecFailureTotal();
    uint64_t getFecFailureInSecond();
    float getFramesInSecond();

    void packetLoss(int64_t lost);
    void fecFailure();

    void tracking(uint64_t frameIndex);
    void estimatedSent(uint64_t frameIndex, uint64_t offset);
    void receivedFirst(uint64_t frameIndex);
    void receivedLast(uint64_t frameIndex);
    void decoderInput(uint64_t frameIndex);
    void decoderOutput(uint64_t frameIndex);
    void rendered1(uint64_t frameIndex);
    void rendered2(uint64_t frameIndex);
    void submit(uint64_t frameIndex);

    void resetAll();
private:
    LatencyCollector();

    void updateLatency(uint64_t *latency);
    void submitNewFrame();

    void resetSecond();
    void checkAndResetSecond();

    static LatencyCollector m_Instance;

    struct FrameTimestamp {
        uint64_t frameIndex;

        // Timestamp in microsec.
        uint64_t tracking;
        uint64_t estimatedSent;
        uint64_t receivedFirst;
        uint64_t receivedLast;
        uint64_t decoderInput;
        uint64_t decoderOutput;
        uint64_t rendered1;
        uint64_t rendered2;
        uint64_t submit;
    };
    static const int MAX_FRAMES = 1024;
    std::vector<FrameTimestamp> m_Frames = std::vector<FrameTimestamp>(MAX_FRAMES);

    uint64_t m_StatisticsTime;
    uint64_t m_PacketsLostTotal = 0;
    uint64_t m_PacketsLostInSecond = 0;
    uint64_t m_PacketsLostPrevious = 0;
    uint64_t m_FecFailureTotal = 0;
    uint64_t m_FecFailureInSecond = 0;
    uint64_t m_FecFailurePrevious = 0;

    // Total/Transport/Decode latency
    // Total/Max/Min/Count
    uint64_t m_Latency[3][4];

    uint64_t m_PreviousLatency[3][4];

    uint64_t m_LastSubmit;
    float m_FramesInSecond = 0;

    FrameTimestamp & getFrame(uint64_t frameIndex);
};

#endif //ALVRCLIENT_LATENCY_COLLECTOR_H
