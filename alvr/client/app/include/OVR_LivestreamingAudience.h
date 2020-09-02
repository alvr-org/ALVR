// This file was @generated with LibOVRPlatform/codegen/main. Do not modify it!

#ifndef OVR_LIVESTREAMING_AUDIENCE_H
#define OVR_LIVESTREAMING_AUDIENCE_H

#include "OVR_Platform_Defs.h"

typedef enum ovrLivestreamingAudience_ {
  ovrLivestreamingAudience_Unknown,
  ovrLivestreamingAudience_Public,
  ovrLivestreamingAudience_Friends,
  ovrLivestreamingAudience_OnlyMe,
} ovrLivestreamingAudience;

/// Converts an ovrLivestreamingAudience enum value to a string
/// The returned string does not need to be freed
OVRPL_PUBLIC_FUNCTION(const char*) ovrLivestreamingAudience_ToString(ovrLivestreamingAudience value);

/// Converts a string representing an ovrLivestreamingAudience enum value to an enum value
///
OVRPL_PUBLIC_FUNCTION(ovrLivestreamingAudience) ovrLivestreamingAudience_FromString(const char* str);

#endif
