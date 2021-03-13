#include "drmdevice.h"

#include <chrono>
#include <errno.h>
#include <xf86drm.h>
#include <xf86drmMode.h>
#include <fcntl.h>
#include <functional>
#include <stdio.h>
#include <unistd.h>
#include <stdexcept>
#include <iostream>

class Defer
{
public:
	Defer(std::function<void()> fn): fn(fn){}
	~Defer() {if (fn) fn();}
	void cancel() {fn = std::function<void()>();}
private:
	std::function<void()> fn;
};

DRMDevice::DRMDevice(int width, int height)
{
	drmDevicePtr devices[16];
	int count = drmGetDevices(devices, 16);
	Defer d([&](){drmFreeDevices(devices, count);});
	for (int i = 0 ; i < count ; ++i)
	{
		for  (int node = 0 ; node < DRM_NODE_MAX ; ++node)
		{
			if (*devices[i]->nodes[node])
			{
				int fd = open(devices[i]->nodes[node], O_RDWR);
				if (fd == -1)
				{
					perror("open drm failed");
					continue;
				}
				Defer close_fd([=](){close(fd);});
				drmSetClientCap(fd, DRM_CLIENT_CAP_UNIVERSAL_PLANES, 1);
				auto planes = drmModeGetPlaneResources(fd);
				if (not planes)
				{
					perror("drmModeGetPlaneResources failed");
					continue;
				}
				Defer dplane([&](){drmModeFreePlaneResources(planes);});
				for (int plane = 0 ; plane < planes->count_planes ; ++plane)
				{
					auto planeptr = drmModeGetPlane(fd, planes->planes[plane]);
					Defer d([&](){drmModeFreePlane(planeptr);});
					if (planeptr->crtc_id)
					{
						auto crtc = drmModeGetCrtc(fd, planeptr->crtc_id);
						Defer d([&](){drmModeFreeCrtc(crtc);});
						if (crtc and crtc->width == width and crtc->height == height)
						{
							device = devices[i]->nodes[node];
							crtc_id = planeptr->crtc_id;
							this->fd = fd;
							close_fd.cancel();
							return;
						}
					}

				}
			}
		}
	}
	throw std::runtime_error("failed to find KMS device matching " + std::to_string(width) + "x" + std::to_string(height));
}

	void DRMDevice::waitVBlank()
	{
		drmVBlank blank;
		blank.request.type = DRM_VBLANK_RELATIVE;
		blank.request.sequence = 1;
		drmWaitVBlank(fd, &blank);
	}

	DRMDevice::~DRMDevice()
	{
		if(fd != -1)
		{
			close(fd);
		}
	}
