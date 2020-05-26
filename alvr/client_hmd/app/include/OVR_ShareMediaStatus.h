// This file was @generated with LibOVRPlatform/codegen/main. Do not modify it!

#ifndef OVR_SHARE_MEDIA_STATUS_H
#define OVR_SHARE_MEDIA_STATUS_H

#include "OVR_Platform_Defs.h"

typedef enum ovrShareMediaStatus_ {
  ovrShareMediaStatus_Unknown,
  ovrShareMediaStatus_Shared,
  ovrShareMediaStatus_Canceled,
} ovrShareMediaStatus;

/// Converts an ovrShareMediaStatus enum value to a string
/// The returned string does not need to be freed
OVRPL_PUBLIC_FUNCTION(const char*) ovrShareMediaStatus_ToString(ovrShareMediaStatus value);

/// Converts a string representing an ovrShareMediaStatus enum value to an enum value
///
OVRPL_PUBLIC_FUNCTION(ovrShareMediaStatus) ovrShareMediaStatus_FromString(const char* str);

#endif
