// This file was @generated with LibOVRPlatform/codegen/main. Do not modify it!

#ifndef OVR_PERMISSION_TYPE_H
#define OVR_PERMISSION_TYPE_H

#include "OVR_Platform_Defs.h"

typedef enum ovrPermissionType_ {
  ovrPermissionType_Unknown,
  ovrPermissionType_Microphone,
  ovrPermissionType_WriteExternalStorage,
} ovrPermissionType;

/// Converts an ovrPermissionType enum value to a string
/// The returned string does not need to be freed
OVRPL_PUBLIC_FUNCTION(const char*) ovrPermissionType_ToString(ovrPermissionType value);

/// Converts a string representing an ovrPermissionType enum value to an enum value
///
OVRPL_PUBLIC_FUNCTION(ovrPermissionType) ovrPermissionType_FromString(const char* str);

#endif
