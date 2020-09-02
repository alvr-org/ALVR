// This file was @generated with LibOVRPlatform/codegen/main. Do not modify it!

#ifndef OVR_RICH_PRESENCE_EXTRA_CONTEXT_H
#define OVR_RICH_PRESENCE_EXTRA_CONTEXT_H

#include "OVR_Platform_Defs.h"

/// Display extra information about the user's presence
typedef enum ovrRichPresenceExtraContext_ {
  ovrRichPresenceExtraContext_Unknown,
  /// Display nothing
  ovrRichPresenceExtraContext_None,
  /// Display the current amount with the user over the max
  ovrRichPresenceExtraContext_CurrentCapacity,
  /// Display how long ago the match/game/race/etc started
  ovrRichPresenceExtraContext_StartedAgo,
  /// Display how soon the match/game/race/etc will end
  ovrRichPresenceExtraContext_EndingIn,
  /// Display that this user is looking for a match
  ovrRichPresenceExtraContext_LookingForAMatch,
} ovrRichPresenceExtraContext;

/// Converts an ovrRichPresenceExtraContext enum value to a string
/// The returned string does not need to be freed
OVRPL_PUBLIC_FUNCTION(const char*) ovrRichPresenceExtraContext_ToString(ovrRichPresenceExtraContext value);

/// Converts a string representing an ovrRichPresenceExtraContext enum value to an enum value
///
OVRPL_PUBLIC_FUNCTION(ovrRichPresenceExtraContext) ovrRichPresenceExtraContext_FromString(const char* str);

#endif
