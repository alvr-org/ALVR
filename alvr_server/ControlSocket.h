#pragma once

#include <WinSock2.h>
#include <memory>
#include <string>
#include <vector>
#include "Poller.h"

class ControlSocket {
public:
	ControlSocket(std::string host, int port, std::shared_ptr<Poller> poller);
	~ControlSocket();

	bool Startup();
	bool Accept();
	bool Recv(std::vector<std::string> &commands);

	void CloseClient();

	void Shutdown();

	void SendCommandResponse(const char *commandResponse);
private:
	std::string m_Host;
	int m_Port;
	std::shared_ptr<Poller> m_Poller;

	SOCKET m_Socket;
	SOCKET m_ClientSocket;

	std::string m_Buf;
};