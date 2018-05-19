#include "SrtSocket.h"
#include "Logger.h"

SrtSocket::SrtSocket(std::string host, int port, std::string srtOptions)
	: m_Host(host)
	, m_Port(port)
	, m_SrtOptions(srtOptions)
	, m_Socket(SRT_INVALID_SOCK)
	, m_PendingClient(SRT_INVALID_SOCK)
	, m_ClientSocket(SRT_INVALID_SOCK)
{

}

SrtSocket::~SrtSocket() {
	Shutdown();
}

bool SrtSocket::Startup() {
	int startup = srt_startup();
	Log("srt_startup %d", startup);

	m_Socket = srt_socket(AF_INET, SOCK_DGRAM, 0);
	if (m_Socket == INVALID_SOCKET) {
		Log("Listener: srt_socket creationg failed. Code=%d", srt_getlasterror_str());
		return false;
	}

	ApplyOptions(m_Socket);

	sockaddr_in addr;
	addr.sin_family = AF_INET;
	addr.sin_port = htons(m_Port);
	inet_pton(AF_INET, m_Host.c_str(), &addr.sin_addr);

	int ret = srt_bind(m_Socket, (struct sockaddr *)&addr, sizeof(addr));
	if (ret < 0) {
		Log("Listener: srt_bind error. Code=%s", srt_getlasterror_str());
		return false;
	}
	Log("Listener Successfully bind socket.");

	ret = srt_listen(m_Socket, 10);
	if (ret < 0) {
		Log("Listener: srt_listen error. Code=%s", srt_getlasterror_str());
		return false;
	}

	m_Epoll = srt_epoll_create();

	int flags = SRT_EPOLL_IN | SRT_EPOLL_ERR;
	srt_epoll_add_usock(m_Epoll, m_Socket, &flags);

	return true;
}


bool SrtSocket::Poll() {
	SRTSOCKET read_fds[2];
	int read_n = 2;

	int ret = srt_epoll_wait(m_Epoll, read_fds, &read_n, NULL, NULL, 1000, NULL, NULL, NULL, NULL);
	if (ret >= 1) {
		if (read_n >= 1) {
			if (read_fds[0] == m_Socket) {
				// New client

				int len = sizeof(sockaddr_in);
				SRTSOCKET ClientSocket = srt_accept(m_Socket, (sockaddr *)&m_PendingClientAddr, &len);
				if (ClientSocket == SRT_INVALID_SOCK) {
					Log("srt_accept Failed: %d %s", srt_getlasterror(NULL), srt_getlasterror_str());
					return true;
				}
				m_PendingClient = ClientSocket;
				ApplyOptions(m_PendingClient);

				return true;
			}else if (read_fds[0] == m_ClientSocket) {
				// New data

				return true;
			}
		}

	}
	return false;
}

void SrtSocket::Shutdown() {
	if (m_Socket != SRT_INVALID_SOCK) {
		srt_close(m_Socket);
	}
	if (m_PendingClient != SRT_INVALID_SOCK) {
		srt_close(m_PendingClient);
	}
	if (m_ClientSocket != SRT_INVALID_SOCK) {
		srt_close(m_ClientSocket);
	}
	if (m_Epoll >= 0) {
		srt_epoll_release(m_Epoll);
	}
	m_Socket = SRT_INVALID_SOCK;
	m_PendingClient = SRT_INVALID_SOCK;
	m_ClientSocket = SRT_INVALID_SOCK;
	m_Epoll = -1;
}

bool SrtSocket::NewClient(std::string &host, int &port) {
	if (m_PendingClient == SRT_INVALID_SOCK) {
		return false;
	}
	if (m_ClientSocket != SRT_INVALID_SOCK) {
		// Close old client (limit sigle connection)
		srt_epoll_remove_usock(m_Epoll, m_ClientSocket);
		srt_close(m_ClientSocket);
	}
	m_ClientSocket = m_PendingClient;
	m_ClientAddr = m_PendingClientAddr;
	m_PendingClient = SRT_INVALID_SOCK;

	char address[100];
	inet_ntop(m_PendingClientAddr.sin_family, &m_PendingClientAddr.sin_addr, address, sizeof(address));
	host = address;
	port = htons(m_PendingClientAddr.sin_port);

	int flags = SRT_EPOLL_IN;
	srt_epoll_add_usock(m_Epoll, m_ClientSocket, &flags);

	return true;
}

bool SrtSocket::Recv(char *buf, int *buflen) {
	int val = 0;
	int len = sizeof(val);
	// Check available data
	srt_getsockflag(m_ClientSocket, SRTO_RCVDATA, &val, &len);
	if (val == 0) {
		return false;
	}
	int ret = srt_recv(m_ClientSocket, buf, *buflen);
	if (ret == 0) {
		// socket closed.
		srt_epoll_remove_usock(m_Epoll, m_ClientSocket);
		srt_close(m_ClientSocket);
		m_ClientSocket = SRT_INVALID_SOCK;
		return false;
	}
	*buflen = ret;
	return true;
}

bool SrtSocket::Send(char *buf, int len) {
	if (m_ClientSocket == SRT_INVALID_SOCK) {
		return false;
	}
	srt_send(m_ClientSocket, buf, len);
	return true;
}

sockaddr_in SrtSocket::GetClientAddr() const {
	return m_ClientAddr;
}

bool SrtSocket::IsClientValid()const {
	return m_ClientSocket != SRT_INVALID_SOCK;
}

void SrtSocket::ApplyOptions(SRTSOCKET socket) {
	std::istringstream stream(m_SrtOptions);
	std::string option;

	while (std::getline(stream, option, ' ')) {
		size_t index = option.find("=");
		if (index == (size_t)-1) {
			Log("Invalid SRT Option: %s", option.c_str());
			continue;
		}
		auto key = option.substr(0, index);
		auto value = option.substr(index + 1);

		if (key == "TSBPDDELAY") {
			int intval = atoi(value.c_str());

			Log("SRT Options: SRTO_%s=%d\n", key.c_str(), intval);
			srt_setsockflag(socket, SRTO_TSBPDDELAY, &intval, sizeof(intval));
		}
		else if (key == "RCVLATENCY") {
			int intval = atoi(value.c_str());

			Log("SRT Options: SRTO_%s=%d\n", key.c_str(), intval);
			srt_setsockflag(socket, SRTO_RCVLATENCY, &intval, sizeof(intval));
		}
		else {
			Log("Unspported SRT Option name: %s\n", key.c_str());
		}
	}
}