// This file was @generated with LibOVRPlatform/codegen/main. Do not modify it!

#ifndef OVR_REQUESTS_MATCHMAKING_H
#define OVR_REQUESTS_MATCHMAKING_H

#include "OVR_Types.h"
#include "OVR_Platform_Defs.h"

#include "OVR_MatchmakingOptions.h"
#include "OVR_MatchmakingStatApproach.h"
#include <stdbool.h>

/// \file
/// # Modes
///
/// There are three modes for matchmaking, each of which behaves very
/// differently.  However, the modes do share some of the same functions.
///
/// ## Quickmatch Mode
///
/// All users are in one pool, rooms are created for them once the match has been made.  This is good for games with 2-4 players
/// and rooms that don't persist after people leave the room.  If you want users to be able to create rooms or join games
/// in-progress, see Advanced Quickmatch.
///
/// ### Main flow
///
/// 0. At https://dashboard.oculus.com/application/[YOUR_APP_ID]/matchmaking, configure "QUICKMATCH" mode.
/// 1. Call ovr_Matchmaking_Enqueue2.  You'll be continually re-enqueued until you join a room or cancel.
/// 1a. You may handle ovrMessage_Matchmaking_Enqueue2 and use several methods, such as ovr_MatchmakingEnqueueResult_GetMatchesInLastHourCount,
///     to show your users information about the health of the queue.
/// (Success path)
/// 2. Success case: Handle ovrMessage_Notification_Matchmaking_MatchFound notification.
/// 3. In your notification handler, call ovr_Room_Join2 to place the user into the room.
/// 4. (for skill matching only) When the match begins, call ovr_Matchmaking_StartMatch. All people in the match should call this.
/// 5. (for skill matching only) When the match ends, call ovr_Matchmaking_ReportResultInsecure. All people in the match should call this.
/// (Failure path)
/// 2. Give up by calling ovr_Matchmaking_Cancel2.  If your app quits and you failed to cancel, the user
///    will be evicted from the queue after approximately 30 seconds.
///
/// ## Quickmatch Mode - Advanced Features.
///
/// If you configure Advanced Quickmatch in the developer dashboard, the system will keep track of two matchmaking queues for you,
/// one for rooms and one for users looking for rooms.  Use this mode when rooms outlive any particular group of players, or when
/// for some reason you want users to be able to create rooms themselves.
///
/// ### Main flow
///
/// For users joining a room:
/// 0. At https://dashboard.oculus.com/application/[YOUR_APP_ID]/matchmaking, configure "QUICKMATCH" mode and then check
///    "Is Advanced Quickmatch" and look over the options.
/// 1... See Quickmatch Mode - Main flow above.
///
/// Depending on how you configure it, users will be grouped together into system-created rooms and/or will join existing rooms.
///
/// For creating and hosting a room:
///
/// 0. At https://dashboard.oculus.com/application/[YOUR_APP_ID]/matchmaking, configure "Can People create and enqueue rooms themselves?"
///    to be true.
/// 1. EITHER ovr_Matchmaking_CreateAndEnqueueRoom2
///    OR ovr_Matchmaking_CreateRoom2 (handle response) and then ovr_Matchmaking_EnqueueRoom2
///    Your room will be continually re-enqueued until you cancel or a match is found.
/// 1a. You may handle ovrMessage_Matchmaking_CreateAndEnqueueRoom2 or ovrMessage_Matchmaking_EnqueueRoom2 and use several methods,
///     such as ovr_MatchmakingEnqueueResult_GetMatchesInLastHourCount, to show your users information about the health of the queue.
/// (Success)
/// 2. Observe room joins and leaves if you specified true for subscribeToUpdates
/// 3. (for skill matching only) When the match begins, call ovr_Matchmaking_StartMatch.  All people in the match should call this.
/// 4. (for skill matching only) When the match begins, call ovr_Matchmaking_ReportResultInsecure.  All people in the match should call this.
/// 5. Remove your room from the queue by calling ovr_Matchmaking_Cancel2.
/// (Failure)
/// 2. Give up by calling ovr_Matchmaking_Cancel2.
///
/// ## BROWSE Mode
///
/// In browse mode, users either host rooms or choose from among a list of rooms.  Note that users looking are enqueued just as
/// Filtering on the server side (such as for ping time) happens just as in the other modes, but we return all eventually-possible
/// matches rather than slowly lowering our quality threshold over time.  We enqueue and track those users so that you can show people
/// hosting rooms how many people have been looking for rooms, as well as better-understand the health of your queue.
/// This mode has some disadvantages relative to Quickmatch mode: there are more race conditions, for example when multiple users try
/// simultaneously to join the same room, or when a user attempts to join a room that has canceled.  However, this mode is
/// provided for when users should be given a choice of which room to join.
///
/// ### Main flow
///
/// For parties joining a room:
///
/// 0. At dashboard.oculus.com, configure "BROWSE" mode.
/// 1. Call ovr_Matchmaking_Browse2 any time you want a refreshed list of what's on the server.  Note that just like in Quickmatch mode,
///    this also enqueues the user!
/// 1a. You may wish to periodically poll this endpoint to refresh the list.
/// 2. Handle ovrMessage_Matchmaking_Browse2; show a list of rooms from returned list.
/// (Success)
/// 3. Call ovr_Room_Join2 when the user has chosen a room.
/// 4. (for skill matching only) ovr_Matchmaking_StartMatch
/// 5. (for skill matching only) ovr_Matchmaking_ReportResultInsecure
/// (Failure)
/// 3. If the user gives up, call ovr_Matchmaking_Cancel2.
///
/// For hosting a room:
/// 0. At dashboard.oculus.com, configure "BROWSE" mode.
/// 1... As in Quickmatch Mode - Advanced Features - Main Flow
///
/// # Pools
///
/// 'pool' - this parameter for all APIs refers to a configuration for a game mode you set up in advance
/// at https://dashboard.oculus.com/application/[YOUR_APP_ID]/matchmaking
/// You can have many pools for a given application, and each will have a completely separate queue.
///
/// ## Matchmaking Criteria
///
/// There are several types of criteria you can use to configure match
/// preferences. You can configure these on a per-pool basis in the Developer
/// Dashboard.
///
/// * Network Speed: prefer matches with shorter ping time
/// * Skill Pools: prefer matches with similar skill level
/// * Matchmaking Queries: prefer matches based on conditional statements that you define
///
/// ## How do we determine who gets matched?
///
/// Each criteria yields a number between 0 and 1. When determining a match between
/// two enqueued users, all the criteria values are multiplied together to get a
/// match value between 0 and 1. 0.5 is considered to be a marginal match, and 0.9
/// an excellent match. A successful match occurs if the match value is greater than
/// or equal to the match threshold, where the match threshold is 1.0 at enqueue
/// time, and drops to 0.5 over a rampdown period of 30 seconds. A high match value
/// (say, 0.9 or above) would result in a match fairly quickly, whereas a lower
/// match (say, 0.6) would end up waiting longer before a match occurs. A match
/// value less than 0.5 would never result in a successful match.
///
/// Note that match calculations are asymmetric, meaning that if we are determining
/// whether users A and B can match each other, the match calculation must succeed
/// in the both the A->B direction and the B->A direction in order for them to be
/// matched.
///
/// # Matchmaking Queries
///
/// Matchmaking Queries allow you to define your own Query Expressions for determining
/// whether enqueued players and rooms can be successfully matched with each
/// other. These expressions compare potential matches' Data Settings against the
/// current user's Data Settings. You define and configure Matchmaking Queries in
/// the Developer Dashboard. At enqueue time, you specify who you are willing to be
/// matched with by providing Data Settings and a Matchmaking Query.
///
/// ## Query Expressions
///
/// A Matchmaking Query is composed of one or more expressions, where each
/// expression defines a conditional statement. The matchmaking service populates
/// each expression with the Data Settings of the enqueuer and the potential match
/// candidate, and then evaluates it to a value between 0 and 1.
///
/// You will configure an importance with each expression. When an expression
/// passes, it evaluates to a value of 1, and otherwise (failure case) evaluates to
/// the value of the associated importance. Note that the match-on-failure delay
/// times below are calculated based on a rampdown time of 30 seconds.
///
/// * Required: 0, i.e. never matches on failure
/// * High: ~0.55, i.e. matches on failure after 27 seconds
/// * Medium: ~0.75, i.e. matches on failure after 15 seconds
/// * Low: ~0.9, i.e. matches on failure after 6 seconds
///
/// See the Matchmaking Criteria section above for more details on how these values
/// are used. In general, the greater the importance, the less likely a match
/// will occur if the expression fails. And in the case of expressions with Required
/// importance, a failure will *never* result in a match.
///
/// ## Data Settings
///
/// Data Settings are the data (key/value pairs) that the enqueuing player or room
/// provides about itself at enqueue time. Data Settings can be used both to determine what
/// a player is looking for, as well as what a player looks like to another
/// player. For example, if a player is enqueued with the following:
///
///   data settings: "map" = "capture_the_flag"
///   query: their map = my map
///
/// Then the matchmaking service will apply this query to potential match candidates:
///   their map = "capture_the_flag"
///
/// And therefore find a match with other enqueued players who have also provided
/// "map"="capture_the_flag" in their Data Settings.
///
/// ## Examples
///
/// ### Example 1
///
/// Two users have a peer-to-peer network round trip time of 50ms.  You configured your application to say that 100ms is a good time.
/// So let's say that 50ms yields a really good value of 0.95.
/// User A has a medium-importance Query Expression that user B wields a banana katana, but B does not have one,
/// yielding a value of 0.75.
/// The total match quality from A's perspective is 0.95 * 0.75 = .71 .  Thus, A and B will be matched, but not before waiting a bit to
/// see if better matches come along.
///
/// ### Example 2
///
/// User A has a high-importance Query Expression that user B speaks English, but user B doesn't, yielding
/// a value of 0.55 from A's perspective.
/// User A and B are a decent skill match, but not superb, yielding a value of 0.8.
/// Users A and B both require that each other are members of the Awesome Guild, and they both are, yielding a value of 1.0.
/// The total is 1.0 * 0.8 * 0.55 = 0.44.  This is less than 0.5, so these users are never going to be matched with each other under the given criteria.
///
/// ### Example (C++)
///
///   // Assume that in https://dashboard.oculus.com/application/[YOUR_APP_ID]/matchmaking,
///   // I have a pool called "my_pool", with the following Data Settings configured:
///   // - player_level (INTEGER)
///   // - game_mode (STRING)
///   // - map_size (INTEGER_BITSET)
///   //
///   // I then create a query called "my_query" in "my_pool" with the following query expressions:
///   // - Their "player_level" is equal to my "player_level".  Importance: Medium
///   // - Their "game_mode" is equal to my "game_mode".  Importance: Required
///   // - Their "map_size" is a bitmask AND of my "map_size".  Importance: Required
///   //   - In my game, this bitmask has the following bit meanings:
///   //   - 0x4: large map size
///   //   - 0x2: medium map size
///   //   - 0x1: small map size
///
///   ovrMatchmakingOptionsHandle matchmakingOptions = ovr_MatchmakingOptions_Create();
///
///   ovr_MatchmakingOptions_SetEnqueueDataSettingsInt(matchmakingOptions, "player_level", 10);
///   ovr_MatchmakingOptions_SetEnqueueDataSettingsString(matchmakingOptions, "game_mode", "CaptureTheFlag");
///   // I want large or medium map size
///   ovr_MatchmakingOptions_SetEnqueueDataSettingsInt(matchmakingOptions, "map_size", 0x4 & 0x2);
///
///   // Specify which Matchmaking Query to use with the Data Settings we provided
///   ovr_MatchmakingOptions_SetEnqueueQueryKey(matchmakingOptions, "my_query");
///
///   ovr_Matchmaking_Enqueue2("my_pool", matchmakingOptions);
///
///   // Destroy the matchmaking options now that we are done with it
///   ovr_MatchmakingOptions_Destroy(matchmakingOptions);
///
///   // The matchmaking service will now look for other players who enqueued with
///   // Data Settings where player_level=10 AND game_mode="CaptureTheFlag" and map_size is large or medium or both
///
/// # Debugging
///
/// To debug what's going on with your matchmaking pool, you can get snapshots of the queues using
/// ovr_Matchmaking_GetAdminSnapshot.  This endpoint is not intended to be called in
/// production.  Below examples will illustrate how to use it, and then there's a reference below
///
/// ## Queue snapshot (instant)
/// Find out what scores the logged-in user will assign to other users in the queue, and vice-versa.
/// This will get the state of the queue the moment the user enqueued
///
///     ovrMatchmakingOptionsHandle matchmakingOptions = ovr_MatchmakingOptions_Create();
///     ovr_MatchmakingOptions_SetIsDebug(matchmakingOptions, true);
///     // set other matchmaking options here ...
///     ovr_Matchmaking_Enqueue2("my_pool", matchmakingOptions);
///     ovr_MatchmakingOptions_Destroy(matchmakingOptions);
///
///     // In your handler
///     case ovrMessage_Matchmaking_Enqueue2:
///       if (!ovr_Message_IsError(message)) {
///         auto enqueueResults = ovr_Message_GetMatchmakingEnqueueResult(message);
///         auto snapshot = ovr_MatchmakingEnqueueResult_GetAdminSnapshot(enqueueResults);
///         auto candidates = ovr_MatchmakingAdminSnapshot_GetCandidates(snapshot);
///         auto firstCandidate = ovr_MatchmakingAdminSnapshotCandidateArray_GetElement(candidates, 0);
///         if (ovr_MatchmakingAdminSnapshotCandidate_GetCanMatch(firstCandidate)) {
///           cout << "Yay!" << endl;
///         }
///       }
///
/// ## Queue snapshot (delayed)
/// Find out what scores the logged-in user will assign to other users in the queue, and vice-versa.
/// This will get the state of the queue at whatever time the ovr_Matchmaking_GetAdminSnapshot() is called
///
///     // In your code, do a matchmaking enqueue first
///
///     // We can now inspect the queue to debug it.
///     ovr_Matchmaking_GetAdminSnapshot();
///
///     // In your handler
///     case ovrMessage_Matchmaking_GetAdminSnapshot:
///       if (!ovr_Message_IsError(message)) {
///         auto snapshot = ovr_Message_GetMatchmakingAdminSnapshot(message);
///         auto candidates = ovr_MatchmakingAdminSnapshot_GetCandidates(snapshot);
///         auto firstCandidate = ovr_MatchmakingAdminSnapshotCandidateArray_GetElement(candidates, 0);
///         if (ovr_MatchmakingAdminSnapshotCandidate_GetCanMatch(firstCandidate)) {
///           cout << "Yay!" << endl;
///         }
///       }
///
/// ## Reference
/// The following fields are currently exported:
///
/// ovr_MatchmakingAdminSnapshot_GetCandidates - A list of all the entries in the queue that the logged-in user is
///     choosing among, along with metadata about them
/// ovr_MatchmakingAdminSnapshot_GetMyCurrentThreshold - The minimum score (between 0 and 1) that is required for
///     the logged-in user to want to be matched with someone.  This number may vary based on factors like how long
///     the user has been enqueued.
///
/// # Matchmaking Options
///
/// Several matchmaking requests accept an optional ovrMatchmakingOptionsHandle,
/// which contains a bag of settings for configuring the request. There is a suite
/// of setter functions that you can use to set the options:
///
/// ## Create/Destroy
///
/// ovr_MatchmakingOptions_Create - Create an instance of ovrMatchmakingOptionsHandle. Call ovr_MatchmakingOptions_Destroy to free the memory.
/// ovr_MatchmakingOptions_Destroy - Call this to destroy an instance of ovrMatchmakingOptionsHandle.
///
/// ## CreateRoom-specific Options
///
/// These options are relevant to CreateRoom2 and CreateAndEnqueueRoom2.
///
/// ovr_MatchmakingOptions_SetCreateRoomMaxUsers - Overrides the value of "Max Users" in pool settings of the Developer Dashboard.
/// ovr_MatchmakingOptions_SetCreateRoomJoinPolicy - Specifies a join policy for the created room. If unset, the join policy defaults to EVERYONE.
///
/// ## Enqueue-specific Options
///
/// These options are relevant to Enqueue2, EnqueueRoom2, and Browse2.
///
/// ovr_MatchmakingOptions_AddEnqueueAdditionalUser - This option is currently in beta. Do not use.
/// ovr_MatchmakingOptions_SetEnqueueDataSettingsInt - Set an integer Data Setting. See "Data Settings" section above.
/// ovr_MatchmakingOptions_SetEnqueueDataSettingsDouble - Set a double Data Setting. See "Data Settings" section above.
/// ovr_MatchmakingOptions_SetEnqueueDataSettingsString - Set a string Data setting. See "Data Settings" section above.
/// ovr_MatchmakingOptions_SetEnqueueIsDebug - If true, debug information is returned with the response payload. See "Debugging" section above.
/// ovr_MatchmakingOptions_SetEnqueueQueryKey - Specify a Matchmaking Query for filtering potential matches. See the "Matchmaking Queries" section above.

/// DEPRECATED. Use Browse2.
/// \param pool A BROWSE type matchmaking pool.
/// \param customQueryData Optional. Custom query data.
///
/// <b>Error codes</b>
/// - \b 100: Pool {pool_key} does not contain custom data key {key}. You can configure matchmaking custom data at https://dashboard.oculus.com/application/&lt;app_id&gt;/matchmaking
/// - \b 12072: Unknown pool: {pool_key}. You can configure matchmaking pools at https://dashboard.oculus.com/application/&lt;app_id&gt;/matchmaking
///
/// A message with type ::ovrMessage_Matchmaking_Browse will be generated in response.
///
/// First call ::ovr_Message_IsError() to check if an error occurred.
///
/// If no error occurred, the message will contain a payload of type ::ovrMatchmakingBrowseResultHandle.
/// Extract the payload from the message handle with ::ovr_Message_GetMatchmakingBrowseResult().
OVRP_PUBLIC_FUNCTION(ovrRequest) ovr_Matchmaking_Browse(const char *pool, ovrMatchmakingCustomQueryData *customQueryData);

/// Modes: BROWSE
///
/// See overview documentation above.
///
/// Return a list of matchmaking rooms in the current pool filtered by skill
/// and ping (if enabled). This also enqueues the user in the matchmaking
/// queue. When the user has made a selection, call ovr_Room_Join2() on one of
/// the rooms that was returned. If the user stops browsing, call
/// ovr_Matchmaking_Cancel2().
///
/// In addition to the list of rooms, enqueue results are also returned. Call
/// ovr_MatchmakingBrowseResult_GetEnqueueResult() to obtain them. See
/// OVR_MatchmakingEnqueueResult.h for details.
/// \param pool A BROWSE type matchmaking pool.
/// \param matchmakingOptions Additional matchmaking configuration for this request. Optional.
///
/// <b>Error codes</b>
/// - \b 100: Pool {pool_key} does not contain custom data key {key}. You can configure matchmaking custom data at https://dashboard.oculus.com/application/&lt;app_id&gt;/matchmaking
/// - \b 12072: Unknown pool: {pool_key}. You can configure matchmaking pools at https://dashboard.oculus.com/application/&lt;app_id&gt;/matchmaking
///
/// A message with type ::ovrMessage_Matchmaking_Browse2 will be generated in response.
///
/// First call ::ovr_Message_IsError() to check if an error occurred.
///
/// If no error occurred, the message will contain a payload of type ::ovrMatchmakingBrowseResultHandle.
/// Extract the payload from the message handle with ::ovr_Message_GetMatchmakingBrowseResult().
OVRP_PUBLIC_FUNCTION(ovrRequest) ovr_Matchmaking_Browse2(const char *pool, ovrMatchmakingOptionsHandle matchmakingOptions);

/// DEPRECATED. Use Cancel2.
/// \param pool The pool in question.
/// \param requestHash Used to find your entry in a queue.
///
/// <b>Error codes</b>
/// - \b 10: Room {room_id}: The user does not have permission to {cannot_action} because the user is currently in another room (perhaps on another device), and thus is no longer in this room. Users can only be in one room at a time. If they are active on two different devices at once, there will be undefined behavior.
/// - \b 10: Room {room_id}: The user does not have permission to {cannot_action} because the user is not in the room (or any room). Perhaps they already left, or they stopped heartbeating. If this is a test environment, make sure you are not using the deprecated initialization methods ovr_PlatformInitializeStandaloneAccessToken (C++)/StandalonePlatform.Initialize(accessToken) (C#).
/// - \b 10: Room {room_id}: The user does not have permission to {cannot_action} because the user is not the owner of the room.
/// - \b 100: Invalid room_id: {room_id}. Either the ID is not a valid room or the user does not have permission to see or act on the room.
///
/// A message with type ::ovrMessage_Matchmaking_Cancel will be generated in response.
///
/// First call ::ovr_Message_IsError() to check if an error occurred.
///
/// This response has no payload. If no error occured, the request was successful. Yay!
OVRP_PUBLIC_FUNCTION(ovrRequest) ovr_Matchmaking_Cancel(const char *pool, const char *requestHash);

/// Modes: QUICKMATCH, BROWSE
///
/// Makes a best effort to cancel a previous Enqueue request before a match
/// occurs. Typically triggered when a user gives up waiting. For BROWSE mode,
/// call this when a user gives up looking through the room list or when the
/// host of a room wants to stop receiving new users. If you don't cancel but
/// the user goes offline, the user/room will be timed out of the queue within
/// 30 seconds.
///
/// <b>Error codes</b>
/// - \b 10: Room {room_id}: The user does not have permission to {cannot_action} because the user is currently in another room (perhaps on another device), and thus is no longer in this room. Users can only be in one room at a time. If they are active on two different devices at once, there will be undefined behavior.
/// - \b 10: Room {room_id}: The user does not have permission to {cannot_action} because the user is not in the room (or any room). Perhaps they already left, or they stopped heartbeating. If this is a test environment, make sure you are not using the deprecated initialization methods ovr_PlatformInitializeStandaloneAccessToken (C++)/StandalonePlatform.Initialize(accessToken) (C#).
/// - \b 10: Room {room_id}: The user does not have permission to {cannot_action} because the user is not the owner of the room.
/// - \b 100: Invalid room_id: {room_id}. Either the ID is not a valid room or the user does not have permission to see or act on the room.
///
/// A message with type ::ovrMessage_Matchmaking_Cancel2 will be generated in response.
///
/// First call ::ovr_Message_IsError() to check if an error occurred.
///
/// This response has no payload. If no error occured, the request was successful. Yay!
OVRP_PUBLIC_FUNCTION(ovrRequest) ovr_Matchmaking_Cancel2();

/// DEPRECATED. Use CreateAndEnqueueRoom2.
/// \param pool The matchmaking pool to use, which is defined for the app.
/// \param maxUsers Overrides the Max Users value, which is configured in pool settings of the Developer Dashboard.
/// \param subscribeToUpdates If true, sends a message with type ovrNotification_Room_RoomUpdate when the room data changes, such as when users join or leave.
/// \param customQueryData Optional.  See "Custom criteria" section above.
///
/// <b>Error codes</b>
/// - \b 100: Pool {pool_key} does not contain custom data key {key}. You can configure matchmaking custom data at https://dashboard.oculus.com/application/&lt;app_id&gt;/matchmaking
/// - \b 12051: Pool '{pool_key}' is configured for Quickmatch mode. In Quickmatch mode, rooms are created on users' behalf when a match is found. Specify Advanced Quickmatch or Browse mode to use this feature.
/// - \b 12072: Unknown pool: {pool_key}. You can configure matchmaking pools at https://dashboard.oculus.com/application/&lt;app_id&gt;/matchmaking
/// - \b 12089: You have asked to enqueue {num_users} users together, but this must be less than the maximum number of users in a room, {max_users}.
///
/// A message with type ::ovrMessage_Matchmaking_CreateAndEnqueueRoom will be generated in response.
///
/// First call ::ovr_Message_IsError() to check if an error occurred.
///
/// If no error occurred, the message will contain a payload of type ::ovrMatchmakingEnqueueResultAndRoomHandle.
/// Extract the payload from the message handle with ::ovr_Message_GetMatchmakingEnqueueResultAndRoom().
OVRP_PUBLIC_FUNCTION(ovrRequest) ovr_Matchmaking_CreateAndEnqueueRoom(const char *pool, unsigned int maxUsers, bool subscribeToUpdates, ovrMatchmakingCustomQueryData *customQueryData);

/// Modes: BROWSE, QUICKMATCH (Advanced; Can Users Create Rooms = true)
///
/// See overview documentation above.
///
/// Create a matchmaking room, join it, and enqueue it. This is the preferred
/// method. But, if you do not wish to automatically enqueue the room, you can
/// call CreateRoom2 instead.
///
/// Visit https://dashboard.oculus.com/application/[YOUR_APP_ID]/matchmaking to
/// set up pools and queries
/// \param pool The matchmaking pool to use, which is defined for the app.
/// \param matchmakingOptions Additional matchmaking configuration for this request. Optional.
///
/// <b>Error codes</b>
/// - \b 100: Pool {pool_key} does not contain custom data key {key}. You can configure matchmaking custom data at https://dashboard.oculus.com/application/&lt;app_id&gt;/matchmaking
/// - \b 12051: Pool '{pool_key}' is configured for Quickmatch mode. In Quickmatch mode, rooms are created on users' behalf when a match is found. Specify Advanced Quickmatch or Browse mode to use this feature.
/// - \b 12072: Unknown pool: {pool_key}. You can configure matchmaking pools at https://dashboard.oculus.com/application/&lt;app_id&gt;/matchmaking
/// - \b 12089: You have asked to enqueue {num_users} users together, but this must be less than the maximum number of users in a room, {max_users}.
///
/// A message with type ::ovrMessage_Matchmaking_CreateAndEnqueueRoom2 will be generated in response.
///
/// First call ::ovr_Message_IsError() to check if an error occurred.
///
/// If no error occurred, the message will contain a payload of type ::ovrMatchmakingEnqueueResultAndRoomHandle.
/// Extract the payload from the message handle with ::ovr_Message_GetMatchmakingEnqueueResultAndRoom().
OVRP_PUBLIC_FUNCTION(ovrRequest) ovr_Matchmaking_CreateAndEnqueueRoom2(const char *pool, ovrMatchmakingOptionsHandle matchmakingOptions);

/// DEPRECATED. Use CreateRoom2.
/// \param pool The matchmaking pool to use, which is defined for the app.
/// \param maxUsers Overrides the Max Users value, which is configured in pool settings of the Developer Dashboard.
/// \param subscribeToUpdates If true, sends a message with type ovrNotification_Room_RoomUpdate when room data changes, such as when users join or leave.
///
/// A message with type ::ovrMessage_Matchmaking_CreateRoom will be generated in response.
///
/// First call ::ovr_Message_IsError() to check if an error occurred.
///
/// If no error occurred, the message will contain a payload of type ::ovrRoomHandle.
/// Extract the payload from the message handle with ::ovr_Message_GetRoom().
OVRP_PUBLIC_FUNCTION(ovrRequest) ovr_Matchmaking_CreateRoom(const char *pool, unsigned int maxUsers, bool subscribeToUpdates);

/// Create a matchmaking room and join it, but do not enqueue the room. After
/// creation, you can call EnqueueRoom2. However, Oculus recommends using
/// CreateAndEnqueueRoom2 instead.
///
/// Modes: BROWSE, QUICKMATCH (Advanced; Can Users Create Rooms = true)
///
/// Create a matchmaking room and join it, but do not enqueue the room. After
/// creation, you can call EnqueueRoom. Consider using CreateAndEnqueueRoom
/// instead.
///
/// Visit https://dashboard.oculus.com/application/[YOUR_APP_ID]/matchmaking to
/// set up pools and queries
/// \param pool The matchmaking pool to use, which is defined for the app.
/// \param matchmakingOptions Additional matchmaking configuration for this request. Optional.
///
/// A message with type ::ovrMessage_Matchmaking_CreateRoom2 will be generated in response.
///
/// First call ::ovr_Message_IsError() to check if an error occurred.
///
/// If no error occurred, the message will contain a payload of type ::ovrRoomHandle.
/// Extract the payload from the message handle with ::ovr_Message_GetRoom().
OVRP_PUBLIC_FUNCTION(ovrRequest) ovr_Matchmaking_CreateRoom2(const char *pool, ovrMatchmakingOptionsHandle matchmakingOptions);

/// DEPRECATED. Use Enqueue2.
/// \param pool The pool to enqueue in.
/// \param customQueryData Optional.  See "Custom criteria" section above.
///
/// <b>Error codes</b>
/// - \b 100: Pool {pool_key} does not contain custom data key {key}. You can configure matchmaking custom data at https://dashboard.oculus.com/application/&lt;app_id&gt;/matchmaking
/// - \b 12072: Unknown pool: {pool_key}. You can configure matchmaking pools at https://dashboard.oculus.com/application/&lt;app_id&gt;/matchmaking
///
/// A message with type ::ovrMessage_Matchmaking_Enqueue will be generated in response.
///
/// First call ::ovr_Message_IsError() to check if an error occurred.
///
/// If no error occurred, the message will contain a payload of type ::ovrMatchmakingEnqueueResultHandle.
/// Extract the payload from the message handle with ::ovr_Message_GetMatchmakingEnqueueResult().
OVRP_PUBLIC_FUNCTION(ovrRequest) ovr_Matchmaking_Enqueue(const char *pool, ovrMatchmakingCustomQueryData *customQueryData);

/// Modes: QUICKMATCH
///
/// See overview documentation above.
///
/// Enqueue yourself to await an available matchmaking room. The platform
/// returns a ovrNotification_Matchmaking_MatchFound message when a match is
/// found. Call ovr_Room_Join2() on the returned room. The response contains
/// useful information to display to the user to set expectations for how long
/// it will take to get a match.
///
/// If the user stops waiting, call ovr_Matchmaking_Cancel2().
/// \param pool The pool to enqueue in.
/// \param matchmakingOptions Additional matchmaking configuration for this request. Optional.
///
/// <b>Error codes</b>
/// - \b 100: Pool {pool_key} does not contain custom data key {key}. You can configure matchmaking custom data at https://dashboard.oculus.com/application/&lt;app_id&gt;/matchmaking
/// - \b 12072: Unknown pool: {pool_key}. You can configure matchmaking pools at https://dashboard.oculus.com/application/&lt;app_id&gt;/matchmaking
///
/// A message with type ::ovrMessage_Matchmaking_Enqueue2 will be generated in response.
///
/// First call ::ovr_Message_IsError() to check if an error occurred.
///
/// If no error occurred, the message will contain a payload of type ::ovrMatchmakingEnqueueResultHandle.
/// Extract the payload from the message handle with ::ovr_Message_GetMatchmakingEnqueueResult().
OVRP_PUBLIC_FUNCTION(ovrRequest) ovr_Matchmaking_Enqueue2(const char *pool, ovrMatchmakingOptionsHandle matchmakingOptions);

/// DEPRECATED. Please use ovr_Matchmaking_EnqueueRoom2() instead.
/// \param roomID Returned either from ovrNotification_Matchmaking_MatchFound or from ovr_Matchmaking_CreateRoom().
/// \param customQueryData Optional.  See the "Custom criteria" section above.
///
/// <b>Error codes</b>
/// - \b 10: Room {room_id}: The user does not have permission to {cannot_action} because the user is currently in another room (perhaps on another device), and thus is no longer in this room. Users can only be in one room at a time. If they are active on two different devices at once, there will be undefined behavior.
/// - \b 10: Room {room_id}: The user does not have permission to {cannot_action} because the user is not in the room (or any room). Perhaps they already left, or they stopped heartbeating. If this is a test environment, make sure you are not using the deprecated initialization methods ovr_PlatformInitializeStandaloneAccessToken (C++)/StandalonePlatform.Initialize(accessToken) (C#).
/// - \b 10: Room {room_id}: The user does not have permission to {cannot_action} because the user is not the owner of the room.
/// - \b 100: Invalid room_id: {room_id}. Either the ID is not a valid room or the user does not have permission to see or act on the room.
/// - \b 12051: Pool '{pool_key}' is configured for Quickmatch mode. In Quickmatch mode, rooms are created on users' behalf when a match is found. Specify Advanced Quickmatch or Browse mode to use this feature.
///
/// A message with type ::ovrMessage_Matchmaking_EnqueueRoom will be generated in response.
///
/// First call ::ovr_Message_IsError() to check if an error occurred.
///
/// If no error occurred, the message will contain a payload of type ::ovrMatchmakingEnqueueResultHandle.
/// Extract the payload from the message handle with ::ovr_Message_GetMatchmakingEnqueueResult().
OVRP_PUBLIC_FUNCTION(ovrRequest) ovr_Matchmaking_EnqueueRoom(ovrID roomID, ovrMatchmakingCustomQueryData *customQueryData);

/// Modes: BROWSE (for Rooms only), ROOM
///
/// See the overview documentation above. Enqueue yourself to await an
/// available matchmaking room. ovrNotification_Matchmaking_MatchFound gets
/// enqueued when a match is found.
///
/// The response contains useful information to display to the user to set
/// expectations for how long it will take to get a match.
///
/// If the user stops waiting, call ovr_Matchmaking_Cancel2().
/// \param roomID Returned either from ovrNotification_Matchmaking_MatchFound or from ovr_Matchmaking_CreateRoom().
/// \param matchmakingOptions Additional matchmaking configuration for this request. Optional.
///
/// <b>Error codes</b>
/// - \b 10: Room {room_id}: The user does not have permission to {cannot_action} because the user is currently in another room (perhaps on another device), and thus is no longer in this room. Users can only be in one room at a time. If they are active on two different devices at once, there will be undefined behavior.
/// - \b 10: Room {room_id}: The user does not have permission to {cannot_action} because the user is not in the room (or any room). Perhaps they already left, or they stopped heartbeating. If this is a test environment, make sure you are not using the deprecated initialization methods ovr_PlatformInitializeStandaloneAccessToken (C++)/StandalonePlatform.Initialize(accessToken) (C#).
/// - \b 10: Room {room_id}: The user does not have permission to {cannot_action} because the user is not the owner of the room.
/// - \b 100: Invalid room_id: {room_id}. Either the ID is not a valid room or the user does not have permission to see or act on the room.
/// - \b 12051: Pool '{pool_key}' is configured for Quickmatch mode. In Quickmatch mode, rooms are created on users' behalf when a match is found. Specify Advanced Quickmatch or Browse mode to use this feature.
///
/// A message with type ::ovrMessage_Matchmaking_EnqueueRoom2 will be generated in response.
///
/// First call ::ovr_Message_IsError() to check if an error occurred.
///
/// If no error occurred, the message will contain a payload of type ::ovrMatchmakingEnqueueResultHandle.
/// Extract the payload from the message handle with ::ovr_Message_GetMatchmakingEnqueueResult().
OVRP_PUBLIC_FUNCTION(ovrRequest) ovr_Matchmaking_EnqueueRoom2(ovrID roomID, ovrMatchmakingOptionsHandle matchmakingOptions);

/// Modes: QUICKMATCH, BROWSE
///
/// Used to debug the state of the current matchmaking pool queue. This is not
/// intended to be used in production.
///
/// A message with type ::ovrMessage_Matchmaking_GetAdminSnapshot will be generated in response.
///
/// First call ::ovr_Message_IsError() to check if an error occurred.
///
/// If no error occurred, the message will contain a payload of type ::ovrMatchmakingAdminSnapshotHandle.
/// Extract the payload from the message handle with ::ovr_Message_GetMatchmakingAdminSnapshot().
OVRP_PUBLIC_FUNCTION(ovrRequest) ovr_Matchmaking_GetAdminSnapshot();

/// Modes: QUICKMATCH, BROWSE
///
/// Gets the matchmaking stats for the current user
///
/// Given a pool it will look up the current user's wins, loss, draws and skill
/// level. The skill level return will be between 1 and maxLevel. The approach
/// will dictate how should the skill level rise toward the max level.
/// \param pool The pool to look in
/// \param maxLevel The maximum skill level achievable
/// \param approach The growth function of how the skill levels should approach to the max level.  TRAILING is recommended for displaying to users
///
/// A message with type ::ovrMessage_Matchmaking_GetStats will be generated in response.
///
/// First call ::ovr_Message_IsError() to check if an error occurred.
///
/// If no error occurred, the message will contain a payload of type ::ovrMatchmakingStatsHandle.
/// Extract the payload from the message handle with ::ovr_Message_GetMatchmakingStats().
OVRP_PUBLIC_FUNCTION(ovrRequest) ovr_Matchmaking_GetStats(const char *pool, unsigned int maxLevel, ovrMatchmakingStatApproach approach);

/// DEPRECATED. Use ovr_Room_Join2.
/// \param roomID ID of a room previously returned from ovrNotification_Matchmaking_MatchFound or ovr_Matchmaking_Browse().
/// \param subscribeToUpdates If true, sends a message with type ovrNotification_Room_RoomUpdate when room data changes, such as when users join or leave.
///
/// A message with type ::ovrMessage_Matchmaking_JoinRoom will be generated in response.
///
/// First call ::ovr_Message_IsError() to check if an error occurred.
///
/// If no error occurred, the message will contain a payload of type ::ovrRoomHandle.
/// Extract the payload from the message handle with ::ovr_Message_GetRoom().
OVRP_PUBLIC_FUNCTION(ovrRequest) ovr_Matchmaking_JoinRoom(ovrID roomID, bool subscribeToUpdates);

/// Modes: QUICKMATCH, BROWSE (+ Skill Pool)
///
/// See the overview documentation above.
///
/// Call this after calling ovr_Matchmaking_StartMatch() to begin a rated skill
/// match and after the match finishes. The service will record the result and
/// update the skills of all players involved, based on the results. This
/// method is insecure because, as a client API, it is susceptible to tampering
/// and therefore cheating to manipulate skill ratings.
/// \param roomID The room ID
/// \param data key value pairs
/// \param numItems The length of data
///
/// <b>Error codes</b>
/// - \b 100: Parameter {parameter}: invalid user id: {user_id}
/// - \b 100: Room id: {room_id}. The match associated with this room does not contain enough users. You must start the match with at least two users in the room. Result given: {result}.
/// - \b 100: There is no active match associated with the room {room_id}.
///
/// A message with type ::ovrMessage_Matchmaking_ReportResultInsecure will be generated in response.
///
/// First call ::ovr_Message_IsError() to check if an error occurred.
///
/// This response has no payload. If no error occured, the request was successful. Yay!
OVRP_PUBLIC_FUNCTION(ovrRequest) ovr_Matchmaking_ReportResultInsecure(ovrID roomID, ovrKeyValuePair *data, unsigned int numItems);

/// Modes: QUICKMATCH, BROWSE (+ Skill Pool)
///
/// For pools with skill-based matching. See overview documentation above.
///
/// Call after calling ovr_Room_Join2() when the players are present to begin a
/// rated match for which you plan to report the results (using
/// ovr_Matchmaking_ReportResultInsecure()).
///
/// <b>Error codes</b>
/// - \b 100: There is no active match associated with the room {room_id}.
/// - \b 100: You can only start matches, report matches, and track skill ratings in matchmaking rooms. {room_id} is a room, but it is not a matchmaking room.
///
/// A message with type ::ovrMessage_Matchmaking_StartMatch will be generated in response.
///
/// First call ::ovr_Message_IsError() to check if an error occurred.
///
/// This response has no payload. If no error occured, the request was successful. Yay!
OVRP_PUBLIC_FUNCTION(ovrRequest) ovr_Matchmaking_StartMatch(ovrID roomID);

#endif
