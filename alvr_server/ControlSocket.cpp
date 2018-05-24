#include <WS2tcpip.h>

#include "ControlSocket.h"
#include "Logger.h"

ControlSocket::ControlSocket(std::string host, int port, std::shared_ptr<Poller> poller) :
	m_Host(host)
	, m_Port(port)
	, m_Poller(poller)
	, m_Socket(INVALID_SOCKET)
	, m_ClientSocket(INVALID_SOCKET)
{

}

ControlSocket::~ControlSocket() {
}

void ControlSocket::Startup() {
	m_Socket = socket(AF_INET, SOCK_STREAM, 0);
	if (m_Socket == INVALID_SOCKET) {
		return;
	}

	sockaddr_in addr;
	addr.sin_family = AF_INET;
	addr.sin_port = htons(m_Port);

	inet_pton(AF_INET, m_Host.c_str(), &addr.sin_addr);

	if (bind(m_Socket, (sockaddr *)&addr, sizeof(addr))) {
		Log("ControlSocket::Startup bind error : %d", WSAGetLastError());
		return;
	}

	if (listen(m_Socket, 10)) {
		Log("ControlSocket::Startup listen error : %d", WSAGetLastError());
		return;
	}

	m_Poller->AddSocket(m_Socket);
}


void ControlSocket::Accept() {
	if (!m_Poller->IsPending(m_Socket)) {
		return;
	}

	sockaddr_in addr;
	int len = sizeof(addr);
	SOCKET s = accept(m_Socket, (sockaddr *)&addr, &len);
	uint32_t local_addr;
	inet_pton(AF_INET, "127.0.0.1", &local_addr);
	if (addr.sin_addr.S_un.S_addr != local_addr) {
		// block connection
		closesocket(s);
		return;
	}

	if (m_ClientSocket != INVALID_SOCKET) {
		Log("Closing old control client");
		m_Buf = "";
		CloseClient();
	}

	m_ClientSocket = s;
	m_Poller->AddSocket(m_ClientSocket);
}

bool ControlSocket::Recv(std::vector<std::string> &commands) {
	if (m_ClientSocket == INVALID_SOCKET || !m_Poller->IsPending(m_ClientSocket)) {
		return false;
	}

	char buf[1000];
	int ret = recv(m_ClientSocket, buf, sizeof(buf), 0);
	if (ret == 0) {
		Log("Control connection has closed");
		m_Buf = "";
		CloseClient();
		return false;
	}
	if (ret < 0) {
		Log("Error on recv. close control client: %d", WSAGetLastError());
		m_Buf = "";
		CloseClient();
		return false;
	}
	buf[ret] = 0;
	m_Buf += buf;

	Log("Control buf: %s", m_Buf.c_str());

	int index;
	while ((index = m_Buf.find("\n")) != -1) {
		commands.push_back(m_Buf.substr(0, index));
		m_Buf.replace(0, index + 1, "");
	}
	return commands.size() > 0;
}


void ControlSocket::CloseClient() {
	if (m_ClientSocket != INVALID_SOCKET) {
		m_Poller->RemoveSocket(m_ClientSocket);
		closesocket(m_ClientSocket);
		m_ClientSocket = INVALID_SOCKET;
	}
}

void ControlSocket::Shutdown() {
	CloseClient();
	if (m_Socket != INVALID_SOCKET) {
		m_Poller->RemoveSocket(m_Socket);
		closesocket(m_Socket);
		m_Socket = INVALID_SOCKET;
	}
}

void ControlSocket::SendCommandResponse(const char * commandResponse)
{
	if (m_ClientSocket != INVALID_SOCKET) {
		send(m_ClientSocket, commandResponse, strlen(commandResponse), 0);
		send(m_ClientSocket, "\nEND\n", 5, 0);
	}
}
