// This file was @generated with LibOVRPlatform/codegen/main. Do not modify it!

#ifndef OVR_LINKEDACCOUNT_H
#define OVR_LINKEDACCOUNT_H

#include "OVR_Platform_Defs.h"
#include "OVR_ServiceProvider.h"

typedef struct ovrLinkedAccount *ovrLinkedAccountHandle;

/// Access token of the linked account.
OVRP_PUBLIC_FUNCTION(const char *) ovr_LinkedAccount_GetAccessToken(const ovrLinkedAccountHandle obj);

/// Service provider with which the linked account is associated.
OVRP_PUBLIC_FUNCTION(ovrServiceProvider) ovr_LinkedAccount_GetServiceProvider(const ovrLinkedAccountHandle obj);

/// User ID of the linked account.
OVRP_PUBLIC_FUNCTION(const char *) ovr_LinkedAccount_GetUserId(const ovrLinkedAccountHandle obj);


#endif
