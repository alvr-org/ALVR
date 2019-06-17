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

	void SleepAndWake();
	void Wake();
private:
	fd_set mOrgReadFDs;
	fd_set mReadFDs;
	fd_set mOrgWriteFDs;
	fd_set mWriteFDs;
	SOCKET mQueueSocket;
	sockaddr_in mQueueAddr;
	bool mSmallSleep = false;
	static const int DEFAULT_WAIT_TIME_US = 10 * 1000;
	static const int SMALL_WAIT_TIME_US = 100;

	bool BindQueueSocket();
	void ReadQueueSocket();
};