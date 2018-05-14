#pragma once

class ISocket {
public:
	virtual bool Startup() = 0;
	virtual bool Poll() = 0;
	virtual bool NewClient(std::string &host, int &port) = 0;
	virtual bool Recv(char *buf, int *buflen) = 0;
	virtual bool Send(char *buf, int len) = 0;
	virtual void Shutdown() = 0;
	virtual bool IsClientValid()const = 0;
	virtual sockaddr_in GetClientAddr()const = 0;
};