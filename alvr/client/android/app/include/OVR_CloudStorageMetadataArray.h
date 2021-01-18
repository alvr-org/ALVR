// This file was @generated with LibOVRPlatform/codegen/main. Do not modify it!

#ifndef OVR_CLOUDSTORAGEMETADATAARRAY_H
#define OVR_CLOUDSTORAGEMETADATAARRAY_H

#include "OVR_Platform_Defs.h"
#include "OVR_CloudStorageMetadata.h"
#include <stdbool.h>
#include <stddef.h>

typedef struct ovrCloudStorageMetadataArray *ovrCloudStorageMetadataArrayHandle;

OVRP_PUBLIC_FUNCTION(ovrCloudStorageMetadataHandle) ovr_CloudStorageMetadataArray_GetElement(const ovrCloudStorageMetadataArrayHandle obj, size_t index);
OVRP_PUBLIC_FUNCTION(const char *)                  ovr_CloudStorageMetadataArray_GetNextUrl(const ovrCloudStorageMetadataArrayHandle obj);
OVRP_PUBLIC_FUNCTION(size_t)                        ovr_CloudStorageMetadataArray_GetSize(const ovrCloudStorageMetadataArrayHandle obj);
OVRP_PUBLIC_FUNCTION(bool)                          ovr_CloudStorageMetadataArray_HasNextPage(const ovrCloudStorageMetadataArrayHandle obj);

#endif
