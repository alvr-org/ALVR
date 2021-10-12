#include "FFR.h"

#include "alvr_server/Settings.h"
#include "alvr_server/Utils.h"
#include "alvr_server/bindings.h"

using Microsoft::WRL::ComPtr;
using namespace d3d_render_utils;

namespace {

	struct FoveationVars {
		uint32_t targetEyeWidth;
		uint32_t targetEyeHeight;
		uint32_t optimizedEyeWidth;
		uint32_t optimizedEyeHeight;

		float eyeWidthRatio;
		float eyeHeightRatio;

		float centerSizeX;
		float centerSizeY;
		float centerShiftX;
		float centerShiftY;
		float edgeRatioX;
		float edgeRatioY;
	};

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

	float CalcOptimalDimensionForWarp(float scale, float distortedDim, float originalDim) {
		float inverseDistortionDerivative = INVERSE_DISTORTION_DERIVATIVE_IN_0 * scale;
		float gradientOnFocus = inverseDistortionDerivative / distortedDim;
		return originalDim / gradientOnFocus;
	}

	float Align4Normalized(float scale, float originalDim) {
		return float(int(scale * originalDim / 4.f) * 4) / originalDim;
	}

	float CalcOptimalDimensionForSlicing(float scale, float originalDim) {
		return (1.f + 3.f * scale) / 4.f * originalDim + 6;
	}

	FoveationVars CalculateFoveationVars() {
		float targetEyeWidth = (float)Settings::Instance().m_renderWidth / 2;
		float targetEyeHeight = (float)Settings::Instance().m_renderHeight;

		float centerSizeX = (float)Settings::Instance().m_foveationCenterSizeX;
		float centerSizeY = (float)Settings::Instance().m_foveationCenterSizeY;
		float centerShiftX = (float)Settings::Instance().m_foveationCenterShiftX;
		float centerShiftY = (float)Settings::Instance().m_foveationCenterShiftY;
		float edgeRatioX = (float)Settings::Instance().m_foveationEdgeRatioX;
		float edgeRatioY = (float)Settings::Instance().m_foveationEdgeRatioY;

		float edgeSizeX = targetEyeWidth-centerSizeX*targetEyeWidth;
		float edgeSizeY = targetEyeHeight-centerSizeY*targetEyeHeight;

		float centerSizeXAligned = 1.-ceil(edgeSizeX/(edgeRatioX*2.))*(edgeRatioX*2.)/targetEyeWidth;
		float centerSizeYAligned = 1.-ceil(edgeSizeY/(edgeRatioY*2.))*(edgeRatioY*2.)/targetEyeHeight;

		float edgeSizeXAligned = targetEyeWidth-centerSizeXAligned*targetEyeWidth;
		float edgeSizeYAligned = targetEyeHeight-centerSizeYAligned*targetEyeHeight;

		float centerShiftXAligned = ceil(centerShiftX*edgeSizeXAligned/(edgeRatioX*2.))*(edgeRatioX*2.)/edgeSizeXAligned;
		float centerShiftYAligned = ceil(centerShiftY*edgeSizeYAligned/(edgeRatioY*2.))*(edgeRatioY*2.)/edgeSizeYAligned;

		float foveationScaleX = (centerSizeXAligned+(1.-centerSizeXAligned)/edgeRatioX);
		float foveationScaleY = (centerSizeYAligned+(1.-centerSizeYAligned)/edgeRatioY);

		float optimizedEyeWidth = foveationScaleX*targetEyeWidth;
		float optimizedEyeHeight = foveationScaleY*targetEyeHeight;

		// round the frame dimensions to a number of pixel multiple of 32 for the encoder
		auto optimizedEyeWidthAligned = (uint32_t)ceil(optimizedEyeWidth / 32.f) * 32;
		auto optimizedEyeHeightAligned = (uint32_t)ceil(optimizedEyeHeight / 32.f) * 32;

		float eyeWidthRatioAligned = optimizedEyeWidth/optimizedEyeWidthAligned;
		float eyeHeightRatioAligned = optimizedEyeHeight/optimizedEyeHeightAligned;

		return { (uint32_t)targetEyeWidth, (uint32_t)targetEyeHeight, optimizedEyeWidthAligned, optimizedEyeHeightAligned,
			eyeWidthRatioAligned, eyeHeightRatioAligned,
			centerSizeXAligned, centerSizeYAligned, centerShiftXAligned, centerShiftYAligned, edgeRatioX, edgeRatioY };
	}
}


void FFR::GetOptimizedResolution(uint32_t* width, uint32_t* height) {
	auto fovVars = CalculateFoveationVars();
	*width = fovVars.optimizedEyeWidth * 2;
	*height = fovVars.optimizedEyeHeight;
}

FFR::FFR(ID3D11Device* device) : mDevice(device) {}

void FFR::Initialize(ID3D11Texture2D* compositionTexture) {
	auto fovVars = CalculateFoveationVars();
	ComPtr<ID3D11Buffer> foveatedRenderingBuffer = CreateBuffer(mDevice.Get(), fovVars);

	std::vector<uint8_t> quadShaderCSO(QUAD_SHADER_CSO_PTR, QUAD_SHADER_CSO_PTR + QUAD_SHADER_CSO_LEN);
	mQuadVertexShader = CreateVertexShader(mDevice.Get(), quadShaderCSO);

	mOptimizedTexture = CreateTexture(mDevice.Get(), fovVars.optimizedEyeWidth * 2,
		fovVars.optimizedEyeHeight, DXGI_FORMAT_R8G8B8A8_UNORM_SRGB);

	if (Settings::Instance().m_enableFoveatedRendering) {
		std::vector<uint8_t> compressSlicesShaderCSO(COMPRESS_SLICES_CSO_PTR, COMPRESS_SLICES_CSO_PTR + COMPRESS_SLICES_CSO_LEN);
		auto compressSlicesPipeline = RenderPipeline(mDevice.Get());
		compressSlicesPipeline.Initialize({ compositionTexture }, mQuadVertexShader.Get(),
			compressSlicesShaderCSO, mOptimizedTexture.Get(), foveatedRenderingBuffer.Get());

		mPipelines.push_back(compressSlicesPipeline);
	} else {
		mOptimizedTexture = compositionTexture;
	}
}

void FFR::Render() {
	for (auto &p : mPipelines) {
		p.Render();
	}
}

ID3D11Texture2D* FFR::GetOutputTexture() {
	return mOptimizedTexture.Get();
}