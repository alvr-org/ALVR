// This file was @generated with LibOVRPlatform/codegen/main. Do not modify it!

#ifndef OVR_NETSYNCCONNECTION_H
#define OVR_NETSYNCCONNECTION_H

#include "OVR_Platform_Defs.h"
#include "OVR_NetSyncConnectionStatus.h"
#include "OVR_NetSyncDisconnectReason.h"
#include "OVR_Types.h"

typedef struct ovrNetSyncConnection *ovrNetSyncConnectionHandle;

/// If status is Disconnected, specifies the reason.
OVRP_PUBLIC_FUNCTION(ovrNetSyncDisconnectReason) ovr_NetSyncConnection_GetDisconnectReason(const ovrNetSyncConnectionHandle obj);

/// The ID of the local session. Will be null if the connection is not active
OVRP_PUBLIC_FUNCTION(ovrID) ovr_NetSyncConnection_GetSessionId(const ovrNetSyncConnectionHandle obj);

OVRP_PUBLIC_FUNCTION(long long)                  ovr_NetSyncConnection_GetConnectionId(const ovrNetSyncConnectionHandle obj);
OVRP_PUBLIC_FUNCTION(ovrNetSyncConnectionStatus) ovr_NetSyncConnection_GetStatus(const ovrNetSyncConnectionHandle obj);
OVRP_PUBLIC_FUNCTION(const char *)               ovr_NetSyncConnection_GetZoneId(const ovrNetSyncConnectionHandle obj);

#endif
