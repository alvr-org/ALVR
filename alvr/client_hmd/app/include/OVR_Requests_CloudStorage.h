// This file was @generated with LibOVRPlatform/codegen/main. Do not modify it!

#ifndef OVR_REQUESTS_CLOUDSTORAGE_H
#define OVR_REQUESTS_CLOUDSTORAGE_H

#include "OVR_Types.h"
#include "OVR_Platform_Defs.h"

#include "OVR_CloudStorage.h"
#include "OVR_CloudStorageConflictMetadata.h"
#include "OVR_CloudStorageMetadata.h"
#include "OVR_CloudStorageMetadataArray.h"
#include <stdbool.h>

/// \file
/// # Cloud Storage
///
/// The cloud storage API allows Apps to save, synchronize and load data between devices and
/// App installs.  This enables three key scenarios:
///
/// 1. Saving progress between installs.  Users are free to uninstall and re-install without
///    loosing saved progress.
/// 2. Sharing progress between devices.  Blobs are synchronized between all devices the user
///    has the App installed on.
/// 3. Disaster recovery.  User data can be restored if devices are lost or damaged.
///
/// The API is defined in terms of Buckets and data Blobs indexed by Key.  Direct file support is not
/// yet implemented.  However, as you add Cloud Storage support, it may be metaphorically
/// useful to think of Bucket as a directory, data Blob as a file, and Key as a file name.
///
/// ## Cloud Storage Bucket
///
/// Storage is conceptually and functionally divided into Buckets of data which are configured
/// on the developer dashboard for the App.  An App can have multiple Buckets defined but that is
/// not generally necessary unless different conflict resolution policies are required. Buckets names
/// are required to conform to Microsoft Windows directory name restrictions and be case-insensitive
/// unique within and Application Grouping.  No default Buckets exist for an application so to
/// enable Cloud Storage at least one must be created on the dashboard.
///
/// ## Saving Data
///
/// Data is stored as an opaque binary Blob using the following API method:
///
///   ovr_CloudStorage_Save(bucket_name, key, data_pointer, data_size, counter, extra_data)
///
/// bucket_name - defined on the Developer Dashboard.
/// key - unique index for the blob.
/// data_pointer - pointer to the data blob.
/// data_size - size in bytes of the data blob.
/// counter - uint64 metadata, optional unless HIGHEST_COUNTER conflict management is specified.
/// extra_data - optional string metadata.
///
/// This call sends the Blob data to the locally running Oculus process and will not trigger network
/// calls.  Network synchronization happens when the App is no longer running.
///
/// ## Loading Data
///
/// To load a stored data Blob, use the following API call:
///
/// ovr_CloudStorage_Load(bucket_name, key)
///
/// The response message type is 'ovrMessage_CloudStorage_Load' and if no error has occurred the data is
/// retrieved as follows:
///
///   ovrCloudStorageDataHandle response = ovr_Message_GetCloudStorageData(message);
///   void* data = ovr_CloudStorageData_GetData(response);
///   uint32 data_size = ovr_CloudStorageData_GetDataSize(response);
///
/// If the Cloud Storage Bucket is configured for MANUAL conflict resolution and a conflict exists
/// between the local and remote versions, the load call will return an error.  The process to resolve
/// conflicts is discussed below.  Loading will not initiate a network call.
///
/// ## Enumerating Data
///
/// To request a list of metadata for all blobs in a Bucket, use the API call:
///
///   ovr_CloudStorage_LoadBucketMetadata(bucket_name)
///
/// Or for a single key in a bucket:
///
///   ovr_CloudStorage_LoadMetadata(bucket_name, key)
///
/// Metadata for a blob includes the counter and extra_data parameters as well as some
/// calculated values.  These can be obtained via the API as follows:
///
///   uint32 data_size = ovr_CloudStorageMetadata_getDataSize(metadataHandle)
///   uint64 saved_time = ovr_CloudStorageMetadata_getSaveTime(metadataHandle)
///   int64 counter = ovr_CloudStorageMetadata_getCounter(metadataHandle)
///   const char* extra_data = ovr_CloudStorageMetadata_getExtraData(metadataHandle)
///   ovrCloudStorageVersionHandle handle = ovr_CloudStorageMetadata_getHandle(metadataHandle)
///   ovrCloudStorageDataStatus status = ovr_CloudStorageMetadata_getStatus(metadataHandle)
///
/// data_size - the size in bytes of the stored data Blob.
/// saved_time - the UTC time in seconds since the UNIX epoch when the blob was locally saved.  Note,
///              this is the time as recorded on the local device so includes local clock skew.
/// counter - the value specified when saved, else zero.
/// extra_data - the value specified when saved, else NULL.
/// handle - used for manual conflict resolution (see below).
/// status - enum describing the state of the Blob.  This state as determine by the Oculus
/// process's most recent network update.  The following states are possible:
///
/// ovrCloudStorageDataStatus_InSync - the local and remote versions are in-sync.
/// ovrCloudStorageDataStatus_NeedsDownload - a newer version exists in the cloud but hasn't yet downloaded.
/// ovrCloudStorageDataStatus_NeedsUpload - the local version is newer and needs to be uploaded.
/// ovrCloudStorageDataStatus_InConflict - the local and remote version have a conflict that must be manually
///                                        resolved.  Only occurs for buckets set to MANUAL conflict resolution.
///
///
/// ## Managing Conflicts
///
/// The Cloud Storage API fully supports synchronizing data between multiple devices and
/// potentially multiple platforms.  This requires developers describe a conflict resolution policy
/// to handle situations where multiple devices try to upload Blobs to the same key at the same time.
/// This situation is more common on mobile devices but can also occur between PCs.  The following
/// strategies are provided:
///
/// ### Latest Timestamp
///
/// This is the simplest method.  It configures the Bucket to prefer the Blob that has the latest
/// timestamp as recorded by the local device.  The timestamp is recorded at the time of the
/// call to ovr_CloudStorage_Save().  Client Blobs with earlier timestamps than the remotely stored
/// version are discarded.
///
/// ### Highest Counter
///
/// Buckets configured this way prefer Blobs with the highest value set in the counter field.  This would
/// be appropriate, for example, if you wanted to preserve the Blob with the highest score.  Blobs stored
/// with the same counter value as a remote version will attempt to be stored, however multiple
/// devices doing this represent a race-condition and either one might win.
///
/// ### Manual
///
/// This setting delegates all conflict resolution responsibility to the App.  When an App saves a new
/// local Blob to a specific Key that's in the state ovrCloudStorageDataStatus_InConflict, the Blob
/// will not be uploaded until the App intentionally resolves the conflict.  Conflict resolution can be
/// done at any time but it's best done during App startup and shutdown.  Detecting the
/// need for manual conflict resolution can be noticed by reading the metadata status, or by checking
/// the response from the save message as follows:
///
///   ovrCloudStorageUpdateResponseHandle response = ovr_Message_GetCloudStorageUpdateResponse(save_message)
///   ovrCloudStorageUpdateStatus status = ovr_CloudStorageUpdateResponse_GetStatus(response))
///   if (status == ovrCloudStorageUpdateStatus_ManualMergeRequired) {
///     // perform manual merge...
///   }
///
/// The first step to resolving is to load the metadata for the local and remote Blobs:
///
///   ovr_CloudStorage_LoadConflictMetadata(bucket, key)
///
/// This is an asynchronous call whose response message is parsed to get the metadata:
///
///   ovrCloudStorageConflictMetadataHandle response = ovr_Message_GetCloudStorageConflictMetadata(message)
///   ovrCloudStorageMetadataHandle local_metadata = ovr_CloudStorageConflictMetadata_GetLocal(response)
///   ovrCloudStorageMetadataHandle remote_metadata = ovr_CloudStorageConflictMetadata_GetRemote(response)
///
/// 1. Resolving based on metadata
///
/// If the metadata for the Blob contain enough information to resolve the conflict, then the next step
/// is to call
///
///   ovr_CloudStorage_ResolveKeepRemote(bucket_name, key, remote_handle)
///
/// to choose the remote Blob, or to choose the local Blob:
///
///   ovr_CloudStorage_ResolveKeepLocal(bucket_name, key, remote_handle)
///
/// Notice the handle from the remote Blob's metadata is passed in to prevent data loss in the
/// rare instance that a new remote Blob appears during conflict resolution.
///
/// 2. Resolving by inspecting the exiting remote and local data
///
/// If the App needs to inspect the actual data Blobs to determine which to keep, they can be
/// loaded using the handle from the metadata:
///
///   ovr_CloudStorage_LoadHandle(handle)
///
/// This works for both the local and remote handles.  However, if the remote Blob has not been
/// cached locally, requesting the remote will initiate a network call to fetch any remaining data.
///
/// 3. Resolving by merging the remote and local data
///
/// If the App needs to merge the saved Blobs, they can be loaded as described above, then the app
/// can simply save a new local version and resolve by choosing that:
///
///   ovr_CloudStorage_Save(bucket_name, key, merged_data_pointer, merged_data_size, counter, extra_data)
///   ovr_CloudStorage_ResolveKeepLocal(bucket_name, key, remote_handle)
///
/// ## Limits
///
/// 1. Blob size <= 10 megabytes.  This limit is driven by the API using contiguous blocks of memory
///    to communicate with the App.
/// 2. Blobs stored per Bucket <= 10,000.  Apps should use smaller limits as the customer experience
///    for synchronizing many keys may not be ideal.
/// 3. Bucket names must be <= 128 bytes and conform to Windows file naming restrictions.
/// 4. Key and extra_data string lengths must be <= 512 bytes.
/// 5. The service itself does not save old versions.  An App that needs them preserved is free to store
///    them using a Bucket/Key backup strategy that fits its need.
///
/// ## Future Ideas
///
/// We look forward to customer feedback on the Cloud Storage API so please send us your input.  Some
/// ideas already in discussion are:
///
/// * Files - direct file support with much a larger maximum allowed size.
/// * Require download before launch - giving Apps the ability to declare they want all data to be
///   downloaded before an App is allowed launch.
/// * In-App sync - give Apps the ability to initiate an upload or download while running.
/// * In-App sync progress - getting progress during in-App synchronization.
/// * Title Storage - Cloud Storage that belongs to the App (not User) and is synchronized to all devices.
/// * Sharing Blobs - allowing users to share their data Blobs with other users.
/// * Leaderboard Attachments - attaching a Blob to a Leaderboard entry.
///
/// ## Undefined behavior
///
/// The following parts the API are currently undefined, but deserve an explanation:
///
/// * What happens to local data when an App is removed before the user's data is fully uploaded?
/// * Detecting and handling an inability to write data due to things like the filesystem being full.
/// * local clock skew (resetting time) when using Latest Timestamp conflict resolution.
/// * How long will the data be retained?  SLA is not yet defined but it will probably be closer to as
///   long as users are able to play the App rather than being forever.

/// Deletes the specified save data buffer. Conflicts are handled just like
/// Saves.
/// \param bucket The name of the storage bucket.
/// \param key The name for this saved data.
///
/// A message with type ::ovrMessage_CloudStorage_Delete will be generated in response.
///
/// First call ::ovr_Message_IsError() to check if an error occurred.
///
/// If no error occurred, the message will contain a payload of type ::ovrCloudStorageUpdateResponseHandle.
/// Extract the payload from the message handle with ::ovr_Message_GetCloudStorageUpdateResponse().
OVRP_PUBLIC_FUNCTION(ovrRequest) ovr_CloudStorage_Delete(const char *bucket, const char *key);

/// Get the next page of entries
///
/// A message with type ::ovrMessage_CloudStorage_GetNextCloudStorageMetadataArrayPage will be generated in response.
///
/// First call ::ovr_Message_IsError() to check if an error occurred.
///
/// If no error occurred, the message will contain a payload of type ::ovrCloudStorageMetadataArrayHandle.
/// Extract the payload from the message handle with ::ovr_Message_GetCloudStorageMetadataArray().
OVRP_PUBLIC_FUNCTION(ovrRequest) ovr_CloudStorage_GetNextCloudStorageMetadataArrayPage(ovrCloudStorageMetadataArrayHandle handle);

/// Loads the saved entry for the specified bucket and key. If a conflict
/// exists with the key then an error message is returned.
/// \param bucket The name of the storage bucket.
/// \param key The name for this saved data.
///
/// A message with type ::ovrMessage_CloudStorage_Load will be generated in response.
///
/// First call ::ovr_Message_IsError() to check if an error occurred.
///
/// If no error occurred, the message will contain a payload of type ::ovrCloudStorageDataHandle.
/// Extract the payload from the message handle with ::ovr_Message_GetCloudStorageData().
OVRP_PUBLIC_FUNCTION(ovrRequest) ovr_CloudStorage_Load(const char *bucket, const char *key);

/// Loads all the metadata for the saves in the specified bucket, including
/// conflicts.
/// \param bucket The name of the storage bucket.
///
/// A message with type ::ovrMessage_CloudStorage_LoadBucketMetadata will be generated in response.
///
/// First call ::ovr_Message_IsError() to check if an error occurred.
///
/// If no error occurred, the message will contain a payload of type ::ovrCloudStorageMetadataArrayHandle.
/// Extract the payload from the message handle with ::ovr_Message_GetCloudStorageMetadataArray().
OVRP_PUBLIC_FUNCTION(ovrRequest) ovr_CloudStorage_LoadBucketMetadata(const char *bucket);

/// Loads the metadata for this bucket-key combination that need to be manually
/// resolved.
/// \param bucket The name of the storage bucket
/// \param key The key for this saved data.
///
/// A message with type ::ovrMessage_CloudStorage_LoadConflictMetadata will be generated in response.
///
/// First call ::ovr_Message_IsError() to check if an error occurred.
///
/// If no error occurred, the message will contain a payload of type ::ovrCloudStorageConflictMetadataHandle.
/// Extract the payload from the message handle with ::ovr_Message_GetCloudStorageConflictMetadata().
OVRP_PUBLIC_FUNCTION(ovrRequest) ovr_CloudStorage_LoadConflictMetadata(const char *bucket, const char *key);

/// Loads the data specified by the storage handle.
///
/// A message with type ::ovrMessage_CloudStorage_LoadHandle will be generated in response.
///
/// First call ::ovr_Message_IsError() to check if an error occurred.
///
/// If no error occurred, the message will contain a payload of type ::ovrCloudStorageDataHandle.
/// Extract the payload from the message handle with ::ovr_Message_GetCloudStorageData().
OVRP_PUBLIC_FUNCTION(ovrRequest) ovr_CloudStorage_LoadHandle(ovrCloudStorageVersionHandle handle);

/// load the metadata for the specified key
/// \param bucket The name of the storage bucket.
/// \param key The name for this saved data.
///
/// A message with type ::ovrMessage_CloudStorage_LoadMetadata will be generated in response.
///
/// First call ::ovr_Message_IsError() to check if an error occurred.
///
/// If no error occurred, the message will contain a payload of type ::ovrCloudStorageMetadataHandle.
/// Extract the payload from the message handle with ::ovr_Message_GetCloudStorageMetadata().
OVRP_PUBLIC_FUNCTION(ovrRequest) ovr_CloudStorage_LoadMetadata(const char *bucket, const char *key);

/// Selects the local save for manual conflict resolution.
/// \param bucket The name of the storage bucket.
/// \param key The name for this saved data.
/// \param remoteHandle The handle of the remote that the local file was resolved against.
///
/// A message with type ::ovrMessage_CloudStorage_ResolveKeepLocal will be generated in response.
///
/// First call ::ovr_Message_IsError() to check if an error occurred.
///
/// If no error occurred, the message will contain a payload of type ::ovrCloudStorageUpdateResponseHandle.
/// Extract the payload from the message handle with ::ovr_Message_GetCloudStorageUpdateResponse().
OVRP_PUBLIC_FUNCTION(ovrRequest) ovr_CloudStorage_ResolveKeepLocal(const char *bucket, const char *key, ovrCloudStorageVersionHandle remoteHandle);

/// Selects the remote save for manual conflict resolution.
/// \param bucket The name of the storage bucket.
/// \param key The name for this saved data.
/// \param remoteHandle The handle of the remote.
///
/// A message with type ::ovrMessage_CloudStorage_ResolveKeepRemote will be generated in response.
///
/// First call ::ovr_Message_IsError() to check if an error occurred.
///
/// If no error occurred, the message will contain a payload of type ::ovrCloudStorageUpdateResponseHandle.
/// Extract the payload from the message handle with ::ovr_Message_GetCloudStorageUpdateResponse().
OVRP_PUBLIC_FUNCTION(ovrRequest) ovr_CloudStorage_ResolveKeepRemote(const char *bucket, const char *key, ovrCloudStorageVersionHandle remoteHandle);

/// Note: Cloud Storage is only available for Rift apps.
///
/// Send a save data buffer to the platform. ovr_CloudStorage_Save() passes a
/// pointer to your data in an async call. You need to maintain the save data
/// until you receive the message indicating that the save was successful.
///
/// If the data is destroyed or modified prior to receiving that message the
/// data will not be saved.
/// \param bucket The name of the storage bucket.
/// \param key The name for this saved data.
/// \param data Start of the data block.
/// \param dataSize Size of the data block.
/// \param counter Optional. Counter used for user data or auto-deconfliction.
/// \param extraData Optional. String data that isn't used by the platform.
///
/// <b>Error codes</b>
/// - \b 100: The stored version has a later timestamp than the data provided. This cloud storage bucket's conflict resolution policy is configured to use the latest timestamp, which is configurable in the developer dashboard.
///
/// A message with type ::ovrMessage_CloudStorage_Save will be generated in response.
///
/// First call ::ovr_Message_IsError() to check if an error occurred.
///
/// If no error occurred, the message will contain a payload of type ::ovrCloudStorageUpdateResponseHandle.
/// Extract the payload from the message handle with ::ovr_Message_GetCloudStorageUpdateResponse().
OVRP_PUBLIC_FUNCTION(ovrRequest) ovr_CloudStorage_Save(const char *bucket, const char *key, const void *data, unsigned int dataSize, long long counter, const char *extraData);

#endif
