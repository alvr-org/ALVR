// This file was @generated with LibOVRPlatform/codegen/main. Do not modify it!

#ifndef OVR_MESSAGE_H
#define OVR_MESSAGE_H

#include "OVR_Platform_Defs.h"
#include "OVR_AbuseReportRecording.h"
#include "OVR_AchievementDefinitionArray.h"
#include "OVR_AchievementProgressArray.h"
#include "OVR_AchievementUpdate.h"
#include "OVR_ApplicationVersion.h"
#include "OVR_AssetDetails.h"
#include "OVR_AssetDetailsArray.h"
#include "OVR_AssetFileDeleteResult.h"
#include "OVR_AssetFileDownloadCancelResult.h"
#include "OVR_AssetFileDownloadResult.h"
#include "OVR_AssetFileDownloadUpdate.h"
#include "OVR_CalApplicationFinalized.h"
#include "OVR_CalApplicationProposed.h"
#include "OVR_CalApplicationSuggestionArray.h"
#include "OVR_CloudStorageConflictMetadata.h"
#include "OVR_CloudStorageData.h"
#include "OVR_CloudStorageMetadata.h"
#include "OVR_CloudStorageMetadataArray.h"
#include "OVR_CloudStorageUpdateResponse.h"
#include "OVR_DestinationArray.h"
#include "OVR_Error.h"
#include "OVR_HttpTransferUpdate.h"
#include "OVR_InstalledApplicationArray.h"
#include "OVR_LaunchBlockFlowResult.h"
#include "OVR_LaunchFriendRequestFlowResult.h"
#include "OVR_LaunchReportFlowResult.h"
#include "OVR_LaunchUnblockFlowResult.h"
#include "OVR_LeaderboardEntryArray.h"
#include "OVR_LeaderboardUpdateStatus.h"
#include "OVR_LinkedAccountArray.h"
#include "OVR_LivestreamingApplicationStatus.h"
#include "OVR_LivestreamingStartResult.h"
#include "OVR_LivestreamingStatus.h"
#include "OVR_LivestreamingVideoStats.h"
#include "OVR_MatchmakingAdminSnapshot.h"
#include "OVR_MatchmakingBrowseResult.h"
#include "OVR_MatchmakingEnqueueResult.h"
#include "OVR_MatchmakingEnqueueResultAndRoom.h"
#include "OVR_MatchmakingRoomArray.h"
#include "OVR_MatchmakingStats.h"
#include "OVR_MessageType.h"
#include "OVR_NetSyncConnection.h"
#include "OVR_NetSyncSessionArray.h"
#include "OVR_NetSyncSessionsChangedNotification.h"
#include "OVR_NetSyncSetSessionPropertyResult.h"
#include "OVR_NetSyncVoipAttenuationValueArray.h"
#include "OVR_NetworkingPeer.h"
#include "OVR_OrgScopedID.h"
#include "OVR_Party.h"
#include "OVR_PartyID.h"
#include "OVR_PartyUpdateNotification.h"
#include "OVR_PidArray.h"
#include "OVR_PingResult.h"
#include "OVR_PlatformInitialize.h"
#include "OVR_ProductArray.h"
#include "OVR_Purchase.h"
#include "OVR_PurchaseArray.h"
#include "OVR_Room.h"
#include "OVR_RoomArray.h"
#include "OVR_RoomInviteNotification.h"
#include "OVR_RoomInviteNotificationArray.h"
#include "OVR_SdkAccountArray.h"
#include "OVR_ShareMediaResult.h"
#include "OVR_SystemPermission.h"
#include "OVR_SystemVoipState.h"
#include "OVR_Types.h"
#include "OVR_User.h"
#include "OVR_UserAndRoomArray.h"
#include "OVR_UserArray.h"
#include "OVR_UserProof.h"
#include "OVR_UserReportID.h"
#include <stdbool.h>

typedef struct ovrMessage *ovrMessageHandle;

OVRP_PUBLIC_FUNCTION(ovrAbuseReportRecordingHandle)               ovr_Message_GetAbuseReportRecording(const ovrMessageHandle obj);
OVRP_PUBLIC_FUNCTION(ovrAchievementDefinitionArrayHandle)         ovr_Message_GetAchievementDefinitionArray(const ovrMessageHandle obj);
OVRP_PUBLIC_FUNCTION(ovrAchievementProgressArrayHandle)           ovr_Message_GetAchievementProgressArray(const ovrMessageHandle obj);
OVRP_PUBLIC_FUNCTION(ovrAchievementUpdateHandle)                  ovr_Message_GetAchievementUpdate(const ovrMessageHandle obj);
OVRP_PUBLIC_FUNCTION(ovrApplicationVersionHandle)                 ovr_Message_GetApplicationVersion(const ovrMessageHandle obj);
OVRP_PUBLIC_FUNCTION(ovrAssetDetailsHandle)                       ovr_Message_GetAssetDetails(const ovrMessageHandle obj);
OVRP_PUBLIC_FUNCTION(ovrAssetDetailsArrayHandle)                  ovr_Message_GetAssetDetailsArray(const ovrMessageHandle obj);
OVRP_PUBLIC_FUNCTION(ovrAssetFileDeleteResultHandle)              ovr_Message_GetAssetFileDeleteResult(const ovrMessageHandle obj);
OVRP_PUBLIC_FUNCTION(ovrAssetFileDownloadCancelResultHandle)      ovr_Message_GetAssetFileDownloadCancelResult(const ovrMessageHandle obj);
OVRP_PUBLIC_FUNCTION(ovrAssetFileDownloadResultHandle)            ovr_Message_GetAssetFileDownloadResult(const ovrMessageHandle obj);
OVRP_PUBLIC_FUNCTION(ovrAssetFileDownloadUpdateHandle)            ovr_Message_GetAssetFileDownloadUpdate(const ovrMessageHandle obj);
OVRP_PUBLIC_FUNCTION(ovrCalApplicationFinalizedHandle)            ovr_Message_GetCalApplicationFinalized(const ovrMessageHandle obj);
OVRP_PUBLIC_FUNCTION(ovrCalApplicationProposedHandle)             ovr_Message_GetCalApplicationProposed(const ovrMessageHandle obj);
OVRP_PUBLIC_FUNCTION(ovrCalApplicationSuggestionArrayHandle)      ovr_Message_GetCalApplicationSuggestionArray(const ovrMessageHandle obj);
OVRP_PUBLIC_FUNCTION(ovrCloudStorageConflictMetadataHandle)       ovr_Message_GetCloudStorageConflictMetadata(const ovrMessageHandle obj);
OVRP_PUBLIC_FUNCTION(ovrCloudStorageDataHandle)                   ovr_Message_GetCloudStorageData(const ovrMessageHandle obj);
OVRP_PUBLIC_FUNCTION(ovrCloudStorageMetadataHandle)               ovr_Message_GetCloudStorageMetadata(const ovrMessageHandle obj);
OVRP_PUBLIC_FUNCTION(ovrCloudStorageMetadataArrayHandle)          ovr_Message_GetCloudStorageMetadataArray(const ovrMessageHandle obj);
OVRP_PUBLIC_FUNCTION(ovrCloudStorageUpdateResponseHandle)         ovr_Message_GetCloudStorageUpdateResponse(const ovrMessageHandle obj);
OVRP_PUBLIC_FUNCTION(ovrDestinationArrayHandle)                   ovr_Message_GetDestinationArray(const ovrMessageHandle obj);
OVRP_PUBLIC_FUNCTION(ovrErrorHandle)                              ovr_Message_GetError(const ovrMessageHandle obj);
OVRP_PUBLIC_FUNCTION(ovrHttpTransferUpdateHandle)                 ovr_Message_GetHttpTransferUpdate(const ovrMessageHandle obj);
OVRP_PUBLIC_FUNCTION(ovrInstalledApplicationArrayHandle)          ovr_Message_GetInstalledApplicationArray(const ovrMessageHandle obj);
OVRP_PUBLIC_FUNCTION(ovrLaunchBlockFlowResultHandle)              ovr_Message_GetLaunchBlockFlowResult(const ovrMessageHandle obj);
OVRP_PUBLIC_FUNCTION(ovrLaunchFriendRequestFlowResultHandle)      ovr_Message_GetLaunchFriendRequestFlowResult(const ovrMessageHandle obj);
OVRP_PUBLIC_FUNCTION(ovrLaunchReportFlowResultHandle)             ovr_Message_GetLaunchReportFlowResult(const ovrMessageHandle obj);
OVRP_PUBLIC_FUNCTION(ovrLaunchUnblockFlowResultHandle)            ovr_Message_GetLaunchUnblockFlowResult(const ovrMessageHandle obj);
OVRP_PUBLIC_FUNCTION(ovrLeaderboardEntryArrayHandle)              ovr_Message_GetLeaderboardEntryArray(const ovrMessageHandle obj);
OVRP_PUBLIC_FUNCTION(ovrLeaderboardUpdateStatusHandle)            ovr_Message_GetLeaderboardUpdateStatus(const ovrMessageHandle obj);
OVRP_PUBLIC_FUNCTION(ovrLinkedAccountArrayHandle)                 ovr_Message_GetLinkedAccountArray(const ovrMessageHandle obj);
OVRP_PUBLIC_FUNCTION(ovrLivestreamingApplicationStatusHandle)     ovr_Message_GetLivestreamingApplicationStatus(const ovrMessageHandle obj);
OVRP_PUBLIC_FUNCTION(ovrLivestreamingStartResultHandle)           ovr_Message_GetLivestreamingStartResult(const ovrMessageHandle obj);
OVRP_PUBLIC_FUNCTION(ovrLivestreamingStatusHandle)                ovr_Message_GetLivestreamingStatus(const ovrMessageHandle obj);
OVRP_PUBLIC_FUNCTION(ovrLivestreamingVideoStatsHandle)            ovr_Message_GetLivestreamingVideoStats(const ovrMessageHandle obj);
OVRP_PUBLIC_FUNCTION(ovrMatchmakingAdminSnapshotHandle)           ovr_Message_GetMatchmakingAdminSnapshot(const ovrMessageHandle obj);
OVRP_PUBLIC_FUNCTION(ovrMatchmakingBrowseResultHandle)            ovr_Message_GetMatchmakingBrowseResult(const ovrMessageHandle obj);
OVRP_PUBLIC_FUNCTION(ovrMatchmakingEnqueueResultHandle)           ovr_Message_GetMatchmakingEnqueueResult(const ovrMessageHandle obj);
OVRP_PUBLIC_FUNCTION(ovrMatchmakingEnqueueResultAndRoomHandle)    ovr_Message_GetMatchmakingEnqueueResultAndRoom(const ovrMessageHandle obj);
OVRP_PUBLIC_FUNCTION(ovrMatchmakingRoomArrayHandle)               ovr_Message_GetMatchmakingRoomArray(const ovrMessageHandle obj);
OVRP_PUBLIC_FUNCTION(ovrMatchmakingStatsHandle)                   ovr_Message_GetMatchmakingStats(const ovrMessageHandle obj);
OVRP_PUBLIC_FUNCTION(ovrMessageHandle)                            ovr_Message_GetNativeMessage(const ovrMessageHandle obj);
OVRP_PUBLIC_FUNCTION(ovrNetSyncConnectionHandle)                  ovr_Message_GetNetSyncConnection(const ovrMessageHandle obj);
OVRP_PUBLIC_FUNCTION(ovrNetSyncSessionArrayHandle)                ovr_Message_GetNetSyncSessionArray(const ovrMessageHandle obj);
OVRP_PUBLIC_FUNCTION(ovrNetSyncSessionsChangedNotificationHandle) ovr_Message_GetNetSyncSessionsChangedNotification(const ovrMessageHandle obj);
OVRP_PUBLIC_FUNCTION(ovrNetSyncSetSessionPropertyResultHandle)    ovr_Message_GetNetSyncSetSessionPropertyResult(const ovrMessageHandle obj);
OVRP_PUBLIC_FUNCTION(ovrNetSyncVoipAttenuationValueArrayHandle)   ovr_Message_GetNetSyncVoipAttenuationValueArray(const ovrMessageHandle obj);
OVRP_PUBLIC_FUNCTION(ovrNetworkingPeerHandle)                     ovr_Message_GetNetworkingPeer(const ovrMessageHandle obj);
OVRP_PUBLIC_FUNCTION(ovrOrgScopedIDHandle)                        ovr_Message_GetOrgScopedID(const ovrMessageHandle obj);
OVRP_PUBLIC_FUNCTION(ovrPartyHandle)                              ovr_Message_GetParty(const ovrMessageHandle obj);
OVRP_PUBLIC_FUNCTION(ovrPartyIDHandle)                            ovr_Message_GetPartyID(const ovrMessageHandle obj);
OVRP_PUBLIC_FUNCTION(ovrPartyUpdateNotificationHandle)            ovr_Message_GetPartyUpdateNotification(const ovrMessageHandle obj);
OVRP_PUBLIC_FUNCTION(ovrPidArrayHandle)                           ovr_Message_GetPidArray(const ovrMessageHandle obj);
OVRP_PUBLIC_FUNCTION(ovrPingResultHandle)                         ovr_Message_GetPingResult(const ovrMessageHandle obj);
OVRP_PUBLIC_FUNCTION(ovrPlatformInitializeHandle)                 ovr_Message_GetPlatformInitialize(const ovrMessageHandle obj);
OVRP_PUBLIC_FUNCTION(ovrProductArrayHandle)                       ovr_Message_GetProductArray(const ovrMessageHandle obj);
OVRP_PUBLIC_FUNCTION(ovrPurchaseHandle)                           ovr_Message_GetPurchase(const ovrMessageHandle obj);
OVRP_PUBLIC_FUNCTION(ovrPurchaseArrayHandle)                      ovr_Message_GetPurchaseArray(const ovrMessageHandle obj);
OVRP_PUBLIC_FUNCTION(ovrRequest)                                  ovr_Message_GetRequestID(const ovrMessageHandle obj);
OVRP_PUBLIC_FUNCTION(ovrRoomHandle)                               ovr_Message_GetRoom(const ovrMessageHandle obj);
OVRP_PUBLIC_FUNCTION(ovrRoomArrayHandle)                          ovr_Message_GetRoomArray(const ovrMessageHandle obj);
OVRP_PUBLIC_FUNCTION(ovrRoomInviteNotificationHandle)             ovr_Message_GetRoomInviteNotification(const ovrMessageHandle obj);
OVRP_PUBLIC_FUNCTION(ovrRoomInviteNotificationArrayHandle)        ovr_Message_GetRoomInviteNotificationArray(const ovrMessageHandle obj);
OVRP_PUBLIC_FUNCTION(ovrSdkAccountArrayHandle)                    ovr_Message_GetSdkAccountArray(const ovrMessageHandle obj);
OVRP_PUBLIC_FUNCTION(ovrShareMediaResultHandle)                   ovr_Message_GetShareMediaResult(const ovrMessageHandle obj);
OVRP_PUBLIC_FUNCTION(const char *)                                ovr_Message_GetString(const ovrMessageHandle obj);
OVRP_PUBLIC_FUNCTION(ovrSystemPermissionHandle)                   ovr_Message_GetSystemPermission(const ovrMessageHandle obj);
OVRP_PUBLIC_FUNCTION(ovrSystemVoipStateHandle)                    ovr_Message_GetSystemVoipState(const ovrMessageHandle obj);
OVRP_PUBLIC_FUNCTION(ovrMessageType)                              ovr_Message_GetType(const ovrMessageHandle obj);
OVRP_PUBLIC_FUNCTION(ovrUserHandle)                               ovr_Message_GetUser(const ovrMessageHandle obj);
OVRP_PUBLIC_FUNCTION(ovrUserAndRoomArrayHandle)                   ovr_Message_GetUserAndRoomArray(const ovrMessageHandle obj);
OVRP_PUBLIC_FUNCTION(ovrUserArrayHandle)                          ovr_Message_GetUserArray(const ovrMessageHandle obj);
OVRP_PUBLIC_FUNCTION(ovrUserProofHandle)                          ovr_Message_GetUserProof(const ovrMessageHandle obj);
OVRP_PUBLIC_FUNCTION(ovrUserReportIDHandle)                       ovr_Message_GetUserReportID(const ovrMessageHandle obj);
OVRP_PUBLIC_FUNCTION(bool)                                        ovr_Message_IsError(const ovrMessageHandle obj);

#endif
