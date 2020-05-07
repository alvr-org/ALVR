//===================== Copyright (c) Valve Corporation. All Rights Reserved. ======================
#include "ipctools.h"

//--------------------------------------------------------------------------------------------------
//--------------------------------------------------------------------------------------------------
IPCMutex::IPCMutex( const char* pName, bool initialOwner )
{
	m_hMutex = CreateMutexA( NULL, initialOwner, pName );
	m_Exist = GetLastError() == ERROR_ALREADY_EXISTS;
}

//--------------------------------------------------------------------------------------------------
//--------------------------------------------------------------------------------------------------
IPCMutex::~IPCMutex()
{
	if ( m_hMutex )
		CloseHandle( m_hMutex );
}

//--------------------------------------------------------------------------------------------------
//--------------------------------------------------------------------------------------------------
bool IPCMutex::Wait( uint32_t nTimeoutMs )
{
	return WaitForSingleObject( m_hMutex, nTimeoutMs ) == WAIT_OBJECT_0;
}

//--------------------------------------------------------------------------------------------------
//--------------------------------------------------------------------------------------------------
void IPCMutex::Release()
{
	ReleaseMutex( m_hMutex );
}

//--------------------------------------------------------------------------------------------------
//--------------------------------------------------------------------------------------------------
bool IPCMutex::AlreadyExist()
{
	return m_Exist;
}

//--------------------------------------------------------------------------------------------------
//--------------------------------------------------------------------------------------------------
IPCEvent::IPCEvent( const char* pName, bool bManualReset, bool bInitiallySet )
{
	m_hEvent = CreateEventA( NULL, bManualReset, bInitiallySet, pName );
}

//--------------------------------------------------------------------------------------------------
//--------------------------------------------------------------------------------------------------
IPCEvent::~IPCEvent()
{
	if ( m_hEvent )
		CloseHandle( m_hEvent );
}

//--------------------------------------------------------------------------------------------------
//--------------------------------------------------------------------------------------------------
bool IPCEvent::Wait( uint32_t nTimeoutMs )
{
	return WaitForSingleObject( m_hEvent, nTimeoutMs ) == WAIT_OBJECT_0;
}

//--------------------------------------------------------------------------------------------------
//--------------------------------------------------------------------------------------------------
void IPCEvent::SetEvent()
{
	::SetEvent( m_hEvent );
}

//--------------------------------------------------------------------------------------------------
//--------------------------------------------------------------------------------------------------
void IPCEvent::ResetEvent()
{
	::ResetEvent( m_hEvent );
}

//
// IPCFileMapping
//

IPCFileMapping::IPCFileMapping(const char* pName)
{
	m_hMapFile = OpenFileMapping(FILE_MAP_READ, false, pName);
}

IPCFileMapping::IPCFileMapping(const char* pName, uint64_t size)
{
	m_hMapFile = CreateFileMapping(INVALID_HANDLE_VALUE, NULL, PAGE_READWRITE, (DWORD)(size >> 32), (DWORD)size, pName);
}

IPCFileMapping::~IPCFileMapping()
{
	if (m_hMapFile)
		CloseHandle(m_hMapFile);
}

void *IPCFileMapping::Map(DWORD access)
{
	return MapViewOfFile(m_hMapFile, access, 0, 0, 0);
}

bool IPCFileMapping::Opened()
{
	return m_hMapFile != NULL;
}
