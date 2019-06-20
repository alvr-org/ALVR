#include "Listener.h"
#include "Bitrate.h"

Listener::Listener()
	: mExiting(false)
	, mEnabled(false)
	, mState(State::NOT_CONNECTED)
	, mLastSeen(0) {
	memset(&mTrackingInfo, 0, sizeof(mTrackingInfo));

	mStatistics = std::make_shared<Statistics>();

	mSettings.type = ALVR_PACKET_TYPE_CHANGE_SETTINGS;
	mSettings.debugFlags = 0;
	mSettings.suspend = 0;

	mPoller.reset(new Poller());
	mControlSocket.reset(new ControlSocket(mPoller));

	reed_solomon_init();
}

Listener::~Listener() {
}

bool Listener::Startup() {
	if (!mControlSocket->Startup()) {
		return false;
	}
	if (Settings::Instance().IsLoaded()) {
		mEnabled = true;
		mSocket = std::make_shared<UdpSocket>(Settings::Instance().mHost, Settings::Instance().mPort
			, mPoller, mStatistics, Settings::Instance().mThrottlingBitrate);
		if (!mSocket->Startup()) {
			return false;
		}
	}
	// Start thread.
	Start();
	return true;
}

void Listener::Run() {
	while (!mExiting) {
		CheckTimeout();
		if (mPoller->Do() == 0) {
			if (mSocket) {
				mSocket->Run();
			}
			continue;
		}

		if (mSocket) {
			sockaddr_in addr;
			int addrlen = sizeof(addr);
			char buf[2000];
			int len = sizeof(buf);
			if (mSocket->Recv(buf, &len, &addr, addrlen)) {
				ProcessRecv(buf, len, &addr);
			}
			mSocket->Run();
		}

		if (mControlSocket->Accept()) {
			if (!mEnabled) {
				mEnabled = true;
				Settings::Instance().Load();
				mSocket = std::make_shared<UdpSocket>(Settings::Instance().mHost, Settings::Instance().mPort
					, mPoller, mStatistics, Settings::Instance().mThrottlingBitrate);
				if (!mSocket->Startup()) {
					return;
				}
			}
			mCallback->OnLauncher();
		}
		std::vector<std::string> commands;
		if (mControlSocket->Recv(commands)) {
			for (auto it = commands.begin(); it != commands.end(); ++it) {
				std::string commandName, args;

				size_t split = it->find(" ");
				if (split != std::string::npos) {
					commandName = it->substr(0, split);
					args = it->substr(split + 1);
				}
				else {
					commandName = *it;
					args = "";
				}

				Log(L"Control Command: %hs %hs", commandName.c_str(), args.c_str());
				ProcessCommand(commandName, args);
			}
		}
	}
}

void Listener::FECSend(uint8_t *buf, int len, uint64_t videoFrameIndex, uint64_t trackingFrameIndex) {
	int shardPackets = CalculateFECShardPackets(len, mFecPercentage);

	int blockSize = shardPackets * ALVR_MAX_VIDEO_BUFFER_SIZE;

	int dataShards = (len + blockSize - 1) / blockSize;
	int totalParityShards = CalculateParityShards(dataShards, mFecPercentage);
	int totalShards = dataShards + totalParityShards;

	assert(totalShards <= DATA_SHARDS_MAX);

	Log(L"reed_solomon_new. dataShards=%d totalParityShards=%d totalShards=%d blockSize=%d shardPackets=%d"
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

	Log(L"Sending video frame. trackingFrameIndex=%llu videoFrameIndex=%llu size=%d", trackingFrameIndex, videoFrameIndex, len);

	header->type = ALVR_PACKET_TYPE_VIDEO_FRAME;
	header->trackingFrameIndex = trackingFrameIndex;
	header->videoFrameIndex = videoFrameIndex;
	header->sentTime = GetTimestampUs();
	header->frameByteSize = len;
	header->fecIndex = 0;
	header->fecPercentage = mFecPercentage;

	// Send data shards.
	for (int i = 0; i < dataShards; i++) {
		for (int j = 0; j < shardPackets; j++) {
			int copyLength = std::min(ALVR_MAX_VIDEO_BUFFER_SIZE, dataRemain);
			if (copyLength <= 0) {
				// Skip to send padding packets.
				break;
			}
			memcpy(payload, shards[i] + j * ALVR_MAX_VIDEO_BUFFER_SIZE, copyLength);
			dataRemain -= ALVR_MAX_VIDEO_BUFFER_SIZE;

			header->packetCounter = mVideoPacketCounter;
			mVideoPacketCounter++;
			mSocket->SendVideo(header, sizeof(VideoFrame) + copyLength, trackingFrameIndex);
			header->fecIndex++;
		}
	}
	// Reset fecIndex to skip padding packets.
	header->fecIndex = dataShards * shardPackets;
	// Send parity shards.
	for (int i = 0; i < totalParityShards; i++) {
		for (int j = 0; j < shardPackets; j++) {
			int copyLength = ALVR_MAX_VIDEO_BUFFER_SIZE;
			memcpy(payload, shards[dataShards + i] + j * ALVR_MAX_VIDEO_BUFFER_SIZE, copyLength);

			header->packetCounter = mVideoPacketCounter;
			mVideoPacketCounter++;
			mSocket->SendVideo(header, sizeof(VideoFrame) + copyLength, trackingFrameIndex);
			header->fecIndex++;
		}
	}

	// Wake poller to immediately start sending.
	mPoller->Wake();

	if (len % blockSize != 0) {
		delete[] shards[dataShards - 1];
	}
	for (int i = 0; i < totalParityShards; i++) {
		delete[] shards[dataShards + i];
	}
}

void Listener::SendVideo(uint8_t *buf, int len, uint64_t videoFrameIndex, uint64_t trackingFrameIndex) {
	if (!mSocket->IsClientValid()) {
		Log(L"Skip sending packet because client is not connected. Packet Length=%d FrameIndex=%llu", len, trackingFrameIndex);
		return;
	}
	if (mState != State::STREAMING) {
		Log(L"Skip sending packet because streaming is off.");
		return;
	}
	FECSend(buf, len, videoFrameIndex, trackingFrameIndex);
}

bool Listener::GetFirstBufferedFrame(uint64_t * videoFrameIndex)
{
	return mSocket ? mSocket->GetFirstBufferedFrame(videoFrameIndex) : false;
}

void Listener::SendAudio(uint8_t *buf, int len, uint64_t presentationTime) {
	uint8_t packetBuffer[2000];

	if (!mSocket->IsClientValid()) {
		Log(L"Skip sending audio packet because client is not connected. Packet Length=%d", len);
		return;
	}
	if (mState != State::STREAMING) {
		Log(L"Skip sending audio packet because streaming is off.");
		return;
	}
	Log(L"Sending audio frame. Size=%d bytes", len);

	int remainBuffer = len;
	for (int i = 0; remainBuffer != 0; i++) {
		int pos = 0;

		if (i == 0) {
			// First fragment
			auto header = (AudioFrameStart *)packetBuffer;

			header->type = ALVR_PACKET_TYPE_AUDIO_FRAME_START;
			header->packetCounter = mSoundPacketCounter;
			header->presentationTime = presentationTime;
			header->frameByteSize = len;

			pos = sizeof(*header);
		}
		else {
			// Following fragments
			auto header = (AudioFrame *)packetBuffer;

			header->type = ALVR_PACKET_TYPE_AUDIO_FRAME;
			header->packetCounter = mSoundPacketCounter;

			pos = sizeof(*header);
		}

		int size = std::min(PACKET_SIZE - pos, remainBuffer);

		memcpy(packetBuffer + pos, buf + (len - remainBuffer), size);
		pos += size;
		remainBuffer -= size;

		mSoundPacketCounter++;

		int ret = mSocket->Send((char *)packetBuffer, pos);

	}
}

void Listener::SendHapticsFeedback(uint64_t startTime, float amplitude, float duration, float frequency, uint8_t hand)
{
	if (!mSocket->IsClientValid()) {
		Log(L"Skip sending audio packet because client is not connected.");
		return;
	}
	if (mState != State::STREAMING) {
		Log(L"Skip sending audio packet because streaming is off.");
		return;
	}
	Log(L"Sending haptics feedback. startTime=%llu amplitude=%f duration=%f frequency=%f", startTime, amplitude, duration, frequency);

	HapticsFeedback packetBuffer;
	packetBuffer.type = ALVR_PACKET_TYPE_HAPTICS;
	packetBuffer.startTime = startTime;
	packetBuffer.amplitude = amplitude;
	packetBuffer.duration = duration;
	packetBuffer.frequency = frequency;
	packetBuffer.hand = hand;
	mSocket->Send((char *)&packetBuffer, sizeof(HapticsFeedback));
}

void Listener::ProcessRecv(char *buf, int len, sockaddr_in *addr) {
	if (len < 4) {
		return;
	}
	int pos = 0;
	uint32_t type = *(uint32_t*)buf;

	Log(L"Received packet. Type=%d", type);
	if (type == ALVR_PACKET_TYPE_HELLO_MESSAGE && len >= sizeof(HelloMessage)) {
		HelloMessage *message = (HelloMessage *)buf;

		// Check signature
		if (memcmp(message->signature, ALVR_HELLO_PACKET_SIGNATURE, sizeof(message->signature)) != 0)
		{
			// Non-ALVR packet or old version.
			Log(L"Received packet with bad signature. sig=%08X", *(uint32_t *)message->signature);
			return;
		}

		SanitizeDeviceName(message->deviceName);

		if (message->version != ALVR_PROTOCOL_VERSION) {
			Log(L"Received hello message which have unsupported version. Received Version=%d Our Version=%d", message->version, ALVR_PROTOCOL_VERSION);
			// We can't connect, but we should do PushRequest to notify user.
		}

		Log(L"Hello Message: %hs Version=%d Hz=%d,%d,%d,%d Size=%dx%d Device=%d-%d Caps=%X,%X", message->deviceName, message->version
			, message->refreshRate[0], message->refreshRate[1]
			, message->refreshRate[2], message->refreshRate[3]
			, message->renderWidth, message->renderHeight
			, message->deviceType, message->deviceSubType
			, message->deviceCapabilityFlags, message->controllerCapabilityFlags);

		PushRequest(message, addr);
		if (AddrToStr(addr) == Settings::Instance().mAutoConnectHost &&
			ntohs(addr->sin_port) == Settings::Instance().mAutoConnectPort) {
			if (!IsConnected()) {
				Log(L"AutoConnect: %hs", AddrPortToStr(addr).c_str());
				Connect(addr);
			}
		}
	}
	else if (type == ALVR_PACKET_TYPE_RECOVER_CONNECTION && len >= sizeof(RecoverConnection)) {
		Log(L"Got recover connection message from %hs.", AddrPortToStr(addr).c_str());
		if (mSocket->IsLegitClient(addr)) {
			Log(L"This is the legit client. Send connection message.");
			Connect(addr);
		}
	}
	else if (type == ALVR_PACKET_TYPE_TRACKING_INFO && len >= sizeof(TrackingInfo)) {
		if (!IsConnected() || !mSocket->IsLegitClient(addr)) {
			Log(L"Recieved message from invalid address: %hs", AddrPortToStr(addr).c_str());
			return;
		}
		UpdateLastSeen();

		{
			IPCCriticalSectionLock lock(mCS);
			mTrackingInfo = *(TrackingInfo *)buf;
		}

		Log(L"got tracking info %d %f %f %f %f", (int)mTrackingInfo.FrameIndex,
			mTrackingInfo.HeadPose_Pose_Orientation.x,
			mTrackingInfo.HeadPose_Pose_Orientation.y,
			mTrackingInfo.HeadPose_Pose_Orientation.z,
			mTrackingInfo.HeadPose_Pose_Orientation.w);
		mCallback->OnPoseUpdated();
	}
	else if (type == ALVR_PACKET_TYPE_TIME_SYNC && len >= sizeof(TimeSync)) {
		if (!IsConnected() || !mSocket->IsLegitClient(addr)) {
			Log(L"Recieved message from invalid address: %hs", AddrPortToStr(addr).c_str());
			return;
		}
		UpdateLastSeen();

		TimeSync *timeSync = (TimeSync*)buf;
		uint64_t Current = GetTimestampUs();

		if (timeSync->mode == 0) {
			mReportedStatistics = *timeSync;
			TimeSync sendBuf = *timeSync;
			sendBuf.mode = 1;
			sendBuf.serverTime = Current;
			mSocket->Send((char *)&sendBuf, sizeof(sendBuf));
		}
		else if (timeSync->mode == 2) {
			// Calclate RTT
			uint64_t RTT = Current - timeSync->serverTime;
			// Estimated difference between server and client clock
			uint64_t TimeDiff = Current - (timeSync->clientTime + RTT / 2);
			mTimeDiff = TimeDiff;
			Log(L"TimeSync: server - client = %lld us RTT = %lld us", TimeDiff, RTT);
		}
	}
	else if (type == ALVR_PACKET_TYPE_STREAM_CONTROL_MESSAGE && len >= sizeof(StreamControlMessage)) {
		if (!IsConnected() || !mSocket->IsLegitClient(addr)) {
			Log(L"Recieved message from invalid address: %s:%d", AddrPortToStr(addr));
			return;
		}
		StreamControlMessage *streamControl = (StreamControlMessage*)buf;

		if (streamControl->mode == 1) {
			Log(L"Stream control message: Start stream.");
			mState = State::STREAMING;
			mCallback->OnStreamStart();
		}
		else if (streamControl->mode == 2) {
			Log(L"Stream control message: Stop stream.");
			mState = State::CONNECTED;
		}
	}
	else if (type == ALVR_PACKET_TYPE_VIDEO_FRAME_ACK && len >= sizeof(VideoFrameAck)) {
		if (!IsConnected() || !mSocket->IsLegitClient(addr)) {
			Log(L"Recieved message from invalid address: %hs", AddrPortToStr(addr).c_str());
			return;
		}
		VideoFrameAck *packet = (VideoFrameAck *)buf;
		mCallback->OnFrameAck(packet->ackType == ALVR_FRAME_ACK_TYPE_ACK, packet->frameType == ALVR_FRAME_ACK_VIDEO_FRAME_TYPE_IDR,
			packet->startFrame, packet->endFrame);
		if (packet->ackType == ALVR_FRAME_ACK_TYPE_NACK && packet->ackType != ALVR_FRAME_ACK_VIDEO_FRAME_TYPE_IDR) {
			OnFecFailure(packet->startFrame, packet->endFrame);
		}
	}
}

void Listener::ProcessCommand(const std::string &commandName, const std::string args) {
	if (commandName == "SetDebugFlags") {
		mSettings.debugFlags = strtol(args.c_str(), NULL, 10);
		SendChangeSettings();
		SendCommandResponse("OK\n");
	}
	else if (commandName == "Suspend") {
		mSettings.suspend = atoi(args.c_str());
		SendChangeSettings();
		SendCommandResponse("OK\n");
	}
	else if (commandName == "GetRequests") {
		std::string str;
		for (auto it = mRequests.begin(); it != mRequests.end(); it++) {
			char buf[500];
			snprintf(buf, sizeof(buf), "%s %d %d %s\n"
				, AddrPortToStr(&it->address).c_str()
				, it->versionOk, 60
				, it->deviceName);
			str += buf;
		}
		SendCommandResponse(str.c_str());
	}
	else if (commandName == "Connect") {
		auto index = args.find(":");
		if (index == std::string::npos) {
			// Invalid format.
			SendCommandResponse("Fail\n");
		}
		else {
			std::string host = args.substr(0, index);
			int port = atoi(args.substr(index + 1).c_str());

			sockaddr_in addr;
			addr.sin_family = AF_INET;
			addr.sin_port = htons(port);
			inet_pton(addr.sin_family, host.c_str(), &addr.sin_addr);

			FindClientName(&addr);
			Connect(&addr);

			SendCommandResponse("OK\n");
		}
	}
	else if (commandName == "Shutdown") {
		Disconnect();
		mCallback->OnShutdown();
		SendCommandResponse("OK\n");
	}
	else if (commandName == "GetStat") {
		char buf[1000];
		snprintf(buf, sizeof(buf),
			"TotalPackets %llu Packets\n"
			"PacketRate %llu Packets/s\n"
			"PacketsLostTotal %llu Packets\n"
			"PacketsLostInSecond %llu Packets/s\n"
			"TotalSent %llu MB\n"
			"SentRate %.1f Mbps\n"
			"TotalLatency %.1f ms\n"
			"EncodeLatency %.1f ms\n"
			"EncodeLatencyMax %.1f ms\n"
			"TransportLatency %.1f ms\n"
			"DecodeLatency %.1f ms\n"
			"FecPercentage %d %%\n"
			"FecFailureTotal %llu Packets\n"
			"FecFailureInSecond %llu Packets/s\n"
			"ClientFPS %d\n"
			"ServerFPS %d\n"
			, mStatistics->GetPacketsSentTotal()
			, mStatistics->GetPacketsSentInSecond()
			, mReportedStatistics.packetsLostTotal
			, mReportedStatistics.packetsLostInSecond
			, mStatistics->GetBitsSentTotal() / 8 / 1000 / 1000
			, mStatistics->GetBitsSentInSecond() / 1000 / 1000.0
			, mReportedStatistics.averageTotalLatency / 1000.0
			, (double)(mStatistics->GetEncodeLatencyAverage()) / US_TO_MS
			, (double)(mStatistics->GetEncodeLatencyMax()) / US_TO_MS
			, mReportedStatistics.averageTransportLatency / 1000.0
			, mReportedStatistics.averageDecodeLatency / 1000.0
			, mFecPercentage
			, mReportedStatistics.fecFailureTotal
			, mReportedStatistics.fecFailureInSecond
			, mReportedStatistics.fps
			, mStatistics->GetFPS());
		SendCommandResponse(buf);
	}
	else if (commandName == "Disconnect") {
		Disconnect();
		SendCommandResponse("OK\n");
	}
	else if (commandName == "SetClientConfig") {
		auto index = args.find(" ");
		if (index == std::string::npos) {
			SendCommandResponse("NG\n");
		}
		else {
			auto name = args.substr(0, index);
			if (name == k_pch_Settings_FrameQueueSize_Int32) {
				Settings::Instance().mFrameQueueSize = atoi(args.substr(index + 1).c_str());
				mSettings.frameQueueSize = Settings::Instance().mFrameQueueSize;
				SendChangeSettings();
			}
			else {
				SendCommandResponse("NG\n");
				return;
			}
			SendCommandResponse("OK\n");
		}
	}
	else {
		mCallback->OnCommand(commandName, args);
	}
}

void Listener::SendChangeSettings() {
	if (!mSocket->IsClientValid()) {
		return;
	}
	mSocket->Send((char *)&mSettings, sizeof(mSettings));
}

void Listener::Stop()
{
	Log(L"Listener::Stop().");
	mExiting = true;
	mSocket->Shutdown();
	mControlSocket->Shutdown();
	Join();
}

bool Listener::HasValidTrackingInfo() const {
	return mTrackingInfo.type == ALVR_PACKET_TYPE_TRACKING_INFO;
}

void Listener::GetTrackingInfo(TrackingInfo &info) {
	IPCCriticalSectionLock lock(mCS);
	info = mTrackingInfo;
}

uint64_t Listener::clientToServerTime(uint64_t clientTime) const {
	return clientTime + mTimeDiff;
}

uint64_t Listener::serverToClientTime(uint64_t serverTime) const {
	return serverTime - mTimeDiff;
}

void Listener::SendCommandResponse(const char *commandResponse) {
	Log(L"SendCommandResponse: %hs", commandResponse);
	mControlSocket->SendCommandResponse(commandResponse);
}

void Listener::PushRequest(HelloMessage *message, sockaddr_in *addr) {
	for (auto it = mRequests.begin(); it != mRequests.end(); ++it) {
		if (it->address.sin_addr.S_un.S_addr == addr->sin_addr.S_un.S_addr && it->address.sin_port == addr->sin_port) {
			mRequests.erase(it);
			break;
		}
	}
	Request request = {};
	request.address = *addr;
	memcpy(request.deviceName, message->deviceName, sizeof(request.deviceName));
	request.timestamp = GetTimestampUs();
	request.versionOk = message->version == ALVR_PROTOCOL_VERSION;
	request.message = *message;

	mRequests.push_back(request);
	if (mRequests.size() > 10) {
		mRequests.pop_back();
	}
}

void Listener::SanitizeDeviceName(char deviceName[32]) {
	deviceName[31] = 0;
	auto len = strlen(deviceName);
	if (len != 31) {
		memset(deviceName + len, 0, 31 - len);
	}
	for (int i = 0; i < len; i++) {
		if (!isalnum(deviceName[i]) && deviceName[i] != '_' && deviceName[i] != '-') {
			deviceName[i] = '_';
		}
	}
}

std::string Listener::DumpConfig() {
	char buf[1000];

	sockaddr_in addr = {};
	if (IsConnected()) {
		addr = mSocket->GetClientAddr();
	}
	else {
		addr.sin_family = AF_INET;
	}
	char host[100];
	inet_ntop(AF_INET, &addr.sin_addr, host, sizeof(host));

	snprintf(buf, sizeof(buf)
		, "Connected %d\n"
		"Client %s:%d\n"
		"ClientName %s\n"
		"Streaming %d\n"
		, IsConnected() ? 1 : 0
		, host, htons(addr.sin_port)
		, mClientDeviceName.c_str()
		, mState == State::STREAMING);

	return buf;
}

void Listener::CheckTimeout() {
	// Remove old requests
	for (auto it = mRequests.begin(); it != mRequests.end(); ) {
		if (GetTimestampUs() - it->timestamp > REQUEST_TIMEOUT) {
			it = mRequests.erase(it);
		}
		else {
			it++;
		}
	}

	if (!IsConnected()) {
		return;
	}

	uint64_t Current = GetTimestampUs();

	if (Current - mLastSeen > CONNECTION_TIMEOUT) {
		// idle for 300 seconcd
		// Invalidate client
		Disconnect();
		Log(L"Client timeout for idle");
	}
}

void Listener::UpdateLastSeen() {
	mLastSeen = GetTimestampUs();
}

void Listener::FindClientName(const sockaddr_in *addr) {
	mClientDeviceName = "";

	bool found = false;

	for (auto it = mRequests.begin(); it != mRequests.end(); it++) {
		if (it->address.sin_addr.S_un.S_addr == addr->sin_addr.S_un.S_addr && it->address.sin_port == addr->sin_port) {
			mClientDeviceName = it->deviceName;
			found = true;
			break;
		}
	}
}

void Listener::Connect(const sockaddr_in *addr) {
	Log(L"Connected to %hs", AddrPortToStr(addr).c_str());

	mSocket->InvalidateClient();

	mCallback->OnNewClient();

	mSocket->SetClientAddr(addr);
	mState = State::CONNECTED;
	mVideoPacketCounter = 0;
	mSoundPacketCounter = 0;
	mFecPercentage = INITIAL_FEC_PERCENTAGE;
	memset(&mReportedStatistics, 0, sizeof(mReportedStatistics));
	mStatistics->ResetAll();
	UpdateLastSeen();

	ConnectionMessage message = {};
	message.type = ALVR_PACKET_TYPE_CONNECTION_MESSAGE;
	message.version = ALVR_PROTOCOL_VERSION;
	message.codec = Settings::Instance().mCodec;
	message.videoWidth = Settings::Instance().mRenderWidth;
	message.videoHeight = Settings::Instance().mRenderHeight;
	message.bufferSize = Settings::Instance().mClientRecvBufferSize;
	message.frameQueueSize = Settings::Instance().mFrameQueueSize;
	message.refreshRate = Settings::Instance().mRefreshRate;

	mSocket->Send((char *)&message, sizeof(message));
}

void Listener::Disconnect() {
	mState = State::NOT_CONNECTED;
	mClientDeviceName = "";

	mSocket->InvalidateClient();
}

void Listener::OnFecFailure(uint64_t startFrame, uint64_t endFrame) {
	Log(L"Listener::OnFecFailure(). %llu - %llu", startFrame, endFrame);
	if (GetTimestampUs() - mLastFecFailure < CONTINUOUS_FEC_FAILURE) {
		if (mFecPercentage < MAX_FEC_PERCENTAGE) {
			mFecPercentage += 5;
		}
	}
	mLastFecFailure = GetTimestampUs();
}

std::shared_ptr<Statistics> Listener::GetStatistics() {
	return mStatistics;
}

bool Listener::IsStreaming() {
	return mState = State::STREAMING;
}

void Listener::SetCallback(Callback * callback)
{
	mCallback = callback;
}
