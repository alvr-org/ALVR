#include "ControlSocket.h"
#include "Logger.h"

const int ControlSocket::CONTROL_PORT = 9944;
const char *ControlSocket::CONTROL_HOST = "127.0.0.1";

ControlSocket::ControlSocket(std::shared_ptr<Poller> poller)
	: m_Poller(poller)
	, m_Socket(INVALID_SOCKET)
	, m_ClientSocket(INVALID_SOCKET)
{
}

ControlSocket::~ControlSocket() {
}

bool ControlSocket::Startup() {
	WSADATA wsaData;

	WSAStartup(MAKEWORD(2, 0), &wsaData);

	m_Socket = socket(AF_INET, SOCK_STREAM, 0);
	if (m_Socket == INVALID_SOCKET) {
		FatalLog(L"ControlSocket::Startup socket error : %d", WSAGetLastError());
		return false;
	}

	int val = 1;
	setsockopt(m_Socket, SOL_SOCKET, SO_REUSEADDR, (const char *)&val, sizeof(val));

	sockaddr_in addr;
	addr.sin_family = AF_INET;
	addr.sin_port = htons(CONTROL_PORT);

	inet_pton(AF_INET, CONTROL_HOST, &addr.sin_addr);

	if (bind(m_Socket, (sockaddr *)&addr, sizeof(addr))) {
		FatalLog(L"ControlSocket::Startup bind error : %d", WSAGetLastError());
		return false;
	}

	if (listen(m_Socket, 10)) {
		FatalLog(L"ControlSocket::Startup listen error : %d", WSAGetLastError());
		return false;
	}

	Log(L"ControlSocket::Startup Successfully bound to %hs:%d", CONTROL_HOST, CONTROL_PORT);

	m_Poller->AddSocket(m_Socket, PollerSocketType::READ);

	return true;
}


bool ControlSocket::Accept() {
	if (!m_Poller->IsPending(m_Socket, PollerSocketType::READ)) {
		return false;
	}

	sockaddr_in addr;
	int len = sizeof(addr);
	SOCKET s = accept(m_Socket, (sockaddr *)&addr, &len);
	uint32_t local_addr;
	inet_pton(AF_INET, "127.0.0.1", &local_addr);
	if (addr.sin_addr.S_un.S_addr != local_addr) {
		// block connection
		closesocket(s);
		return false;
	}

	if (m_ClientSocket != INVALID_SOCKET) {
		Log(L"Closing old control client");
		m_Buf = "";
		CloseClient();
	}

	m_ClientSocket = s;
	m_Poller->AddSocket(m_ClientSocket, PollerSocketType::READ);

	return true;
}

bool ControlSocket::Recv(std::vector<std::string> &commands) {
	if (m_ClientSocket == INVALID_SOCKET || !m_Poller->IsPending(m_ClientSocket, PollerSocketType::READ)) {
		return false;
	}

	Log(L"ControlSocket::Recv(). recv");

	char buf[1000];
	int ret = recv(m_ClientSocket, buf, sizeof(buf) - 1, 0);
	Log(L"ControlSocket::Recv(). recv leave: ret=%d", ret);
	if (ret == 0) {
		Log(L"Control connection has closed");
		m_Buf = "";
		CloseClient();
		return false;
	}
	if (ret < 0) {
		Log(L"Error on recv. close control client: %d", WSAGetLastError());
		m_Buf = "";
		CloseClient();
		return false;
	}
	buf[ret] = 0;
	m_Buf += buf;

	Log(L"ControlSocket::Recv(). while");
	size_t index;
	while ((index = m_Buf.find("\n")) != std::string::npos) {
		commands.push_back(m_Buf.substr(0, index));
		m_Buf.replace(0, index + 1, "");
	}
	return commands.size() > 0;
}


void ControlSocket::CloseClient() {
	if (m_ClientSocket != INVALID_SOCKET) {
		m_Poller->RemoveSocket(m_ClientSocket, PollerSocketType::READ);
		closesocket(m_ClientSocket);
		m_ClientSocket = INVALID_SOCKET;
	}
}

void ControlSocket::Shutdown() {
	CloseClient();
	if (m_Socket != INVALID_SOCKET) {
		m_Poller->RemoveSocket(m_Socket, PollerSocketType::READ);
		closesocket(m_Socket);
		m_Socket = INVALID_SOCKET;
	}
}

void ControlSocket::SendCommandResponse(const char * commandResponse)
{
	if (m_ClientSocket != INVALID_SOCKET) {
		// Send including NULL.
		send(m_ClientSocket, commandResponse, static_cast<int>(strlen(commandResponse)) + 1, 0);
	}
}
