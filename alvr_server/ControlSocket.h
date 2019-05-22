#pragma once

#include <memory>
#include <string>
#include <vector>
#include "Poller.h"
#include "Utils.h"

class ControlSocket {
public:
	ControlSocket(std::shared_ptr<Poller> poller);
	~ControlSocket();

	bool Startup();
	bool Accept();
	bool Recv(std::vector<std::string> &commands);

	void CloseClient();

	void Shutdown();

	void SendCommandResponse(const char *commandResponse);
private:
	static const int CONTROL_PORT;
	static const char *CONTROL_HOST;
	std::shared_ptr<Poller> m_Poller;

	SOCKET m_Socket;
	SOCKET m_ClientSocket;

	std::string m_Buf;
};