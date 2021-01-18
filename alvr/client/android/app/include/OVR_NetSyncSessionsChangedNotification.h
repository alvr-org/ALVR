// This file was @generated with LibOVRPlatform/codegen/main. Do not modify it!

#ifndef OVR_NETSYNCSESSIONSCHANGEDNOTIFICATION_H
#define OVR_NETSYNCSESSIONSCHANGEDNOTIFICATION_H

#include "OVR_Platform_Defs.h"
#include "OVR_NetSyncSessionArray.h"

typedef struct ovrNetSyncSessionsChangedNotification *ovrNetSyncSessionsChangedNotificationHandle;

/// The new list of sessions
OVRP_PUBLIC_FUNCTION(ovrNetSyncSessionArrayHandle) ovr_NetSyncSessionsChangedNotification_GetSessions(const ovrNetSyncSessionsChangedNotificationHandle obj);

OVRP_PUBLIC_FUNCTION(long long) ovr_NetSyncSessionsChangedNotification_GetConnectionId(const ovrNetSyncSessionsChangedNotificationHandle obj);

#endif
