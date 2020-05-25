// This file was @generated with LibOVRPlatform/codegen/main. Do not modify it!

#ifndef OVR_SYSTEM_VOIP_STATUS_H
#define OVR_SYSTEM_VOIP_STATUS_H

#include "OVR_Platform_Defs.h"

typedef enum ovrSystemVoipStatus_ {
  ovrSystemVoipStatus_Unknown,
  ovrSystemVoipStatus_Unavailable,
  ovrSystemVoipStatus_Suppressed,
  ovrSystemVoipStatus_Active,
} ovrSystemVoipStatus;

/// Converts an ovrSystemVoipStatus enum value to a string
/// The returned string does not need to be freed
OVRPL_PUBLIC_FUNCTION(const char*) ovrSystemVoipStatus_ToString(ovrSystemVoipStatus value);

/// Converts a string representing an ovrSystemVoipStatus enum value to an enum value
///
OVRPL_PUBLIC_FUNCTION(ovrSystemVoipStatus) ovrSystemVoipStatus_FromString(const char* str);

#endif
