#pragma once

#include "threadtools.h"
#include "Logger.h"
#include <WinSock2.h>

class Listener : public CThread {
public:
	Listener(std::string host, int port, std::function<void(sockaddr_in *)> callback) : m_bExiting(false) {
		m_Host = host;
		m_Port = port;
		m_LastSeen = 0;
		m_NewClientCallback = callback;
	}

	void Run() override
	{
		SetThreadPriority(GetCurrentThread(), THREAD_PRIORITY_BELOW_NORMAL);
		WSAData wsaData;

		struct sockaddr_in addr;

		char buf[2048];

		WSAStartup(MAKEWORD(2, 0), &wsaData);

		m_Socket = socket(AF_INET, SOCK_DGRAM, 0);
		if (m_Socket == INVALID_SOCKET) {
			Log("Listener: socket creationg failed. Code=%d", WSAGetLastError());
			return;
		}

		addr.sin_family = AF_INET;
		addr.sin_port = htons(m_Port);
		inet_pton(AF_INET, m_Host.c_str(), &addr.sin_addr);

		int ret = bind(m_Socket, (struct sockaddr *)&addr, sizeof(addr));
		if (ret < 0) {
			Log("Listener: bind error. Code=%d", WSAGetLastError());
			return;
		}
		Log("Listener Successfully bind socket.");

		while (!m_bExiting)
		{
			struct sockaddr_in client_addr;
			int len = sizeof(client_addr);

			ret = recvfrom(m_Socket, buf, sizeof(buf), 0, (sockaddr *)&client_addr, &len);
			if (ret > 0) {
				char buf[100];
				inet_ntop(client_addr.sin_family, &client_addr.sin_addr, buf, sizeof(buf));
				Log("Listener Client request detected. %s:%d", buf, htons(client_addr.sin_port));
				m_ConnectedClientAddr = client_addr;

				m_NewClientCallback(&m_ConnectedClientAddr);
			}
		}

		closesocket(m_Socket);
	}

	void Send(uint8_t *buf, int len) {
		uint8_t packetBuffer[2000];
		for (int i = 0; i < (len + PACKET_SIZE - 1) / PACKET_SIZE; i++) {
			int size = min(PACKET_SIZE, len - (i * PACKET_SIZE));

			*(uint32_t *)packetBuffer = packetCounter;
			packetCounter++;

			memcpy(packetBuffer + 4, buf + i * PACKET_SIZE, size);
			sendto(m_Socket, (char *)packetBuffer, size + 4, 0, (struct sockaddr *)&m_ConnectedClientAddr, sizeof(m_ConnectedClientAddr));

			if (i % 5 == 4) {
				Sleep(1);
			}
		}
	}

	void Stop()
	{
		m_bExiting = true;
		closesocket(m_Socket);
		Join();
	}

private:
	bool m_bExiting;
	std::string m_Host;
	int m_Port;
	SOCKET m_Socket;

	static const int PACKET_SIZE = 1000;

	uint32_t packetCounter = 0;

	sockaddr_in m_ConnectedClientAddr;
	time_t m_LastSeen;
	std::function<void(sockaddr_in *)> m_NewClientCallback;
};
