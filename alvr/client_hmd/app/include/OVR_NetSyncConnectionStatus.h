// This file was @generated with LibOVRPlatform/codegen/main. Do not modify it!

#ifndef OVR_NET_SYNC_CONNECTION_STATUS_H
#define OVR_NET_SYNC_CONNECTION_STATUS_H

#include "OVR_Platform_Defs.h"

typedef enum ovrNetSyncConnectionStatus_ {
  ovrNetSyncConnectionStatus_Unknown,
  ovrNetSyncConnectionStatus_Connecting,
  ovrNetSyncConnectionStatus_Disconnected,
  ovrNetSyncConnectionStatus_Connected,
} ovrNetSyncConnectionStatus;

/// Converts an ovrNetSyncConnectionStatus enum value to a string
/// The returned string does not need to be freed
OVRPL_PUBLIC_FUNCTION(const char*) ovrNetSyncConnectionStatus_ToString(ovrNetSyncConnectionStatus value);

/// Converts a string representing an ovrNetSyncConnectionStatus enum value to an enum value
///
OVRPL_PUBLIC_FUNCTION(ovrNetSyncConnectionStatus) ovrNetSyncConnectionStatus_FromString(const char* str);

#endif
