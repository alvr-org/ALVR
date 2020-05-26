// This file was @generated with LibOVRPlatform/codegen/main. Do not modify it!

#ifndef OVR_NET_SYNC_VOIP_STREAM_MODE_H
#define OVR_NET_SYNC_VOIP_STREAM_MODE_H

#include "OVR_Platform_Defs.h"

typedef enum ovrNetSyncVoipStreamMode_ {
  ovrNetSyncVoipStreamMode_Unknown,
  ovrNetSyncVoipStreamMode_Ambisonic,
  ovrNetSyncVoipStreamMode_Mono,
} ovrNetSyncVoipStreamMode;

/// Converts an ovrNetSyncVoipStreamMode enum value to a string
/// The returned string does not need to be freed
OVRPL_PUBLIC_FUNCTION(const char*) ovrNetSyncVoipStreamMode_ToString(ovrNetSyncVoipStreamMode value);

/// Converts a string representing an ovrNetSyncVoipStreamMode enum value to an enum value
///
OVRPL_PUBLIC_FUNCTION(ovrNetSyncVoipStreamMode) ovrNetSyncVoipStreamMode_FromString(const char* str);

#endif
