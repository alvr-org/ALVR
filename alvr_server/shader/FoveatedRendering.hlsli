// https://www.shadertoy.com/view/3l2GRR

// MIT License
//
// Copyright (c) 2019 Riccardo Zaglia
//
// Permission is hereby granted, free of charge, to any person obtaining a copy
// of this software and associated documentation files (the "Software"), to deal
// in the Software without restriction, including without limitation the rights
// to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
// copies of the Software, and to permit persons to whom the Software is
// furnished to do so, subject to the following conditions:
//
// The above copyright notice and this permission notice shall be included in all
// copies or substantial portions of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
// IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
// AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
// LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
// OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
// SOFTWARE.

static const float BLUR_STRENGTH = 0.1;// 50000000;
static const int KERNEL_HALF_SIZE = 4;

cbuffer FoveationVars {
	uint2 targetResolution;
	uint2 optimizedResolution;
	float2 focusPosition;
	float2 foveationScale;
	float2 boundStart;
	float2 distortedSize;
};

float UnscaledGaussian(float a, float stddev) {
	return exp(-a * a / (2. * stddev * stddev));
}

//Choose one distortion function:

// ARCTANGENT: good for fixed foveated rendering
static const float EPS = 0.000001;
#define DISTORTION_FN(a)           tan(a)
#define INV_DIST_DERIVATIVE(a)     atanDerivative(a)
float atanDerivative(float a) {
	return 1. / (a * a + 1.);
}

// HYPERBOLIC TANGENT: good compression but the periphery is too squished
//static const float EPS = 0.000001;
//#define DISTORTION_FN(a)           atanh(a)
//#define INV_DIST_DERIVATIVE(a)     tanhDerivative(a)
//float tanhDerivative(float a) {
//	float tanh_a = tanh(a);
//	return 1. - tanh_a * tanh_a;
//}

// POW: good for tracked foveated rendering
//static float POWER = 4. * sqrt(foveationScale.x * foveationScale.y);
//static const float EPS = 0.01;
//#define DISTORTION_FN(a)           pow(a, POWER)
//#define INV_DIST_DERIVATIVE(a)     (pow(a, 1. / POWER - 1.) / POWER)

// Other functions for distortion:
// https://en.wikipedia.org/wiki/Sigmoid_function


float2 RadialDistortion(float2 xy) {
	float radius = length(xy);
	return (DISTORTION_FN(radius) * xy / radius) / foveationScale;
}

// Inverse radial distortion derivative wrt length(xy)
float2 InverseRadialDistortionDerivative(float2 xy) {
	float2 scaledXY = xy * foveationScale;
	float scaledRadius = length(scaledXY);
	return (INV_DIST_DERIVATIVE(scaledRadius) * foveationScale) * scaledXY / scaledRadius;
}

float2 Distort(float2 uv) {
	return RadialDistortion(uv * distortedSize + boundStart) + focusPosition;
}

float2 UndistortRadialDerivative(float2 uv) {
	return InverseRadialDistortionDerivative(uv - focusPosition) / distortedSize;
}

float GetFilteringWeight(float2 uv, float targetDimension, float distortedDimension) {
	float radialExpansion = length(UndistortRadialDerivative(uv));
	float resScale = targetDimension / distortedDimension;
	float contraction = 1. / (radialExpansion * resScale);

	float modifiedContraction = contraction - 1. / contraction; // -> ?

	return max(modifiedContraction, EPS);
}

float2 TextureToEyeUV(float2 textureUV, bool isRightEye) {
	// flip distortion horizontally for right eye
	// left: x * 2; right: (1 - x) * 2
	return float2((textureUV.x + float(isRightEye) * (1. - 2. * textureUV.x)) * 2., textureUV.y);
}

float2 EyeToTextureUV(float2 eyeUV, bool isRightEye) {
	// saturate is used to avoid color bleeding between the two sides of the texture or with the black border when filtering
	float2 clampedUV = saturate(eyeUV);
	// left: x / 2; right 1 - (x / 2)
	return float2(clampedUV.x / 2. + float(isRightEye) * (1. - clampedUV.x), clampedUV.y);
}