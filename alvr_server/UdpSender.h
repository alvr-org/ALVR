#pragma once
#include <string>
#include <algorithm>
#include <Windows.h>
#include <WinSock2.h>
#include <Ws2tcpip.h>
#include "Logger.h"

class UdpSender
{
	SOCKET m_Socket;

	struct sockaddr_in destAddr;

	static const int PACKET_SIZE = 1000;

	uint32_t packetCounter = 0;

public:
	UdpSender(sockaddr_in *addr) {
		WSAData wsaData;
		WSAStartup(MAKEWORD(2, 0), &wsaData);

		m_Socket = socket(AF_INET, SOCK_DGRAM, 0);

		destAddr = *addr;
	}

	~UdpSender() {
		closesocket(m_Socket);
	}

	void Send(uint8_t *buf, int len) {
		uint8_t packetBuffer[2000];
		for (int i = 0; i < (len + PACKET_SIZE - 1) / PACKET_SIZE; i++) {
			int size = min(PACKET_SIZE, len - (i * PACKET_SIZE));

			*(uint32_t *)packetBuffer = packetCounter;
			packetCounter++;

			memcpy(packetBuffer + 4, buf + i * PACKET_SIZE, size);
			sendto(m_Socket, (char *)packetBuffer, size + 4, 0, (struct sockaddr *)&destAddr, sizeof(destAddr));

			if (i % 5 == 4) {
				Sleep(1);
			}
		}
	}
};