#pragma once

#include <stdint.h>
#include "openvr-utils/threadtools.h"
#include "Utils.h"

class VSyncThread : public CThread
{
public:
	VSyncThread(int refreshRate);

	void Run()override;
	void Shutdown();
	void SetRefreshRate(int refreshRate);
private:
	bool mExit;
	uint64_t mPreviousVsync;
	int mRefreshRate = 60;
};
