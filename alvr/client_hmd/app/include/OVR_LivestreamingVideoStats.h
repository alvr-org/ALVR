// This file was @generated with LibOVRPlatform/codegen/main. Do not modify it!

#ifndef OVR_LIVESTREAMINGVIDEOSTATS_H
#define OVR_LIVESTREAMINGVIDEOSTATS_H

#include "OVR_Platform_Defs.h"

typedef struct ovrLivestreamingVideoStats *ovrLivestreamingVideoStatsHandle;

OVRP_PUBLIC_FUNCTION(int)          ovr_LivestreamingVideoStats_GetCommentCount(const ovrLivestreamingVideoStatsHandle obj);
OVRP_PUBLIC_FUNCTION(int)          ovr_LivestreamingVideoStats_GetReactionCount(const ovrLivestreamingVideoStatsHandle obj);
OVRP_PUBLIC_FUNCTION(const char *) ovr_LivestreamingVideoStats_GetTotalViews(const ovrLivestreamingVideoStatsHandle obj);

#endif
