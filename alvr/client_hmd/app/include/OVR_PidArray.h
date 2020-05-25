// This file was @generated with LibOVRPlatform/codegen/main. Do not modify it!

#ifndef OVR_PIDARRAY_H
#define OVR_PIDARRAY_H

#include "OVR_Platform_Defs.h"
#include "OVR_Pid.h"
#include <stddef.h>

typedef struct ovrPidArray *ovrPidArrayHandle;

OVRP_PUBLIC_FUNCTION(ovrPidHandle) ovr_PidArray_GetElement(const ovrPidArrayHandle obj, size_t index);
OVRP_PUBLIC_FUNCTION(size_t)       ovr_PidArray_GetSize(const ovrPidArrayHandle obj);

#endif
