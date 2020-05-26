// This file was @generated with LibOVRPlatform/codegen/main. Do not modify it!

#ifndef OVR_LIVESTREAMINGSTATUS_H
#define OVR_LIVESTREAMINGSTATUS_H

#include "OVR_Platform_Defs.h"
#include <stdbool.h>

typedef struct ovrLivestreamingStatus *ovrLivestreamingStatusHandle;

OVRP_PUBLIC_FUNCTION(bool) ovr_LivestreamingStatus_GetCommentsVisible(const ovrLivestreamingStatusHandle obj);
OVRP_PUBLIC_FUNCTION(bool) ovr_LivestreamingStatus_GetIsPaused(const ovrLivestreamingStatusHandle obj);
OVRP_PUBLIC_FUNCTION(bool) ovr_LivestreamingStatus_GetLivestreamingEnabled(const ovrLivestreamingStatusHandle obj);
OVRP_PUBLIC_FUNCTION(int)  ovr_LivestreamingStatus_GetLivestreamingType(const ovrLivestreamingStatusHandle obj);
OVRP_PUBLIC_FUNCTION(bool) ovr_LivestreamingStatus_GetMicEnabled(const ovrLivestreamingStatusHandle obj);

#endif
