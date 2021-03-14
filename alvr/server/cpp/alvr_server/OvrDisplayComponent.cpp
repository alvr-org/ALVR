#include "OvrDisplayComponent.h"

#include <cmath>
#ifndef M_PI //On Windows not defined by include
    #define M_PI 3.14159265358979323846
#endif

#include "Settings.h"
#include "Logger.h"

OvrDisplayComponent::OvrDisplayComponent() {};
OvrDisplayComponent::~OvrDisplayComponent() {};

void OvrDisplayComponent::GetWindowBounds(int32_t *pnX, int32_t *pnY, uint32_t *pnWidth, uint32_t *pnHeight)
{
	Debug("GetWindowBounds %dx%d - %dx%d\n", 0, 0, Settings::Instance().m_renderWidth, Settings::Instance().m_renderHeight);
	*pnX = 0;
	*pnY = 0;
	*pnWidth = Settings::Instance().m_renderWidth;
	*pnHeight = Settings::Instance().m_renderHeight;
}

 bool OvrDisplayComponent::IsDisplayOnDesktop()
{
#ifdef _WIN32
	return false;
#else
	return false;
#endif
}

 bool OvrDisplayComponent::IsDisplayRealDisplay()
{
#ifdef _WIN32
	return false;
#else
	return true;
#endif
}

 void OvrDisplayComponent::GetRecommendedRenderTargetSize(uint32_t *pnWidth, uint32_t *pnHeight)
{
	*pnWidth = Settings::Instance().m_recommendedTargetWidth / 2;
	*pnHeight = Settings::Instance().m_recommendedTargetHeight;
	Debug("GetRecommendedRenderTargetSize %dx%d\n", *pnWidth, *pnHeight);
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
	Debug("GetEyeOutputViewport Eye=%d %dx%d %dx%d\n", eEye, *pnX, *pnY, *pnWidth, *pnHeight);
}

void OvrDisplayComponent::GetProjectionRaw(vr::EVREye eEye, float *pfLeft, float *pfRight, float *pfTop, float *pfBottom)
{
	auto eyeFov = Settings::Instance().m_eyeFov[eEye];
	*pfLeft = -tanf(static_cast<float>(eyeFov.left / 180.0 * M_PI));
	*pfRight = tanf(static_cast<float>(eyeFov.right / 180.0 * M_PI));
	*pfTop = -tanf(static_cast<float>(eyeFov.top / 180.0 * M_PI));
	*pfBottom = tanf(static_cast<float>(eyeFov.bottom / 180.0 * M_PI));

	Debug("GetProjectionRaw Eye=%d (l,r,t,b)=(%f,%f,%f,%f)\n", eEye, eyeFov.left, eyeFov.right, eyeFov.top, eyeFov.bottom);
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
	Debug("ComputeDistortion %f,%f\n", fU, fV);
	return coordinates;
}
