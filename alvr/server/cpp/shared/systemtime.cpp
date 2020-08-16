//===================== Copyright (c) Valve Corporation. All Rights Reserved. ======================
#include "systemtime.h"
#include <windows.h>

static bool s_bInitialized = false;
static uint64_t s_nTicksPerSecond = 0;
static double s_flSecondsPerTick = 0;
static uint64_t s_nBaseTicks = 0;

//--------------------------------------------------------------------------------------------------
//--------------------------------------------------------------------------------------------------
void SystemTime::Init( uint64_t nBaseTicks )
{
	s_bInitialized = true;

	LARGE_INTEGER frequency;
	s_nTicksPerSecond = QueryPerformanceFrequency( &frequency ) ? frequency.QuadPart : 1000;
	s_flSecondsPerTick = 1.0 / s_nTicksPerSecond;
	s_nBaseTicks = nBaseTicks ? nBaseTicks : GetInTicks();
}

//--------------------------------------------------------------------------------------------------
//--------------------------------------------------------------------------------------------------
uint64_t SystemTime::GetBaseTicks()
{
	if ( !s_bInitialized )
		Init();

	return s_nBaseTicks;
}

//--------------------------------------------------------------------------------------------------
//--------------------------------------------------------------------------------------------------
uint64_t SystemTime::GetInTicks()
{
	LARGE_INTEGER counter;
	return QueryPerformanceCounter( &counter ) ? counter.QuadPart : GetTickCount64();
}

//--------------------------------------------------------------------------------------------------
//--------------------------------------------------------------------------------------------------
double SystemTime::GetInSeconds()
{
	if ( !s_bInitialized )
		Init();

	return ( GetInTicks() - s_nBaseTicks ) * s_flSecondsPerTick;
}

//--------------------------------------------------------------------------------------------------
//--------------------------------------------------------------------------------------------------
double SystemTime::GetInSeconds( uint64_t nTicks )
{
	if ( !s_bInitialized )
		Init();

	return ( ( int64_t )( nTicks - s_nBaseTicks ) ) * s_flSecondsPerTick;
}

