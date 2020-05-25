// This file was @generated with LibOVRPlatform/codegen/main. Do not modify it!

#ifndef OVR_ERROR_H
#define OVR_ERROR_H

#include "OVR_Platform_Defs.h"

typedef struct ovrError *ovrErrorHandle;

/// Human readable description of the error that can be displayed to the user.
/// Might be the empty string if there is no user-appropriate description
/// available. Not intended to be parsed as it might change at any time or be
/// translated.
OVRP_PUBLIC_FUNCTION(const char *) ovr_Error_GetDisplayableMessage(const ovrErrorHandle obj);

/// Technical description of what went wrong intended for developers. For use
/// in logs or developer consoles.
OVRP_PUBLIC_FUNCTION(const char *) ovr_Error_GetMessage(const ovrErrorHandle obj);

OVRP_PUBLIC_FUNCTION(int) ovr_Error_GetCode(const ovrErrorHandle obj);
OVRP_PUBLIC_FUNCTION(int) ovr_Error_GetHttpCode(const ovrErrorHandle obj);

#endif
