// This file was @generated with LibOVRPlatform/codegen/main. Do not modify it!

#ifndef OVR_NETSYNCSESSIONARRAY_H
#define OVR_NETSYNCSESSIONARRAY_H

#include "OVR_Platform_Defs.h"
#include "OVR_NetSyncSession.h"
#include <stddef.h>

typedef struct ovrNetSyncSessionArray *ovrNetSyncSessionArrayHandle;

OVRP_PUBLIC_FUNCTION(ovrNetSyncSessionHandle) ovr_NetSyncSessionArray_GetElement(const ovrNetSyncSessionArrayHandle obj, size_t index);
OVRP_PUBLIC_FUNCTION(size_t)                  ovr_NetSyncSessionArray_GetSize(const ovrNetSyncSessionArrayHandle obj);

#endif
