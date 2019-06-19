#pragma once

#include <iostream>
#include <string>
#include <sstream>
#include <vector>
#include <algorithm>
#include "openvr-utils\threadtools.h"
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

	bool Startup();
	void Run() override;
	void FECSend(uint8_t *buf, int len, uint64_t videoFrameIndex, uint64_t trackingFrameIndex);
	void SendVideo(uint8_t *buf, int len, uint64_t videoFrameIndex, uint64_t trackingFrameIndex);
	bool GetFirstBufferedFrame(uint64_t *videoFrameIndex);
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
	void OnFecFailure(uint64_t startFrame, uint64_t endFrame);
	std::shared_ptr<Statistics> GetStatistics();
	bool IsStreaming();

	class Callback {
	public:
		virtual void OnLauncher() {};
		virtual void OnCommand(std::string commandName, std::string args) {};
		virtual void OnPoseUpdated() {};
		virtual void OnNewClient() {};
		virtual void OnStreamStart() {};
		virtual void OnFrameAck(bool result, bool isIDR, uint64_t, uint64_t) {};
		virtual void OnShutdown() {};
	};

	void SetCallback(Callback *callback);
private:
	bool mExiting;
	bool mEnabled;
	std::shared_ptr<Poller> mPoller;
	std::shared_ptr<UdpSocket> mSocket;
	std::shared_ptr<ControlSocket> mControlSocket;
	std::shared_ptr<Statistics> mStatistics;
	Callback mNullCallback;
	Callback *mCallback = &mNullCallback;

	// Maximum UDP payload
	static const int PACKET_SIZE = 1400;
	static const int64_t REQUEST_TIMEOUT = 5 * 1000 * 1000;
	static const int64_t CONNECTION_TIMEOUT = 5 * 1000 * 1000;

	uint32_t mVideoPacketCounter = 0;
	uint32_t mSoundPacketCounter = 0;

	time_t mLastSeen;
	TrackingInfo mTrackingInfo;

	uint64_t mTimeDiff = 0;
	IPCCriticalSection mCS;

	ChangeSettings mSettings;

	enum State {
		NOT_CONNECTED,
		CONNECTED,
		STREAMING
	};
	State mState;
	bool IsConnected() { return mState != NOT_CONNECTED; }

	struct Request {
		uint64_t timestamp;
		sockaddr_in address;
		char deviceName[32];
		bool versionOk;
		HelloMessage message;
	};
	std::list<Request> mRequests;

	std::string mClientDeviceName;
	TimeSync mReportedStatistics;
	uint64_t mLastFecFailure = 0;
	static const uint64_t CONTINUOUS_FEC_FAILURE = 60 * 1000 * 1000;
	static const int INITIAL_FEC_PERCENTAGE = 5;
	static const int MAX_FEC_PERCENTAGE = 30;
	int mFecPercentage = INITIAL_FEC_PERCENTAGE;
};
