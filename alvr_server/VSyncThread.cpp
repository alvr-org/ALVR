#include "VSyncThread.h"

VSyncThread::VSyncThread(int refreshRate)
	: m_bExit(false)
	, m_refreshRate(refreshRate) {}

// Trigger VSync if elapsed time from previous VSync is larger than 30ms.
void VSyncThread::Run() {
	m_PreviousVsync = 0;

	while (!m_bExit) {
		uint64_t current = GetTimestampUs();
		uint64_t interval = 1000 * 1000 / m_refreshRate;

		if (m_PreviousVsync + interval > current) {
			uint64_t sleepTimeMs = (m_PreviousVsync + interval - current) / 1000;

			if (sleepTimeMs > 0) {
				Log(L"Sleep %llu ms for next VSync.", sleepTimeMs);
				Sleep(static_cast<DWORD>(sleepTimeMs));
			}

			m_PreviousVsync += interval;
		}
		else {
			m_PreviousVsync = current;
		}
		Log(L"Generate VSync Event by VSyncThread");
		vr::VRServerDriverHost()->VsyncEvent(0);
	}
}

void VSyncThread::Shutdown() {
	m_bExit = true;
}

void VSyncThread::SetRefreshRate(int refreshRate) {
	m_refreshRate = refreshRate;
}