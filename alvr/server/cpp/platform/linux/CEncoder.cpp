#include "CEncoder.h"

#include <algorithm>
#include <chrono>
#include <exception>
#include <fstream>
#include <memory>
#include <openvr_driver.h>
#include <stdexcept>
#include <string>

#include "ALVR-common/packet_types.h"
#include "alvr_server/ClientConnection.h"
#include "alvr_server/Logger.h"
#include "alvr_server/PoseHistory.h"
#include "alvr_server/Settings.h"
#include "alvr_server/Statistics.h"
#include "subprocess.hpp"


CEncoder::CEncoder(std::shared_ptr<ClientConnection> listener, std::shared_ptr<PoseHistory> poseHistory):
	m_listener(listener),
	m_poseHistory(poseHistory)
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

auto get_hmd_pose_for_timing(std::chrono::system_clock::time_point /*frame_time*/)
{
	std::array<vr::Compositor_FrameTiming, 1> frame_timings;
	frame_timings[0].m_nSize = sizeof(vr::Compositor_FrameTiming);
	int num_frame_timings = vr::VRServerDriverHost()->GetFrameTimings(frame_timings.data(), frame_timings.size());
	//FIXME: actually find the correct pose based on absolute time
	for (int i = 0 ; i < num_frame_timings ; ++i)
	{
		return frame_timings[i].m_HmdPose;
	}
	return vr::TrackedDevicePose_t{};
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
		std::vector<char> frame_data;
		for(int frame = 0; not m_exiting; ++frame)
		{
			auto frame_start = std::chrono::steady_clock::now();

			std::chrono::system_clock::time_point grab_time;
			auto frame_pose = get_hmd_pose_for_timing(grab_time);
			read_exactly(pipe, (char*)&grab_time, sizeof(grab_time), m_exiting);

			int size;
			read_exactly(pipe, (char*)&size, sizeof(int), m_exiting);
			frame_data.resize(size);
			read_exactly(pipe, frame_data.data(), size, m_exiting);

			auto hmd_pose = m_poseHistory->GetBestPoseMatch(frame_pose.mDeviceToAbsoluteTracking);
			if (hmd_pose)
				m_poseSubmitIndex = hmd_pose->info.FrameIndex;

			m_listener->GetStatistics()->EncodeOutput(
					std::chrono::duration_cast<std::chrono::microseconds>(std::chrono::steady_clock::now() - frame_start).count());
			m_listener->SendVideo((uint8_t*)frame_data.data(), size, m_poseSubmitIndex);
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
