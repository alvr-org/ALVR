// This file was @generated with LibOVRPlatform/codegen/main. Do not modify it!

#ifndef OVR_DESTINATIONARRAY_H
#define OVR_DESTINATIONARRAY_H

#include "OVR_Platform_Defs.h"
#include "OVR_Destination.h"
#include <stdbool.h>
#include <stddef.h>

typedef struct ovrDestinationArray *ovrDestinationArrayHandle;

OVRP_PUBLIC_FUNCTION(ovrDestinationHandle) ovr_DestinationArray_GetElement(const ovrDestinationArrayHandle obj, size_t index);
OVRP_PUBLIC_FUNCTION(const char *)         ovr_DestinationArray_GetNextUrl(const ovrDestinationArrayHandle obj);
OVRP_PUBLIC_FUNCTION(size_t)               ovr_DestinationArray_GetSize(const ovrDestinationArrayHandle obj);
OVRP_PUBLIC_FUNCTION(bool)                 ovr_DestinationArray_HasNextPage(const ovrDestinationArrayHandle obj);

#endif
