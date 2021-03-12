#include "CEncoder.h"

#include <chrono>
#include <exception>
#include <memory>
#include <stdexcept>
#include <string>

#include "ALVR-common/packet_types.h"
#include "alvr_server/ClientConnection.h"
#include "alvr_server/Logger.h"
#include "alvr_server/Settings.h"
#include "alvr_server/Statistics.h"
#include "subprocess.hpp"


CEncoder::CEncoder(std::shared_ptr<ClientConnection> listener):
	m_listener(listener)
{
}

CEncoder::~CEncoder()
{
	Stop();
}

namespace
{
void read_exactly(FILE* stream, char* out, size_t size)
{
	while (size)
	{
		int read = subprocess::util::read_atmost_n(stream, out, size);
		if (read == -1)
		{
			throw std::runtime_error("read failed");
		}
		out+= read;
		size -= read;
	}
}


}

void CEncoder::Run()
{
	try {
		auto p = subprocess::Popen(
				{"grabber", //FIXME: get the installation path
				std::to_string(Settings::Instance().m_renderWidth),
				std::to_string(Settings::Instance().m_renderHeight),
				std::to_string(Settings::Instance().m_refreshRate),
				std::to_string(Settings::Instance().m_codec),
				std::to_string(Settings::Instance().mEncodeBitrateMBs)},
				subprocess::input{subprocess::PIPE},
				subprocess::output{subprocess::PIPE}
				);

		auto pipe = p.output();
		char size_raw[sizeof(int)];
		std::vector<char> frame_data;
		for(int frame = 0; not m_exiting; ++frame)
		{
			auto frame_start = std::chrono::steady_clock::now();

			read_exactly(pipe, size_raw, sizeof(int));
			int size;
			memcpy(&size, size_raw, sizeof(int));
			frame_data.resize(size);
			read_exactly(pipe, frame_data.data(), size);

			m_listener->GetStatistics()->EncodeOutput(
					std::chrono::duration_cast<std::chrono::microseconds>(std::chrono::steady_clock::now() - frame_start).count());
			m_listener->SendVideo((uint8_t*)frame_data.data(), size, m_lastPoseFrame);
		}
		p.kill(7);
	}
	catch (std::exception& e)
	{
		Error("encoder failed with error %s", e.what());
	}
}

void CEncoder::Stop()
{
	m_exiting = true;
	Join();
}

void CEncoder::OnPacketLoss()
{
	m_scheduler.OnPacketLoss();
}

void CEncoder::InsertIDR() {
	m_scheduler.InsertIDR();
}

void CEncoder::OnPoseUpdated(const TrackingInfo& pose)
{
	m_lastPoseFrame = pose.FrameIndex;
}
