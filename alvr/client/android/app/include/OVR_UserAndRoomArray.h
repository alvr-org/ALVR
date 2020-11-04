// This file was @generated with LibOVRPlatform/codegen/main. Do not modify it!

#ifndef OVR_USERANDROOMARRAY_H
#define OVR_USERANDROOMARRAY_H

#include "OVR_Platform_Defs.h"
#include "OVR_UserAndRoom.h"
#include <stdbool.h>
#include <stddef.h>

typedef struct ovrUserAndRoomArray *ovrUserAndRoomArrayHandle;

OVRP_PUBLIC_FUNCTION(ovrUserAndRoomHandle) ovr_UserAndRoomArray_GetElement(const ovrUserAndRoomArrayHandle obj, size_t index);
OVRP_PUBLIC_FUNCTION(const char *)         ovr_UserAndRoomArray_GetNextUrl(const ovrUserAndRoomArrayHandle obj);
OVRP_PUBLIC_FUNCTION(size_t)               ovr_UserAndRoomArray_GetSize(const ovrUserAndRoomArrayHandle obj);
OVRP_PUBLIC_FUNCTION(bool)                 ovr_UserAndRoomArray_HasNextPage(const ovrUserAndRoomArrayHandle obj);

#endif
