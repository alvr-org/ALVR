#include "IDRScheduler.h"

#include "Utils.h"

IDRScheduler::IDRScheduler()
{
}


IDRScheduler::~IDRScheduler()
{
}

void IDRScheduler::OnPacketLoss()
{
	IPCCriticalSectionLock lock(m_IDRCS);
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
	m_minIDRFrameInterval = Settings::Instance().m_keyframeResendIntervalMs * 1000;
	IPCCriticalSectionLock lock(m_IDRCS);
	// Force insert IDR-frame
	m_insertIDRTime = GetTimestampUs() - MIN_IDR_FRAME_INTERVAL * 2;
	m_scheduled = true;
}

bool IDRScheduler::CheckIDRInsertion() {
	IPCCriticalSectionLock lock(m_IDRCS);
	if (m_scheduled) {
		if (m_insertIDRTime <= GetTimestampUs()) {
			m_scheduled = false;
			return true;
		}
	}
	return false;
}
