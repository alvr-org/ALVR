// This file was @generated with LibOVRPlatform/codegen/main. Do not modify it!

#ifndef OVR_PARTYUPDATENOTIFICATION_H
#define OVR_PARTYUPDATENOTIFICATION_H

#include "OVR_Platform_Defs.h"
#include "OVR_PartyUpdateAction.h"
#include "OVR_Types.h"

typedef struct ovrPartyUpdateNotification *ovrPartyUpdateNotificationHandle;

OVRP_PUBLIC_FUNCTION(ovrPartyUpdateAction) ovr_PartyUpdateNotification_GetAction(const ovrPartyUpdateNotificationHandle obj);
OVRP_PUBLIC_FUNCTION(ovrID)                ovr_PartyUpdateNotification_GetPartyId(const ovrPartyUpdateNotificationHandle obj);
OVRP_PUBLIC_FUNCTION(ovrID)                ovr_PartyUpdateNotification_GetSenderId(const ovrPartyUpdateNotificationHandle obj);
OVRP_PUBLIC_FUNCTION(const char *)         ovr_PartyUpdateNotification_GetUpdateTimestamp(const ovrPartyUpdateNotificationHandle obj);
OVRP_PUBLIC_FUNCTION(const char *)         ovr_PartyUpdateNotification_GetUserAlias(const ovrPartyUpdateNotificationHandle obj);
OVRP_PUBLIC_FUNCTION(ovrID)                ovr_PartyUpdateNotification_GetUserId(const ovrPartyUpdateNotificationHandle obj);
OVRP_PUBLIC_FUNCTION(const char *)         ovr_PartyUpdateNotification_GetUserName(const ovrPartyUpdateNotificationHandle obj);

#endif
