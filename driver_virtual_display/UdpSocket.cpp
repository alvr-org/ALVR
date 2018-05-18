#include <WinSock2.h>
#include <WS2tcpip.h>
#include <Windows.h>
#include "UdpSocket.h"
#include "Logger.h"
#include "Utils.h"


UdpSocket::UdpSocket(std::string host, int port, std::shared_ptr<Poller> poller)
	: m_Host(host)
	, m_Port(port)
	, m_Socket(INVALID_SOCKET)
	, m_PendingData(false)
	, m_NewClient(false)
	, m_LastSeen(0)
	, m_Poller(poller)
	
{
	m_ClientAddr.sin_family = 0;
}


UdpSocket::~UdpSocket()
{
}

bool UdpSocket::Startup() {
	WSADATA wsaData;

	WSAStartup(MAKEWORD(2, 0), &wsaData);

	m_Socket = socket(AF_INET, SOCK_DGRAM, 0);
	if (m_Socket == INVALID_SOCKET) {
		Log("UdpSocket::Startup socket creation error: %d %s", WSAGetLastError(), ErrorStr().c_str());
		return false;
	}

	sockaddr_in addr;
	addr.sin_family = AF_INET;
	addr.sin_port = htons(m_Port);
	InetPton(AF_INET, m_Host.c_str(), &addr.sin_addr);

	int ret = bind(m_Socket, (sockaddr *)&addr, sizeof(addr));
	if (ret != 0) {
		Log("UdpSocket::Startup bind error : %d %s",  WSAGetLastError(), ErrorStr().c_str());
		return false;
	}

	m_Poller->AddSocket(m_Socket);

	Log("UdpSocket::Startup success");

	return true;
}

bool UdpSocket::NewClient(std::string &host, int &port) {
	if (m_NewClient) {
		m_NewClient = false;

		char address[100] = {};
		inet_ntop(m_ClientAddr.sin_family, &m_ClientAddr.sin_addr, address, sizeof(address));
		host = address;
		port = htons(m_ClientAddr.sin_port);

		return true;
	}
	return false;
}

sockaddr_in UdpSocket::GetClientAddr()const {
	return m_ClientAddr;
}


bool UdpSocket::IsClientValid()const {
	return m_ClientAddr.sin_family != 0;
}

bool UdpSocket::Recv(char *buf, int *buflen) {
	if (!m_Poller->IsPending(m_Socket)) {
		return false;
	}

	sockaddr_in addr;
	int addrlen = sizeof(addr);
	recvfrom(m_Socket, buf, *buflen, 0, (sockaddr *)&addr, &addrlen);

	if (m_ClientAddr.sin_family == 0) {
		// New client
		m_ClientAddr = addr;
		m_NewClient = true;
	}
	else if (m_ClientAddr.sin_addr.S_un.S_addr != addr.sin_addr.S_un.S_addr || m_ClientAddr.sin_port != addr.sin_port) {
		// New client
		m_ClientAddr = addr;
		m_NewClient = true;
	}
	UpdateLastSeen();

	m_PendingData = false;
	return true;
}


bool UdpSocket::Send(char *buf, int len) {
	if (m_ClientAddr.sin_family == 0) {
		return false;
	}
	sendto(m_Socket, buf, len, 0, (sockaddr *)&m_ClientAddr, sizeof(m_ClientAddr));

	return true;
}

void UdpSocket::Shutdown() {
	if (m_Socket != INVALID_SOCKET) {
		closesocket(m_Socket);
	}
	m_Socket = INVALID_SOCKET;
}

std::string UdpSocket::ErrorStr() {
	char *s = NULL;
	std::string ret;
	FormatMessageA(FORMAT_MESSAGE_ALLOCATE_BUFFER | FORMAT_MESSAGE_FROM_SYSTEM | FORMAT_MESSAGE_IGNORE_INSERTS,
		NULL, WSAGetLastError(),
		MAKELANGID(LANG_NEUTRAL, SUBLANG_DEFAULT),
		(LPSTR)&s, 0, NULL);
	ret = s;
	LocalFree(s);
	return ret;
}


void UdpSocket::CheckTimeout() {
	if (!IsClientValid()) {
		return;
	}

	uint64_t Current = GetTimestampUs();

	if (Current - m_LastSeen > 60 * 1000 * 1000) {
		// idle for 60 seconcd
		// Invalidate client
		m_ClientAddr.sin_family = 0;
		Log("Client timeout for idle");
	}
}

void UdpSocket::UpdateLastSeen() {
	m_LastSeen = GetTimestampUs();
}
