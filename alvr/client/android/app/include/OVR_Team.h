// This file was @generated with LibOVRPlatform/codegen/main. Do not modify it!

#ifndef OVR_TEAM_H
#define OVR_TEAM_H

#include "OVR_Platform_Defs.h"
#include "OVR_UserArray.h"

typedef struct ovrTeam *ovrTeamHandle;

OVRP_PUBLIC_FUNCTION(ovrUserArrayHandle) ovr_Team_GetAssignedUsers(const ovrTeamHandle obj);
OVRP_PUBLIC_FUNCTION(int)                ovr_Team_GetMaxUsers(const ovrTeamHandle obj);
OVRP_PUBLIC_FUNCTION(int)                ovr_Team_GetMinUsers(const ovrTeamHandle obj);
OVRP_PUBLIC_FUNCTION(const char *)       ovr_Team_GetName(const ovrTeamHandle obj);

#endif
