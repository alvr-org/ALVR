#pragma once

#include <cuda.h>

extern "C"
cudaError_t RGBA2NV12(cudaArray *srcImage,
	uint8_t *dstImage, size_t destPitch,
	uint32_t width, uint32_t height);