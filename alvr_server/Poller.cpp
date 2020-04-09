#include "Poller.h"
#include "Logger.h"
#include "Utils.h"

Poller::Poller() {
	FD_ZERO(&mOrgReadFDs);
	FD_ZERO(&mReadFDs);
	FD_ZERO(&mOrgWriteFDs);
	FD_ZERO(&mWriteFDs);
	BindQueueSocket();
}

Poller::~Poller() {
}

int Poller::Do() {
	timeval timeout;
	timeout.tv_sec = 0;
	timeout.tv_usec = CalculateNextWake();
	if (timeout.tv_usec == 0) {
		Log(L"Poller::Do(). Wake.");
		ClearNextWake();
		return 0;
	}
	Log(L"Poller::Do(). Select %ld us", timeout.tv_usec);
	memcpy(&mReadFDs, &mOrgReadFDs, sizeof(fd_set));
	memcpy(&mWriteFDs, &mOrgWriteFDs, sizeof(fd_set));
	int ret = select(0, &mReadFDs, &mWriteFDs, NULL, &timeout);
	if (ret == SOCKET_ERROR) {
		Log(L"select error : %d %s", WSAGetLastError(), GetErrorStr(WSAGetLastError()).c_str());
		return ret;
	}
	Log(L"Poller::Do(). Select done. %d", ret);
	ReadQueueSocket();
	return ret;
}

void Poller::AddSocket(SOCKET s, PollerSocketType type) {
	if (type == PollerSocketType::READ) {
		FD_SET(s, &mOrgReadFDs);
	}
	else {
		FD_SET(s, &mOrgWriteFDs);
	}
}

bool Poller::IsPending(SOCKET s, PollerSocketType type) {
	if (type == PollerSocketType::READ) {
		return FD_ISSET(s, &mReadFDs);
	}
	else {
		return FD_ISSET(s, &mWriteFDs);
	}
}

void Poller::RemoveSocket(SOCKET s, PollerSocketType type) {
	if (type == PollerSocketType::READ) {
		FD_CLR(s, &mOrgReadFDs);
	}
	else {
		FD_CLR(s, &mOrgWriteFDs);
	}
}

void Poller::WakeLater(uint64_t elapsedMs)
{
	mNextWake = 900LLU + GetTimestampUs();

	//sendto(mQueueSocket, "1", 1, 0, (sockaddr *)&mQueueAddr, sizeof(mQueueAddr));
}

bool Poller::BindQueueSocket()
{
	mQueueSocket = socket(AF_INET, SOCK_DGRAM, 0);
	if (mQueueSocket == INVALID_SOCKET) {
		FatalLog("Poller::BindQueueSocket socket creation error: %d %s", WSAGetLastError(), GetErrorStr(WSAGetLastError()).c_str());
		return false;
	}

	int val = 1;
	setsockopt(mQueueSocket, SOL_SOCKET, SO_REUSEADDR, (const char *)&val, sizeof(val));

	val = 1;
	ioctlsocket(mQueueSocket, FIONBIO, (u_long *)&val);

	sockaddr_in addr = {};
	addr.sin_family = AF_INET;
	addr.sin_port = htons(0); // bind to random port
	inet_pton(AF_INET, "127.0.0.1", &addr.sin_addr);

	int ret = bind(mQueueSocket, (sockaddr *)&addr, sizeof(addr));
	if (ret != 0) {
		FatalLog("Poller::BindQueueSocket bind error : %d %s", WSAGetLastError(), GetErrorStr(WSAGetLastError()).c_str());
		return false;
	}

	memset(&mQueueAddr, 0, sizeof(mQueueAddr));
	int len = sizeof(mQueueAddr);
	ret = getsockname(mQueueSocket, (sockaddr *)&mQueueAddr, &len);
	if (ret != 0) {
		FatalLog("Poller::BindQueueSocket getsockname error : %d %hs", WSAGetLastError(), GetErrorStr(WSAGetLastError()).c_str());
		return false;
	}
	char buf[30];
	inet_ntop(AF_INET, &mQueueAddr, buf, sizeof(buf));
	Log(L"Poller::BindQueueSocket bound queue socket. port=%d", htons(mQueueAddr.sin_port));

	AddSocket(mQueueSocket, PollerSocketType::READ);

	return true;
}

void Poller::ReadQueueSocket()
{
	sockaddr_in addr;
	int addrlen = sizeof(addr);
	char dummyBuf[1000];
	while (true) {
		int recvret = recvfrom(mQueueSocket, dummyBuf, sizeof(dummyBuf), 0, (sockaddr *)&addr, &addrlen);
		if (recvret < 0) {
			break;
		}
	}
}

int Poller::CalculateNextWake()
{
	if (mNextWake == 0) {
		return DEFAULT_WAIT_TIME_US;
	}
	uint64_t current = GetTimestampUs();
	if (mNextWake < current) {
		return 0;
	}
	if (mNextWake - current > DEFAULT_WAIT_TIME_US) {
		return DEFAULT_WAIT_TIME_US;
	}
	return mNextWake - current;
}

void Poller::ClearNextWake()
{
	mNextWake = 0;
}
