#pragma once
#include "openvr_driver.h"
#include "Logger.h"
#include "ClientConnection.h"
#include "Utils.h"

class OvrDisplayComponent : public vr::IVRDisplayComponent
{
public:
	OvrDisplayComponent();
	virtual ~OvrDisplayComponent();

	virtual void GetWindowBounds(int32_t *pnX, int32_t *pnY, uint32_t *pnWidth, uint32_t *pnHeight);

	virtual bool IsDisplayOnDesktop();

	virtual bool IsDisplayRealDisplay();

	virtual void GetRecommendedRenderTargetSize(uint32_t *pnWidth, uint32_t *pnHeight);

	virtual void GetEyeOutputViewport(vr::EVREye eEye, uint32_t *pnX, uint32_t *pnY, uint32_t *pnWidth, uint32_t *pnHeight);

	virtual void GetProjectionRaw(vr::EVREye eEye, float *pfLeft, float *pfRight, float *pfTop, float *pfBottom);

	virtual vr::DistortionCoordinates_t ComputeDistortion(vr::EVREye eEye, float fU, float fV);
};