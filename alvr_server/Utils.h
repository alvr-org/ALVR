#pragma once

#include <WinSock2.h>
#include <WinInet.h>
#include <WS2tcpip.h>
#include <Windows.h>
#include <stdint.h>
#include <string>
#include <vector>
#include <locale>
#include <codecvt>
#include <d3d11.h>
#define _USE_MATH_DEFINES
#include <math.h>
#include <VersionHelpers.h>

#include "openvr_driver.h"
#include "packet_types.h"

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

inline std::string AddrToStr(const sockaddr_in *addr) {
	char buf[1000];
	inet_ntop(AF_INET, &addr->sin_addr, buf, sizeof(buf));
	return buf;
}

inline std::string AddrPortToStr(const sockaddr_in *addr) {
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

inline void HmdMatrix_QuatToMat(double w, double x, double y, double z, vr::HmdMatrix34_t *pMatrix)
{
	pMatrix->m[0][0] = (float)(1.0f - 2.0f * y * y - 2.0f * z * z);
	pMatrix->m[0][1] = (float)(2.0f * x * y - 2.0f * z * w);
	pMatrix->m[0][2] = (float)(2.0f * x * z + 2.0f * y * w);
	pMatrix->m[0][3] = (float)(0.0f);
	pMatrix->m[1][0] = (float)(2.0f * x * y + 2.0f * z * w);
	pMatrix->m[1][1] = (float)(1.0f - 2.0f * x * x - 2.0f * z * z);
	pMatrix->m[1][2] = (float)(2.0f * y * z - 2.0f * x * w);
	pMatrix->m[1][3] = (float)(0.0f);
	pMatrix->m[2][0] = (float)(2.0f * x * z - 2.0f * y * w);
	pMatrix->m[2][1] = (float)(2.0f * y * z + 2.0f * x * w);
	pMatrix->m[2][2] = (float)(1.0f - 2.0f * x * x - 2.0f * y * y);
	pMatrix->m[2][3] = (float)(0.0f);
}

inline double PitchFromQuaternion(double x, double y, double z, double w) {
	// (xx, yy, zz) = rotate (0, 0, -1) by quaternion
	double xx = -2 * y * w
		- 2 * x * y;
	double zz = -w * w
		+ x * x
		+ y * y
		- z * z;
	return atan2(xx, zz);
}

inline double PitchFromQuaternion(const TrackingQuat &q) {
	return PitchFromQuaternion(q.x, q.y, q.z, q.w);
}

inline vr::HmdQuaternion_t MultiplyPitchQuaternion(double pitch, double x, double y, double z, double w) {
	// Multiply quaternion (x=0, y=1, z=0, theta=pitch)

	vr::HmdQuaternion_t a;
	a.w = cos(pitch * 0.5);
	a.x = 0;
	a.y = sin(pitch * 0.5);
	a.z = 0;

	vr::HmdQuaternion_t dest;
	dest.x = a.w * x + a.y * z;
	dest.y = a.y * w + a.w * y;
	dest.z = a.w * z - a.y * x;
	dest.w = a.w * w - a.y * y;
	return dest;
}

inline TrackingVector3 RotateVectorQuaternion_add(const TrackingVector3& v1, const TrackingVector3& v2) {
	TrackingVector3 dest;
	dest.x = v1.x + v2.x;
	dest.y = v1.y + v2.y;
	dest.z = v1.z + v2.z;
	return dest;
}

inline TrackingVector3 RotateVectorQuaternion_scale(double scale, const TrackingVector3& v1) {
	TrackingVector3 dest;
	dest.x = (float)(scale * v1.x);
	dest.y = (float)(scale * v1.y);
	dest.z = (float)(scale * v1.z);
	return dest;
}

inline double RotateVectorQuaternion_dot(const TrackingVector3& v1, const TrackingVector3& v2) {
	return v1.x * v2.x + v1.y * v2.y + v1.z * v2.z;
}

inline TrackingVector3 RotateVectorQuaternion_cross(const TrackingVector3& v1, const TrackingVector3& v2) {
	TrackingVector3 dest;
	dest.x = v1.y * v2.z - v1.z * v2.y;
	dest.y = v1.z * v2.x - v1.x * v2.z;
	dest.z = v1.y * v2.y - v1.y * v2.y;
	return dest;
}

inline TrackingVector3 RotateVectorQuaternion(const TrackingVector3& v, double pitch)
{
	TrackingVector3 dest;
	/*

	TrackingQuat q;
	q.w = cos(pitch * 0.5);
	q.x = 0;
	q.y = sin(pitch * 0.5);
	q.z = 0;

	// Extract the vector part of the quaternion
	TrackingVector3 u = { q.x, q.y, q.z };

	// Extract the scalar part of the quaternion
	float s = q.w;

	TrackingVector3 c = RotateVectorQuaternion_cross(u, v);
	// Do the math
	dest = RotateVectorQuaternion_scale(2.0f * RotateVectorQuaternion_dot(u, v), u);
	dest = RotateVectorQuaternion_add(dest, RotateVectorQuaternion_scale((s*s - RotateVectorQuaternion_dot(u, u)), v));
	dest = RotateVectorQuaternion_add(dest, RotateVectorQuaternion_scale(2.0f * s, c));*/

	dest.y = v.y;
	dest.x = (float)(v.x * cos(pitch) - v.z * sin(pitch));
	dest.z = (float)(v.x * sin(pitch) + v.z * cos(pitch));
	return dest;
}

// Use NV12 texture on Windows 7
inline bool ShouldUseNV12Texture() {
	return IsWindows8OrGreater() == FALSE;
}

inline std::wstring ToWstring(const std::string &src) {
	// TODO: src is really UTF-8?
	std::wstring_convert<std::codecvt_utf8_utf16<wchar_t>> converter;
	return converter.from_bytes(src);
}

inline std::string ToUTF8(const std::wstring &src) {
	std::wstring_convert<std::codecvt_utf8_utf16<wchar_t>> converter;
	return converter.to_bytes(src);
}