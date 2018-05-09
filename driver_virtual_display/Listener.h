#pragma once

#include "threadtools.h"
#include "Logger.h"
#include <WinSock2.h>
#include <srt.h>
#include <udt.h>

class Listener : public CThread {
public:
	Listener(std::string host, int port, std::function<void(sockaddr_in *)> callback) : m_bExiting(false) {
		m_Host = host;
		m_Port = port;
		m_LastSeen = 0;
		m_NewClientCallback = callback;
		m_ClientSocket = SRT_INVALID_SOCK;
	}

	void Run() override
	{
		struct sockaddr_in addr;

		SetThreadPriority(GetCurrentThread(), THREAD_PRIORITY_BELOW_NORMAL);

		int startup = srt_startup();
		Log("srt_startup %d", startup);

		m_Socket = srt_socket(AF_INET, SOCK_DGRAM, 0);
		if (m_Socket == INVALID_SOCKET) {
			Log("Listener: srt_socket creationg failed. Code=%d", srt_getlasterror_str());
			return;
		}

		addr.sin_family = AF_INET;
		addr.sin_port = htons(m_Port);
		inet_pton(AF_INET, m_Host.c_str(), &addr.sin_addr);

		int ret = srt_bind(m_Socket, (struct sockaddr *)&addr, sizeof(addr));
		if (ret < 0) {
			Log("Listener: srt_bind error. Code=%s", srt_getlasterror_str());
			return;
		}
		Log("Listener Successfully bind socket.");

		ret = srt_listen(m_Socket, 10);
		if (ret < 0) {
			Log("Listener: srt_listen error. Code=%s", srt_getlasterror_str());
			return;
		}

		while (!m_bExiting)
		{
			struct sockaddr_in client_addr;
			int len = sizeof(client_addr);
			
			SRTSOCKET ClientSocket = srt_accept(m_Socket, (sockaddr *)&client_addr, &len);
			if (ClientSocket == SRT_INVALID_SOCK) {
				break;
			}
			if (m_ClientSocket != SRT_INVALID_SOCK) {
				Log("New client request coming. close old client.");
				srt_close(m_ClientSocket);
			}
			m_ClientSocket = ClientSocket;
			LogNewClient(&client_addr, len);

			m_ConnectedClientAddr = client_addr;

			m_NewClientCallback(&m_ConnectedClientAddr);
		}

		srt_close(m_Socket);
	}

	void Send(uint8_t *buf, int len) {
		uint8_t packetBuffer[2000];

		if (m_ClientSocket == SRT_INVALID_SOCK) {
			return;
		}

		int chunks = (len + PACKET_SIZE - 1) / PACKET_SIZE;
		for (int i = 0; i < chunks; i++) {
			int size = min(PACKET_SIZE, len - (i * PACKET_SIZE));

			*(uint32_t *)packetBuffer = packetCounter;
			packetCounter++;

			memcpy(packetBuffer + 4, buf + i * PACKET_SIZE, size);

			if (i == chunks - 1) {
				// Insert padding so that client can detect end of packet
				memcpy(packetBuffer + size + 4, "\x00\x00\x00\x02", 4);
				size += 4;
			}
			int ret = srt_send(m_ClientSocket, (char *)packetBuffer, size + 4);
			//sendto(m_Socket, (char *)packetBuffer, size + 4, 0, (struct sockaddr *)&m_ConnectedClientAddr, sizeof(m_ConnectedClientAddr));
			if (ret <= 0) {
				Log("srt_send Error %d. %s\n", ret, srt_getlasterror_str());
			}
		}
	}

	void Stop()
	{
		m_bExiting = true;
		if (m_ClientSocket != SRT_INVALID_SOCK) {
			srt_close(m_ClientSocket);
		}
		if(m_Socket != SRT_INVALID_SOCK) {
			srt_close(m_Socket);
		}
		Join();
	}

	void LogNewClient(struct sockaddr_in *client_addr, int len)
	{
		char clienthost[NI_MAXHOST];
		char clientservice[NI_MAXSERV];
		getnameinfo((sockaddr *)client_addr, len, clienthost, sizeof(clienthost), clientservice, sizeof(clientservice), NI_NUMERICHOST | NI_NUMERICSERV);
		Log("New client: %s:%s", clienthost, clientservice);
	}

private:
	bool m_bExiting;
	std::string m_Host;
	int m_Port;
	SRTSOCKET m_Socket;
	SRTSOCKET m_ClientSocket;

	static const int PACKET_SIZE = 1000;

	uint32_t packetCounter = 0;

	sockaddr_in m_ConnectedClientAddr;
	time_t m_LastSeen;
	std::function<void(sockaddr_in *)> m_NewClientCallback;
};
