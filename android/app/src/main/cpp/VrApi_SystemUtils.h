/************************************************************************************

Filename    :   VrApi_SystemUtils.h
Content     :   Interface for SystemUtils functionality.
Created     :   August 15, 2014
Authors     :   Gloria Kennickell, Jonathan E. Wright
Language    :   C99

Copyright   :   Copyright (c) Facebook Technologies, LLC and its affiliates. All rights reserved.

*************************************************************************************/
#ifndef OVR_VrApi_SystemUtils_h
#define OVR_VrApi_SystemUtils_h

#include "VrApi_Config.h"
#include "VrApi_Types.h"

#if defined(__cplusplus)
extern "C" {
#endif

typedef enum ovrSystemUIType_ {
    VRAPI_SYS_UI_CONFIRM_QUIT_MENU = 1, // Display the 'Confirm Quit' Menu.
    } ovrSystemUIType;

/// Display a specific System UI.
OVR_VRAPI_EXPORT bool vrapi_ShowSystemUI(const ovrJava* java, const ovrSystemUIType type);


/// \deprecated Display a Fatal Error Message using the System UI.
OVR_VRAPI_DEPRECATED(OVR_VRAPI_EXPORT void vrapi_ShowFatalError(
    const ovrJava* java,
    const char* title,
    const char* message,
    const char* fileName,
    const unsigned int lineNumber));

#if defined(__cplusplus)
} // extern "C"
#endif

#endif // OVR_VrApi_SystemUtils_h
