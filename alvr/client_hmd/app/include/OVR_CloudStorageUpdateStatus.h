// This file was @generated with LibOVRPlatform/codegen/main. Do not modify it!

#ifndef OVR_CLOUD_STORAGE_UPDATE_STATUS_H
#define OVR_CLOUD_STORAGE_UPDATE_STATUS_H

#include "OVR_Platform_Defs.h"

typedef enum ovrCloudStorageUpdateStatus_ {
  ovrCloudStorageUpdateStatus_Unknown,
  ovrCloudStorageUpdateStatus_Ok,
  ovrCloudStorageUpdateStatus_BetterVersionStored,
  ovrCloudStorageUpdateStatus_ManualMergeRequired,
} ovrCloudStorageUpdateStatus;

/// Converts an ovrCloudStorageUpdateStatus enum value to a string
/// The returned string does not need to be freed
OVRPL_PUBLIC_FUNCTION(const char*) ovrCloudStorageUpdateStatus_ToString(ovrCloudStorageUpdateStatus value);

/// Converts a string representing an ovrCloudStorageUpdateStatus enum value to an enum value
///
OVRPL_PUBLIC_FUNCTION(ovrCloudStorageUpdateStatus) ovrCloudStorageUpdateStatus_FromString(const char* str);

#endif
