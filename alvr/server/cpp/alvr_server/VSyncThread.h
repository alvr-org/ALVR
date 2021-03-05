#pragma once
#include "shared/threadtools.h"

// VSync Event Thread

class VSyncThread : public CThread
{
public:
	VSyncThread(int refreshRate);

	
	virtual void Run();

	virtual void Shutdown();

	void SetRefreshRate(int refreshRate);

private:
	bool m_bExit;
	uint64_t m_PreviousVsync;
	int m_refreshRate = 60;
};
