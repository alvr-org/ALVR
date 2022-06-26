#pragma once

#include <chrono>
#ifdef _WIN32
	#pragma warning(disable:4005)
	#include <WinSock2.h>
	#pragma warning(default:4005)
	#include <WinInet.h>
	#include <WS2tcpip.h>
	#include <Windows.h>
	#include <delayimp.h>
	#include <stdint.h>
	#include <string>
	#include <vector>
	#include <d3d11.h>
	#define _USE_MATH_DEFINES
	#include <VersionHelpers.h>
#else
	#include <netinet/in.h>
	#include <arpa/inet.h>
	#include <string.h>
#endif

#include <math.h>

#include "openvr_driver.h"
#include "ALVR-common/packet_types.h"

const float DEG_TO_RAD = (float)(M_PI / 180.);

// Get elapsed time in us from Unix Epoch
inline uint64_t GetTimestampUs() {
	auto duration = std::chrono::system_clock::now().time_since_epoch();
	return std::chrono::duration_cast<std::chrono::microseconds>(duration).count();
}

#ifdef _WIN32
inline std::wstring GetErrorStr(HRESULT hr) {
	wchar_t *s = NULL;
	std::wstring ret;
	FormatMessageW(FORMAT_MESSAGE_ALLOCATE_BUFFER | FORMAT_MESSAGE_FROM_SYSTEM | FORMAT_MESSAGE_IGNORE_INSERTS,
		NULL, hr,
		MAKELANGID(LANG_NEUTRAL, SUBLANG_DEFAULT),
		(LPWSTR)&s, 0, NULL);
	ret = s;
	LocalFree(s);

	if (ret.size() >= 1 && ret[ret.size() - 1] == L'\n') {
		ret.erase(ret.size() - 1, 1);
	}
	if (ret.size() >= 1 && ret[ret.size() - 1] == L'\r') {
		ret.erase(ret.size() - 1, 1);
	}
	return ret;
}
#endif

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

inline vr::HmdQuaternion_t EulerAngleToQuaternion(const double *yaw_pitch_roll)
{
	vr::HmdQuaternion_t q;
	// Abbreviations for the various angular functions
	double cy = cos(yaw_pitch_roll[0] * 0.5);
	double sy = sin(yaw_pitch_roll[0] * 0.5);
	double cr = cos(yaw_pitch_roll[2] * 0.5);
	double sr = sin(yaw_pitch_roll[2] * 0.5);
	double cp = cos(yaw_pitch_roll[1] * 0.5);
	double sp = sin(yaw_pitch_roll[1] * 0.5);

	q.w = cy * cr * cp + sy * sr * sp;
	q.x = cy * sr * cp - sy * cr * sp;
	q.y = cy * cr * sp + sy * sr * cp;
	q.z = sy * cr * cp - cy * sr * sp;
	return q;
}

inline vr::HmdVector4_t Lerp(vr::HmdVector4_t& v1, vr::HmdVector4_t& v2, double lambda)
{
	vr::HmdVector4_t res;
	res.v[0] = (float)((1 - lambda) * v1.v[0] + lambda * v2.v[0]);
	res.v[1] = (float)((1 - lambda) * v1.v[1] + lambda * v2.v[1]);
	res.v[2] = (float)((1 - lambda) * v1.v[2] + lambda * v2.v[2]);
	res.v[3] = 1;

	return res;
}

inline vr::HmdQuaternionf_t Slerp(vr::HmdQuaternionf_t &q1, vr::HmdQuaternionf_t &q2, double lambda)
{
	if (q1.w != q2.w || q1.x != q2.x || q1.y != q2.y || q1.z != q2.z) {
		float dotproduct = q1.x * q2.x + q1.y * q2.y + q1.z * q2.z + q1.w * q2.w;
		float theta, st, sut, sout, coeff1, coeff2;

		// algorithm adapted from Shoemake's paper

		theta = (float)acos(dotproduct);
		if (theta < 0.0) theta = -theta;

		st = (float)sin(theta);
		sut = (float)sin(lambda * theta);
		sout = (float)sin((1 - lambda) * theta);
		coeff1 = sout / st;
		coeff2 = sut / st;

		vr::HmdQuaternionf_t res;
		res.w = coeff1 * q1.w + coeff2 * q2.w;
		res.x = coeff1 * q1.x + coeff2 * q2.x;
		res.y = coeff1 * q1.y + coeff2 * q2.y;
		res.z = coeff1 * q1.z + coeff2 * q2.z;

		float norm = res.w * res.w + res.x * res.x + res.y * res.y + res.z * res.z;
		res.w /= norm;
		res.x /= norm;
		res.y /= norm;
		res.z /= norm;

		return res;
	}
	else {
		return q1;
	}
}

#ifdef _WIN32
typedef void (WINAPI *RtlGetVersion_FUNC)(OSVERSIONINFOEXW*);

inline std::wstring GetWindowsOSVersion() {
	HMODULE hModule;
	OSVERSIONINFOEXW ver;

	hModule = LoadLibraryW(L"ntdll.dll");
	if (hModule == NULL) {
		return L"Unknown";
	}
	RtlGetVersion_FUNC RtlGetVersion = (RtlGetVersion_FUNC)GetProcAddress(hModule, "RtlGetVersion");
	if (RtlGetVersion == NULL) {
		FreeLibrary(hModule);
		return L"Unknown";
	}
	memset(&ver, 0, sizeof(ver));
	ver.dwOSVersionInfoSize = sizeof(ver);
	RtlGetVersion(&ver);

	FreeLibrary(hModule);

	wchar_t buf[1000];
	_snwprintf_s(buf, sizeof(buf) / sizeof(buf[0]), L"MajorVersion=%d MinorVersion=%d Build=%d",
		ver.dwMajorVersion, ver.dwMinorVersion, ver.dwBuildNumber);
	return buf;
}
#endif
