// This file was @generated with LibOVRPlatform/codegen/main. Do not modify it!

#ifndef OVR_ROOM_JOINABILITY_H
#define OVR_ROOM_JOINABILITY_H

#include "OVR_Platform_Defs.h"

typedef enum ovrRoomJoinability_ {
  ovrRoom_JoinabilityUnknown,
  ovrRoom_JoinabilityAreIn,
  ovrRoom_JoinabilityAreKicked,
  ovrRoom_JoinabilityCanJoin,
  ovrRoom_JoinabilityIsFull,
  ovrRoom_JoinabilityNoViewer,
  ovrRoom_JoinabilityPolicyPrevents,
} ovrRoomJoinability;

/// Converts an ovrRoomJoinability enum value to a string
/// The returned string does not need to be freed
OVRPL_PUBLIC_FUNCTION(const char*) ovrRoomJoinability_ToString(ovrRoomJoinability value);

/// Converts a string representing an ovrRoomJoinability enum value to an enum value
///
OVRPL_PUBLIC_FUNCTION(ovrRoomJoinability) ovrRoomJoinability_FromString(const char* str);

#endif
