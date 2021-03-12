#pragma once

#include <atomic>
#include "alvr_server/IDRScheduler.h"
#include "shared/threadtools.h"

class ClientConnection;

struct AVBufferRef;
struct AVCodec;
struct TrakingInfo;

class CEncoder : public CThread
{
public:
	CEncoder(std::shared_ptr<ClientConnection> listener);
	~CEncoder();
	bool Init() override { return true; }
	void Run() override;

	void Stop();
	void OnPacketLoss();
	void InsertIDR();

	void OnPoseUpdated(const TrackingInfo &info);
private:
	std::shared_ptr<ClientConnection> m_listener;
	std::atomic_bool m_exiting{false};
	IDRScheduler m_scheduler;
	std::atomic<uint64_t> m_lastPoseFrame;
};
