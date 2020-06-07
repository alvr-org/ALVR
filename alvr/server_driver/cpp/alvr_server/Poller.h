#pragma once

#include "Utils.h"

enum PollerSocketType {
	READ, WRITE
};

class Poller {
public:
	Poller();
	~Poller();

	int Do();
	void AddSocket(SOCKET s, PollerSocketType type);
	bool IsPending(SOCKET s, PollerSocketType type);
	void RemoveSocket(SOCKET s, PollerSocketType type);

	void WakeLater(uint64_t elapsedMs);
private:
	fd_set mOrgReadFDs;
	fd_set mReadFDs;
	fd_set mOrgWriteFDs;
	fd_set mWriteFDs;
	SOCKET mQueueSocket;
	sockaddr_in mQueueAddr;
	uint64_t mNextWake = 0;
	static const int DEFAULT_WAIT_TIME_US = 10 * 1000;

	bool BindQueueSocket();
	void ReadQueueSocket();
	int CalculateNextWake();
	void ClearNextWake();
};