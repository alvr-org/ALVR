// This file was @generated with LibOVRPlatform/codegen/main. Do not modify it!

#ifndef OVR_NETSYNCSETSESSIONPROPERTYRESULT_H
#define OVR_NETSYNCSETSESSIONPROPERTYRESULT_H

#include "OVR_Platform_Defs.h"
#include "OVR_NetSyncSession.h"

typedef struct ovrNetSyncSetSessionPropertyResult *ovrNetSyncSetSessionPropertyResultHandle;

/// Which session the operation was modifying
OVRP_PUBLIC_FUNCTION(ovrNetSyncSessionHandle) ovr_NetSyncSetSessionPropertyResult_GetSession(const ovrNetSyncSetSessionPropertyResultHandle obj);


#endif
