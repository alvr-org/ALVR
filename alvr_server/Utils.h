#pragma once

#include <WinSock2.h>
#include <WinInet.h>
#include <WS2tcpip.h>
#include <Windows.h>
#include <stdint.h>
#include <string>
#include <vector>
#include <d3d11.h>

#include "openvr_driver.h"

extern HINSTANCE g_hInstance;

// Get elapsed time in us from Unix Epoch
inline uint64_t GetTimestampUs() {
	FILETIME ft;
	GetSystemTimeAsFileTime(&ft);

	uint64_t Current = (((uint64_t)ft.dwHighDateTime) << 32) | ft.dwLowDateTime;
	// Convert to Unix Epoch
	Current -= 116444736000000000LL;
	Current /= 10;

	return Current;
}

inline std::string DumpMatrix(const float *m) {
	char buf[200];
	snprintf(buf, sizeof(buf),
		"%f, %f, %f, %f,\n"
		"%f, %f, %f, %f,\n"
		"%f, %f, %f, %f,\n"
		"%f, %f, %f, %f,\n"
		, m[0], m[1], m[2], m[3]
		, m[4], m[5], m[6], m[7]
		, m[8], m[9], m[10], m[11]
		, m[12], m[13], m[14], m[15]);
	return std::string(buf);
}

inline std::string GetDxErrorStr(HRESULT hr) {
	char *s = NULL;
	std::string ret;
	FormatMessageA(FORMAT_MESSAGE_ALLOCATE_BUFFER | FORMAT_MESSAGE_FROM_SYSTEM | FORMAT_MESSAGE_IGNORE_INSERTS,
		NULL, hr,
		MAKELANGID(LANG_NEUTRAL, SUBLANG_DEFAULT),
		(LPSTR)&s, 0, NULL);
	ret = s;
	LocalFree(s);

	if (ret.size() >= 1) {
		if (ret[ret.size() - 1] == '\n') {
			ret.erase(ret.size() - 1, 1);
		}
		if (ret[ret.size() - 1] == '\r') {
			ret.erase(ret.size() - 1, 1);
		}
	}
	return ret;
}


inline void DrawDigitPixels(D3D11_MAPPED_SUBRESOURCE &mapped, int x, int y, int digit) {
	static const char map[][15] = {
		{ 1, 1, 1,
		1, 0, 1,
		1, 0, 1,
		1, 0, 1,
		1, 1, 1 },
	{ 0, 1, 0,
	1, 1, 0,
	0, 1, 0,
	0, 1, 0,
	1, 1, 1 },
	{ 1, 1, 0,
	1, 0, 1,
	0, 1, 0,
	1, 0, 0,
	1, 1, 1 },
	{ 1, 1, 1,
	0, 0, 1,
	0, 1, 1,
	0, 0, 1,
	1, 1, 1 },
	{ 1, 0, 1,
	1, 0, 1,
	1, 1, 1,
	0, 0, 1,
	0, 0, 1 },
	{ 1, 1, 1,
	1, 0, 0,
	1, 1, 1,
	0, 0, 1,
	1, 1, 1 },
	{ 1, 1, 0,
	1, 0, 0,
	1, 1, 1,
	1, 0, 1,
	1, 1, 1 },
	{ 1, 1, 1,
	0, 0, 1,
	0, 1, 0,
	0, 1, 0,
	0, 1, 0 },
	{ 1, 1, 1,
	1, 0, 1,
	1, 1, 1,
	1, 0, 1,
	1, 1, 1 },
	{ 1, 1, 1,
	1, 0, 1,
	1, 1, 1,
	0, 0, 1,
	0, 0, 1 }
	};
	if (digit < 0 || 9 < digit) {
		digit = 0;
	}
	uint8_t *p = (uint8_t *)mapped.pData;

	for (int i = 0; i < 5 * 2; i++) {
		for (int j = 0; j < 3 * 2; j++) {
			if (map[digit][i / 2 * 3 + j / 2]) {
				p[(y + i) * mapped.RowPitch + (x + j) * 4 + 0] = 0xff;
				p[(y + i) * mapped.RowPitch + (x + j) * 4 + 1] = 0xff;
				p[(y + i) * mapped.RowPitch + (x + j) * 4 + 2] = 0xff;
				p[(y + i) * mapped.RowPitch + (x + j) * 4 + 3] = 0xff;
			}

		}
	}

}


inline std::string AddrToStr(sockaddr_in *addr) {
	char buf[1000];
	inet_ntop(AF_INET, &addr->sin_addr, buf, sizeof(buf));
	return buf;
}

inline std::string AddrPortToStr(sockaddr_in *addr) {
	char buf[1000];
	char buf2[1000];
	inet_ntop(AF_INET, &addr->sin_addr, buf, sizeof(buf));
	snprintf(buf2, sizeof(buf2), "%s:%d", buf, htons(addr->sin_port));
	return buf2;
}

inline bool ReadBinaryResource(std::vector<char> &buffer, int resource) {
	HRSRC hResource = FindResource(g_hInstance, MAKEINTRESOURCE(resource), RT_RCDATA);
	if (hResource == NULL) {
		return false;
	}
	HGLOBAL hResData = LoadResource(g_hInstance, hResource);
	if (hResData == NULL) {
		return false;
	}
	void *data = LockResource(hResData);
	int dataSize = SizeofResource(g_hInstance, hResource);

	buffer.resize(dataSize);
	memcpy(&buffer[0], data, dataSize);

	return true;
}

inline std::string GetNextToken(std::string &str, const char *splitter) {
	auto pos = str.find(splitter);
	if (pos != std::string::npos) {
		std::string ret = str.substr(0, pos);
		str = str.substr(pos + strlen(splitter));
		return ret;
	}
	std::string ret = str;
	str = "";
	return ret;
}



inline vr::HmdQuaternion_t HmdQuaternion_Init(double w, double x, double y, double z)
{
	vr::HmdQuaternion_t quat;
	quat.w = w;
	quat.x = x;
	quat.y = y;
	quat.z = z;
	return quat;
}

inline void HmdMatrix_SetIdentity(vr::HmdMatrix34_t *pMatrix)
{
	pMatrix->m[0][0] = 1.f;
	pMatrix->m[0][1] = 0.f;
	pMatrix->m[0][2] = 0.f;
	pMatrix->m[0][3] = 0.f;
	pMatrix->m[1][0] = 0.f;
	pMatrix->m[1][1] = 1.f;
	pMatrix->m[1][2] = 0.f;
	pMatrix->m[1][3] = 0.f;
	pMatrix->m[2][0] = 0.f;
	pMatrix->m[2][1] = 0.f;
	pMatrix->m[2][2] = 1.f;
	pMatrix->m[2][3] = 0.f;
}

inline void HmdMatrix_QuatToMat(float w, float x, float y, float z, vr::HmdMatrix34_t *pMatrix)
{
	pMatrix->m[0][0] = 1.0f - 2.0f * y * y - 2.0f * z * z;
	pMatrix->m[0][1] = 2.0f * x * y - 2.0f * z * w;
	pMatrix->m[0][2] = 2.0f * x * z + 2.0f * y * w;
	pMatrix->m[0][3] = 0.0f;
	pMatrix->m[1][0] = 2.0f * x * y + 2.0f * z * w;
	pMatrix->m[1][1] = 1.0f - 2.0f * x * x - 2.0f * z * z;
	pMatrix->m[1][2] = 2.0f * y * z - 2.0f * x * w;
	pMatrix->m[1][3] = 0.0f;
	pMatrix->m[2][0] = 2.0f * x * z - 2.0f * y * w;
	pMatrix->m[2][1] = 2.0f * y * z + 2.0f * x * w;
	pMatrix->m[2][2] = 1.0f - 2.0f * x * x - 2.0f * y * y;
	pMatrix->m[2][3] = 0.f;
}
