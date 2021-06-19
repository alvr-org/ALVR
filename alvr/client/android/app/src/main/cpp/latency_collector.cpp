#include <jni.h>
#include "latency_collector.h"
#include "utils.h"
#include "bindings.h"

LatencyCollector LatencyCollector::m_Instance;

LatencyCollector::LatencyCollector(){
    m_StatisticsTime = getTimestampUs();
}

LatencyCollector::FrameTimestamp &LatencyCollector::getFrame(uint64_t frameIndex) {
    auto &frame = m_Frames[frameIndex % MAX_FRAMES];
    if(frame.frameIndex != frameIndex) {
        memset(&frame, 0, sizeof(FrameTimestamp));
        frame.frameIndex = frameIndex;
    }
    return frame;
}

void LatencyCollector::tracking(uint64_t frameIndex) {
    getFrame(frameIndex).tracking = getTimestampUs();
}
void LatencyCollector::estimatedSent(uint64_t frameIndex, uint64_t offset) {
    getFrame(frameIndex).estimatedSent = getTimestampUs() + offset;
}
void LatencyCollector::receivedFirst(uint64_t frameIndex) {
    getFrame(frameIndex).receivedFirst = getTimestampUs();
}
void LatencyCollector::receivedLast(uint64_t frameIndex) {
    getFrame(frameIndex).receivedLast = getTimestampUs();
}
void LatencyCollector::decoderInput(uint64_t frameIndex) {
    getFrame(frameIndex).decoderInput = getTimestampUs();
}
void LatencyCollector::decoderOutput(uint64_t frameIndex) {
    getFrame(frameIndex).decoderOutput = getTimestampUs();
}
void LatencyCollector::rendered1(uint64_t frameIndex) {
    getFrame(frameIndex).rendered1 = getTimestampUs();
}
void LatencyCollector::rendered2(uint64_t frameIndex) {
    getFrame(frameIndex).rendered2 = getTimestampUs();
}

void LatencyCollector::submit(uint64_t frameIndex) {
    FrameTimestamp timestamp = getFrame(frameIndex);
    timestamp.submit = getTimestampUs();

    m_TrackingPredictionTime = timestamp.submit + (timestamp.submit - timestamp.tracking);

    uint64_t latency[3];
    latency[0] = timestamp.submit - timestamp.tracking;
    latency[1] = timestamp.receivedLast - timestamp.estimatedSent;
    latency[2] = timestamp.decoderOutput - timestamp.decoderInput;

    updateLatency(latency);

    submitNewFrame();

    FrameLog(frameIndex, "totalLatency=%.1f transportLatency=%.1f decodeLatency=%.1f renderLatency1=%.1f renderLatency2=%.1f"
            , latency[0] / 1000.0, latency[1] / 1000.0, latency[2] / 1000.0
            , (timestamp.rendered2 - timestamp.decoderOutput) / 1000.0
            , (timestamp.submit - timestamp.rendered2) / 1000.0);
}

void LatencyCollector::updateLatency(uint64_t *latency) {
    checkAndResetSecond();

    for(int i = 0; i < 3; i++) {
        // Total
        m_Latency[i][0] += latency[i];
        // Max
        m_Latency[i][1] = std::max(m_Latency[i][1], latency[i]);
        // Min
        m_Latency[i][2] = std::min(m_Latency[i][2], latency[i]);
        // Count
        m_Latency[i][3]++;
    }
}

void LatencyCollector::resetAll() {
    m_PacketsLostTotal = 0;
    m_PacketsLostInSecond = 0;
    m_PacketsLostPrevious = 0;

    m_FecFailureTotal = 0;
    m_FecFailureInSecond = 0;
    m_FecFailurePrevious = 0;

    m_framesInSecond = 0;
    m_framesPrevious = 0;

    m_StatisticsTime = getTimestampUs() / USECS_IN_SEC;

    for(int i = 0; i < 3; i++) {
        for(int j = 0; j < 4; j++) {
            m_Latency[i][j] = 0;
            m_PreviousLatency[i][j] = 0;
        }
    }
}

void LatencyCollector::resetSecond(){
    memcpy(m_PreviousLatency, m_Latency, sizeof(m_Latency));
    memset(m_Latency, 0, sizeof(m_Latency));

    m_PacketsLostPrevious = m_PacketsLostInSecond;
    m_PacketsLostInSecond = 0;

    m_FecFailurePrevious = m_FecFailureInSecond;
    m_FecFailureInSecond = 0;

    m_framesPrevious = m_framesInSecond;
    m_framesInSecond = 0;
}

void LatencyCollector::checkAndResetSecond() {
    uint64_t current = getTimestampUs() / USECS_IN_SEC;
    if(m_StatisticsTime != current){
        m_StatisticsTime = current;
        resetSecond();
    }
}

void LatencyCollector::packetLoss(int64_t lost) {
    checkAndResetSecond();

    m_PacketsLostTotal += lost;
    m_PacketsLostInSecond += lost;
}

void LatencyCollector::fecFailure() {
    checkAndResetSecond();

    m_FecFailureTotal++;
    m_FecFailureInSecond++;
}

void LatencyCollector::submitNewFrame() {
    checkAndResetSecond();

    m_framesInSecond++;
}

uint64_t LatencyCollector::getTrackingPredictionLatency() {
    uint64_t current = getTimestampUs();
    if (current >= m_TrackingPredictionTime)
        return 0;
    else if (current + 1e5 < m_TrackingPredictionTime)
        return current + 1e5;
    else
        return m_TrackingPredictionTime - current;
}

uint64_t LatencyCollector::getLatency(uint32_t i, uint32_t j) {
    if(j == 1 || j == 2) {
        // Min/Max
        return m_PreviousLatency[i][j];
    }
    if(m_PreviousLatency[i][3] == 0) {
        return 0;
    }
    return m_PreviousLatency[i][0] / m_PreviousLatency[i][3];
}
uint64_t LatencyCollector::getPacketsLostTotal() {
    return m_PacketsLostTotal;
}
uint64_t LatencyCollector::getPacketsLostInSecond() {
    return m_PacketsLostPrevious;
}
uint64_t LatencyCollector::getFecFailureTotal() {
    return m_FecFailureTotal;
}
uint64_t LatencyCollector::getFecFailureInSecond() {
    return m_FecFailurePrevious;
}
uint32_t LatencyCollector::getFramesInSecond() {
    return m_framesPrevious;
}

LatencyCollector &LatencyCollector::Instance() {
    return m_Instance;
}

void decoderInput(long long frameIndex) {
    LatencyCollector::Instance().decoderInput((uint64_t)frameIndex);
}

void decoderOutput(long long frameIndex) {
    LatencyCollector::Instance().decoderOutput((uint64_t)frameIndex);
}
