// This file was @generated with LibOVRPlatform/codegen/main. Do not modify it!

#ifndef OVR_ROOM_JOIN_POLICY_H
#define OVR_ROOM_JOIN_POLICY_H

#include "OVR_Platform_Defs.h"

typedef enum ovrRoomJoinPolicy_ {
  ovrRoom_JoinPolicyNone,
  ovrRoom_JoinPolicyEveryone,
  ovrRoom_JoinPolicyFriendsOfMembers,
  ovrRoom_JoinPolicyFriendsOfOwner,
  ovrRoom_JoinPolicyInvitedUsers,
  ovrRoom_JoinPolicyUnknown,
} ovrRoomJoinPolicy;

/// Converts an ovrRoomJoinPolicy enum value to a string
/// The returned string does not need to be freed
OVRPL_PUBLIC_FUNCTION(const char*) ovrRoomJoinPolicy_ToString(ovrRoomJoinPolicy value);

/// Converts a string representing an ovrRoomJoinPolicy enum value to an enum value
///
OVRPL_PUBLIC_FUNCTION(ovrRoomJoinPolicy) ovrRoomJoinPolicy_FromString(const char* str);

#endif
