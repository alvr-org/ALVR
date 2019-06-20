#include "VSyncThread.h"
#include "Logger.h"

VSyncThread::VSyncThread(int refreshRate)
	: mExit(false)
	, mRefreshRate(refreshRate) {}

// Trigger VSync if elapsed time from previous VSync is larger than 30ms.
void VSyncThread::Run() {
	mPreviousVsync = 0;

	while (!mExit) {
		uint64_t current = GetTimestampUs();
		uint64_t interval = 1000 * 1000 / mRefreshRate;

		if (mPreviousVsync + interval > current) {
			uint64_t sleepTimeMs = (mPreviousVsync + interval - current) / 1000;

			if (sleepTimeMs > 0) {
				Log(L"Sleep %llu ms for next VSync.", sleepTimeMs);
				Sleep(static_cast<DWORD>(sleepTimeMs));
			}

			mPreviousVsync += interval;
		}
		else {
			mPreviousVsync = current;
		}
		Log(L"Generate VSync Event by VSyncThread");
		vr::VRServerDriverHost()->VsyncEvent(0);
	}
}

void VSyncThread::Shutdown() {
	mExit = true;
}

void VSyncThread::SetRefreshRate(int refreshRate) {
	mRefreshRate = refreshRate;
}
