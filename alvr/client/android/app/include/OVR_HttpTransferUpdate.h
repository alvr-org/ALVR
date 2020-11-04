// This file was @generated with LibOVRPlatform/codegen/main. Do not modify it!

#ifndef OVR_HTTPTRANSFERUPDATE_H
#define OVR_HTTPTRANSFERUPDATE_H

#include "OVR_Platform_Defs.h"
#include "OVR_Types.h"
#include <stdbool.h>
#include <stddef.h>

typedef struct ovrHttpTransferUpdate *ovrHttpTransferUpdateHandle;

OVRP_PUBLIC_FUNCTION(const void *) ovr_HttpTransferUpdate_GetBytes(const ovrHttpTransferUpdateHandle obj);
OVRP_PUBLIC_FUNCTION(ovrRequest)   ovr_HttpTransferUpdate_GetID(const ovrHttpTransferUpdateHandle obj);
OVRP_PUBLIC_FUNCTION(size_t)       ovr_HttpTransferUpdate_GetSize(const ovrHttpTransferUpdateHandle obj);
OVRP_PUBLIC_FUNCTION(bool)         ovr_HttpTransferUpdate_IsCompleted(const ovrHttpTransferUpdateHandle obj);

#endif
