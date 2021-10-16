#include "ClientConnection.h"
#include <mutex>
#include <string.h>

#include "Statistics.h"
#include "Logger.h"
#include "bindings.h"
#include "Utils.h"
#include "Settings.h"

ClientConnection::ClientConnection(
	std::function<void()> poseUpdatedCallback,
	std::function<void()> packetLossCallback)
	: m_bExiting(false)
	, m_LastStatisticsUpdate(0) {
	m_PoseUpdatedCallback = poseUpdatedCallback;
	m_PacketLossCallback = packetLossCallback;

	m_TrackingInfo = {};

	m_Statistics = std::make_shared<Statistics>();

	reed_solomon_init();
	
	videoPacketCounter = 0;
	soundPacketCounter = 0;
	m_fecPercentage = INITIAL_FEC_PERCENTAGE;
	memset(&m_reportedStatistics, 0, sizeof(m_reportedStatistics));
	m_Statistics->ResetAll();
}

ClientConnection::~ClientConnection() {
}

void ClientConnection::FECSend(uint8_t *buf, int len, uint64_t frameIndex, uint64_t videoFrameIndex) {
	int shardPackets = CalculateFECShardPackets(len, m_fecPercentage);

	int blockSize = shardPackets * ALVR_MAX_VIDEO_BUFFER_SIZE;

	int dataShards = (len + blockSize - 1) / blockSize;
	int totalParityShards = CalculateParityShards(dataShards, m_fecPercentage);
	int totalShards = dataShards + totalParityShards;

	assert(totalShards <= DATA_SHARDS_MAX);

	Debug("reed_solomon_new. dataShards=%d totalParityShards=%d totalShards=%d blockSize=%d shardPackets=%d\n"
		, dataShards, totalParityShards, totalShards, blockSize, shardPackets);

	reed_solomon *rs = reed_solomon_new(dataShards, totalParityShards);

	std::vector<uint8_t *> shards(totalShards);

	for (int i = 0; i < dataShards; i++) {
		shards[i] = buf + i * blockSize;
	}
	if (len % blockSize != 0) {
		// Padding
		shards[dataShards - 1] = new uint8_t[blockSize];
		memset(shards[dataShards - 1], 0, blockSize);
		memcpy(shards[dataShards - 1], buf + (dataShards - 1) * blockSize, len % blockSize);
	}
	for (int i = 0; i < totalParityShards; i++) {
		shards[dataShards + i] = new uint8_t[blockSize];
	}

	int ret = reed_solomon_encode(rs, &shards[0], totalShards, blockSize);
	assert(ret == 0);

	reed_solomon_release(rs);

	uint8_t packetBuffer[2000];
	VideoFrame *header = (VideoFrame *)packetBuffer;
	uint8_t *payload = packetBuffer + sizeof(VideoFrame);
	int dataRemain = len;

	Debug("Sending video frame. trackingFrameIndex=%llu videoFrameIndex=%llu size=%d\n", frameIndex, videoFrameIndex, len);

	header->type = ALVR_PACKET_TYPE_VIDEO_FRAME;
	header->trackingFrameIndex = frameIndex;
	header->videoFrameIndex = videoFrameIndex;
	header->sentTime = GetTimestampUs();
	header->frameByteSize = len;
	header->fecIndex = 0;
	header->fecPercentage = (uint16_t)m_fecPercentage;
	for (int i = 0; i < dataShards; i++) {
		for (int j = 0; j < shardPackets; j++) {
			int copyLength = std::min(ALVR_MAX_VIDEO_BUFFER_SIZE, dataRemain);
			if (copyLength <= 0) {
				break;
			}
			memcpy(payload, shards[i] + j * ALVR_MAX_VIDEO_BUFFER_SIZE, copyLength);
			dataRemain -= ALVR_MAX_VIDEO_BUFFER_SIZE;

			header->packetCounter = videoPacketCounter;
			videoPacketCounter++;
			LegacySend((unsigned char *)packetBuffer, sizeof(VideoFrame) + copyLength);
			m_Statistics->CountPacket(sizeof(VideoFrame) + copyLength);
			header->fecIndex++;
		}
	}
	header->fecIndex = dataShards * shardPackets;
	for (int i = 0; i < totalParityShards; i++) {
		for (int j = 0; j < shardPackets; j++) {
			int copyLength = ALVR_MAX_VIDEO_BUFFER_SIZE;
			memcpy(payload, shards[dataShards + i] + j * ALVR_MAX_VIDEO_BUFFER_SIZE, copyLength);

			header->packetCounter = videoPacketCounter;
			videoPacketCounter++;
			
			LegacySend((unsigned char *)packetBuffer, sizeof(VideoFrame) + copyLength);
			m_Statistics->CountPacket(sizeof(VideoFrame) + copyLength);
			header->fecIndex++;
		}
	}

	if (len % blockSize != 0) {
		delete[] shards[dataShards - 1];
	}
	for (int i = 0; i < totalParityShards; i++) {
		delete[] shards[dataShards + i];
	}
}

void ClientConnection::SendVideo(uint8_t *buf, int len, uint64_t frameIndex) {
	FECSend(buf, len, frameIndex, mVideoFrameIndex);
	mVideoFrameIndex++;
}

void ClientConnection::SendHapticsFeedback(uint64_t startTime, float amplitude, float duration, float frequency, uint8_t hand)
{
	Debug("Sending haptics feedback. startTime=%llu amplitude=%f duration=%f frequency=%f\n", startTime, amplitude, duration, frequency);

	if (duration < m_hapticsMinDuration * 0.5)
		duration = m_hapticsMinDuration * 0.5;

	HapticsFeedback packetBuffer;
	packetBuffer.type = ALVR_PACKET_TYPE_HAPTICS;
	packetBuffer.startTime = startTime;
	// packetBuffer.amplitude = pow(amplitude * ((m_hapticsLowDurationAmplitudeMultiplier - 1) * m_hapticsMinDuration / (pow(m_hapticsMinDuration, 2) * 0.25 / duration + duration) + 1), 1 - m_hapticsAmplitudeCurve);
	packetBuffer.amplitude = pow(amplitude * ((m_hapticsLowDurationAmplitudeMultiplier - 1) * m_hapticsMinDuration * m_hapticsLowDurationRange / (pow(m_hapticsMinDuration * m_hapticsLowDurationRange, 2) * 0.25 / (duration - 0.5 * m_hapticsMinDuration * (1 - m_hapticsLowDurationRange)) + (duration - 0.5 * m_hapticsMinDuration * (1 - m_hapticsLowDurationRange))) + 1), 1 - m_hapticsAmplitudeCurve);
	packetBuffer.duration = pow(m_hapticsMinDuration, 2) * 0.25 / duration + duration;
	packetBuffer.frequency = frequency;
	packetBuffer.hand = hand;
	LegacySend((unsigned char *)&packetBuffer, sizeof(HapticsFeedback));
}

void ClientConnection::ProcessRecv(unsigned char *buf, size_t len) {
	m_Statistics->CountPacket(len);

	uint32_t type = *(uint32_t*)buf;

	if (type == ALVR_PACKET_TYPE_TRACKING_INFO && len >= sizeof(TrackingInfo)) {
		uint64_t Current = GetTimestampUs();
		TimeSync sendBuf = {};
		sendBuf.type = ALVR_PACKET_TYPE_TIME_SYNC;
		sendBuf.mode = 3;
		sendBuf.serverTime = serverToClientTime(Current);
		sendBuf.trackingRecvFrameIndex = m_TrackingInfo.FrameIndex;
		LegacySend((unsigned char *)&sendBuf, sizeof(sendBuf));

		{
			std::unique_lock lock(m_CS);
			m_TrackingInfo = *(TrackingInfo *)buf;
		}

		// if 3DOF, zero the positional data!
		if (Settings::Instance().m_force3DOF) {
			m_TrackingInfo.HeadPose_Pose_Position.x = 0;
			m_TrackingInfo.HeadPose_Pose_Position.y = 0;
			m_TrackingInfo.HeadPose_Pose_Position.z = 0;
		}
		Debug("got battery level: %d\n", (int)m_TrackingInfo.battery);
		Debug("got tracking info %d %f %f %f %f\n", (int)m_TrackingInfo.FrameIndex,
			m_TrackingInfo.HeadPose_Pose_Orientation.x,
			m_TrackingInfo.HeadPose_Pose_Orientation.y,
			m_TrackingInfo.HeadPose_Pose_Orientation.z,
			m_TrackingInfo.HeadPose_Pose_Orientation.w);
		m_PoseUpdatedCallback();
	}
	else if (type == ALVR_PACKET_TYPE_TIME_SYNC && len >= sizeof(TimeSync)) {
		TimeSync *timeSync = (TimeSync*)buf;
		uint64_t Current = GetTimestampUs();

		if (timeSync->mode == 0) {
			//timings might be a little incorrect since it is a mix from a previous sent frame and latest frame

			vr::Compositor_FrameTiming timing[2];
			timing[0].m_nSize = sizeof(vr::Compositor_FrameTiming);
			vr::VRServerDriverHost()->GetFrameTimings(&timing[0], 2);

			m_Statistics->NetworkSend(m_reportedStatistics.averageTransportLatency);

			m_reportedStatistics = *timeSync;
			TimeSync sendBuf = *timeSync;
			sendBuf.mode = 1;
			sendBuf.serverTime = Current;
			sendBuf.serverTotalLatency = (int)(m_reportedStatistics.averageSendLatency + (timing[0].m_flPreSubmitGpuMs + timing[0].m_flPostSubmitGpuMs + timing[0].m_flTotalRenderGpuMs + timing[0].m_flCompositorRenderGpuMs + timing[0].m_flCompositorRenderCpuMs + timing[0].m_flCompositorIdleCpuMs + timing[0].m_flClientFrameIntervalMs + timing[0].m_flPresentCallCpuMs + timing[0].m_flWaitForPresentCpuMs + timing[0].m_flSubmitFrameMs) * 1000 + m_Statistics->GetEncodeLatencyAverage() + m_reportedStatistics.averageTransportLatency + m_reportedStatistics.averageDecodeLatency);
			LegacySend((unsigned char *)&sendBuf, sizeof(sendBuf));

			float renderTime = timing[0].m_flPreSubmitGpuMs + timing[0].m_flPostSubmitGpuMs + timing[0].m_flTotalRenderGpuMs + timing[0].m_flCompositorRenderGpuMs + timing[0].m_flCompositorRenderCpuMs;
			float idleTime = timing[0].m_flCompositorIdleCpuMs;
			float waitTime = timing[0].m_flClientFrameIntervalMs + timing[0].m_flPresentCallCpuMs + timing[0].m_flWaitForPresentCpuMs + timing[0].m_flSubmitFrameMs;

			if (timeSync->fecFailure) {
				OnFecFailure();
			}

			m_Statistics->Add(sendBuf.serverTotalLatency / 1000.0, 
				(double)(m_Statistics->GetEncodeLatencyAverage()) / US_TO_MS,
				m_reportedStatistics.averageTransportLatency / 1000.0,
				m_reportedStatistics.averageDecodeLatency / 1000.0,
				m_reportedStatistics.fps);

			uint64_t now = GetTimestampUs();
			if (now - m_LastStatisticsUpdate > STATISTICS_TIMEOUT_US)
			{
				// Text statistics only, some values averaged
				Info("#{ \"id\": \"Statistics\", \"data\": {"
					"\"totalPackets\": %llu, "
					"\"packetRate\": %llu, "
					"\"packetsLostTotal\": %llu, "
					"\"packetsLostPerSecond\": %llu, "
					"\"totalSent\": %llu, "
					"\"sentRate\": %.3f, "
					"\"bitrate\": %llu, "
					"\"totalLatency\": %.3f, "
					"\"encodeLatency\": %.3f, "
					"\"sendLatency\": %.3f, "
					"\"decodeLatency\": %.3f, "
					"\"fecPercentage\": %d, "
					"\"fecFailureTotal\": %llu, "
					"\"fecFailureInSecond\": %llu, "
					"\"clientFPS\": %.3f, "
					"\"serverFPS\": %.3f"
					"} }#\n",
					m_Statistics->GetPacketsSentTotal(),
					m_Statistics->GetPacketsSentInSecond(),
					m_reportedStatistics.packetsLostTotal,
					m_reportedStatistics.packetsLostInSecond,
					m_Statistics->GetBitsSentTotal() / 8 / 1000 / 1000,
					m_Statistics->GetBitsSentInSecond() / 1000. / 1000.0,
					m_Statistics->GetBitrate(),
					m_Statistics->Get(0),  //totalLatency
					m_Statistics->Get(1),  //encodeLatency
					m_Statistics->Get(2),  //sendLatency
					m_Statistics->Get(3),  //decodeLatency
					m_fecPercentage,
					m_reportedStatistics.fecFailureTotal,
					m_reportedStatistics.fecFailureInSecond,
					m_Statistics->Get(4),  //clientFPS
					m_Statistics->GetFPS());

				m_LastStatisticsUpdate = now;
				m_Statistics->Reset();
			};

			// Continously send statistics info for updating graphs
			Info("#{ \"id\": \"GraphStatistics\", \"data\": [%llu,%.3f,%.3f,%.3f,%.3f,%.3f,%.3f,%.3f,%.3f,%.3f,%.3f] }#\n",
				Current / 1000,                                                //time
				sendBuf.serverTotalLatency / 1000.0,                           //totalLatency
				m_reportedStatistics.averageSendLatency / 1000.0,              //receiveLatency
				renderTime,                                                    //renderTime
				idleTime,                                                      //idleTime
				waitTime,                                                      //waitTime
				(double)(m_Statistics->GetEncodeLatencyAverage()) / US_TO_MS,  //encodeLatency
				m_reportedStatistics.averageTransportLatency / 1000.0,         //sendLatency
				m_reportedStatistics.averageDecodeLatency / 1000.0,            //decodeLatency
				m_reportedStatistics.fps,                                      //clientFPS
				m_Statistics->GetFPS());                                       //serverFPS

		}
		else if (timeSync->mode == 2) {
			// Calclate RTT
			uint64_t RTT = Current - timeSync->serverTime;
			// Estimated difference between server and client clock
			uint64_t TimeDiff = Current - (timeSync->clientTime + RTT / 2);
			m_TimeDiff = TimeDiff;
			Debug("TimeSync: server - client = %lld us RTT = %lld us\n", TimeDiff, RTT);
		}
	}
	else if (type == ALVR_PACKET_TYPE_PACKET_ERROR_REPORT && len >= sizeof(PacketErrorReport)) {
		auto *packetErrorReport = (PacketErrorReport *)buf;
		Debug("Packet loss was reported. Type=%d %lu - %lu\n", packetErrorReport->lostFrameType, packetErrorReport->fromPacketCounter, packetErrorReport->toPacketCounter);
		if (packetErrorReport->lostFrameType == ALVR_LOST_FRAME_TYPE_VIDEO) {
			// Recover video frame.
			OnFecFailure();
		}
	}
}

bool ClientConnection::HasValidTrackingInfo() const {
	return m_TrackingInfo.type == ALVR_PACKET_TYPE_TRACKING_INFO;
}

void ClientConnection::GetTrackingInfo(TrackingInfo &info) {
	std::unique_lock<std::mutex> lock(m_CS);
	info = m_TrackingInfo;
}

uint64_t ClientConnection::clientToServerTime(uint64_t clientTime) const {
	return clientTime + m_TimeDiff;
}

uint64_t ClientConnection::serverToClientTime(uint64_t serverTime) const {
	return serverTime - m_TimeDiff;
}

void ClientConnection::OnFecFailure() {
	Debug("Listener::OnFecFailure()\n");
	if (GetTimestampUs() - m_lastFecFailure < CONTINUOUS_FEC_FAILURE) {
		if (m_fecPercentage < MAX_FEC_PERCENTAGE) {
			m_fecPercentage += 5;
		}
	}
	m_lastFecFailure = GetTimestampUs();
	m_PacketLossCallback();
}

std::shared_ptr<Statistics> ClientConnection::GetStatistics() {
	return m_Statistics;
}
