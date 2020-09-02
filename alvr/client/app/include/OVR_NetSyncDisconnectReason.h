// This file was @generated with LibOVRPlatform/codegen/main. Do not modify it!

#ifndef OVR_NET_SYNC_DISCONNECT_REASON_H
#define OVR_NET_SYNC_DISCONNECT_REASON_H

#include "OVR_Platform_Defs.h"

typedef enum ovrNetSyncDisconnectReason_ {
  ovrNetSyncDisconnectReason_Unknown,
  /// when disconnect was requested
  ovrNetSyncDisconnectReason_LocalTerminated,
  /// server intentionally closed the connection
  ovrNetSyncDisconnectReason_ServerTerminated,
  /// initial connection never succeeded
  ovrNetSyncDisconnectReason_Failed,
  /// network timeout
  ovrNetSyncDisconnectReason_Lost,
} ovrNetSyncDisconnectReason;

/// Converts an ovrNetSyncDisconnectReason enum value to a string
/// The returned string does not need to be freed
OVRPL_PUBLIC_FUNCTION(const char*) ovrNetSyncDisconnectReason_ToString(ovrNetSyncDisconnectReason value);

/// Converts a string representing an ovrNetSyncDisconnectReason enum value to an enum value
///
OVRPL_PUBLIC_FUNCTION(ovrNetSyncDisconnectReason) ovrNetSyncDisconnectReason_FromString(const char* str);

#endif
