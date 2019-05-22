#pragma once

#include "Utils.h"

class Poller {
public:
	Poller();
	~Poller();

	int Do();
	void AddSocket(SOCKET s);
	bool IsPending(SOCKET s);
	void RemoveSocket(SOCKET s);

private:

	fd_set m_org_fds;
	fd_set m_fds;
};