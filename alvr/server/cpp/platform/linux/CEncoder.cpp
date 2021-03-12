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
void read_exactly(int fd, char* out, size_t size, std::atomic_bool& exiting)
{
	while (not exiting and size != 0)
	{
		timeval timeout{
			.tv_sec = 0,
			.tv_usec = 15000
		};
		fd_set read_fd, write_fd, except_fd;
		FD_ZERO(&read_fd);
		FD_SET(fd, &read_fd);
		FD_ZERO(&write_fd);
		FD_ZERO(&except_fd);
		int count = select(fd + 1, &read_fd, &write_fd, &except_fd, &timeout);
		if (count < 0) {
			throw MakeException("select failed: %s", strerror(errno));
		} else if (count == 1)
		{
			int s = read(fd, out, size);
			if (s == -1)
			{
				throw MakeException("read failed: %s", strerror(errno));
			}
			out+= s;
			size -= s;
		}
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
				subprocess::output{subprocess::PIPE}
				);

		int pipe = fileno(p.output());
		char size_raw[sizeof(int)];
		std::vector<char> frame_data;
		for(int frame = 0; not m_exiting; ++frame)
		{
			auto frame_start = std::chrono::steady_clock::now();

			read_exactly(pipe, size_raw, sizeof(int), m_exiting);
			int size;
			memcpy(&size, size_raw, sizeof(int));
			frame_data.resize(size);
			read_exactly(pipe, frame_data.data(), size, m_exiting);

			m_listener->GetStatistics()->EncodeOutput(
					std::chrono::duration_cast<std::chrono::microseconds>(std::chrono::steady_clock::now() - frame_start).count());
			m_listener->SendVideo((uint8_t*)frame_data.data(), size, m_lastPoseFrame);
		}
	}
	catch (std::exception& e)
	{
		if (not m_exiting)
			Error("encoder failed with error %s", e.what());
	}
}

void CEncoder::Stop()
{
	m_exiting = true;
	kill(m_subprocess, SIGTERM);
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
