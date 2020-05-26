// This file was @generated with LibOVRPlatform/codegen/main. Do not modify it!

#ifndef OVR_CLOUDSTORAGEDATA_H
#define OVR_CLOUDSTORAGEDATA_H

#include "OVR_Platform_Defs.h"

typedef struct ovrCloudStorageData *ovrCloudStorageDataHandle;

OVRP_PUBLIC_FUNCTION(const char *) ovr_CloudStorageData_GetBucket(const ovrCloudStorageDataHandle obj);
OVRP_PUBLIC_FUNCTION(const void *) ovr_CloudStorageData_GetData(const ovrCloudStorageDataHandle obj);
OVRP_PUBLIC_FUNCTION(unsigned int) ovr_CloudStorageData_GetDataSize(const ovrCloudStorageDataHandle obj);
OVRP_PUBLIC_FUNCTION(const char *) ovr_CloudStorageData_GetKey(const ovrCloudStorageDataHandle obj);

#endif
