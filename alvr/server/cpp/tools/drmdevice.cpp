#include "drmdevice.h"
#include "drm.h"

#include <cstdint>
#include <string.h>
#include <errno.h>
#include <xf86drm.h>
#include <xf86drmMode.h>
#include <fcntl.h>
#include <functional>
#include <stdio.h>
#include <unistd.h>
#include <stdexcept>
#include <poll.h>

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

void DRMDevice::waitVBlank(volatile bool &exiting)
{
	uint64_t queued_seq;
	uint64_t current_seq = 0;
	drmCrtcQueueSequence(fd, crtc_id, DRM_CRTC_SEQUENCE_RELATIVE, 1, &queued_seq, uintptr_t(&current_seq));

	drmEventContext handlers{
		.version = 4,
		.sequence_handler = DRMDevice::sequence_handler
	};

	while (current_seq < queued_seq and not exiting)
	{
		pollfd pfd[1];
		pfd[0] = pollfd{.fd = fd, .events = POLLIN, .revents = 0};
		int c = poll(pfd, 1, 10);
		if (c < 0)
		{
			throw std::runtime_error(std::string("poll failed: ") + strerror(errno));
		}
		if (c == 1)
		{
			drmHandleEvent(fd, &handlers);
		}
	}
}

void DRMDevice::sequence_handler(int /*fd*/,
		uint64_t sequence,
		uint64_t /*ns*/,
		uint64_t user_data)
{
	uint64_t *seq = (uint64_t*)user_data;
	*seq = sequence;
}

DRMDevice::~DRMDevice()
{
	if(fd != -1)
	{
		close(fd);
	}
}
