// This file was @generated with LibOVRPlatform/codegen/main. Do not modify it!

#ifndef OVR_KEY_VALUE_PAIR_TYPE_H
#define OVR_KEY_VALUE_PAIR_TYPE_H

#include "OVR_Platform_Defs.h"

typedef enum ovrKeyValuePairType_ {
  ovrKeyValuePairType_String,
  ovrKeyValuePairType_Int,
  ovrKeyValuePairType_Double,
  ovrKeyValuePairType_Unknown,
} ovrKeyValuePairType;

/// Converts an ovrKeyValuePairType enum value to a string
/// The returned string does not need to be freed
OVRPL_PUBLIC_FUNCTION(const char*) ovrKeyValuePairType_ToString(ovrKeyValuePairType value);

/// Converts a string representing an ovrKeyValuePairType enum value to an enum value
///
OVRPL_PUBLIC_FUNCTION(ovrKeyValuePairType) ovrKeyValuePairType_FromString(const char* str);

#endif
