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
    // enum 0 used to be VRAPI_SYS_UI_GLOBAL_MENU.

    VRAPI_SYS_UI_CONFIRM_QUIT_MENU = 1, // Display the 'Confirm Quit' Menu.

    VRAPI_SYS_UI_KEYBOARD_MENU = 2, // Display a Keyboard Menu for editing a single string.
    VRAPI_SYS_UI_FILE_DIALOG_MENU =
        3, // Display a Folder Browser Menu for selecting the path to a file or folder.

    } ovrSystemUIType;

/// Display a specific System UI.
OVR_VRAPI_EXPORT bool vrapi_ShowSystemUI(const ovrJava* java, const ovrSystemUIType type);

/// \deprecated Display a specific System UI and pass extra JSON text data.
OVR_VRAPI_DEPRECATED(OVR_VRAPI_EXPORT bool vrapi_ShowSystemUIWithExtra(
    const ovrJava* java,
    const ovrSystemUIType type,
    const char* extraJsonText));


/// Display a Fatal Error Message using the System UI.
OVR_VRAPI_EXPORT void vrapi_ShowFatalError(
    const ovrJava* java,
    const char* title,
    const char* message,
    const char* fileName,
    const unsigned int lineNumber);

#if defined(__cplusplus)
} // extern "C"
#endif

#endif // OVR_VrApi_SystemUtils_h
