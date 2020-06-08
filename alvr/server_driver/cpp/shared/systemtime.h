//===================== Copyright (c) Valve Corporation. All Rights Reserved. ======================
//
// Functions for working with the system clock.
//
//==================================================================================================
#pragma once

#include <stdint.h>

namespace SystemTime
{
	// Automatically invoked, but can be called to specify a common base ticks for synchronization between processes.
	void Init( uint64_t nBaseTicks = 0 );

	// Returns the base ticks (for synchronizing with another process).
	uint64_t GetBaseTicks();

	// Returns current system time in ticks.
	uint64_t GetInTicks();

	// Returns current system time in seconds.
	double GetInSeconds();

	// Converts ticks to seconds.
	double GetInSeconds( uint64_t nTicks );
}

