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
	IPCCriticalSectionLock lock(mCS);
	if (mScheduled) {
		// Waiting next insertion.
		return;
	}
	if (GetTimestampUs() - mInsertIDRTime > MIN_IDR_FRAME_INTERVAL) {
		// Insert immediately
		mInsertIDRTime = GetTimestampUs();
		mScheduled = true;
	}
	else {
		// Schedule next insertion.
		mInsertIDRTime += MIN_IDR_FRAME_INTERVAL;
		mScheduled = true;
	}
}

void IDRScheduler::OnStreamStart()
{
	IPCCriticalSectionLock lock(mCS);
	// Force insert IDR-frame
	mInsertIDRTime = GetTimestampUs() - MIN_IDR_FRAME_INTERVAL * 2;
	mScheduled = true;
}

bool IDRScheduler::CheckIDRInsertion() {
	IPCCriticalSectionLock lock(mCS);
	if (mScheduled) {
		if (mInsertIDRTime <= GetTimestampUs()) {
			mScheduled = false;
			return true;
		}
	}
	return false;
}
