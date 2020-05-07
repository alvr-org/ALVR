//===================== Copyright (c) Valve Corporation. All Rights Reserved. ======================
//
// Memory mapped file for copying backbuffer between processes.
//
//==================================================================================================
#pragma once

#include "ipctools.h"

#define assert( a )

template< typename T >
class TSharedState
{
public:
	TSharedState()
	{
		m_pMutex = new IPCMutex( T::GetMutexName() );
		m_hMapFile = CreateFileMapping( INVALID_HANDLE_VALUE, NULL, PAGE_READWRITE, 0, sizeof( T ), T::GetMemName() );
		m_pData = m_hMapFile ? MapViewOfFile( m_hMapFile, FILE_MAP_ALL_ACCESS, 0, 0, sizeof( T ) ) : NULL;
	}
	~TSharedState()
	{
		if ( m_pData )
			UnmapViewOfFile( m_pData );
		if ( m_hMapFile )
			CloseHandle( m_hMapFile );
		if ( m_pMutex )
			delete m_pMutex;
	}
	bool IsValid() const { return m_pData != NULL; }

	// Scoped wrapper to safely access shared data.
	class Ptr
	{
	public:
		Ptr( TSharedState *pSharedState )
			: m_pSharedState( pSharedState )
		{
			assert( m_pSharedState && m_pSharedState->IsValid() );
			m_pSharedState->m_pMutex->Wait();
		}
		~Ptr()
		{
			m_pSharedState->m_pMutex->Release();
		}
		T *operator ->( ) { return ( T * )m_pSharedState->m_pData; }
		T *operator &( ) { return ( T * )m_pSharedState->m_pData; }
	private:
		TSharedState *m_pSharedState;
	};

private:
	IPCMutex *m_pMutex;
	HANDLE m_hMapFile;
	void *m_pData;
};

#pragma pack( push, 8 )

struct SharedState_t
{
	static const char * GetMemName() { return "RemoteDisplayState"; }
	static const char * GetMutexName() { return "RemoteDisplayMutex"; }

	static const uint32_t MAX_TEXTURE_WIDTH = 4096;
	static const uint32_t MAX_TEXTURE_HEIGHT = 2048;
	static const uint32_t TEXTURE_PITCH = 4;

	// Input
	uint8_t m_nTextureData[ MAX_TEXTURE_WIDTH * MAX_TEXTURE_HEIGHT * TEXTURE_PITCH ];
	uint32_t m_nTextureWidth, m_nTextureHeight, m_nTextureFormat;
	double m_flVsyncTimeInSeconds;

	// Output
	double m_flLastVsyncTimeInSeconds;
	uint32_t m_nVsyncCounter;

	// Initialization
	uint64_t m_nSystemBaseTimeTicks;

	// Shutdown
	bool m_bShutdown;
};

typedef TSharedState< SharedState_t > CSharedState;

#pragma pack(pop)

