// This file was @generated with LibOVRPlatform/codegen/main. Do not modify it!

#ifndef OVR_SDK_ACCOUNT_TYPE_H
#define OVR_SDK_ACCOUNT_TYPE_H

#include "OVR_Platform_Defs.h"

typedef enum ovrSdkAccountType_ {
  ovrSdkAccountType_Unknown,
  ovrSdkAccountType_Oculus,
  ovrSdkAccountType_FacebookGameroom,
} ovrSdkAccountType;

/// Converts an ovrSdkAccountType enum value to a string
/// The returned string does not need to be freed
OVRPL_PUBLIC_FUNCTION(const char*) ovrSdkAccountType_ToString(ovrSdkAccountType value);

/// Converts a string representing an ovrSdkAccountType enum value to an enum value
///
OVRPL_PUBLIC_FUNCTION(ovrSdkAccountType) ovrSdkAccountType_FromString(const char* str);

#endif
