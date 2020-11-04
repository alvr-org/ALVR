// This file was @generated with LibOVRPlatform/codegen/main. Do not modify it!

#ifndef OVR_PARTY_UPDATE_ACTION_H
#define OVR_PARTY_UPDATE_ACTION_H

#include "OVR_Platform_Defs.h"

typedef enum ovrPartyUpdateAction_ {
  ovrPartyUpdateAction_Unknown,
  ovrPartyUpdateAction_Join,
  ovrPartyUpdateAction_Leave,
  ovrPartyUpdateAction_Invite,
  ovrPartyUpdateAction_Uninvite,
} ovrPartyUpdateAction;

/// Converts an ovrPartyUpdateAction enum value to a string
/// The returned string does not need to be freed
OVRPL_PUBLIC_FUNCTION(const char*) ovrPartyUpdateAction_ToString(ovrPartyUpdateAction value);

/// Converts a string representing an ovrPartyUpdateAction enum value to an enum value
///
OVRPL_PUBLIC_FUNCTION(ovrPartyUpdateAction) ovrPartyUpdateAction_FromString(const char* str);

#endif
