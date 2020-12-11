#pragma once

#include <string>
#include <memory>
#include <vector>
#include <list>
#include "Poller.h"
#include "Statistics.h"
#include "Utils.h"
#include "ThrottlingBuffer.h"

#define CONTROL_NAMED_PIPE "\\\\.\\pipe\\RemoteGlass_Control"

class UdpSocket
{
public:
	UdpSocket(std::string host, int port, std::shared_ptr<Poller> poller, std::shared_ptr<Statistics> statistics, const Bitrate &bitrate);
	virtual ~UdpSocket();

	virtual bool Recv(char *buf, int *buflen, sockaddr_in *addr, int addrlen);
	void Run();
	virtual bool Send(char *buf, int len, uint64_t frameIndex = 0);
	virtual void Shutdown();
	void SetClientAddr(const sockaddr_in *addr);
	virtual bool IsClientValid()const;
	bool IsLegitClient(const sockaddr_in *addr);
	void InvalidateClient();

private:
	std::string mHost;
	int mPort;

	SOCKET mSocket;
	sockaddr_in mClientAddr;
	
	std::shared_ptr<Poller> mPoller;
	std::shared_ptr<Statistics> mStatistics;

	ThrottlingBuffer mBuffer;

	bool DoSend(char *buf, int len);
};

