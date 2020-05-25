// This file was @generated with LibOVRPlatform/codegen/main. Do not modify it!

#ifndef OVR_TIME_WINDOW_H
#define OVR_TIME_WINDOW_H

#include "OVR_Platform_Defs.h"

/// How far should we go back in time looking at history? This is used in some
/// requests such as ovr_User_GetLoggedInUserRecentlyMetUsersAndRooms()
typedef enum ovrTimeWindow_ {
  ovrTimeWindow_Unknown,
  ovrTimeWindow_OneHour,
  ovrTimeWindow_OneDay,
  ovrTimeWindow_OneWeek,
  ovrTimeWindow_ThirtyDays,
  ovrTimeWindow_NinetyDays,
} ovrTimeWindow;

/// Converts an ovrTimeWindow enum value to a string
/// The returned string does not need to be freed
OVRPL_PUBLIC_FUNCTION(const char*) ovrTimeWindow_ToString(ovrTimeWindow value);

/// Converts a string representing an ovrTimeWindow enum value to an enum value
///
OVRPL_PUBLIC_FUNCTION(ovrTimeWindow) ovrTimeWindow_FromString(const char* str);

#endif
