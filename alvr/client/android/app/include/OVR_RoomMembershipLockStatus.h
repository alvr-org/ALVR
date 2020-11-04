// This file was @generated with LibOVRPlatform/codegen/main. Do not modify it!

#ifndef OVR_ROOM_MEMBERSHIP_LOCK_STATUS_H
#define OVR_ROOM_MEMBERSHIP_LOCK_STATUS_H

#include "OVR_Platform_Defs.h"

typedef enum ovrRoomMembershipLockStatus_ {
  ovrRoomMembershipLockStatus_Unknown,
  ovrRoomMembershipLockStatus_Lock,
  ovrRoomMembershipLockStatus_Unlock,
} ovrRoomMembershipLockStatus;

/// Converts an ovrRoomMembershipLockStatus enum value to a string
/// The returned string does not need to be freed
OVRPL_PUBLIC_FUNCTION(const char*) ovrRoomMembershipLockStatus_ToString(ovrRoomMembershipLockStatus value);

/// Converts a string representing an ovrRoomMembershipLockStatus enum value to an enum value
///
OVRPL_PUBLIC_FUNCTION(ovrRoomMembershipLockStatus) ovrRoomMembershipLockStatus_FromString(const char* str);

#endif
