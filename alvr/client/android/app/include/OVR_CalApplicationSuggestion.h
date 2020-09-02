// This file was @generated with LibOVRPlatform/codegen/main. Do not modify it!

#ifndef OVR_CALAPPLICATIONSUGGESTION_H
#define OVR_CALAPPLICATIONSUGGESTION_H

#include "OVR_Platform_Defs.h"
#include "OVR_Types.h"

typedef struct ovrCalApplicationSuggestion *ovrCalApplicationSuggestionHandle;

/// Application ID of the suggested app.
OVRP_PUBLIC_FUNCTION(ovrID) ovr_CalApplicationSuggestion_GetID(const ovrCalApplicationSuggestionHandle obj);

/// Localized, privacy aware social context string to go with the app
/// suggestion. Intended to be directly displayed in UI.
OVRP_PUBLIC_FUNCTION(const char *) ovr_CalApplicationSuggestion_GetSocialContext(const ovrCalApplicationSuggestionHandle obj);


#endif
