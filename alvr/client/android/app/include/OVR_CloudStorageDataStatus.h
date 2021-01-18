// This file was @generated with LibOVRPlatform/codegen/main. Do not modify it!

#ifndef OVR_CLOUD_STORAGE_DATA_STATUS_H
#define OVR_CLOUD_STORAGE_DATA_STATUS_H

#include "OVR_Platform_Defs.h"

typedef enum ovrCloudStorageDataStatus_ {
  ovrCloudStorageDataStatus_Unknown,
  ovrCloudStorageDataStatus_InSync,
  ovrCloudStorageDataStatus_NeedsDownload,
  ovrCloudStorageDataStatus_RemoteDownloading,
  ovrCloudStorageDataStatus_NeedsUpload,
  ovrCloudStorageDataStatus_LocalUploading,
  ovrCloudStorageDataStatus_InConflict,
} ovrCloudStorageDataStatus;

/// Converts an ovrCloudStorageDataStatus enum value to a string
/// The returned string does not need to be freed
OVRPL_PUBLIC_FUNCTION(const char*) ovrCloudStorageDataStatus_ToString(ovrCloudStorageDataStatus value);

/// Converts a string representing an ovrCloudStorageDataStatus enum value to an enum value
///
OVRPL_PUBLIC_FUNCTION(ovrCloudStorageDataStatus) ovrCloudStorageDataStatus_FromString(const char* str);

#endif
