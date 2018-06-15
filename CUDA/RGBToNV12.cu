#include <stdint.h>

#include "RGBToNV12.h"

__device__ float rgb2y(uchar4 c) {
	return 0.257f * c.x + 0.504f * c.y + 0.098f * c.z + 16.0f;
}

__device__ float rgb2u(uchar4 c) {
	return -0.148f * c.x - 0.291f * c.y + 0.439f * c.z + 128.0f;
}

__device__ float rgb2v(uchar4 c) {
	return 0.439f * c.x - 0.368f * c.y - 0.071f * c.z + 128.0f;
}

texture<uchar4, cudaTextureType2D, cudaReadModeElementType> texRef;

__global__ void RGBA2NV12_kernel(uint8_t *dstImage, size_t destPitch,
	uint32_t width, uint32_t height)
{
	// Pad borders with duplicate pixels, and we multiply by 2 because we process 2 pixels per thread
	int32_t x = blockIdx.x * (blockDim.x << 1) + (threadIdx.x << 1);
	int32_t y = blockIdx.y * (blockDim.y << 1) + (threadIdx.y << 1);

	int x1 = x + 1;
	int y1 = y + 1;

	if (x1 >= width)
		return; //x = width - 1;

	if (y1 >= height)
		return; // y = height - 1;

	uchar4 c00 = tex2D(texRef, x, y);
	uchar4 c01 = tex2D(texRef, x1, y);
	uchar4 c10 = tex2D(texRef, x, y1);
	uchar4 c11 = tex2D(texRef, x1, y1);

	uint8_t y00 = (uint8_t)(rgb2y(c00) + 0.5f);
	uint8_t y01 = (uint8_t)(rgb2y(c01) + 0.5f);
	uint8_t y10 = (uint8_t)(rgb2y(c10) + 0.5f);
	uint8_t y11 = (uint8_t)(rgb2y(c11) + 0.5f);

	uint8_t u = (uint8_t)((rgb2u(c00) + rgb2u(c01) + rgb2u(c10) + rgb2u(c11)) * 0.25f + 0.5f);
	uint8_t v = (uint8_t)((rgb2v(c00) + rgb2v(c01) + rgb2v(c10) + rgb2v(c11)) * 0.25f + 0.5f);

	dstImage[destPitch * y + x] = y00;
	dstImage[destPitch * y + x1] = y01;
	dstImage[destPitch * y1 + x] = y10;
	dstImage[destPitch * y1 + x1] = y11;

	uint32_t chromaOffset = destPitch * height;
	int32_t x_chroma = x;
	int32_t y_chroma = y >> 1;

	dstImage[chromaOffset + destPitch * y_chroma + x_chroma] = u;
	dstImage[chromaOffset + destPitch * y_chroma + x_chroma + 1] = v;
}

extern "C"
cudaError_t RGBA2NV12(cudaArray *srcImage,
	uint8_t *dstImage, size_t destPitch,
	uint32_t width, uint32_t height)
{
	cudaChannelFormatDesc channelDesc = cudaCreateChannelDesc(8, 8, 8, 8, cudaChannelFormatKindUnsigned);

	// Set texture parameters
	texRef.addressMode[0] = cudaAddressModeWrap;
	texRef.addressMode[1] = cudaAddressModeWrap;
	texRef.filterMode = cudaFilterModePoint;
	texRef.normalized = false;

	cudaError_t cudaStatus = cudaBindTextureToArray(texRef, srcImage, channelDesc);
	if (cudaStatus != cudaSuccess) {
		return cudaStatus;
	}

	dim3 block(32, 16, 1);
	dim3 grid((width + (2 * block.x - 1)) / (2 * block.x), (height + (2 * block.y - 1)) / (2 * block.y), 1);

	RGBA2NV12_kernel<<<grid, block>>>(dstImage, destPitch, width, height);

	cudaThreadSynchronize();

	cudaStatus = cudaGetLastError();
	return cudaStatus;
}