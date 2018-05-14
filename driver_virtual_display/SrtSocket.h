#pragma once

#include <WinSock2.h>
#include <windows.h>
#include <string>
#include <srt.h>
#include <udt.h>
#include "ISocket.h"

class SrtSocket : public ISocket
{

public:

	SrtSocket(std::string host, int port, std::string srtOptions);
	virtual ~SrtSocket();

	virtual bool Startup();

	virtual bool Poll();

	virtual void Shutdown();

	void ApplyOptions(SRTSOCKET socket);

	virtual bool NewClient(std::string &host, int &port);

	virtual bool Recv(char *buf, int *buflen);

	virtual bool Send(char *buf, int len);

	bool IsClientValid()const;

	virtual sockaddr_in GetClientAddr()const;
private:
	SRTSOCKET m_Socket;
	std::string m_Host;
	int m_Port;
	std::string m_SrtOptions;

	SRTSOCKET m_PendingClient;
	sockaddr_in m_PendingClientAddr;
	SRTSOCKET m_ClientSocket;
	sockaddr_in m_ClientAddr;

	int m_Epoll;
};
