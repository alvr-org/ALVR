#pragma once

#include "threadtools.h"
#include "Logger.h"
#include <WinSock2.h>
#include <srt.h>
#include <udt.h>

class Listener : public CThread {
public:
#pragma pack(push, 1)
	struct TrackingInfo {
		uint32_t type;
		uint64_t FrameIndex;
		double predictedDisplayTime;
		struct {
			float x;
			float y;
			float z;
			float w;
		} HeadPose_Pose_Orientation;
		struct {
			float x;
			float y;
			float z;
		} HeadPose_Pose_Position;
		struct {
			float x;
			float y;
			float z;
		} HeadPose_AngularVelocity;
		struct {
			float x;
			float y;
			float z;
		} HeadPose_LinearVelocity;
		struct {
			float x;
			float y;
			float z;
		} HeadPose_AngularAcceleration;
		struct {
			float x;
			float y;
			float z;
		} HeadPose_LinearAcceleration;
		struct Matrix {
			float M[16];
		};
		struct {
			Matrix ProjectionMatrix;
			Matrix ViewMatrix;
		}Eye[2];

	};
#pragma pack(pop)

	Listener(std::string host, int port, std::function<void(sockaddr_in *)> callback) : m_bExiting(false) {
		m_Host = host;
		m_Port = port;
		m_LastSeen = 0;
		m_NewClientCallback = callback;
		m_ClientSocket = SRT_INVALID_SOCK;
		memset(&m_TrackingInfo, 0, sizeof(m_TrackingInfo));
	}

	void Run() override
	{
		struct sockaddr_in addr;

		SetThreadPriority(GetCurrentThread(), THREAD_PRIORITY_BELOW_NORMAL);

		int startup = srt_startup();
		Log("srt_startup %d", startup);

		m_Socket = srt_socket(AF_INET, SOCK_DGRAM, 0);
		if (m_Socket == INVALID_SOCKET) {
			Log("Listener: srt_socket creationg failed. Code=%d", srt_getlasterror_str());
			return;
		}

		addr.sin_family = AF_INET;
		addr.sin_port = htons(m_Port);
		inet_pton(AF_INET, m_Host.c_str(), &addr.sin_addr);

		int ret = srt_bind(m_Socket, (struct sockaddr *)&addr, sizeof(addr));
		if (ret < 0) {
			Log("Listener: srt_bind error. Code=%s", srt_getlasterror_str());
			return;
		}
		Log("Listener Successfully bind socket.");

		ret = srt_listen(m_Socket, 10);
		if (ret < 0) {
			Log("Listener: srt_listen error. Code=%s", srt_getlasterror_str());
			return;
		}

		int epoll = srt_epoll_create();

		int flags = SRT_EPOLL_IN | SRT_EPOLL_ERR;
		srt_epoll_add_usock(epoll, m_Socket, &flags);

		while (!m_bExiting)
		{
			struct sockaddr_in client_addr;
			int len = sizeof(client_addr);
			
			SRTSOCKET ClientSocket = srt_accept(m_Socket, (sockaddr *)&client_addr, &len);
			if (ClientSocket == SRT_INVALID_SOCK) {
				break;
			}
			if (m_ClientSocket != SRT_INVALID_SOCK) {
				Log("New client request coming. close old client.");
				srt_close(m_ClientSocket);
			}
			m_ClientSocket = ClientSocket;
			LogNewClient(&client_addr, len);

			m_ConnectedClientAddr = client_addr;

			m_NewClientCallback(&m_ConnectedClientAddr);
			
			srt_epoll_add_usock(epoll, m_ClientSocket, &flags);

			SRTSOCKET read_fds[2];
			int read_n = 2;
			while (1) {
				char buf[2000];
				int ret = srt_epoll_wait(epoll, read_fds, &read_n, NULL, NULL, 1000, NULL, NULL, NULL, NULL);
				if (m_bExiting) {
					break;
				}
				Log("epoll %d", ret);
				if (ret < 0) {
					if (srt_getlasterror(NULL) == SRT_ETIMEOUT) {
						continue;
					}

					Log("epoll error %d %s", ret, srt_getlasterror(NULL), srt_getlasterror_str());
					break;
				}
				if (read_fds[0] == m_Socket) {
					srt_epoll_remove_usock(epoll, m_ClientSocket);
					break;
				}
				if (read_fds[0] == m_ClientSocket) {
					int ret = srt_recv(m_ClientSocket, buf, 2000);
					Log("received data %d %s", ret, srt_getlasterror_str());
					if (ret >= 4) {
						int pos = 0;
						uint32_t type = *(uint32_t*)buf;

						Log("received type %d (%d)", type, sizeof(TrackingInfo));
						if (type == 1 && ret >= sizeof(TrackingInfo)) {
							m_TrackingInfo = *(TrackingInfo *)buf;
							Log("got tracking info %d %f %f %f %f", (int)m_TrackingInfo.FrameIndex,
								m_TrackingInfo.HeadPose_Pose_Orientation.x,
								m_TrackingInfo.HeadPose_Pose_Orientation.y,
								m_TrackingInfo.HeadPose_Pose_Orientation.z,
								m_TrackingInfo.HeadPose_Pose_Orientation.w);
						}
					}
					if (ret <= 0) {
						break;
					}
				}
			}

		}

		srt_epoll_release(epoll);
		srt_close(m_Socket);
	}

	void Send(uint8_t *buf, int len, uint64_t presentationTime, uint64_t frameIndex) {
		uint8_t packetBuffer[2000];

		if (m_ClientSocket == SRT_INVALID_SOCK) {
			return;
		}

		int chunks = (len + PACKET_SIZE - 1) / PACKET_SIZE;
		for (int i = 0; i < chunks; i++) {
			int size = min(PACKET_SIZE, len - (i * PACKET_SIZE));
			int pos = 0;

			*(uint32_t *)packetBuffer = packetCounter;
			pos += sizeof(uint32_t);
			if (i == 0) {
				// Insert presentation time header in first packet.
				*(uint32_t *)packetBuffer |= (1 << 31);
				*(uint64_t *)(packetBuffer + pos) = presentationTime;
				pos += sizeof(uint64_t);
				*(uint64_t *)(packetBuffer + pos) = frameIndex;
				pos += sizeof(uint64_t);
			}
			packetCounter++;

			memcpy(packetBuffer + pos, buf + i * PACKET_SIZE, size);
			pos += size;

			if (i == chunks - 1) {
				// Insert padding so that client can detect end of packet
				memcpy(packetBuffer + pos, "\x00\x00\x00\x02", 4);
				pos += 4;
			}
			int ret = srt_send(m_ClientSocket, (char *)packetBuffer, pos);
			//sendto(m_Socket, (char *)packetBuffer, size + 4, 0, (struct sockaddr *)&m_ConnectedClientAddr, sizeof(m_ConnectedClientAddr));
			if (ret <= 0) {
				Log("srt_send Error %d. %s\n", ret, srt_getlasterror_str());
				if (srt_getlasterror(NULL) == SRT_EINVSOCK) {
					m_ClientSocket = SRT_INVALID_SOCK;
				}
			}
		}
	}

	void Stop()
	{
		m_bExiting = true;
		if (m_ClientSocket != SRT_INVALID_SOCK) {
			srt_close(m_ClientSocket);
		}
		if(m_Socket != SRT_INVALID_SOCK) {
			srt_close(m_Socket);
		}
		Join();
	}

	void LogNewClient(struct sockaddr_in *client_addr, int len)
	{
		char clienthost[NI_MAXHOST];
		char clientservice[NI_MAXSERV];
	
		//getnameinfo((sockaddr *)client_addr, len, clienthost, sizeof(clienthost), clientservice, sizeof(clientservice), NI_NUMERICHOST | NI_NUMERICSERV);
		inet_ntop(client_addr->sin_family, &client_addr->sin_addr, clienthost, NI_MAXHOST);
		Log("New client: %s:%d", clienthost, htons(client_addr->sin_port));
	}

	const TrackingInfo &GetTrackingInfo()const {
		return m_TrackingInfo;
	}

private:
	bool m_bExiting;
	std::string m_Host;
	int m_Port;
	SRTSOCKET m_Socket;
	SRTSOCKET m_ClientSocket;

	// Maximum SRT(or UDP) payload is PACKET_SIZE + 16
	static const int PACKET_SIZE = 1000;

	uint32_t packetCounter = 0;

	sockaddr_in m_ConnectedClientAddr;
	time_t m_LastSeen;
	std::function<void(sockaddr_in *)> m_NewClientCallback;
	TrackingInfo m_TrackingInfo;
};
