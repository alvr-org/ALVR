// This file was @generated with LibOVRPlatform/codegen/main. Do not modify it!

#ifndef OVR_MEDIA_CONTENT_TYPE_H
#define OVR_MEDIA_CONTENT_TYPE_H

#include "OVR_Platform_Defs.h"

typedef enum ovrMediaContentType_ {
  ovrMediaContentType_Unknown,
  ovrMediaContentType_Photo,
} ovrMediaContentType;

/// Converts an ovrMediaContentType enum value to a string
/// The returned string does not need to be freed
OVRPL_PUBLIC_FUNCTION(const char*) ovrMediaContentType_ToString(ovrMediaContentType value);

/// Converts a string representing an ovrMediaContentType enum value to an enum value
///
OVRPL_PUBLIC_FUNCTION(ovrMediaContentType) ovrMediaContentType_FromString(const char* str);

#endif
