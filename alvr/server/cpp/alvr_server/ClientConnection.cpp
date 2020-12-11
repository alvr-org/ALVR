#include "ClientConnection.h"
#include "Bitrate.h"

ClientConnection::ClientConnection(std::function<void()> streamStartCallback,
	std::function<void()> poseUpdatedCallback, std::function<void()> packetLossCallback)
	: m_bExiting(false)
	, m_Streaming(false)
	, m_LastStatisticsUpdate(0) {
	m_StreamStartCallback = streamStartCallback;
	m_PoseUpdatedCallback = poseUpdatedCallback;
	m_PacketLossCallback = packetLossCallback;

	memset(&m_TrackingInfo, 0, sizeof(m_TrackingInfo));
	InitializeCriticalSection(&m_CS);

	m_Statistics       = std::make_shared<Statistics>();
	m_MicPlayer	       = std::make_shared<MicPlayer>();
	m_ChaperoneUpdater = std::make_shared<ChaperoneUpdater>();

	m_Poller.reset(new Poller());

	reed_solomon_init();
	
	m_Socket = std::make_shared<UdpSocket>(Settings::Instance().m_Host, Settings::Instance().m_Port
		, m_Poller, m_Statistics, Settings::Instance().mThrottlingBitrate);
	m_Socket->Startup();

	sockaddr_in addr;
	addr.sin_family = AF_INET;
	addr.sin_port = htons((u_short)Settings::Instance().m_Port);
	inet_pton(addr.sin_family, Settings::Instance().m_ConnectedClient.c_str(), &(addr.sin_addr));
	Connect(&addr);

	// Start thread.
	Start();

	m_Streaming = true;
}

ClientConnection::~ClientConnection() {
	DeleteCriticalSection(&m_CS);
}

void ClientConnection::Run() {
	while (!m_bExiting) {
		if (m_Poller->Do() == 0) {
			if (m_Socket) {
				m_Socket->Run();
			}
			continue;
		}

		if (m_Socket) {
			sockaddr_in addr;
			int addrlen = sizeof(addr);
			char buf[2000];
			int len = sizeof(buf);
			if (m_Socket->Recv(buf, &len, &addr, addrlen)) {
				ProcessRecv(buf, len, &addr);
			}
			m_Socket->Run();
		}

		uint64_t now = GetTimestampUs();
		if (now - m_LastStatisticsUpdate > STATISTICS_TIMEOUT_US)
		{
			Info("#{ \"id\": \"statistics\", \"content\": {"
				 "\"totalPackets\": %llu, "
				 "\"packetRate\": %llu, "
				 "\"packetsLostTotal\": %llu, "
				 "\"packetsLostPerSecond\": %llu, "
				 "\"totalSent\": %llu, "
				 "\"sentRate\": %f, "
				 "\"totalLatency\": %f, "
				 "\"encodeLatency\": %f, "
				 "\"encodeLatencyMax\": %f, "
				 "\"transportLatency\": %f, "
				 "\"decodeLatency\": %f, "
				 "\"fecPercentage\": %d, "
				 "\"fecFailureTotal\": %llu, "
				 "\"fecFailureInSecond\": %llu, "
				 "\"clientFPS\": %d, "
				 "\"serverFPS\": %d"
				 "} }#\n",
				 m_Statistics->GetPacketsSentTotal(),
				 m_Statistics->GetPacketsSentInSecond(),
				 m_reportedStatistics.packetsLostTotal,
				 m_reportedStatistics.packetsLostInSecond,
				 m_Statistics->GetBitsSentTotal() / 8 / 1000 / 1000,
				 m_Statistics->GetBitsSentInSecond() / 1000 / 1000.0,
				 m_reportedStatistics.averageTotalLatency / 1000.0,
				 (double)(m_Statistics->GetEncodeLatencyAverage()) / US_TO_MS,
				 (double)(m_Statistics->GetEncodeLatencyMax()) / US_TO_MS,
				 m_reportedStatistics.averageTransportLatency / 1000.0,
				 m_reportedStatistics.averageDecodeLatency / 1000.0, m_fecPercentage,
				 m_reportedStatistics.fecFailureTotal,
				 m_reportedStatistics.fecFailureInSecond,
				 m_reportedStatistics.fps,
				 m_Statistics->GetFPS());
			
			m_LastStatisticsUpdate = now;
		};
	}
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
			m_Socket->Send((char *)packetBuffer, sizeof(VideoFrame) + copyLength, frameIndex);
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
			m_Socket->Send((char *)packetBuffer, sizeof(VideoFrame) + copyLength, frameIndex);
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
	if (!m_Socket->IsClientValid()) {
		Debug("Skip sending packet because client is not connected. Packet Length=%d FrameIndex=%llu\n", len, frameIndex);
		return;
	}
	if (!m_Streaming) {
		Debug("Skip sending packet because streaming is off.\n");
		return;
	}
	FECSend(buf, len, frameIndex, mVideoFrameIndex);
	mVideoFrameIndex++;
}

void ClientConnection::SendAudio(uint8_t *buf, int len, uint64_t presentationTime) {
	uint8_t packetBuffer[2000];

	if (!m_Socket->IsClientValid()) {
		Debug("Skip sending audio packet because client is not connected. Packet Length=%d\n", len);
		return;
	}
	if (!m_Streaming) {
		Debug("Skip sending audio packet because streaming is off.\n");
		return;
	}
	Debug("Sending audio frame. Size=%d bytes\n", len);

	int remainBuffer = len;
	for (int i = 0; remainBuffer != 0; i++) {
		int pos = 0;

		if (i == 0) {
			// First fragment
			auto header = (AudioFrameStart *)packetBuffer;

			header->type = ALVR_PACKET_TYPE_AUDIO_FRAME_START;
			header->packetCounter = soundPacketCounter;
			header->presentationTime = presentationTime;
			header->frameByteSize = len;

			pos = sizeof(*header);
		}
		else {
			// Following fragments
			auto header = (AudioFrame *)packetBuffer;

			header->type = ALVR_PACKET_TYPE_AUDIO_FRAME;
			header->packetCounter = soundPacketCounter;

			pos = sizeof(*header);
		}

		int size = std::min(PACKET_SIZE - pos, remainBuffer);

		memcpy(packetBuffer + pos, buf + (len - remainBuffer), size);
		pos += size;
		remainBuffer -= size;

		soundPacketCounter++;

		m_Socket->Send((char *)packetBuffer, pos);

	}
}

void ClientConnection::SendHapticsFeedback(uint64_t startTime, float amplitude, float duration, float frequency, uint8_t hand)
{
	if (!m_Socket->IsClientValid()) {
		Debug("Skip sending haptics packet because client is not connected.\n");
		return;
	}
	if (!m_Streaming) {
		Debug("Skip sending haptics packet because streaming is off.\n");
		return;
	}
	Debug("Sending haptics feedback. startTime=%llu amplitude=%f duration=%f frequency=%f\n", startTime, amplitude, duration, frequency);

	HapticsFeedback packetBuffer;
	packetBuffer.type = ALVR_PACKET_TYPE_HAPTICS;
	packetBuffer.startTime = startTime;
	packetBuffer.amplitude = amplitude;
	packetBuffer.duration = duration;
	packetBuffer.frequency = frequency;
	packetBuffer.hand = hand;
	m_Socket->Send((char *)&packetBuffer, sizeof(HapticsFeedback));
}

void ClientConnection::ProcessRecv(char *buf, int len, sockaddr_in *addr) {
	if (len < 4) {
		return;
	}
	uint32_t type = *(uint32_t*)buf;

	Debug("Received packet. Type=%d\n", type);
	if (type == ALVR_PACKET_TYPE_TRACKING_INFO && len >= sizeof(TrackingInfo)) {
		if (!m_Socket->IsLegitClient(addr)) {
			Debug("Recieved message from invalid address: %hs\n", AddrPortToStr(addr).c_str());
			return;
		}

		EnterCriticalSection(&m_CS);
		m_TrackingInfo = *(TrackingInfo *)buf;
		LeaveCriticalSection(&m_CS);

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
		if (!m_Socket->IsLegitClient(addr)) {
			Debug("Recieved message from invalid address: %hs\n", AddrPortToStr(addr).c_str());
			return;
		}

		TimeSync *timeSync = (TimeSync*)buf;
		uint64_t Current = GetTimestampUs();

		if (timeSync->mode == 0) {
			m_reportedStatistics = *timeSync;
			TimeSync sendBuf = *timeSync;
			sendBuf.mode = 1;
			sendBuf.serverTime = Current;
			m_Socket->Send((char *)&sendBuf, sizeof(sendBuf), 0);

			if (timeSync->fecFailure) {
				OnFecFailure();
			}
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
	else if (type == ALVR_PACKET_TYPE_STREAM_CONTROL_MESSAGE && len >= sizeof(StreamControlMessage)) {
		if (!m_Socket->IsLegitClient(addr)) {
			auto saddr = AddrPortToStr(addr);
			Debug("Recieved message from invalid address: %s:%d\n", saddr);
			return;
		}
		StreamControlMessage *streamControl = (StreamControlMessage*)buf;

		if (streamControl->mode == 1) {
			Debug("Stream control message: Start stream.\n");
			m_Streaming = true;
			m_StreamStartCallback();
		}
		else if (streamControl->mode == 2) {
			Debug("Stream control message: Stop stream.\n");
			m_Streaming = false;
		}
	}
	else if (type == ALVR_PACKET_TYPE_PACKET_ERROR_REPORT && len >= sizeof(PacketErrorReport)) {
		if (!m_Socket->IsLegitClient(addr)) {
			Debug("Recieved message from invalid address: %hs\n", AddrPortToStr(addr).c_str());
			return;
		}
		auto *packetErrorReport = (PacketErrorReport *)buf;
		Debug("Packet loss was reported. Type=%d %lu - %lu\n", packetErrorReport->lostFrameType, packetErrorReport->fromPacketCounter, packetErrorReport->toPacketCounter);
		if (packetErrorReport->lostFrameType == ALVR_LOST_FRAME_TYPE_VIDEO) {
			// Recover video frame.
			OnFecFailure();
		}
	}
	else if (type == ALVR_PACKET_TYPE_MIC_AUDIO && len >= sizeof(MicAudioFrame)) {
		if (!m_Socket->IsLegitClient(addr)) {
			Debug("Recieved message from invalid address: %hs\n", AddrPortToStr(addr).c_str());
			return;
		}
		auto *frame = (MicAudioFrame *)buf;
		Debug("Got MicAudio Frame with length - %zu  %zu index: %i\n", frame->outputBufferNumElements, frame->completeSize, frame->packetIndex);

		m_MicPlayer->playAudio( (char*)frame->micBuffer , (int)(sizeof(int16_t)  *  frame->outputBufferNumElements));
	
	}
	else if (type == ALVR_PACKET_TYPE_GUARDIAN_SYNC_START && len >= sizeof(GuardianSyncStart)) {
		if (!m_Socket->IsLegitClient(addr)) {
			Debug("Recieved message from invalid address: %hs\n", AddrPortToStr(addr).c_str());
			return;
		}

		auto* gsync = (GuardianSyncStart*)buf;

		if (gsync->timestamp <= m_ChaperoneUpdater->GetDataTimestamp()) {
			return; // Ignore old data
		}

		GuardianSyncStartAck ack;
		ack.type = ALVR_PACKET_TYPE_GUARDIAN_SYNC_ACK;
		ack.timestamp = gsync->timestamp;
		m_Socket->Send((char*)&ack, sizeof(ack), 0);

		Debug("Starting Guardian sync - total points: %i\n", gsync->totalPointCount);

		m_ChaperoneUpdater->ResetData(gsync->timestamp, gsync->totalPointCount);
		m_ChaperoneUpdater->SetTransform(gsync->standingPosPosition, gsync->standingPosRotation, gsync->playAreaSize);

		if (gsync->totalPointCount <= 0) {
			m_ChaperoneUpdater->GenerateStandingChaperone();
			m_ChaperoneUpdater->MaybeCommitData();
		}
	}
	else if (type == ALVR_PACKET_TYPE_GUARDIAN_SEGMENT_DATA && len >= sizeof(GuardianSegmentData)) {
		if (!m_Socket->IsLegitClient(addr)) {
			Debug("Recieved message from invalid address: %hs\n", AddrPortToStr(addr).c_str());
			return;
		}

		auto* gsegment = (GuardianSegmentData*)buf;

		if (gsegment->timestamp != m_ChaperoneUpdater->GetDataTimestamp()) {
			return; // Ignore old data
		}

		GuardianSegmentAck ack;
		ack.type = ALVR_PACKET_TYPE_GUARDIAN_SEGMENT_ACK;
		ack.timestamp = gsegment->timestamp;
		ack.segmentIndex = gsegment->segmentIndex;
		m_Socket->Send((char*)&ack, sizeof(ack), 0);

		Debug("Received Guardian sync segment - index: %i\n", gsegment->segmentIndex);

		m_ChaperoneUpdater->SetSegment(gsegment->segmentIndex, gsegment->points);

		if (gsegment->segmentIndex >= m_ChaperoneUpdater->GetSegmentCount() - 1) {
			if (m_ChaperoneUpdater->MaybeCommitData()) {
				Info("Synced Guardian data to SteamVR Chaperone.\n");
			}
		}
	}
}

void ClientConnection::Stop()
{
	Debug("Listener::Stop()\n");
	m_bExiting = true;

	if (m_Socket) {
		m_Socket->Shutdown();
	}
	Join();
}

bool ClientConnection::HasValidTrackingInfo() const {
	return m_TrackingInfo.type == ALVR_PACKET_TYPE_TRACKING_INFO;
}

void ClientConnection::GetTrackingInfo(TrackingInfo &info) {
	EnterCriticalSection(&m_CS);
	info = m_TrackingInfo;
	LeaveCriticalSection(&m_CS);
}

uint64_t ClientConnection::clientToServerTime(uint64_t clientTime) const {
	return clientTime + m_TimeDiff;
}

uint64_t ClientConnection::serverToClientTime(uint64_t serverTime) const {
	return serverTime - m_TimeDiff;
}

void ClientConnection::Connect(const sockaddr_in *addr) {
	Debug("Connected to %hs\n", AddrPortToStr(addr).c_str());

	m_Socket->SetClientAddr(addr);
	videoPacketCounter = 0;
	soundPacketCounter = 0;
	m_fecPercentage = INITIAL_FEC_PERCENTAGE;
	memset(&m_reportedStatistics, 0, sizeof(m_reportedStatistics));
	m_Statistics->ResetAll();
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

bool ClientConnection::IsStreaming() {
	return m_Streaming;
}
