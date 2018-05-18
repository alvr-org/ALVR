#pragma once

#include <WinSock2.h>
#include <WinInet.h>
#include <string>
#include <memory>
#include "ISocket.h"
#include "Poller.h"

#define CONTROL_NAMED_PIPE "\\\\.\\pipe\\RemoteGlass_Control"

class UdpSocket : public ISocket
{
public:
	UdpSocket(std::string host, int port, std::shared_ptr<Poller> poller);
	virtual ~UdpSocket();

	virtual bool Startup();
	virtual bool NewClient(std::string &host, int &port);
	virtual bool Recv(char *buf, int *buflen);
	virtual bool Send(char *buf, int len);
	virtual void Shutdown();
	virtual sockaddr_in GetClientAddr()const;
	virtual bool IsClientValid()const;

	std::string ErrorStr();

	void CheckTimeout();
	void UpdateLastSeen();

private:
	std::string m_Host;
	int m_Port;
	SOCKET m_Socket;
	sockaddr_in m_ClientAddr;
	
	bool m_PendingData;
	bool m_NewClient;

	uint64_t m_LastSeen;
	std::shared_ptr<Poller> m_Poller;
};

