#pragma once
#include <string>
#include <algorithm>
#include <Windows.h>
#include <WinSock2.h>
#include <Ws2tcpip.h>

class UdpSender
{
	SOCKET sock;

	struct sockaddr_in destAddr;

	static const int PACKET_SIZE = 1000;

public:
	UdpSender(std::string addr, int port) {
		WSAData wsaData;
		WSAStartup(MAKEWORD(2, 0), &wsaData);

		sock = socket(AF_INET, SOCK_DGRAM, 0);

		destAddr.sin_family = AF_INET;
		destAddr.sin_port = htons(port);
		inet_pton(destAddr.sin_family, addr.c_str(), &destAddr.sin_addr);
	}

	~UdpSender() {
		closesocket(sock);
	}

	void Send(uint8_t *buf, int len) {
		for (int i = 0; i < len / PACKET_SIZE; i++) {
			int size = min(PACKET_SIZE, len - (i * PACKET_SIZE));
			sendto(sock, (char *)(buf + i * PACKET_SIZE), size, 0, (struct sockaddr *)&destAddr, sizeof(destAddr));
		}
	}
};