#include "ClientConnection.h"
#include "Bitrate.h"

ClientConnection::ClientConnection()
	: m_bExiting(false)
	, m_Enabled(false)
	, m_Connected(false)
	, m_Streaming(false)
	, m_LastSeen(0) {
	memset(&m_TrackingInfo, 0, sizeof(m_TrackingInfo));
	InitializeCriticalSection(&m_CS);

	m_Statistics = std::make_shared<Statistics>();

	m_Settings.type = ALVR_PACKET_TYPE_CHANGE_SETTINGS;
	m_Settings.debugFlags = 0;
	m_Settings.suspend = 0;

	m_Poller.reset(new Poller());
	m_ControlSocket.reset(new ControlSocket(m_Poller));

	m_Streaming = false;

	reed_solomon_init();
}

ClientConnection::~ClientConnection() {
	DeleteCriticalSection(&m_CS);
}

void ClientConnection::SetLauncherCallback(std::function<void()> callback) {
	m_LauncherCallback = callback;
}
void ClientConnection::SetCommandCallback(std::function<void(std::string, std::string)> callback) {
	m_CommandCallback = callback;
}
void ClientConnection::SetPoseUpdatedCallback(std::function<void()> callback) {
	m_PoseUpdatedCallback = callback;
}
void ClientConnection::SetNewClientCallback(std::function<void()> callback) {
	m_NewClientCallback = callback;
}
void ClientConnection::SetStreamStartCallback(std::function<void()> callback) {
	m_StreamStartCallback = callback;
}
void ClientConnection::SetPacketLossCallback(std::function<void()> callback) {
	m_PacketLossCallback = callback;
}
void ClientConnection::SetShutdownCallback(std::function<void()> callback) {
	m_ShutdownCallback = callback;
}

bool ClientConnection::Startup() {
	if (!m_ControlSocket->Startup()) {
		return false;
	}
	if (Settings::Instance().IsLoaded()) {
		m_Enabled = true;
		m_Socket = std::make_shared<UdpSocket>(Settings::Instance().m_Host, Settings::Instance().m_Port
			, m_Poller, m_Statistics, Settings::Instance().mThrottlingBitrate);
		if (!m_Socket->Startup()) {
			return false;
		}
	}
	// Start thread.
	Start();
	return true;
}

void ClientConnection::Run() {
	while (!m_bExiting) {
		CheckTimeout();
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

		if (m_ControlSocket->Accept()) {
			if (!m_Enabled) {
				m_Enabled = true;
				Settings::Instance().Load();
				m_Socket = std::make_shared<UdpSocket>(Settings::Instance().m_Host, Settings::Instance().m_Port
					, m_Poller, m_Statistics, Settings::Instance().mThrottlingBitrate);
				if (!m_Socket->Startup()) {
					return;
				}
			}
			m_LauncherCallback();
		}
		std::vector<std::string> commands;
		if (m_ControlSocket->Recv(commands)) {
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

void ClientConnection::FECSend(uint8_t *buf, int len, uint64_t frameIndex, uint64_t videoFrameIndex) {
	int shardPackets = CalculateFECShardPackets(len, m_fecPercentage);

	int blockSize = shardPackets * ALVR_MAX_VIDEO_BUFFER_SIZE;

	int dataShards = (len + blockSize - 1) / blockSize;
	int totalParityShards = CalculateParityShards(dataShards, m_fecPercentage);
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

	Log(L"Sending video frame. trackingFrameIndex=%llu videoFrameIndex=%llu size=%d", frameIndex, videoFrameIndex, len);

	header->type = ALVR_PACKET_TYPE_VIDEO_FRAME;
	header->trackingFrameIndex = frameIndex;
	header->videoFrameIndex = videoFrameIndex;
	header->sentTime = GetTimestampUs();
	header->frameByteSize = len;
	header->fecIndex = 0;
	header->fecPercentage = m_fecPercentage;
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
		Log(L"Skip sending packet because client is not connected. Packet Length=%d FrameIndex=%llu", len, frameIndex);
		return;
	}
	if (!m_Streaming) {
		Log(L"Skip sending packet because streaming is off.");
		return;
	}
	FECSend(buf, len, frameIndex, mVideoFrameIndex);
	mVideoFrameIndex++;
}

void ClientConnection::SendAudio(uint8_t *buf, int len, uint64_t presentationTime) {
	uint8_t packetBuffer[2000];

	if (!m_Socket->IsClientValid()) {
		Log(L"Skip sending audio packet because client is not connected. Packet Length=%d", len);
		return;
	}
	if (!m_Streaming) {
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

		int ret = m_Socket->Send((char *)packetBuffer, pos);

	}
}

void ClientConnection::SendHapticsFeedback(uint64_t startTime, float amplitude, float duration, float frequency, uint8_t hand)
{
	if (!m_Socket->IsClientValid()) {
		Log(L"Skip sending audio packet because client is not connected.");
		return;
	}
	if (!m_Streaming) {
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
	m_Socket->Send((char *)&packetBuffer, sizeof(HapticsFeedback));
}

void ClientConnection::ProcessRecv(char *buf, int len, sockaddr_in *addr) {
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
		if (AddrToStr(addr) == Settings::Instance().m_AutoConnectHost &&
			ntohs(addr->sin_port) == Settings::Instance().m_AutoConnectPort) {
			if (!m_Connected) {
				Log(L"AutoConnect: %hs", AddrPortToStr(addr).c_str());
				Connect(addr);
			}
		}
	}
	else if (type == ALVR_PACKET_TYPE_RECOVER_CONNECTION && len >= sizeof(RecoverConnection)) {
		Log(L"Got recover connection message from %hs.", AddrPortToStr(addr).c_str());
		if (m_Socket->IsLegitClient(addr)) {
			Log(L"This is the legit client. Send connection message.");
			Connect(addr);
		}
	}
	else if (type == ALVR_PACKET_TYPE_TRACKING_INFO && len >= sizeof(TrackingInfo)) {
		if (!m_Connected || !m_Socket->IsLegitClient(addr)) {
			Log(L"Recieved message from invalid address: %hs", AddrPortToStr(addr).c_str());
			return;
		}
		UpdateLastSeen();

		EnterCriticalSection(&m_CS);
		m_TrackingInfo = *(TrackingInfo *)buf;
		LeaveCriticalSection(&m_CS);

		Log(L"got tracking info %d %f %f %f %f", (int)m_TrackingInfo.FrameIndex,
			m_TrackingInfo.HeadPose_Pose_Orientation.x,
			m_TrackingInfo.HeadPose_Pose_Orientation.y,
			m_TrackingInfo.HeadPose_Pose_Orientation.z,
			m_TrackingInfo.HeadPose_Pose_Orientation.w);
		m_PoseUpdatedCallback();
	}
	else if (type == ALVR_PACKET_TYPE_TIME_SYNC && len >= sizeof(TimeSync)) {
		if (!m_Connected || !m_Socket->IsLegitClient(addr)) {
			Log(L"Recieved message from invalid address: %hs", AddrPortToStr(addr).c_str());
			return;
		}
		UpdateLastSeen();

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
			Log(L"TimeSync: server - client = %lld us RTT = %lld us", TimeDiff, RTT);
		}
	}
	else if (type == ALVR_PACKET_TYPE_STREAM_CONTROL_MESSAGE && len >= sizeof(StreamControlMessage)) {
		if (!m_Connected || !m_Socket->IsLegitClient(addr)) {
			Log(L"Recieved message from invalid address: %s:%d", AddrPortToStr(addr));
			return;
		}
		StreamControlMessage *streamControl = (StreamControlMessage*)buf;

		if (streamControl->mode == 1) {
			Log(L"Stream control message: Start stream.");
			m_Streaming = true;
			m_StreamStartCallback();
		}
		else if (streamControl->mode == 2) {
			Log(L"Stream control message: Stop stream.");
			m_Streaming = false;
		}
	}
	else if (type == ALVR_PACKET_TYPE_PACKET_ERROR_REPORT && len >= sizeof(PacketErrorReport)) {
		if (!m_Connected || !m_Socket->IsLegitClient(addr)) {
			Log(L"Recieved message from invalid address: %hs", AddrPortToStr(addr).c_str());
			return;
		}
		auto *packetErrorReport = (PacketErrorReport *)buf;
		Log(L"Packet loss was reported. Type=%d %lu - %lu", packetErrorReport->lostFrameType, packetErrorReport->fromPacketCounter, packetErrorReport->toPacketCounter);
		if (packetErrorReport->lostFrameType == ALVR_LOST_FRAME_TYPE_VIDEO) {
			// Recover video frame.
			OnFecFailure();
		}
	}
}

void ClientConnection::ProcessCommand(const std::string &commandName, const std::string args) {
	if (commandName == "SetDebugFlags") {
		m_Settings.debugFlags = strtol(args.c_str(), NULL, 10);
		SendChangeSettings();
		SendCommandResponse("OK\n");
	}
	else if (commandName == "Suspend") {
		m_Settings.suspend = atoi(args.c_str());
		SendChangeSettings();
		SendCommandResponse("OK\n");
	}
	else if (commandName == "GetRequests") {
		std::string str;
		for (auto it = m_Requests.begin(); it != m_Requests.end(); it++) {
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
		m_ShutdownCallback();
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
			, m_Statistics->GetPacketsSentTotal()
			, m_Statistics->GetPacketsSentInSecond()
			, m_reportedStatistics.packetsLostTotal
			, m_reportedStatistics.packetsLostInSecond
			, m_Statistics->GetBitsSentTotal() / 8 / 1000 / 1000
			, m_Statistics->GetBitsSentInSecond() / 1000 / 1000.0
			, m_reportedStatistics.averageTotalLatency / 1000.0
			, (double)(m_Statistics->GetEncodeLatencyAverage()) / US_TO_MS
			, (double)(m_Statistics->GetEncodeLatencyMax()) / US_TO_MS
			, m_reportedStatistics.averageTransportLatency / 1000.0
			, m_reportedStatistics.averageDecodeLatency / 1000.0
			, m_fecPercentage
			, m_reportedStatistics.fecFailureTotal
			, m_reportedStatistics.fecFailureInSecond
			, m_reportedStatistics.fps
			, m_Statistics->GetFPS());
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
				Settings::Instance().m_frameQueueSize = atoi(args.substr(index + 1).c_str());
				m_Settings.frameQueueSize = Settings::Instance().m_frameQueueSize;
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
		m_CommandCallback(commandName, args);
	}
}

void ClientConnection::SendChangeSettings() {
	if (!m_Socket->IsClientValid()) {
		return;
	}
	m_Socket->Send((char *)&m_Settings, sizeof(m_Settings), 0);
}

void ClientConnection::Stop()
{
	Log(L"Listener::Stop().");
	m_bExiting = true;
	m_Socket->Shutdown();
	m_ControlSocket->Shutdown();
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

void ClientConnection::SendCommandResponse(const char *commandResponse) {
	Log(L"SendCommandResponse: %hs", commandResponse);
	m_ControlSocket->SendCommandResponse(commandResponse);
}

void ClientConnection::PushRequest(HelloMessage *message, sockaddr_in *addr) {
	for (auto it = m_Requests.begin(); it != m_Requests.end(); ++it) {
		if (it->address.sin_addr.S_un.S_addr == addr->sin_addr.S_un.S_addr && it->address.sin_port == addr->sin_port) {
			m_Requests.erase(it);
			break;
		}
	}
	Request request = {};
	request.address = *addr;
	memcpy(request.deviceName, message->deviceName, sizeof(request.deviceName));
	request.timestamp = GetTimestampUs();
	request.versionOk = message->version == ALVR_PROTOCOL_VERSION;
	request.message = *message;

	m_Requests.push_back(request);
	if (m_Requests.size() > 10) {
		m_Requests.pop_back();
	}
}

void ClientConnection::SanitizeDeviceName(char deviceName[32]) {
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

std::string ClientConnection::DumpConfig() {
	char buf[1000];

	sockaddr_in addr = {};
	if (m_Connected) {
		addr = m_Socket->GetClientAddr();
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
		, m_Connected ? 1 : 0
		, host, htons(addr.sin_port)
		, m_clientDeviceName.c_str()
		, m_Streaming);

	return buf;
}

void ClientConnection::CheckTimeout() {
	// Remove old requests
	for (auto it = m_Requests.begin(); it != m_Requests.end(); ) {
		if (GetTimestampUs() - it->timestamp > REQUEST_TIMEOUT) {
			it = m_Requests.erase(it);
		}
		else {
			it++;
		}
	}

	if (!m_Connected) {
		return;
	}

	uint64_t Current = GetTimestampUs();

	if (Current - m_LastSeen > CONNECTION_TIMEOUT) {
		// idle for 300 seconcd
		// Invalidate client
		Disconnect();
		Log(L"Client timeout for idle");
	}
}

void ClientConnection::UpdateLastSeen() {
	m_LastSeen = GetTimestampUs();
}

void ClientConnection::FindClientName(const sockaddr_in *addr) {
	m_clientDeviceName = "";

	bool found = false;

	for (auto it = m_Requests.begin(); it != m_Requests.end(); it++) {
		if (it->address.sin_addr.S_un.S_addr == addr->sin_addr.S_un.S_addr && it->address.sin_port == addr->sin_port) {
			m_clientDeviceName = it->deviceName;
			found = true;
			break;
		}
	}
}

void ClientConnection::Connect(const sockaddr_in *addr) {
	Log(L"Connected to %hs", AddrPortToStr(addr).c_str());

	m_NewClientCallback();

	m_Socket->SetClientAddr(addr);
	m_Connected = true;
	videoPacketCounter = 0;
	soundPacketCounter = 0;
	m_fecPercentage = INITIAL_FEC_PERCENTAGE;
	memset(&m_reportedStatistics, 0, sizeof(m_reportedStatistics));
	m_Statistics->ResetAll();
	UpdateLastSeen();

	ConnectionMessage message = {};
	message.type = ALVR_PACKET_TYPE_CONNECTION_MESSAGE;
	message.version = ALVR_PROTOCOL_VERSION;
	message.codec = Settings::Instance().m_codec;
	message.videoWidth = Settings::Instance().m_renderWidth;
	message.videoHeight = Settings::Instance().m_renderHeight;
	message.bufferSize = Settings::Instance().m_clientRecvBufferSize;
	message.frameQueueSize = Settings::Instance().m_frameQueueSize;
	message.refreshRate = Settings::Instance().m_refreshRate;

	m_Socket->Send((char *)&message, sizeof(message), 0);
}

void ClientConnection::Disconnect() {
	m_Connected = false;
	m_clientDeviceName = "";

	m_Socket->InvalidateClient();
}

void ClientConnection::OnFecFailure() {
	Log(L"Listener::OnFecFailure().");
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
