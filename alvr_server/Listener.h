#pragma once

#include <iostream>
#include <string>
#include <sstream>
#include <vector>
#include <algorithm>
#include "threadtools.h"
#include "Logger.h"
#include "UdpSocket.h"
#include "Utils.h"
#include "Poller.h"
#include "ControlSocket.h"
#include "packet_types.h"
#include "Settings.h"
#include "Statistics.h"
extern "C" {
#include "reedsolomon/rs.h"
};

class Listener : public CThread {
public:

	Listener();
	~Listener();
	void SetLauncherCallback(std::function<void()> callback);
	void SetCommandCallback(std::function<void(std::string, std::string)> callback);
	void SetPoseUpdatedCallback(std::function<void()> callback);
	void SetNewClientCallback(std::function<void()> callback);
	void SetStreamStartCallback(std::function<void()> callback);
	void SetPacketLossCallback(std::function<void()> callback);
	void SetShutdownCallback(std::function<void()> callback);

	bool Startup();
	void Run() override;
	void FECSend(uint8_t *buf, int len, uint64_t frameIndex, uint64_t videoFrameIndex);
	void SendVideo(uint8_t *buf, int len, uint64_t frameIndex);
	void SendAudio(uint8_t *buf, int len, uint64_t presentationTime);
	void SendHapticsFeedback(uint64_t startTime, float amplitude, float duration, float frequency, uint8_t hand);
	void ProcessRecv(char *buf, int len, sockaddr_in *addr);
	void ProcessCommand(const std::string &commandName, const std::string args);
	void SendChangeSettings();
	void Stop();
	bool HasValidTrackingInfo() const;
	void GetTrackingInfo(TrackingInfo &info);
	uint64_t clientToServerTime(uint64_t clientTime) const;
	uint64_t serverToClientTime(uint64_t serverTime) const;
	void SendCommandResponse(const char *commandResponse);
	void PushRequest(HelloMessage *message, sockaddr_in *addr);
	void SanitizeDeviceName(char deviceName[32]);
	std::string DumpConfig();
	void CheckTimeout();
	void UpdateLastSeen();
	void FindClientName(const sockaddr_in *addr);
	void Connect(const sockaddr_in *addr);
	void Disconnect();
	void OnFecFailure();
	std::shared_ptr<Statistics> GetStatistics();
	bool IsStreaming();
private:
	bool m_bExiting;
	bool m_Enabled;
	std::shared_ptr<Poller> m_Poller;
	std::shared_ptr<UdpSocket> m_Socket;
	std::shared_ptr<ControlSocket> m_ControlSocket;
	std::shared_ptr<Statistics> m_Statistics;

	// Maximum UDP payload
	static const int PACKET_SIZE = 1400;
	static const int64_t REQUEST_TIMEOUT = 5 * 1000 * 1000;
	static const int64_t CONNECTION_TIMEOUT = 5 * 1000 * 1000;

	uint32_t videoPacketCounter = 0;
	uint32_t soundPacketCounter = 0;

	time_t m_LastSeen;
	std::function<void()> m_LauncherCallback;
	std::function<void(std::string, std::string)> m_CommandCallback;
	std::function<void()> m_PoseUpdatedCallback;
	std::function<void()> m_NewClientCallback;
	std::function<void()> m_StreamStartCallback;
	std::function<void()> m_PacketLossCallback;
	std::function<void()> m_ShutdownCallback;
	TrackingInfo m_TrackingInfo;

	uint64_t m_TimeDiff = 0;
	CRITICAL_SECTION m_CS;

	ChangeSettings m_Settings;

	bool m_Connected;
	bool m_Streaming;

	struct Request {
		uint64_t timestamp;
		sockaddr_in address;
		char deviceName[32];
		bool versionOk;
		HelloMessage message;
	};
	std::list<Request> m_Requests;

	std::string m_clientDeviceName;
	TimeSync m_reportedStatistics;
	uint64_t m_lastFecFailure = 0;
	static const uint64_t CONTINUOUS_FEC_FAILURE = 60 * 1000 * 1000;
	static const int INITIAL_FEC_PERCENTAGE = 5;
	static const int MAX_FEC_PERCENTAGE = 10;
	int m_fecPercentage = INITIAL_FEC_PERCENTAGE;

	uint64_t mVideoFrameIndex = 1;
};
