#pragma once

#include <WinSock2.h>
#include <WinInet.h>
#include <WS2tcpip.h>
#include <Windows.h>
#include <stdint.h>
#include <string>
#include <d3d11.h>

#include "openvr_driver.h"

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