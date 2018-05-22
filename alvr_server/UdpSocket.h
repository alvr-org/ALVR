#pragma once

#include <WinSock2.h>
#include <WinInet.h>
#include <string>
#include <memory>
#include <vector>
#include <list>
#include "ISocket.h"
#include "Poller.h"

#define CONTROL_NAMED_PIPE "\\\\.\\pipe\\RemoteGlass_Control"

class UdpSocket
{
public:
	UdpSocket(std::string host, int port, std::shared_ptr<Poller> poller);
	virtual ~UdpSocket();

	virtual bool Startup();
	virtual bool NewClient(std::string &host, int &port);
	virtual bool Recv(char *buf, int *buflen);
	virtual bool Send(char *buf, int len, uint64_t frameIndex);
	virtual void Shutdown();
	virtual sockaddr_in GetClientAddr()const;
	virtual bool IsClientValid()const;

	std::string ErrorStr();

	bool BindSocket();
	bool BindQueueSocket();
	void CheckTimeout();
	void UpdateLastSeen();

private:
	std::string m_Host;
	int m_Port;

	uint64_t m_PreviousSentUs;
	uint64_t m_CurrentTimeslotPackets;

	SOCKET m_Socket;
	SOCKET m_QueueSocket;
	sockaddr_in m_ClientAddr;
	sockaddr_in m_QueueAddr;
	
	bool m_PendingData;
	bool m_NewClient;

	uint64_t m_LastSeen;
	std::shared_ptr<Poller> m_Poller;

	struct SendBuffer {
		std::shared_ptr<char> buf;
		int len;
		uint64_t frameIndex;

		SendBuffer() : buf(NULL, [](char *p) { delete[] p; }) {
		}
	};
	std::list<SendBuffer> m_SendQueue;
	CRITICAL_SECTION m_CS;
};

