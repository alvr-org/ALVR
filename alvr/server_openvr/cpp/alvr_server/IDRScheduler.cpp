#include "IDRScheduler.h"

#include "Utils.h"
#include <mutex>

IDRScheduler::IDRScheduler() { }

IDRScheduler::~IDRScheduler() { }

void IDRScheduler::OnStreamStart() {
    std::unique_lock lock(m_mutex);
    m_minIDRFrameInterval = Settings_Instance()->m_minimumIdrIntervalMs * 1000;
    m_lastIDRTime = 0;
    m_scheduled = true;
}

void IDRScheduler::InsertIDR() {
    std::unique_lock lock(m_mutex);
    m_scheduled = true;
}

bool IDRScheduler::CheckIDRInsertion() {
    std::unique_lock lock(m_mutex);

    if (m_scheduled) {
        uint64_t now = GetTimestampUs();
        if (m_lastIDRTime == 0 || (now >= m_lastIDRTime + m_minIDRFrameInterval)) {
            m_scheduled = false;
            m_lastIDRTime = now;
            return true;
        }
    }
    return false;
}
