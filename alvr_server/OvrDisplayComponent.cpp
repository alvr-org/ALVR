#include "OvrDisplayComponent.h"

OvrDisplayComponent::OvrDisplayComponent() {};
OvrDisplayComponent::~OvrDisplayComponent() {};

void OvrDisplayComponent::GetWindowBounds(int32_t *pnX, int32_t *pnY, uint32_t *pnWidth, uint32_t *pnHeight)
{
	Log(L"GetWindowBounds %dx%d - %dx%d", 0, 0, Settings::Instance().m_renderWidth, Settings::Instance().m_renderHeight);
	*pnX = 0;
	*pnY = 0;
	*pnWidth = Settings::Instance().m_renderWidth;
	*pnHeight = Settings::Instance().m_renderHeight;
}

 bool OvrDisplayComponent::IsDisplayOnDesktop()
{
	return false;
}

 bool OvrDisplayComponent::IsDisplayRealDisplay()
{
	return false;
}

 void OvrDisplayComponent::GetRecommendedRenderTargetSize(uint32_t *pnWidth, uint32_t *pnHeight)
{
	*pnWidth = Settings::Instance().m_recommendedTargetWidth / 2;
	*pnHeight = Settings::Instance().m_recommendedTargetHeight;
	Log(L"GetRecommendedRenderTargetSize %dx%d", *pnWidth, *pnHeight);
}

void OvrDisplayComponent::GetEyeOutputViewport(vr::EVREye eEye, uint32_t *pnX, uint32_t *pnY, uint32_t *pnWidth, uint32_t *pnHeight)
{
	*pnY = 0;
	*pnWidth = Settings::Instance().m_renderWidth / 2;
	*pnHeight = Settings::Instance().m_renderHeight;

	if (eEye == vr::Eye_Left)
	{
		*pnX = 0;
	}
	else
	{
		*pnX = Settings::Instance().m_renderWidth / 2;
	}
	Log(L"GetEyeOutputViewport Eye=%d %dx%d %dx%d", eEye, *pnX, *pnY, *pnWidth, *pnHeight);
}

void OvrDisplayComponent::GetProjectionRaw(vr::EVREye eEye, float *pfLeft, float *pfRight, float *pfTop, float *pfBottom)
{
	auto eyeFov = Settings::Instance().m_eyeFov[eEye];
	*pfLeft = -tanf(static_cast<float>(eyeFov.left / 180.0 * M_PI));
	*pfRight = tanf(static_cast<float>(eyeFov.right / 180.0 * M_PI));
	*pfTop = -tanf(static_cast<float>(eyeFov.top / 180.0 * M_PI));
	*pfBottom = tanf(static_cast<float>(eyeFov.bottom / 180.0 * M_PI));

	Log(L"GetProjectionRaw Eye=%d (l,r,t,b)=(%f,%f,%f,%f)", eEye, eyeFov.left, eyeFov.right, eyeFov.top, eyeFov.bottom);
}

vr::DistortionCoordinates_t OvrDisplayComponent::ComputeDistortion(vr::EVREye eEye, float fU, float fV) 
{
	vr::DistortionCoordinates_t coordinates;
	coordinates.rfBlue[0] = fU;
	coordinates.rfBlue[1] = fV;
	coordinates.rfGreen[0] = fU;
	coordinates.rfGreen[1] = fV;
	coordinates.rfRed[0] = fU;
	coordinates.rfRed[1] = fV;
	Log(L"ComputeDistortion %f,%f", fU, fV);
	return coordinates;
}