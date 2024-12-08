#pragma once

#include "alvr_server/IDRScheduler.h"
#include "shared/threadtools.h"
#include <atomic>
#include <memory>
#include <poll.h>
#include <sys/types.h>

class PoseHistory;

class CEncoder : public CThread {
public:
    CEncoder(std::shared_ptr<PoseHistory> poseHistory) {}
    ~CEncoder() {}
    bool Init() override { return true; }
    void Run() override {};

    void Stop() {}
    void OnStreamStart() {}
    void OnPacketLoss() {}
    void InsertIDR() {}
    bool IsConnected() { return true; }
    void CaptureFrame() {}
};
