#pragma once

#include <WinSock2.h>

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