#include "IDRScheduler.h"

#include "Utils.h"
#include <mutex>

IDRScheduler::IDRScheduler()
{
}


IDRScheduler::~IDRScheduler()
{
}

void IDRScheduler::OnPacketLoss()
{
	std::unique_lock lock(m_mutex);

	if (m_scheduled) {
		// Waiting next insertion.
		return;
	}
	if (GetTimestampUs() - m_insertIDRTime > m_minIDRFrameInterval) {
		// Insert immediately
		m_insertIDRTime = GetTimestampUs();
		m_scheduled = true;
	}
	else {
		// Schedule next insertion.
		m_insertIDRTime += m_minIDRFrameInterval;
		m_scheduled = true;
	}
}

void IDRScheduler::OnStreamStart()
{
	if (Settings::Instance().IsLoaded() && Settings::Instance().m_aggressiveKeyframeResend) {
		m_minIDRFrameInterval = MIN_IDR_FRAME_INTERVAL_AGGRESSIVE;
	} else {
		m_minIDRFrameInterval = MIN_IDR_FRAME_INTERVAL;
	}
	InsertIDR();
}

void IDRScheduler::InsertIDR()
{
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
