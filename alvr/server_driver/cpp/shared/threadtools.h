//===================== Copyright (c) Valve Corporation. All Rights Reserved. ======================
//
// Helper classes for working with threads.
//
//==================================================================================================
#pragma once

#include <thread>
#include <windows.h>

#define THREAD_PRIORITY_MOST_URGENT 15

class CThread
{
public:
	CThread();
	virtual ~CThread();
	virtual bool Init() { return true; }
	virtual void Run() = 0;
	void Start();
	void Join();
private:
	std::thread *m_pThread;
};

class CThreadEvent
{
public:
	CThreadEvent( bool bManualReset = false );
	~CThreadEvent();
	bool Wait( uint32_t nTimeoutMs = INFINITE );
	bool Set();
	bool Reset();
private:
	HANDLE m_hSyncObject;
};

