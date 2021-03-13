#pragma once

#include <string>

class DRMDevice
{
public:
	DRMDevice(int width, int height);

	void waitVBlank();

	~DRMDevice();
	std::string device;
	int crtc_id;
private:
	int fd = -1;
};
