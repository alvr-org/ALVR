// This file was @generated with LibOVRPlatform/codegen/main. Do not modify it!

#ifndef OVR_VOIP_MUTE_STATE_H
#define OVR_VOIP_MUTE_STATE_H

#include "OVR_Platform_Defs.h"

typedef enum ovrVoipMuteState_ {
  ovrVoipMuteState_Unknown,
  ovrVoipMuteState_Muted,
  ovrVoipMuteState_Unmuted,
} ovrVoipMuteState;

/// Converts an ovrVoipMuteState enum value to a string
/// The returned string does not need to be freed
OVRPL_PUBLIC_FUNCTION(const char*) ovrVoipMuteState_ToString(ovrVoipMuteState value);

/// Converts a string representing an ovrVoipMuteState enum value to an enum value
///
OVRPL_PUBLIC_FUNCTION(ovrVoipMuteState) ovrVoipMuteState_FromString(const char* str);

#endif
