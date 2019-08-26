#include "FFR.h"

#include "Settings.h"
#include "resource.h"
#include "Utils.h"

using Microsoft::WRL::ComPtr;
using namespace d3d_render_utils;

namespace {

	const float DEG_TO_RAD = (float)M_PI / 180;

#define INVERSE_DISTORTION_FN(a) atan(a);
	const float INVERSE_DISTORTION_DERIVATIVE_IN_0 = 1; // d(atan(0))/dx = 1

	float CalcBoundStart(float focusPos, float fovScale) {
		return INVERSE_DISTORTION_FN(-focusPos * fovScale);
	}

	float CalcBoundEnd(float focusPos, float fovScale) {
		return INVERSE_DISTORTION_FN((1.f - focusPos) * fovScale);
	}

	float CalcDistortedDimension(float focusPos, float fovScale) {
		float boundEnd = CalcBoundEnd(focusPos, fovScale);
		float boundStart = CalcBoundStart(focusPos, fovScale);
		return boundEnd - boundStart;
	}

	float CalcOptimalDimension(float scale, float distortedDim, float originalDim) {
		float inverseDistortionDerivative = INVERSE_DISTORTION_DERIVATIVE_IN_0 * scale;
		float gradientOnFocus = inverseDistortionDerivative / distortedDim;
		return originalDim / gradientOnFocus;
	}
}

FFR::FoveationVars FFR::CalculateFoveationVars() {
	auto leftEye = Settings::Instance().m_eyeFov[0];

	// left and right side screen plane width with unit focal
	float leftHalfWidth = tan(leftEye.left * DEG_TO_RAD);
	float rightHalfWidth = tan(leftEye.right * DEG_TO_RAD);
	// foveated center X assuming screen plane with unit width
	float focusPositionX = leftHalfWidth / (leftHalfWidth + rightHalfWidth);

	// NB: swapping top/bottom fov
	float topHalfHeight = tan(leftEye.bottom * DEG_TO_RAD);
	float bottomHalfHeight = tan(leftEye.top * DEG_TO_RAD);
	float focusPositionY = topHalfHeight / (topHalfHeight + bottomHalfHeight);

	float targetEyeWidth = (float)Settings::Instance().m_renderWidth / 2;
	float targetEyeHeight = (float)Settings::Instance().m_renderHeight;

	//calculate foveation scale such as the "area" of the foveation region remains equal to (mFoveationStrengthMean)^2
	// solve for {foveationScaleX, foveationScaleY}:
	// /{ foveationScaleX * foveationScaleY = (mFoveationStrengthMean)^2
	// \{ foveationScaleX / foveationScaleY = 1 / mFoveationShapeRatio
	// then foveationScaleX := foveationScaleX / (targetEyeWidth / targetEyeHeight) to compensate for non square frame.
	float strengthMean = Settings::Instance().m_foveationStrengthMean;
	float strengthRatio = Settings::Instance().m_foveationShapeRatio;
	float scaleCoeff = strengthMean * sqrt(strengthRatio);
	float foveationScaleX = scaleCoeff / strengthRatio / (targetEyeWidth / targetEyeHeight);
	float foveationScaleY = scaleCoeff;

	float boundStartX = CalcBoundStart(focusPositionX, foveationScaleX);
	float boundStartY = CalcBoundStart(focusPositionY, foveationScaleY);

	float distortedWidth = CalcDistortedDimension(focusPositionX, foveationScaleX);
	float distortedHeight = CalcDistortedDimension(focusPositionY, foveationScaleY);

	float optimizedEyeWidth = CalcOptimalDimension(foveationScaleX, distortedWidth, targetEyeWidth);
	float optimizedEyeHeight = CalcOptimalDimension(foveationScaleY, distortedHeight, targetEyeHeight);

	// round the frame dimensions to a number of pixel multiple of 16 for the encoder
	uint32_t optimizedEyeWidthAligned = uint32_t(optimizedEyeWidth / 16) * 16;
	uint32_t optimizedEyeHeightAligned = uint32_t(optimizedEyeHeight / 16) * 16;

	return { (uint32_t)targetEyeWidth, (uint32_t)targetEyeHeight, optimizedEyeWidthAligned, optimizedEyeHeightAligned,
		focusPositionX, focusPositionY, foveationScaleX, foveationScaleY,
		boundStartX, boundStartY, distortedWidth, distortedHeight };
}

void FFR::GetOptimizedResolution(uint32_t *width, uint32_t *height) {
	if (mFoveationVars.optimizedEyeWidth == 0)
		mFoveationVars = CalculateFoveationVars();
	*width = mFoveationVars.optimizedEyeWidth * 2;
	*height = mFoveationVars.optimizedEyeHeight;
}

FFR::FFR(ID3D11Device *device) : mDevice(device), mHorizontalBlurPipeline(device), mDistortionPipeline(device) {}

void FFR::Initialize(ID3D11Texture2D *compositionTexture) {
	mFoveationVars = CalculateFoveationVars();
	ComPtr<ID3D11Buffer> foveatedRenderingBuffer = CreateBuffer(mDevice.Get(), mFoveationVars);

	ComPtr<ID3D11Texture2D> horizontalBlurredTexture = CreateTexture(mDevice.Get(),
		mFoveationVars.targetEyeWidth * 2, mFoveationVars.targetEyeHeight,
		DXGI_FORMAT_R8G8B8A8_UNORM_SRGB);

	mDistortedTexture = CreateTexture(mDevice.Get(), mFoveationVars.optimizedEyeWidth * 2,
		mFoveationVars.optimizedEyeHeight, DXGI_FORMAT_R8G8B8A8_UNORM_SRGB);

	std::vector<uint8_t> quadShaderCSO;
	if (!ReadBinaryResource(quadShaderCSO, IDR_QUAD_SHADER)) {
		throw MakeException(L"Failed to load resource for IDR_QUAD_SHADER.");
	}
	mQuadVertexShader = CreateVertexShader(mDevice.Get(), quadShaderCSO);

	std::vector<uint8_t> horizontalBlurShaderCSO;
	if (!ReadBinaryResource(horizontalBlurShaderCSO, IDR_HORIZ_BLUR_SHADER)) {
		throw MakeException(L"Failed to load resource for IDR_HORIZ_BLUR_SHADER.");
	}

	std::vector<uint8_t> distortionShaderCSO;
	if (!ReadBinaryResource(distortionShaderCSO, IDR_DISTORTION_SHADER)) {
		throw MakeException(L"Failed to load resource for IDR_DISTORTION_SHADER.");
	}

	mHorizontalBlurPipeline.Initialize({ compositionTexture }, mQuadVertexShader.Get(),
		horizontalBlurShaderCSO, horizontalBlurredTexture.Get(), foveatedRenderingBuffer.Get());

	mDistortionPipeline.Initialize({ horizontalBlurredTexture.Get() }, mQuadVertexShader.Get(),
		distortionShaderCSO, mDistortedTexture.Get(), foveatedRenderingBuffer.Get());
}

void FFR::Render() {
	mHorizontalBlurPipeline.Render();
	mDistortionPipeline.Render();
}

ID3D11Texture2D *FFR::GetOutputTexture() {
	return mDistortedTexture.Get();
}