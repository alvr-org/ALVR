#include <WinSock2.h>
#include <WS2tcpip.h>
#include <Windows.h>
#include "UdpSocket.h"
#include "Logger.h"
#include "Utils.h"


UdpSocket::UdpSocket(std::string host, int port, std::shared_ptr<Poller> poller, uint64_t sendingTimeslotUs, uint64_t limitTimeslotPackets)
	: m_Host(host)
	, m_Port(port)
	, m_Socket(INVALID_SOCKET)
	, m_PendingData(false)
	, m_NewClient(false)
	, m_LastSeen(0)
	, m_Poller(poller)
	, m_SendingTimeslotUs(sendingTimeslotUs)
	, m_LimitTimeslotPackets(limitTimeslotPackets)
	, m_PreviousSentUs(0)
	
{
	m_ClientAddr.sin_family = 0;
	InitializeCriticalSection(&m_CS);
}


UdpSocket::~UdpSocket()
{
}

bool UdpSocket::Startup() {
	WSADATA wsaData;

	WSAStartup(MAKEWORD(2, 0), &wsaData);

	if (!BindSocket()) {
		return false;
	}
	if (!BindQueueSocket()) {
		return false;
	}

	m_Poller->AddSocket(m_Socket);
	m_Poller->AddSocket(m_QueueSocket);

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
	bool ret = false;
	if (m_Poller->IsPending(m_Socket)){
		ret = true;

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
	}

	if (m_Poller->IsPending(m_QueueSocket)) {
		EnterCriticalSection(&m_CS);

		//Log("Sending queued packet. QueueSize=%d", m_SendQueue.size());

		if (!IsClientValid()) {
			m_SendQueue.clear();

			sockaddr_in addr;
			int addrlen = sizeof(addr);
			char dummyBuf[1000];
			while (true) {
				int recvret = recvfrom(m_QueueSocket, dummyBuf, sizeof(dummyBuf), 0, (sockaddr *)&addr, &addrlen);
				if (recvret < 0) {
					break;
				}
			}
		}else if (m_SendQueue.size() > 0) {
			while (m_SendQueue.size() > 0) {
				SendBuffer buffer = m_SendQueue.front();

				uint64_t current = GetTimestampUs();
				if (current % m_SendingTimeslotUs != m_PreviousSentUs % m_SendingTimeslotUs) {
					// Next window arrived.
					m_CurrentTimeslotPackets = 0;
				}
				if (m_LimitTimeslotPackets > 0 && m_CurrentTimeslotPackets >= m_LimitTimeslotPackets) {
					// Exceed limit!
					// TODO: Remove busy loop!
					//Log("Timeslot packet limit exceeded: CurrentTimeslotPackets=%llu", m_CurrentTimeslotPackets);
					break;
				}
				else {
					Log("sendto: CurrentTimeslotPackets=%llu FrameIndex=%llu", m_CurrentTimeslotPackets, buffer.frameIndex);
					int sendret = sendto(m_Socket, buffer.buf.get(), buffer.len, 0, (sockaddr *)&m_ClientAddr, sizeof(m_ClientAddr));
					if (sendret < 0) {
						Log("sendto error: %d %s", WSAGetLastError(), ErrorStr().c_str());
						if (WSAGetLastError() != WSAEWOULDBLOCK) {
							// Fatal Error!
							abort();
						}
						// TODO: Remove busy loop!
						break;
					}
					else {
						m_SendQueue.pop_front();
						m_CurrentTimeslotPackets++;
						m_PreviousSentUs = current;
					}
				}
			}
		}
		else {
			sockaddr_in addr;
			int addrlen = sizeof(addr);
			char dummyBuf[1000];
			while (true) {
				int recvret = recvfrom(m_QueueSocket, dummyBuf, sizeof(dummyBuf), 0, (sockaddr *)&addr, &addrlen);
				if (recvret < 0) {
					break;
				}
			}
		}

		LeaveCriticalSection(&m_CS);
	}
	return ret;
}


bool UdpSocket::Send(char *buf, int len, uint64_t frameIndex) {
	if (!IsClientValid()) {
		return false;
	}
	EnterCriticalSection(&m_CS);

	SendBuffer buffer;
	buffer.buf.reset(new char [len]);
	buffer.len = len;
	buffer.frameIndex = frameIndex;
	memcpy(buffer.buf.get(), buf, len);
	m_SendQueue.push_back(buffer);

	sendto(m_QueueSocket, "1", 1, 0, (sockaddr *) &m_QueueAddr, sizeof(m_QueueAddr));

	LeaveCriticalSection(&m_CS);

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

bool UdpSocket::BindSocket()
{
	m_Socket = socket(AF_INET, SOCK_DGRAM, 0);
	if (m_Socket == INVALID_SOCKET) {
		Log("UdpSocket::BindSocket socket creation error: %d %s", WSAGetLastError(), ErrorStr().c_str());
		return false;
	}

	sockaddr_in addr;
	addr.sin_family = AF_INET;
	addr.sin_port = htons(m_Port);
	InetPton(AF_INET, m_Host.c_str(), &addr.sin_addr);

	int ret = bind(m_Socket, (sockaddr *)&addr, sizeof(addr));
	if (ret != 0) {
		Log("UdpSocket::BindSocket bind error : %d %s", WSAGetLastError(), ErrorStr().c_str());
		return false;
	}
	Log("UdpSocket::BindSocket successfully bound to %s:%d", m_Host.c_str(), m_Port);

	u_long val = 1;
	ioctlsocket(m_Socket, FIONBIO, &val);

	return true;
}

bool UdpSocket::BindQueueSocket()
{
	m_QueueSocket = socket(AF_INET, SOCK_DGRAM, 0);
	if (m_QueueSocket == INVALID_SOCKET) {
		Log("UdpSocket::BindQueueSocket socket creation error: %d %s", WSAGetLastError(), ErrorStr().c_str());
		return false;
	}
	sockaddr_in addr = {};
	addr.sin_family = AF_INET;
	addr.sin_port = htons(0); // bind to random port
	InetPton(AF_INET, "127.0.0.1", &addr.sin_addr);

	int ret = bind(m_QueueSocket, (sockaddr *)&addr, sizeof(addr));
	if (ret != 0) {
		Log("UdpSocket::BindQueueSocket bind error : %d %s", WSAGetLastError(), ErrorStr().c_str());
		return false;
	}

	memset(&m_QueueAddr, 0, sizeof(m_QueueAddr));
	int len = sizeof(m_QueueAddr);
	ret = getsockname(m_QueueSocket, (sockaddr *)&m_QueueAddr, &len);
	if (ret != 0) {
		Log("UdpSocket::BindQueueSocket getsockname error : %d %s", WSAGetLastError(), ErrorStr().c_str());
		return false;
	}
	char buf[30];
	inet_ntop(AF_INET, &m_QueueAddr, buf, sizeof(buf));
	Log("UdpSocket::BindQueueSocket bound queue port: %s:%d\n", buf, htons(m_QueueAddr.sin_port));

	u_long val = 1;
	ioctlsocket(m_QueueSocket, FIONBIO, &val);

	return true;
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
