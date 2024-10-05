#include "IDRScheduler.h"

#include "Utils.h"
#include <mutex>

IDRScheduler::IDRScheduler() { }

IDRScheduler::~IDRScheduler() { }

void IDRScheduler::OnPacketLoss() { InsertIDR(); }

void IDRScheduler::OnStreamStart() {
    m_minIDRFrameInterval = Settings::Instance().m_minimumIdrIntervalMs * 1000;
    m_scheduled = false;
    InsertIDR();
}

void IDRScheduler::InsertIDR() {
    std::unique_lock lock(m_mutex);

    m_insertIDRTime = GetTimestampUs() - MIN_IDR_FRAME_INTERVAL * 2;
    m_scheduled = true;
}

bool IDRScheduler::CheckIDRInsertion() {
    std::unique_lock lock(m_mutex);

    if (m_scheduled) {
        if (m_insertIDRTime <= GetTimestampUs()) {
            m_scheduled = false;
            return true;
        }
    }
    return false;
}
