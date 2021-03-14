#pragma once

#include <string>

class DRMDevice
{
public:
	DRMDevice(int width, int height);

	void waitVBlank(volatile bool &exiting);

	~DRMDevice();
	std::string device;
	int crtc_id;
private:
	int fd = -1;

	static void sequence_handler(int fd,
				 uint64_t sequence,
				 uint64_t ns,
				 uint64_t user_data);
};
