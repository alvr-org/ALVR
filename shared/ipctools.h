//===================== Copyright (c) Valve Corporation. All Rights Reserved. ======================
//
// Tools for inter-process communication.
//
//==================================================================================================
#pragma once

#include <stdint.h>
#include <windows.h>

// A named mutex for synchronization between processes.
class IPCMutex
{
public:
	IPCMutex( const char* pName );
	~IPCMutex();

	bool Wait( uint32_t nTimeoutMs = INFINITE );
	void Release();

private:
	HANDLE m_hMutex;
};

// A named event for synchronization between processes.
class IPCEvent
{
public:
	IPCEvent( const char* pName, bool bManualReset = false, bool bInitiallySet = false );
	~IPCEvent();

	bool Wait( uint32_t nTimeoutMs = INFINITE );
	void SetEvent();
	void ResetEvent();

private:
	HANDLE m_hEvent;
};

