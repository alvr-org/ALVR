// This file was @generated with LibOVRPlatform/codegen/main. Do not modify it!

#ifndef OVR_REQUESTS_PARTY_H
#define OVR_REQUESTS_PARTY_H

#include "OVR_Types.h"
#include "OVR_Platform_Defs.h"


/// \file
/// # Overview
///
/// Parties are groups that can communicate and easily move across different
/// app experiences together. Users in parties will have a System VoIP connection
/// allowing them to talk to everyone in their party. Users can only be in
/// a single party at a time, but they are not necessarily always in a party.
/// Parties are primarily intended to be a system that allows users to more easily
/// communicate and organize meetings with their friends in VR.
///
/// # Fields
///
/// ## Users
///
/// It's possible to fetch the list of users in a room with `ovr_Party_GetUsers`.
///
/// ## Invited Users
///
/// It's possible to fetch a list of all currently invited users with
/// `ovr_Party_GetInvitedUsers`.

/// Load the party the current user is in.
///
/// A message with type ::ovrMessage_Party_GetCurrent will be generated in response.
///
/// First call ::ovr_Message_IsError() to check if an error occurred.
///
/// If no error occurred, the message will contain a payload of type ::ovrPartyHandle.
/// Extract the payload from the message handle with ::ovr_Message_GetParty().
OVRP_PUBLIC_FUNCTION(ovrRequest) ovr_Party_GetCurrent();

#endif
