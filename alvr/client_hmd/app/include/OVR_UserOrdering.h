// This file was @generated with LibOVRPlatform/codegen/main. Do not modify it!

#ifndef OVR_USER_ORDERING_H
#define OVR_USER_ORDERING_H

#include "OVR_Platform_Defs.h"

/// The ordering that is used when returning a list of users. This is used in
/// some requests such as ovr_Room_GetInvitableUsers2()
typedef enum ovrUserOrdering_ {
  ovrUserOrdering_Unknown,
  /// No preference for ordering (could be in any or no order)
  ovrUserOrdering_None,
  /// Orders by online users first and then offline users. Within each group the
  /// users are ordered alphabetically by display name
  ovrUserOrdering_PresenceAlphabetical,
} ovrUserOrdering;

/// Converts an ovrUserOrdering enum value to a string
/// The returned string does not need to be freed
OVRPL_PUBLIC_FUNCTION(const char*) ovrUserOrdering_ToString(ovrUserOrdering value);

/// Converts a string representing an ovrUserOrdering enum value to an enum value
///
OVRPL_PUBLIC_FUNCTION(ovrUserOrdering) ovrUserOrdering_FromString(const char* str);

#endif
