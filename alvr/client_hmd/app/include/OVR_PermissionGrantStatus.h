// This file was @generated with LibOVRPlatform/codegen/main. Do not modify it!

#ifndef OVR_PERMISSION_GRANT_STATUS_H
#define OVR_PERMISSION_GRANT_STATUS_H

#include "OVR_Platform_Defs.h"

typedef enum ovrPermissionGrantStatus_ {
  ovrPermissionGrantStatus_Unknown,
  ovrPermissionGrantStatus_Granted,
  ovrPermissionGrantStatus_Denied,
  ovrPermissionGrantStatus_Blocked,
} ovrPermissionGrantStatus;

/// Converts an ovrPermissionGrantStatus enum value to a string
/// The returned string does not need to be freed
OVRPL_PUBLIC_FUNCTION(const char*) ovrPermissionGrantStatus_ToString(ovrPermissionGrantStatus value);

/// Converts a string representing an ovrPermissionGrantStatus enum value to an enum value
///
OVRPL_PUBLIC_FUNCTION(ovrPermissionGrantStatus) ovrPermissionGrantStatus_FromString(const char* str);

#endif
