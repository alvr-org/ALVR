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
	IPCMutex( const char* pName, bool initialOwner = false );
	~IPCMutex();

	bool Wait( uint32_t nTimeoutMs = INFINITE );
	void Release();
	bool AlreadyExist();

private:
	HANDLE m_hMutex;
	bool m_Exist;
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

	bool IsValid() {
		return m_hEvent != NULL;
	}
	HANDLE Get() {
		return m_hEvent;
	}

private:
	HANDLE m_hEvent;
};

// Readonly access handle to existing filemapping object.
class IPCFileMapping
{
public:
	IPCFileMapping(const char* pName);
	~IPCFileMapping();

	void *Map();

	bool Opened();
private:
	HANDLE m_hMapFile;
	bool m_Exist;
};

