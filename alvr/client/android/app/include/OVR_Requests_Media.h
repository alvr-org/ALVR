// This file was @generated with LibOVRPlatform/codegen/main. Do not modify it!

#ifndef OVR_REQUESTS_MEDIA_H
#define OVR_REQUESTS_MEDIA_H

#include "OVR_Types.h"
#include "OVR_Platform_Defs.h"


/// \file
/// # Media Overview
///
/// Sharing a screenshot or other media to Facebook is a great way to increase visibility of your
/// application to people that might not have a VR Headset.
///
/// ## Using Share Media to Facebook
///
/// Oculus currently supports sharing photos to Facebook. You can pass us the path to a screenshot
/// or other photo on the user's phone, and we'll launch a share-to-facebook modal, allowing
/// the user to share that photo to Facebook from VR. You can also pass us a default comment to
/// prepopulate the user's facebook status for the post.

/// Launch the Share to Facebook modal via a deeplink to Home on Gear VR,
/// allowing users to share local media files to Facebook. Accepts a
/// postTextSuggestion string for the default text of the Facebook post.
/// Requires a filePath string as the path to the image to be shared to
/// Facebook. This image should be located in your app's internal storage
/// directory. Requires a contentType indicating the type of media to be shared
/// (only 'photo' is currently supported.)
/// \param postTextSuggestion this text will prepopulate the facebook status text-input box within the share modal
/// \param filePath path to the file to be shared to facebook
/// \param contentType content type of the media to be shared
///
/// A message with type ::ovrMessage_Media_ShareToFacebook will be generated in response.
///
/// First call ::ovr_Message_IsError() to check if an error occurred.
///
/// If no error occurred, the message will contain a payload of type ::ovrShareMediaResultHandle.
/// Extract the payload from the message handle with ::ovr_Message_GetShareMediaResult().
OVRP_PUBLIC_FUNCTION(ovrRequest) ovr_Media_ShareToFacebook(const char *postTextSuggestion, const char *filePath, ovrMediaContentType contentType);

#endif
