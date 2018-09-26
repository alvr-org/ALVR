#include "Poller.h"
#include "Logger.h"
#include "Utils.h"

Poller::Poller() {
	FD_ZERO(&m_org_fds);
}

Poller::~Poller() {
}

int Poller::Do() {
	timeval timeout;
	timeout.tv_sec = 0;
	timeout.tv_usec = 10 * 1000;
	memcpy(&m_fds, &m_org_fds, sizeof(fd_set));
	int ret = select(0, &m_fds, NULL, NULL, &timeout);
	if (ret == SOCKET_ERROR) {
		Log(L"select error : %d %hs", WSAGetLastError(), GetDxErrorStr(WSAGetLastError()).c_str());
	}
	return ret;
}

void Poller::AddSocket(SOCKET s) {
	FD_SET(s, &m_org_fds);
}


bool Poller::IsPending(SOCKET s) {
	return FD_ISSET(s, &m_fds);
}

void Poller::RemoveSocket(SOCKET s) {
	FD_CLR(s, &m_org_fds);
}