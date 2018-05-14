#pragma once

#include <WinSock2.h>
#include <srt.h>
#include <udt.h>
#include <iostream>
#include <string>
#include <sstream>
#include <vector>
#include "threadtools.h"
#include "Logger.h"
#include "SrtSocket.h"
#include "UdpSocket.h"
#include "Utils.h"

class Listener : public CThread {
public:
#pragma pack(push, 1)
	struct HelloMessage {
		uint32_t type; // 1
		char deviceName[32];
	};
	struct TrackingInfo {
		uint32_t type; // 2
		uint64_t clientTime;
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
		} Eye[2];

	};
	// Client >----(mode 0)----> Server
	// Client <----(mode 1)----< Server
	// Client >----(mode 2)----> Server
	struct TimeSync {
		uint32_t type; // 3
		uint32_t mode; // 0,1,2
		uint64_t sequence;
		uint64_t serverTime;
		uint64_t clientTime;
	};
#pragma pack(pop)

	Listener(std::string host, int port, std::string SrtOptions, std::function<void(sockaddr_in *)> callback, std::function<void()> poseCallback) : m_bExiting(false)
		//, m_Socket(host, port, SrtOptions) {
		{
		m_LastSeen = 0;
		m_NewClientCallback = callback;
		m_PoseUpdatedCallback = poseCallback;
		memset(&m_TrackingInfo, 0, sizeof(m_TrackingInfo));
		m_Socket.reset(new UdpSocket(host, port));
		m_UseUdp = true;
	}

	void Run() override
	{
		SetThreadPriority(GetCurrentThread(), THREAD_PRIORITY_BELOW_NORMAL);

		m_Socket->Startup();
		
		while (!m_bExiting) {
			if (!m_Socket->Poll()) {
				continue;
			}
			char buf[2000];
			int len = sizeof(buf);
			if (m_Socket->Recv(buf, &len)) {
				if (len >= 4) {
					int pos = 0;
					uint32_t type = *(uint32_t*)buf;

					Log("received type %d (%d)", type, sizeof(TrackingInfo));
					if (type == 1 && len >= sizeof(HelloMessage)) {
						HelloMessage *message = (HelloMessage *)buf;
						message->deviceName[31] = 0;
						Log("Hello Message: %s", message->deviceName);
					}
					else if (type == 2 && len >= sizeof(TrackingInfo)) {
						m_TrackingInfo = *(TrackingInfo *)buf;
						
						Log("got tracking info %d %f %f %f %f", (int)m_TrackingInfo.FrameIndex,
							m_TrackingInfo.HeadPose_Pose_Orientation.x,
							m_TrackingInfo.HeadPose_Pose_Orientation.y,
							m_TrackingInfo.HeadPose_Pose_Orientation.z,
							m_TrackingInfo.HeadPose_Pose_Orientation.w);
						m_PoseUpdatedCallback();
					}
					else if (type == 3 && len >= sizeof(TimeSync)) {
						TimeSync *timeSync = (TimeSync*)buf;
						uint64_t Current = GetTimestampUs();

						if (timeSync->mode == 0) {
							TimeSync sendBuf = *timeSync;
							sendBuf.mode = 1;
							sendBuf.serverTime = Current;
							m_Socket->Send((char *)&sendBuf, sizeof(sendBuf));
						}
						else if (timeSync->mode == 2) {
							// Calclate RTT
							uint64_t RTT = Current - timeSync->serverTime;
							// Estimated difference between server and client clock
							uint64_t TimeDiff = Current - (timeSync->clientTime + RTT / 2);
							m_TimeDiff = TimeDiff;
							Log("TimeSync: server - client = %lld us RTT = %lld us", TimeDiff, RTT);
						}
					}
				}
			}

			std::string host;
			int port;
			if (m_Socket->NewClient(host, port)) {
				Log("New client: %s:%d", host.c_str(), port);
				m_NewClientCallback(&m_Socket->GetClientAddr());
			}

		}
	}

	void Send(uint8_t *buf, int len, uint64_t presentationTime, uint64_t frameIndex) {
		uint8_t packetBuffer[2000];

		if (!m_Socket->IsClientValid()) {
			return;
		}
		Log("Sending %d bytes", len);

		int chunks = (len + PACKET_SIZE - 1) / PACKET_SIZE;
		for (int i = 0; i < chunks; i++) {
			int size = min(PACKET_SIZE, len - (i * PACKET_SIZE));
			int pos = 0;

			if (i == 0) {
				*(uint32_t *)packetBuffer = 1;
				pos += sizeof(uint32_t);
				*(uint32_t *)(packetBuffer + pos) = packetCounter;
				pos += sizeof(uint32_t);

				// Insert presentation time header in first packet.
				*(uint64_t *)(packetBuffer + pos) = presentationTime;
				pos += sizeof(uint64_t);
				*(uint64_t *)(packetBuffer + pos) = frameIndex;
				pos += sizeof(uint64_t);
			}else{
				*(uint32_t *)packetBuffer = 2;
				pos += sizeof(uint32_t);
				*(uint32_t *)(packetBuffer + pos) = packetCounter;
				pos += sizeof(uint32_t);
			}
			packetCounter++;

			memcpy(packetBuffer + pos, buf + i * PACKET_SIZE, size);
			pos += size;

			if (i == chunks - 1) {
				// Insert padding so that client can detect end of packet
				memcpy(packetBuffer + pos, "\x00\x00\x00\x02", 4);
				pos += 4;
			}
			int ret = m_Socket->Send((char *)packetBuffer, pos);
			
		}
	}

	void Stop()
	{
		m_bExiting = true;
		m_Socket->Shutdown();
		Join();
	}

	bool HasValidTrackingInfo() const {
		return m_TrackingInfo.type == 2;
	}

	const TrackingInfo &GetTrackingInfo()const {
		return m_TrackingInfo;
	}

	uint64_t clientToServerTime(uint64_t clientTime) const {
		return clientTime + m_TimeDiff;
	}

	uint64_t serverToClientTime(uint64_t serverTime) const {
		return serverTime - m_TimeDiff;
	}

private:
	bool m_bExiting;
	bool m_UseUdp;
	std::shared_ptr<ISocket> m_Socket;

	// Maximum SRT(or UDP) payload is PACKET_SIZE + 16
	static const int PACKET_SIZE = 1000;

	uint32_t packetCounter = 0;

	time_t m_LastSeen;
	std::function<void(sockaddr_in *)> m_NewClientCallback;
	std::function<void()> m_PoseUpdatedCallback;
	TrackingInfo m_TrackingInfo;

	uint64_t m_TimeDiff = 0;
};
