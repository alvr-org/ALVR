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
	IPCFileMapping(const char* pName, uint64_t size);
	~IPCFileMapping();

	void *Map(DWORD access = FILE_MAP_READ);

	bool Opened();
private:
	HANDLE m_hMapFile;
	bool m_Exist;
};

class IPCCreateFileMapping
{
public:
	IPCCreateFileMapping(const char* pName, uint64_t size);
	~IPCCreateFileMapping();

	void *Map();

	bool Opened();
private:
	HANDLE m_hMapFile;
	bool m_Exist;
};

class IPCCriticalSection
{
public:
	IPCCriticalSection() {
		InitializeCriticalSection(&m_cs);
	}

	~IPCCriticalSection() {
		DeleteCriticalSection(&m_cs);
	}

	void Lock() {
		EnterCriticalSection(&m_cs);
	}
	void Unlock() {
		LeaveCriticalSection(&m_cs);
	}

private:
	CRITICAL_SECTION m_cs;
};

class IPCCriticalSectionLock
{
public:
	IPCCriticalSectionLock(IPCCriticalSection &cs) {
		m_cs = &cs;
		m_cs->Lock();
	}

	~IPCCriticalSectionLock() {
		m_cs->Unlock();
	}

private:
	IPCCriticalSection * m_cs;
};