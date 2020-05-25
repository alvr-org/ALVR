// This file was @generated with LibOVRPlatform/codegen/main. Do not modify it!

#ifndef OVR_CLOUDSTORAGECONFLICTMETADATA_H
#define OVR_CLOUDSTORAGECONFLICTMETADATA_H

#include "OVR_Platform_Defs.h"
#include "OVR_CloudStorageMetadata.h"

typedef struct ovrCloudStorageConflictMetadata *ovrCloudStorageConflictMetadataHandle;

OVRP_PUBLIC_FUNCTION(ovrCloudStorageMetadataHandle) ovr_CloudStorageConflictMetadata_GetLocal(const ovrCloudStorageConflictMetadataHandle obj);
OVRP_PUBLIC_FUNCTION(ovrCloudStorageMetadataHandle) ovr_CloudStorageConflictMetadata_GetRemote(const ovrCloudStorageConflictMetadataHandle obj);

#endif
