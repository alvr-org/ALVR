// This file was @generated with LibOVRPlatform/codegen/main. Do not modify it!

#ifndef OVR_CLOUDSTORAGEMETADATA_H
#define OVR_CLOUDSTORAGEMETADATA_H

#include "OVR_Platform_Defs.h"
#include "OVR_CloudStorage.h"
#include "OVR_CloudStorageDataStatus.h"

typedef struct ovrCloudStorageMetadata *ovrCloudStorageMetadataHandle;

OVRP_PUBLIC_FUNCTION(const char *)                 ovr_CloudStorageMetadata_GetBucket(const ovrCloudStorageMetadataHandle obj);
OVRP_PUBLIC_FUNCTION(long long)                    ovr_CloudStorageMetadata_GetCounter(const ovrCloudStorageMetadataHandle obj);
OVRP_PUBLIC_FUNCTION(unsigned int)                 ovr_CloudStorageMetadata_GetDataSize(const ovrCloudStorageMetadataHandle obj);
OVRP_PUBLIC_FUNCTION(const char *)                 ovr_CloudStorageMetadata_GetExtraData(const ovrCloudStorageMetadataHandle obj);
OVRP_PUBLIC_FUNCTION(const char *)                 ovr_CloudStorageMetadata_GetKey(const ovrCloudStorageMetadataHandle obj);
OVRP_PUBLIC_FUNCTION(unsigned long long)           ovr_CloudStorageMetadata_GetSaveTime(const ovrCloudStorageMetadataHandle obj);
OVRP_PUBLIC_FUNCTION(ovrCloudStorageDataStatus)    ovr_CloudStorageMetadata_GetStatus(const ovrCloudStorageMetadataHandle obj);
OVRP_PUBLIC_FUNCTION(ovrCloudStorageVersionHandle) ovr_CloudStorageMetadata_GetVersionHandle(const ovrCloudStorageMetadataHandle obj);

#endif
