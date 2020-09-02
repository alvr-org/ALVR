// This file was @generated with LibOVRPlatform/codegen/main. Do not modify it!

#ifndef OVR_NETSYNCSESSION_H
#define OVR_NETSYNCSESSION_H

#include "OVR_Platform_Defs.h"
#include "OVR_Types.h"
#include <stdbool.h>

typedef struct ovrNetSyncSession *ovrNetSyncSessionHandle;

/// Which connection this session exists within
OVRP_PUBLIC_FUNCTION(long long) ovr_NetSyncSession_GetConnectionId(const ovrNetSyncSessionHandle obj);

/// True if the local session has muted this session.
OVRP_PUBLIC_FUNCTION(bool) ovr_NetSyncSession_GetMuted(const ovrNetSyncSessionHandle obj);

/// The cloud networking internal session ID that represents this connection.
OVRP_PUBLIC_FUNCTION(ovrID) ovr_NetSyncSession_GetSessionId(const ovrNetSyncSessionHandle obj);

/// The ovrID of the user behind this session
OVRP_PUBLIC_FUNCTION(ovrID) ovr_NetSyncSession_GetUserId(const ovrNetSyncSessionHandle obj);

/// The name of the voip group that this session is subscribed to
OVRP_PUBLIC_FUNCTION(const char *) ovr_NetSyncSession_GetVoipGroup(const ovrNetSyncSessionHandle obj);


#endif
