// This file was @generated with LibOVRPlatform/codegen/main. Do not modify it!

#ifndef OVR_ROOM_TYPE_H
#define OVR_ROOM_TYPE_H

#include "OVR_Platform_Defs.h"

typedef enum ovrRoomType_ {
  ovrRoom_TypeUnknown,
  ovrRoom_TypeMatchmaking,
  ovrRoom_TypeModerated,
  ovrRoom_TypePrivate,
  ovrRoom_TypeSolo,
} ovrRoomType;

/// Converts an ovrRoomType enum value to a string
/// The returned string does not need to be freed
OVRPL_PUBLIC_FUNCTION(const char*) ovrRoomType_ToString(ovrRoomType value);

/// Converts a string representing an ovrRoomType enum value to an enum value
///
OVRPL_PUBLIC_FUNCTION(ovrRoomType) ovrRoomType_FromString(const char* str);

#endif
