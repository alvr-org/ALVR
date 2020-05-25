// This file was @generated with LibOVRPlatform/codegen/main. Do not modify it!

#ifndef OVR_LANGUAGEPACKINFO_H
#define OVR_LANGUAGEPACKINFO_H

#include "OVR_Platform_Defs.h"

typedef struct ovrLanguagePackInfo *ovrLanguagePackInfoHandle;

/// Language name in English language.
OVRP_PUBLIC_FUNCTION(const char *) ovr_LanguagePackInfo_GetEnglishName(const ovrLanguagePackInfoHandle obj);

/// Language name in the native language.
OVRP_PUBLIC_FUNCTION(const char *) ovr_LanguagePackInfo_GetNativeName(const ovrLanguagePackInfoHandle obj);

/// Language tag in BCP47 format.
OVRP_PUBLIC_FUNCTION(const char *) ovr_LanguagePackInfo_GetTag(const ovrLanguagePackInfoHandle obj);


#endif
