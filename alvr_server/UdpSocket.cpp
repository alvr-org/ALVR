#include <WinSock2.h>
#include <WS2tcpip.h>
#include <Windows.h>
#include "UdpSocket.h"
#include "Logger.h"
#include "Utils.h"
#include "Settings.h"

UdpSocket::UdpSocket(std::string host, int port, std::shared_ptr<Poller> poller, std::shared_ptr<Statistics> statistics)
	: m_Host(host)
	, m_Port(port)
	, m_Socket(INVALID_SOCKET)
	, m_Poller(poller)
	, m_Statistics(statistics)
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

sockaddr_in UdpSocket::GetClientAddr()const {
	return m_ClientAddr;
}


bool UdpSocket::IsClientValid()const {
	return m_ClientAddr.sin_family != 0;
}

bool UdpSocket::IsLegitClient(const sockaddr_in * addr)
{
	if (m_ClientAddr.sin_family == AF_INET && m_ClientAddr.sin_addr.S_un.S_addr == addr->sin_addr.S_un.S_addr && m_ClientAddr.sin_port == addr->sin_port) {
		return true;
	}
	else {
		return false;
	}
}

void UdpSocket::InvalidateClient()
{
	m_ClientAddr.sin_family = 0;
}

bool UdpSocket::Recv(char *buf, int *buflen, sockaddr_in *addr, int addrlen) {
	bool ret = false;
	if (m_Poller->IsPending(m_Socket)){
		ret = true;

		recvfrom(m_Socket, buf, *buflen, 0, (sockaddr *)addr, &addrlen);
	}

	if (m_Poller->IsPending(m_QueueSocket)) {
		EnterCriticalSection(&m_CS);

		//Log("Sending queued packet. QueueSize=%d", m_SendQueue.size());

		if (!IsClientValid()) {
			m_SendQueue.clear();

			sockaddr_in addr2;
			int addrlen2 = sizeof(addr2);
			char dummyBuf[1000];
			while (true) {
				int recvret = recvfrom(m_QueueSocket, dummyBuf, sizeof(dummyBuf), 0, (sockaddr *)&addr2, &addrlen2);
				if (recvret < 0) {
					break;
				}
			}
		}else if (m_SendQueue.size() > 0) {
			while (m_SendQueue.size() > 0) {
				SendBuffer buffer = m_SendQueue.front();

				uint64_t current = GetTimestampUs();
				if (current / Settings::Instance().m_SendingTimeslotUs != m_PreviousSentUs / Settings::Instance().m_SendingTimeslotUs) {
					// Next window arrived.
					m_CurrentTimeslotPackets = 0;
				}
				if (Settings::Instance().m_LimitTimeslotPackets > 0 && m_CurrentTimeslotPackets >= Settings::Instance().m_LimitTimeslotPackets) {
					// Exceed limit!
					// TODO: Remove busy loop!
					//Log("Timeslot packet limit exceeded: CurrentTimeslotPackets=%llu", m_CurrentTimeslotPackets);
					break;
				}
				else {
					bool fakePacketLoss = false;
					if (Settings::Instance().m_causePacketLoss > 0) {
						Settings::Instance().m_causePacketLoss--;
						fakePacketLoss = true;
					}
					int sendret = 0;
					if (!fakePacketLoss) {
						//Log("sendto: CurrentTimeslotPackets=%llu FrameIndex=%llu", m_CurrentTimeslotPackets, buffer.frameIndex);
						sendret = sendto(m_Socket, buffer.buf.get(), buffer.len, 0, (sockaddr *)&m_ClientAddr, sizeof(m_ClientAddr));
					}
					else {
						Log("Cause packet loss for debugging.");
					}
					if (sendret < 0) {
						Log("sendto error: %d %s", WSAGetLastError(), ErrorStr(WSAGetLastError()).c_str());
						if (WSAGetLastError() != WSAEWOULDBLOCK) {
							// Fatal Error!
							abort();
						}
						// TODO: Remove busy loop!
						break;
					}
					else {
						m_SendQueue.pop_front();
						m_Statistics->CountPacket(buffer.len);
						m_CurrentTimeslotPackets++;
						m_PreviousSentUs = current;
					}
				}
			}
		}
		else {
			sockaddr_in addr2;
			int addrlen2 = sizeof(addr2);
			char dummyBuf[1000];
			while (true) {
				int recvret = recvfrom(m_QueueSocket, dummyBuf, sizeof(dummyBuf), 0, (sockaddr *)&addr2, &addrlen2);
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

void UdpSocket::SetClientAddr(const sockaddr_in * addr)
{
	m_ClientAddr = *addr;
}

std::string UdpSocket::ErrorStr(int err) {
	char *s = NULL;
	std::string ret;
	FormatMessageA(FORMAT_MESSAGE_ALLOCATE_BUFFER | FORMAT_MESSAGE_FROM_SYSTEM | FORMAT_MESSAGE_IGNORE_INSERTS,
		NULL, err,
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
		FatalLog("UdpSocket::BindSocket socket creation error: %d %s", WSAGetLastError(), ErrorStr(WSAGetLastError()).c_str());
		return false;
	}

	int val = 1;
	setsockopt(m_Socket, SOL_SOCKET, SO_REUSEADDR, (const char *)&val, sizeof(val));

	val = 1;
	ioctlsocket(m_Socket, FIONBIO, (u_long *)&val);

	sockaddr_in addr;
	addr.sin_family = AF_INET;
	addr.sin_port = htons(m_Port);
	inet_pton(AF_INET, m_Host.c_str(), &addr.sin_addr);

	int ret = bind(m_Socket, (sockaddr *)&addr, sizeof(addr));
	if (ret != 0) {
		FatalLog("UdpSocket::BindSocket bind error : Address=%s:%d %d %s", m_Host.c_str(), m_Port, WSAGetLastError(), ErrorStr(WSAGetLastError()).c_str());
		return false;
	}
	Log("UdpSocket::BindSocket successfully bound to %s:%d", m_Host.c_str(), m_Port);
	
	return true;
}

bool UdpSocket::BindQueueSocket()
{
	m_QueueSocket = socket(AF_INET, SOCK_DGRAM, 0);
	if (m_QueueSocket == INVALID_SOCKET) {
		FatalLog("UdpSocket::BindQueueSocket socket creation error: %d %s", WSAGetLastError(), ErrorStr(WSAGetLastError()).c_str());
		return false;
	}

	int val = 1;
	setsockopt(m_QueueSocket, SOL_SOCKET, SO_REUSEADDR, (const char *)&val, sizeof(val));

	val = 1;
	ioctlsocket(m_QueueSocket, FIONBIO, (u_long *)&val);

	sockaddr_in addr = {};
	addr.sin_family = AF_INET;
	addr.sin_port = htons(0); // bind to random port
	inet_pton(AF_INET, "127.0.0.1", &addr.sin_addr);

	int ret = bind(m_QueueSocket, (sockaddr *)&addr, sizeof(addr));
	if (ret != 0) {
		FatalLog("UdpSocket::BindQueueSocket bind error : %d %s", WSAGetLastError(), ErrorStr(WSAGetLastError()).c_str());
		return false;
	}

	memset(&m_QueueAddr, 0, sizeof(m_QueueAddr));
	int len = sizeof(m_QueueAddr);
	ret = getsockname(m_QueueSocket, (sockaddr *)&m_QueueAddr, &len);
	if (ret != 0) {
		FatalLog("UdpSocket::BindQueueSocket getsockname error : %d %s", WSAGetLastError(), ErrorStr(WSAGetLastError()).c_str());
		return false;
	}
	char buf[30];
	inet_ntop(AF_INET, &m_QueueAddr, buf, sizeof(buf));
	Log("UdpSocket::BindQueueSocket bound queue socket. port=%d", htons(m_QueueAddr.sin_port));

	return true;
}
