#include "ControlSocket.h"
#include "Logger.h"

const int ControlSocket::CONTROL_PORT = 9944;
const char *ControlSocket::CONTROL_HOST = "127.0.0.1";

ControlSocket::ControlSocket(std::shared_ptr<Poller> poller)
	: mPoller(poller)
	, mSocket(INVALID_SOCKET)
	, mClientSocket(INVALID_SOCKET)
{
}

ControlSocket::~ControlSocket() {
}

bool ControlSocket::Startup() {
	WSADATA wsaData;

	WSAStartup(MAKEWORD(2, 0), &wsaData);

	mSocket = socket(AF_INET, SOCK_STREAM, 0);
	if (mSocket == INVALID_SOCKET) {
		FatalLog(L"ControlSocket::Startup socket error : %d", WSAGetLastError());
		return false;
	}

	int val = 1;
	setsockopt(mSocket, SOL_SOCKET, SO_REUSEADDR, (const char *)&val, sizeof(val));

	sockaddr_in addr;
	addr.sin_family = AF_INET;
	addr.sin_port = htons(CONTROL_PORT);

	inet_pton(AF_INET, CONTROL_HOST, &addr.sin_addr);

	if (bind(mSocket, (sockaddr *)&addr, sizeof(addr))) {
		FatalLog(L"ControlSocket::Startup bind error : %d", WSAGetLastError());
		return false;
	}

	if (listen(mSocket, 10)) {
		FatalLog(L"ControlSocket::Startup listen error : %d", WSAGetLastError());
		return false;
	}

	Log(L"ControlSocket::Startup Successfully bound to %hs:%d", CONTROL_HOST, CONTROL_PORT);

	mPoller->AddSocket(mSocket, PollerSocketType::READ);

	return true;
}


bool ControlSocket::Accept() {
	if (!mPoller->IsPending(mSocket, PollerSocketType::READ)) {
		return false;
	}

	sockaddr_in addr;
	int len = sizeof(addr);
	SOCKET s = accept(mSocket, (sockaddr *)&addr, &len);
	uint32_t local_addr;
	inet_pton(AF_INET, "127.0.0.1", &local_addr);
	if (addr.sin_addr.S_un.S_addr != local_addr) {
		// block connection
		closesocket(s);
		return false;
	}

	if (mClientSocket != INVALID_SOCKET) {
		Log(L"Closing old control client");
		mBuf = "";
		CloseClient();
	}

	mClientSocket = s;
	mPoller->AddSocket(mClientSocket, PollerSocketType::READ);

	return true;
}

bool ControlSocket::Recv(std::vector<std::string> &commands) {
	if (mClientSocket == INVALID_SOCKET || !mPoller->IsPending(mClientSocket, PollerSocketType::READ)) {
		return false;
	}

	Log(L"ControlSocket::Recv(). recv");

	char buf[1000];
	int ret = recv(mClientSocket, buf, sizeof(buf) - 1, 0);
	Log(L"ControlSocket::Recv(). recv leave: ret=%d", ret);
	if (ret == 0) {
		Log(L"Control connection has closed");
		mBuf = "";
		CloseClient();
		return false;
	}
	if (ret < 0) {
		Log(L"Error on recv. close control client: %d", WSAGetLastError());
		mBuf = "";
		CloseClient();
		return false;
	}
	buf[ret] = 0;
	mBuf += buf;

	Log(L"ControlSocket::Recv(). while");
	size_t index;
	while ((index = mBuf.find("\n")) != std::string::npos) {
		commands.push_back(mBuf.substr(0, index));
		mBuf.replace(0, index + 1, "");
	}
	return commands.size() > 0;
}


void ControlSocket::CloseClient() {
	if (mClientSocket != INVALID_SOCKET) {
		mPoller->RemoveSocket(mClientSocket, PollerSocketType::READ);
		closesocket(mClientSocket);
		mClientSocket = INVALID_SOCKET;
	}
}

void ControlSocket::Shutdown() {
	CloseClient();
	if (mSocket != INVALID_SOCKET) {
		mPoller->RemoveSocket(mSocket, PollerSocketType::READ);
		closesocket(mSocket);
		mSocket = INVALID_SOCKET;
	}
}

void ControlSocket::SendCommandResponse(const char * commandResponse)
{
	if (mClientSocket != INVALID_SOCKET) {
		// Send including NULL.
		send(mClientSocket, commandResponse, static_cast<int>(strlen(commandResponse)) + 1, 0);
	}
}
