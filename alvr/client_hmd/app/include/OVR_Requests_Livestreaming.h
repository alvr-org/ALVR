// This file was @generated with LibOVRPlatform/codegen/main. Do not modify it!

#ifndef OVR_REQUESTS_LIVESTREAMING_H
#define OVR_REQUESTS_LIVESTREAMING_H

#include "OVR_Types.h"
#include "OVR_Platform_Defs.h"


/// \file
/// # Livestreaming Overview
///
/// Livestreaming to Facebook is a great way to increase visibility of your
/// application to people that might not have a VR Headset.
///
/// ## Handling Livestreaming Status Changes
/// We provide both a push and a pull API for Livestreaming status changes.
///
/// ### Pull
/// You can call ovr_Livestreaming_GetStatus() to retrieve the current livestreaming
/// status.
///
/// ### Push
/// You can register for the ovrMessage_Notification_Livestreaming_StatusChange
/// notification to be alerted when the user starts or stops an active livestream.
///
/// ## Pausing / Resuming the livestreaming.
/// Potentially your application has content that don't make sense to livestream
/// publicly. For example: a pin entry screen or a sensitive social interaction.
///
/// You can toggle the livestreaming state (INCLUDING audio) using
/// ovr_Livestreaming_PauseStream() and ovr_Livestreaming_ResumeStream().
///
/// NOTE both of these methods are safe to call when no livestream is active.
///
/// ## Enabling / Disabling the feature.
/// If want to disable (or re-enable) livestreaming for your application, you
/// can visit the developer dashboard:
///
/// https://dashboard.oculus.com/app/sharing

/// Return the status of the current livestreaming session if there is one.
///
/// A message with type ::ovrMessage_Livestreaming_GetStatus will be generated in response.
///
/// First call ::ovr_Message_IsError() to check if an error occurred.
///
/// If no error occurred, the message will contain a payload of type ::ovrLivestreamingStatusHandle.
/// Extract the payload from the message handle with ::ovr_Message_GetLivestreamingStatus().
OVRP_PUBLIC_FUNCTION(ovrRequest) ovr_Livestreaming_GetStatus();

/// Pauses the livestreaming session if there is one. NOTE: this function is
/// safe to call if no session is active.
///
/// A message with type ::ovrMessage_Livestreaming_PauseStream will be generated in response.
///
/// First call ::ovr_Message_IsError() to check if an error occurred.
///
/// If no error occurred, the message will contain a payload of type ::ovrLivestreamingStatusHandle.
/// Extract the payload from the message handle with ::ovr_Message_GetLivestreamingStatus().
OVRP_PUBLIC_FUNCTION(ovrRequest) ovr_Livestreaming_PauseStream();

/// Resumes the livestreaming session if there is one. NOTE: this function is
/// safe to call if no session is active.
///
/// A message with type ::ovrMessage_Livestreaming_ResumeStream will be generated in response.
///
/// First call ::ovr_Message_IsError() to check if an error occurred.
///
/// If no error occurred, the message will contain a payload of type ::ovrLivestreamingStatusHandle.
/// Extract the payload from the message handle with ::ovr_Message_GetLivestreamingStatus().
OVRP_PUBLIC_FUNCTION(ovrRequest) ovr_Livestreaming_ResumeStream();

#endif
