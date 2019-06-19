#include "IDRScheduler.h"

#include "Utils.h"

IDRScheduler::IDRScheduler()
 : mState(State::NOT_STREAMING)
{
}


IDRScheduler::~IDRScheduler()
{
}

void IDRScheduler::OnPacketLoss()
{
	mState = State::REQUESTING_IDR;
}

void IDRScheduler::OnStreamStart()
{
	mState = State::REQUESTING_IDR;
}

bool IDRScheduler::CheckIDRInsertion() {
	if (mState == State::REQUESTING_IDR) {
		mState = State::SENDING_IDR;
		return true;
	}
	return false;
}

void IDRScheduler::OnFrameAck(bool result, bool isIDR)
{
	if (isIDR) {
		if (result) {
			mState = State::STREAMING;
		}
		else {
			mState = State::REQUESTING_IDR;
		}
	}
}

bool IDRScheduler::CanEncodeFrame()
{
	return mState == State::REQUESTING_IDR || mState == State::STREAMING;
}
