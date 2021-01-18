// This file was @generated with LibOVRPlatform/codegen/main. Do not modify it!

#ifndef OVR_REQUESTS_ROOM_H
#define OVR_REQUESTS_ROOM_H

#include "OVR_Types.h"
#include "OVR_Platform_Defs.h"

#include "OVR_RoomArray.h"
#include "OVR_RoomJoinPolicy.h"
#include "OVR_RoomMembershipLockStatus.h"
#include "OVR_RoomOptions.h"
#include <stdbool.h>

/// \file
/// # Overview
///
/// Rooms are what brings us together. This system allows you to gather groups of
/// users for a shared experience in virtual reality, complete with shared state
/// and automatic updates as people join and leave. Users are always in a room of
/// some type while your application is active. There are multiple types of rooms
/// available to suit a variety of game designs, let's dive in!
///
/// # Types
///
/// ## SOLO
///
/// Users are automatically added to their own solo room on app launch, and whenever
/// they leave (or are kicked) from a room they end up in one of these. These rooms
/// only have space for person in them (no one can be invited).
///
/// ## PRIVATE
///
/// Private rooms are created by clients and have mutable join permissions
/// (unlike SOLO, MODERATED, or MATCHMAKING). You can create them with
/// `ovr_Room_CreateAndJoinPrivate`, and they're best used in situations where you
/// expect users to play with friends and friends of friends.
///
/// ## MATCHMAKING
///
/// These rooms allow you to use the matchmaking system to find other players who
/// want to join in your experience. Find out more in OVR_Requests_Matchmaking.h
///
/// ## MODERATED
///
/// Moderated rooms have their lifecycle managed by the app developers. You'll need
/// to get your app secret from `https://dashboard.oculus.com/application/<YOUR_APP_ID>/api`
/// in order to create/delete them. DO NOT BRING THIS APP SECRET WITHIN 100 YARDS
/// OF YOUR CLIENTS. You can format an app secret token like:
///
///     <APP_SECRET_TOKEN> = OC|<YOUR_APP_ID>|<YOUR_APP_SECRET>
///
/// You can use this to create moderated rooms with our rest API from your server:
///
///     POST: graph.oculus.com/room_moderated_create?max_users=<MAX_USER_COUNT>&access_token=<APP_SECRET_TOKEN>
///     RESULT: { 'id': <MODERATED_ROOM_ID> }
///
/// The client can fetch the list of all moderated rooms with `ovr_Room_GetModeratedRooms`
///
/// You can delete moderated rooms with our rest API from your server:
///
///     DELETE: graph.oculus.com/<MODERATED_ROOM_ID>?access_token=<APP_SECRET_TOKEN>
///     RESULT: { 'success': true }
///
/// Moderated rooms are always public, meaning that (up to the maximum number of
/// users) anyone is in your app is allowed to join. This means they're best suited
/// for experiences that have a few public hosted options for a user to pick from.
///
/// # Fields
///
/// ## Description
///
/// The description is a public string field on every room that the room owner can
/// update with `ovr_Room_SetDescription`. This is best used to share information
/// about the current experience you (and the other room members) are having with
/// their friends. It can be fetched with `ovr_Room_GetDescription`.
///
/// ## Data Store
///
/// The data store is a public key/value map on MODERATED, PRIVATE, and MATCHMAKING
/// rooms that only the owner is able to update. It's most useful for allowing the
/// owner to set shared game state, and can be fetched with `ovr_Room_GetDataStore`.
///
/// The current owner of the room can update the data store with `ovr_Room_UpdateDataStore`.
/// If a user is creating a room, the data store can immediately be set with the room
/// options in `ovr_Room_CreateAndJoinPrivate2`. If the room has no owner, a user
/// joining the room can immediately update the data store with the room options
/// in `ovr_Room_Join2`.
///
/// IMPORTANT NOTE: You can update multiple keys/values in a single request, but
/// should not dispatch multiple parallel requests to mutate it or data loss will
/// result.
///
/// ## Join Policy
///
/// This field only exists for private rooms, and determines who is allowed to
/// join them without an invite from a current member. It is set at the time the
/// private room is created.
///
/// ## Joinability
///
/// This field informs the viewing user whether or not they can join the room
/// they're looking at, and if not why.
///
/// ## Max Users
///
/// The maximum number of users that can be in a room, fetched with `ovr_Room_getMaxUsers`.
///
/// ## Owner
///
/// The owner defaults to the user who has been in the room the longest (or no-one
/// if there isn't anyone in the room). The owner is able to pass ownership to
/// another room member via `ovr_Room_UpdateOwner`, and this can be fetched
/// with `ovr_Room_GetOwner`. Only the owner can update the description, data store,
/// join policy, and kick users from the room.
///
/// ## Users
///
/// It's possible to fetch the list of users in a room with `ovr_Room_GetUsers`.
/// The owner could use that list with `ovr_Room_KickUser` to kick a user from
/// the room. Rooms can be joined with `ovr_Room_Join` and left with
/// `ovr_Room_Leave`.
///
/// # Inviting Users
///
/// Room members are able to invite others to join them in room (space permitting)
/// by fetching the list of invitable users with `ovr_Room_GetInvitableUsers`,
/// allowing the user to select which they want to invite, and then using
/// `ovr_Room_InviteUser` to send the actual invite.
///
/// # Room Options
///
/// Several room requests accept an optional ovrMatchmakingOptionsHandle, which
/// contains settings for configuring the request. There is a suite of setter
/// functions that can be used to set the options:
///
/// ## Create/Destroy
///
/// ovr_RoomOptions_Create - Create an instance of ovrRoomOptionsHandle. Call ovr_RoomOptions_Destroy to
///     free the memory.
/// ovr_RoomOptions_Destroy - Call this to destroy an instance of ovrRoomOptionsHandle.
///
/// ## Join-specific Options
///
/// These options are relevant to CreateAndJoinPrivate2 and Join2.
///
/// ovr_RoomOptions_SetTurnOffUpdates - Defaults to false. Turning off updates will mean this user will
///     not get room update notifications
/// ovr_RoomOptions_SetDataStoreString - Only applies when joining as the owner of the room. If the user
///     does not become the owner, this field is ignored. See "Data Store" section above.
///
/// ## GetInvitableUsers-specific Options
///
/// These options are relevant to GetInvitableUsers2.
///
/// ovr_RoomOptions_SetRoomId - Defaults to the current room.  Sets which room to find invitable users for.
/// ovr_RoomOptions_SetOrdering - Set to specify what order should the users should be return in.

/// DEPRECATED. Use CreateAndJoinPrivate2.
/// \param joinPolicy Specifies who can join the room without an invite.
/// \param maxUsers The maximum number of users allowed in the room, including the creator.
/// \param subscribeToUpdates If true, sends a message with type ovrNotification_Room_RoomUpdate when room data changes, such as when users join or leave.
///
/// A message with type ::ovrMessage_Room_CreateAndJoinPrivate will be generated in response.
///
/// First call ::ovr_Message_IsError() to check if an error occurred.
///
/// If no error occurred, the message will contain a payload of type ::ovrRoomHandle.
/// Extract the payload from the message handle with ::ovr_Message_GetRoom().
OVRP_PUBLIC_FUNCTION(ovrRequest) ovr_Room_CreateAndJoinPrivate(ovrRoomJoinPolicy joinPolicy, unsigned int maxUsers, bool subscribeToUpdates);

/// Creates a new private (client controlled) room and adds the caller to it.
/// This type of room is good for matches where the user wants to play with
/// friends, as they're primarially discoverable by examining which rooms your
/// friends are in.
/// \param joinPolicy Specifies who can join the room without an invite.
/// \param maxUsers The maximum number of users allowed in the room, including the creator.
/// \param roomOptions Additional room configuration for this request. Optional.
///
/// A message with type ::ovrMessage_Room_CreateAndJoinPrivate2 will be generated in response.
///
/// First call ::ovr_Message_IsError() to check if an error occurred.
///
/// If no error occurred, the message will contain a payload of type ::ovrRoomHandle.
/// Extract the payload from the message handle with ::ovr_Message_GetRoom().
OVRP_PUBLIC_FUNCTION(ovrRequest) ovr_Room_CreateAndJoinPrivate2(ovrRoomJoinPolicy joinPolicy, unsigned int maxUsers, ovrRoomOptionsHandle roomOptions);

/// Allows arbitrary rooms for the application to be loaded.
/// \param roomID The room to load.
///
/// A message with type ::ovrMessage_Room_Get will be generated in response.
///
/// First call ::ovr_Message_IsError() to check if an error occurred.
///
/// If no error occurred, the message will contain a payload of type ::ovrRoomHandle.
/// Extract the payload from the message handle with ::ovr_Message_GetRoom().
OVRP_PUBLIC_FUNCTION(ovrRequest) ovr_Room_Get(ovrID roomID);

/// Easy loading of the room you're currently in. If you don't want live
/// updates on your current room (by using subscribeToUpdates), you can use
/// this to refresh the data.
///
/// A message with type ::ovrMessage_Room_GetCurrent will be generated in response.
///
/// First call ::ovr_Message_IsError() to check if an error occurred.
///
/// If no error occurred, the message will contain a payload of type ::ovrRoomHandle.
/// Extract the payload from the message handle with ::ovr_Message_GetRoom().
OVRP_PUBLIC_FUNCTION(ovrRequest) ovr_Room_GetCurrent();

/// Allows the current room for a given user to be loaded. Remember that the
/// user's privacy settings may not allow their room to be loaded. Because of
/// this, it's often possible to load the users in a room, but not to take
/// those users and load their room.
/// \param userID ID of the user for which to load the room.
///
/// A message with type ::ovrMessage_Room_GetCurrentForUser will be generated in response.
///
/// First call ::ovr_Message_IsError() to check if an error occurred.
///
/// If no error occurred, the message will contain a payload of type ::ovrRoomHandle.
/// Extract the payload from the message handle with ::ovr_Message_GetRoom().
OVRP_PUBLIC_FUNCTION(ovrRequest) ovr_Room_GetCurrentForUser(ovrID userID);

/// DEPRECATED. Use GetInvitableUsers2.
///
/// A message with type ::ovrMessage_Room_GetInvitableUsers will be generated in response.
///
/// First call ::ovr_Message_IsError() to check if an error occurred.
///
/// If no error occurred, the message will contain a payload of type ::ovrUserArrayHandle.
/// Extract the payload from the message handle with ::ovr_Message_GetUserArray().
OVRP_PUBLIC_FUNCTION(ovrRequest) ovr_Room_GetInvitableUsers();

/// Loads a list of users you can invite to a room. These are pulled from your
/// friends list and recently met lists and filtered for relevance and
/// interest. If the room cannot be joined, this list will be empty. By
/// default, the invitable users returned will be for the user's current room.
///
/// If your application grouping was created after September 9 2017, recently
/// met users will be included by default. If your application grouping was
/// created before then, you can go to edit the setting in the "Rooms and
/// Matchmaking" section of Platform Services at dashboard.oculus.com
///
/// Customization can be done via RoomOptions. Create this object with
/// ovr_RoomOptions_Create. The params that could be used are:
///
/// 1. ovr_RoomOptions_SetRoomId- will return the invitable users for this room
/// (instead of the current room).
///
/// 2. ovr_RoomOptions_SetOrdering - returns the list of users in the provided
/// ordering (see UserOrdering enum).
///
/// 3. ovr_RoomOptions_SetRecentlyMetTimeWindow - how long long ago should we
/// include users you've recently met in the results?
///
/// 4. ovr_RoomOptions_SetMaxUserResults - we will limit the number of results
/// returned. By default, the number is unlimited, but the server may choose to
/// limit results for performance reasons.
///
/// 5. ovr_RoomOptions_SetExcludeRecentlyMet - Don't include users recently in
/// rooms with this user in the result. Also, see the above comment.
///
/// Example custom C++ usage:
///
///   auto roomOptions = ovr_RoomOptions_Create();
///   ovr_RoomOptions_SetOrdering(roomOptions, ovrUserOrdering_PresenceAlphabetical);
///   ovr_RoomOptions_SetRoomId(roomOptions, roomID);
///   ovr_Room_GetInvitableUsers2(roomOptions);
///   ovr_RoomOptions_Destroy(roomOptions);
/// \param roomOptions Additional configuration for this request. Optional.
///
/// A message with type ::ovrMessage_Room_GetInvitableUsers2 will be generated in response.
///
/// First call ::ovr_Message_IsError() to check if an error occurred.
///
/// If no error occurred, the message will contain a payload of type ::ovrUserArrayHandle.
/// Extract the payload from the message handle with ::ovr_Message_GetUserArray().
OVRP_PUBLIC_FUNCTION(ovrRequest) ovr_Room_GetInvitableUsers2(ovrRoomOptionsHandle roomOptions);

/// Fetches the list of moderated rooms created for the application.
///
/// A message with type ::ovrMessage_Room_GetModeratedRooms will be generated in response.
///
/// First call ::ovr_Message_IsError() to check if an error occurred.
///
/// If no error occurred, the message will contain a payload of type ::ovrRoomArrayHandle.
/// Extract the payload from the message handle with ::ovr_Message_GetRoomArray().
OVRP_PUBLIC_FUNCTION(ovrRequest) ovr_Room_GetModeratedRooms();

/// Get the next page of entries
///
/// A message with type ::ovrMessage_Room_GetNextRoomArrayPage will be generated in response.
///
/// First call ::ovr_Message_IsError() to check if an error occurred.
///
/// If no error occurred, the message will contain a payload of type ::ovrRoomArrayHandle.
/// Extract the payload from the message handle with ::ovr_Message_GetRoomArray().
OVRP_PUBLIC_FUNCTION(ovrRequest) ovr_Room_GetNextRoomArrayPage(ovrRoomArrayHandle handle);

/// Invites a user to the specified room. They will receive a notification via
/// ovrNotification_Room_InviteReceived if they are in your game, and/or they
/// can poll for room invites using ovr_Notification_GetRoomInvites().
/// \param roomID The ID of your current room.
/// \param inviteToken A user's invite token, returned by ovr_Room_GetInvitableUsers().
///
/// <b>Error codes</b>
/// - \b 100: The invite token has expired, the user will need to be reinvited to the room.
/// - \b 100: The target user cannot join you in your current experience
/// - \b 100: You cannot send an invite to a room you are not in
///
/// A message with type ::ovrMessage_Room_InviteUser will be generated in response.
///
/// First call ::ovr_Message_IsError() to check if an error occurred.
///
/// If no error occurred, the message will contain a payload of type ::ovrRoomHandle.
/// Extract the payload from the message handle with ::ovr_Message_GetRoom().
OVRP_PUBLIC_FUNCTION(ovrRequest) ovr_Room_InviteUser(ovrID roomID, const char *inviteToken);

/// Joins the target room (leaving the one you're currently in).
/// \param roomID The room to join.
/// \param subscribeToUpdates If true, sends a message with type ovrNotification_Room_RoomUpdate when room data changes, such as when users join or leave.
///
/// <b>Error codes</b>
/// - \b 10: The room you're attempting to join is currently locked. Please try again later.
/// - \b 10: You don't have permission to enter this room. You may need to be invited first.
/// - \b 100: Invalid room_id: {room_id}. Either the ID is not a valid room or the user does not have permission to see or act on the room.
/// - \b 100: The room you're attempting to join is full. Please try again later.
/// - \b 100: This game isn't available. If it already started or was canceled, you can host a new game at any point.
///
/// A message with type ::ovrMessage_Room_Join will be generated in response.
///
/// First call ::ovr_Message_IsError() to check if an error occurred.
///
/// If no error occurred, the message will contain a payload of type ::ovrRoomHandle.
/// Extract the payload from the message handle with ::ovr_Message_GetRoom().
OVRP_PUBLIC_FUNCTION(ovrRequest) ovr_Room_Join(ovrID roomID, bool subscribeToUpdates);

/// Joins the target room (leaving the one you're currently in).
/// \param roomID The room to join.
/// \param roomOptions Additional room configuration for this request. Optional.
///
/// <b>Error codes</b>
/// - \b 10: The room you're attempting to join is currently locked. Please try again later.
/// - \b 10: You don't have permission to enter this room. You may need to be invited first.
/// - \b 100: Invalid room_id: {room_id}. Either the ID is not a valid room or the user does not have permission to see or act on the room.
/// - \b 100: The room you're attempting to join is full. Please try again later.
/// - \b 100: This game isn't available. If it already started or was canceled, you can host a new game at any point.
///
/// A message with type ::ovrMessage_Room_Join2 will be generated in response.
///
/// First call ::ovr_Message_IsError() to check if an error occurred.
///
/// If no error occurred, the message will contain a payload of type ::ovrRoomHandle.
/// Extract the payload from the message handle with ::ovr_Message_GetRoom().
OVRP_PUBLIC_FUNCTION(ovrRequest) ovr_Room_Join2(ovrID roomID, ovrRoomOptionsHandle roomOptions);

/// Allows the room owner to kick a user out of the current room.
/// \param roomID The room that you currently own (check ovr_Room_GetOwner()).
/// \param userID The user to be kicked (cannot be yourself).
/// \param kickDurationSeconds Length of the ban, in seconds.
///
/// <b>Error codes</b>
/// - \b 10: Room {room_id}: The user does not have permission to {cannot_action} because the user is not in the room (or any room). Perhaps they already left, or they stopped heartbeating. If this is a test environment, make sure you are not using the deprecated initialization methods ovr_PlatformInitializeStandaloneAccessToken (C++)/StandalonePlatform.Initialize(accessToken) (C#).
/// - \b 10: Room {room_id}: The user does not have permission to {cannot_action} because the user is not the owner of the room.
/// - \b 100: You cannot remove yourself from room {room_id}
///
/// A message with type ::ovrMessage_Room_KickUser will be generated in response.
///
/// First call ::ovr_Message_IsError() to check if an error occurred.
///
/// If no error occurred, the message will contain a payload of type ::ovrRoomHandle.
/// Extract the payload from the message handle with ::ovr_Message_GetRoom().
OVRP_PUBLIC_FUNCTION(ovrRequest) ovr_Room_KickUser(ovrID roomID, ovrID userID, int kickDurationSeconds);

/// Launch the invitable user flow to invite to the logged in user's current
/// room. This is intended to be a nice shortcut for developers not wanting to
/// build out their own Invite UI although it has the same rules as if you
/// build it yourself.
///
/// A message with type ::ovrMessage_Room_LaunchInvitableUserFlow will be generated in response.
///
/// First call ::ovr_Message_IsError() to check if an error occurred.
///
/// This response has no payload. If no error occured, the request was successful. Yay!
OVRP_PUBLIC_FUNCTION(ovrRequest) ovr_Room_LaunchInvitableUserFlow(ovrID roomID);

/// Removes you from your current room. Returns the solo room you are now in if
/// it succeeds
/// \param roomID The room you're currently in.
///
/// A message with type ::ovrMessage_Room_Leave will be generated in response.
///
/// First call ::ovr_Message_IsError() to check if an error occurred.
///
/// If no error occurred, the message will contain a payload of type ::ovrRoomHandle.
/// Extract the payload from the message handle with ::ovr_Message_GetRoom().
OVRP_PUBLIC_FUNCTION(ovrRequest) ovr_Room_Leave(ovrID roomID);

/// Allows the room owner to set the description of their room.
/// \param roomID The room that you currently own (check ovr_Room_GetOwner()).
/// \param description The new name of the room.
///
/// <b>Error codes</b>
/// - \b 10: Room {room_id}: The user does not have permission to {cannot_action} because the user is currently in another room (perhaps on another device), and thus is no longer in this room. Users can only be in one room at a time. If they are active on two different devices at once, there will be undefined behavior.
/// - \b 10: Room {room_id}: The user does not have permission to {cannot_action} because the user is not in the room (or any room). Perhaps they already left, or they stopped heartbeating. If this is a test environment, make sure you are not using the deprecated initialization methods ovr_PlatformInitializeStandaloneAccessToken (C++)/StandalonePlatform.Initialize(accessToken) (C#).
/// - \b 10: Room {room_id}: The user does not have permission to {cannot_action} because the user is not the owner of the room.
///
/// A message with type ::ovrMessage_Room_SetDescription will be generated in response.
///
/// First call ::ovr_Message_IsError() to check if an error occurred.
///
/// If no error occurred, the message will contain a payload of type ::ovrRoomHandle.
/// Extract the payload from the message handle with ::ovr_Message_GetRoom().
OVRP_PUBLIC_FUNCTION(ovrRequest) ovr_Room_SetDescription(ovrID roomID, const char *description);

/// Allows the room owner to update the data store (set of key value pairs) of
/// their room.
///
/// NOTE: Room datastores only allow string values. If you provide int or
/// double values, this returns an error.
/// \param roomID The room that you currently own (check ovr_Room_GetOwner()).
/// \param data The key/value pairs to add or update; null values clear a given key.
/// \param numItems The length of data
///
/// <b>Error codes</b>
/// - \b 10: Room {room_id}: The user does not have permission to {cannot_action} because the user is currently in another room (perhaps on another device), and thus is no longer in this room. Users can only be in one room at a time. If they are active on two different devices at once, there will be undefined behavior.
/// - \b 10: Room {room_id}: The user does not have permission to {cannot_action} because the user is not in the room (or any room). Perhaps they already left, or they stopped heartbeating. If this is a test environment, make sure you are not using the deprecated initialization methods ovr_PlatformInitializeStandaloneAccessToken (C++)/StandalonePlatform.Initialize(accessToken) (C#).
/// - \b 10: Room {room_id}: The user does not have permission to {cannot_action} because the user is not the owner of the room.
///
/// A message with type ::ovrMessage_Room_UpdateDataStore will be generated in response.
///
/// First call ::ovr_Message_IsError() to check if an error occurred.
///
/// If no error occurred, the message will contain a payload of type ::ovrRoomHandle.
/// Extract the payload from the message handle with ::ovr_Message_GetRoom().
OVRP_PUBLIC_FUNCTION(ovrRequest) ovr_Room_UpdateDataStore(ovrID roomID, ovrKeyValuePair *data, unsigned int numItems);

/// Disallow new members from being able to join the room. This will prevent
/// joins from ovr_Room_Join(), invites, 'Join From Home', etc. Users that are
/// in the room at the time of lockdown WILL be able to rejoin.
/// \param roomID The room whose membership you want to lock or unlock.
/// \param membershipLockStatus The new LockStatus for the room
///
/// <b>Error codes</b>
/// - \b 10: Room {room_id}: The user does not have permission to {cannot_action} because the user is currently in another room (perhaps on another device), and thus is no longer in this room. Users can only be in one room at a time. If they are active on two different devices at once, there will be undefined behavior.
/// - \b 10: Room {room_id}: The user does not have permission to {cannot_action} because the user is not in the room (or any room). Perhaps they already left, or they stopped heartbeating. If this is a test environment, make sure you are not using the deprecated initialization methods ovr_PlatformInitializeStandaloneAccessToken (C++)/StandalonePlatform.Initialize(accessToken) (C#).
/// - \b 10: Room {room_id}: The user does not have permission to {cannot_action} because the user is not the owner of the room.
///
/// A message with type ::ovrMessage_Room_UpdateMembershipLockStatus will be generated in response.
///
/// First call ::ovr_Message_IsError() to check if an error occurred.
///
/// If no error occurred, the message will contain a payload of type ::ovrRoomHandle.
/// Extract the payload from the message handle with ::ovr_Message_GetRoom().
OVRP_PUBLIC_FUNCTION(ovrRequest) ovr_Room_UpdateMembershipLockStatus(ovrID roomID, ovrRoomMembershipLockStatus membershipLockStatus);

/// Allows the room owner to transfer ownership to someone else.
/// \param roomID The room that the user owns (check ovr_Room_GetOwner()).
/// \param userID The new user to make an owner; the user must be in the room.
///
/// A message with type ::ovrMessage_Room_UpdateOwner will be generated in response.
///
/// First call ::ovr_Message_IsError() to check if an error occurred.
///
/// This response has no payload. If no error occured, the request was successful. Yay!
OVRP_PUBLIC_FUNCTION(ovrRequest) ovr_Room_UpdateOwner(ovrID roomID, ovrID userID);

/// Sets the join policy of the user's private room.
/// \param roomID The room ID that the user owns (check ovr_Room_GetOwner()).
/// \param newJoinPolicy The new join policy for the room.
///
/// A message with type ::ovrMessage_Room_UpdatePrivateRoomJoinPolicy will be generated in response.
///
/// First call ::ovr_Message_IsError() to check if an error occurred.
///
/// If no error occurred, the message will contain a payload of type ::ovrRoomHandle.
/// Extract the payload from the message handle with ::ovr_Message_GetRoom().
OVRP_PUBLIC_FUNCTION(ovrRequest) ovr_Room_UpdatePrivateRoomJoinPolicy(ovrID roomID, ovrRoomJoinPolicy newJoinPolicy);

#endif
