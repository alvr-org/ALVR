#include "UdpSocket.h"
#include "Logger.h"
#include "Utils.h"
#include "Settings.h"

UdpSocket::UdpSocket(std::string host, int port, std::shared_ptr<Poller> poller, std::shared_ptr<Statistics> statistics, const Bitrate &bitrate)
	: mHost(host)
	, mPort(port)
	, mSocket(INVALID_SOCKET)
	, mPoller(poller)
	, mStatistics(statistics)
	, mBuffer(bitrate)
	
{
	mClientAddr.sin_family = 0;

	WSADATA wsaData;

	WSAStartup(MAKEWORD(2, 0), &wsaData);

	{
		std::string host = "0.0.0.0";

		mSocket = socket(AF_INET, SOCK_DGRAM, 0);

		int val = 1;
		setsockopt(mSocket, SOL_SOCKET, SO_REUSEADDR, (const char *)&val, sizeof(val));

		val = 1;
		ioctlsocket(mSocket, FIONBIO, (u_long *)&val);

		sockaddr_in addr;
		addr.sin_family = AF_INET;
		addr.sin_port = htons(mPort);
		inet_pton(AF_INET, host.c_str(), &addr.sin_addr);

		int ret = bind(mSocket, (sockaddr *)&addr, sizeof(addr));
		Debug("UdpSocket::BindSocket successfully bound to %hs:%d\n", host.c_str(), mPort);
	}

	mPoller->AddSocket(mSocket, PollerSocketType::READ);

	Debug("UdpSocket::Startup success\n");
}


UdpSocket::~UdpSocket()
{
}

bool UdpSocket::IsClientValid()const {
	return mClientAddr.sin_family != 0;
}

bool UdpSocket::IsLegitClient(const sockaddr_in * addr)
{
	if (mClientAddr.sin_family == AF_INET && mClientAddr.sin_addr.S_un.S_addr == addr->sin_addr.S_un.S_addr && mClientAddr.sin_port == addr->sin_port) {
		return true;
	}
	else {
		return false;
	}
}

void UdpSocket::InvalidateClient()
{
	mClientAddr.sin_family = 0;
}

bool UdpSocket::Recv(char *buf, int *buflen, sockaddr_in *addr, int addrlen) {
	bool ret = false;
	if (mPoller->IsPending(mSocket, PollerSocketType::READ)){
		ret = true;

		recvfrom(mSocket, buf, *buflen, 0, (sockaddr *)addr, &addrlen);
	}

	return ret;
}

void UdpSocket::Run()
{
	Debug("Try to send.\n");
	while (mBuffer.Send([this](char *buf, int len) {return DoSend(buf, len); })) {}

	if (!mBuffer.IsEmpty()) {
		mPoller->WakeLater(1);
	}
}

bool UdpSocket::Send(char *buf, int len, uint64_t frameIndex) {
	if (!IsClientValid()) {
		return false;
	}
	mBuffer.Push(buf, len, frameIndex);

	return true;
}

void UdpSocket::Shutdown() {
	if (mSocket != INVALID_SOCKET) {
		closesocket(mSocket);
	}
	mSocket = INVALID_SOCKET;
}

void UdpSocket::SetClientAddr(const sockaddr_in * addr)
{
	mClientAddr = *addr;
}

bool UdpSocket::DoSend(char * buf, int len)
{
	int ret2 = sendto(mSocket, buf, len, 0, (sockaddr *)&mClientAddr, sizeof(mClientAddr));
	if (ret2 >= 0) {
		mStatistics->CountPacket(len);
		return true;
	}
	if (WSAGetLastError() != WSAEWOULDBLOCK) {
		Error("UdpSocket::DoSend() Error on sendto. %d %ls\n", WSAGetLastError(), GetErrorStr(WSAGetLastError()).c_str());
	}
	return false;
}
