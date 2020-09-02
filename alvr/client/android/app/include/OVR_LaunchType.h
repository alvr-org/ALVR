// This file was @generated with LibOVRPlatform/codegen/main. Do not modify it!

#ifndef OVR_LAUNCH_TYPE_H
#define OVR_LAUNCH_TYPE_H

#include "OVR_Platform_Defs.h"

typedef enum ovrLaunchType_ {
  ovrLaunchType_Unknown,
  ovrLaunchType_Normal,
  ovrLaunchType_Invite,
  ovrLaunchType_Coordinated,
  ovrLaunchType_Deeplink,
} ovrLaunchType;

/// Converts an ovrLaunchType enum value to a string
/// The returned string does not need to be freed
OVRPL_PUBLIC_FUNCTION(const char*) ovrLaunchType_ToString(ovrLaunchType value);

/// Converts a string representing an ovrLaunchType enum value to an enum value
///
OVRPL_PUBLIC_FUNCTION(ovrLaunchType) ovrLaunchType_FromString(const char* str);

#endif
