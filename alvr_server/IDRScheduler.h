#pragma once

#include <stdint.h>
#include "ipctools.h"

class IDRScheduler
{
public:
	IDRScheduler();
	~IDRScheduler();

	void OnPacketLoss();

	void OnClientConnected();

	bool CheckIDRInsertion();
private:
	static const int MIN_IDR_FRAME_INTERVAL = 2 * 1000 * 1000; // 2-seconds
	uint64_t m_insertIDRTime;
	bool m_scheduled;
	IPCCriticalSection m_IDRCS;
};

