#pragma once

#include <stdint.h>
#include "openvr-utils\ipctools.h"

class IDRScheduler
{
public:
	IDRScheduler();
	~IDRScheduler();

	void OnPacketLoss();

	void OnStreamStart();

	bool CheckIDRInsertion();

	void OnFrameAck(bool result, bool isIDR);

	bool CanEncodeFrame();
private:
	enum State {
		NOT_STREAMING, // Client not connected or not requested streaming
		REQUESTING_IDR, // Wait for IDR insertion. After streaming requested or IDR lost.
		SENDING_IDR, // Wait for acknoledgement of sent IDR from client.
		STREAMING // Sending P-Frames after successful ack.
	};
	State mState;
};

