// This file was @generated with LibOVRPlatform/codegen/main. Do not modify it!

#ifndef OVR_TEAMARRAY_H
#define OVR_TEAMARRAY_H

#include "OVR_Platform_Defs.h"
#include "OVR_Team.h"
#include <stddef.h>

typedef struct ovrTeamArray *ovrTeamArrayHandle;

OVRP_PUBLIC_FUNCTION(ovrTeamHandle) ovr_TeamArray_GetElement(const ovrTeamArrayHandle obj, size_t index);
OVRP_PUBLIC_FUNCTION(size_t)        ovr_TeamArray_GetSize(const ovrTeamArrayHandle obj);

#endif
