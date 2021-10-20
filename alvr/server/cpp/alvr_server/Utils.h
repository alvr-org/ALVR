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

const uint64_t US_TO_MS = 1000;
const float DEG_TO_RAD = (float)(M_PI / 180.);
extern uint64_t gPerformanceCounterFrequency;

// Get elapsed time in us from Unix Epoch
inline uint64_t GetTimestampUs() {
	auto duration = std::chrono::system_clock::now().time_since_epoch();
	return std::chrono::duration_cast<std::chrono::microseconds>(duration).count();
}

// Get performance counter in us
inline uint64_t GetCounterUs() {
#ifdef _WIN32
	if (gPerformanceCounterFrequency == 0) {
		LARGE_INTEGER freq;
		QueryPerformanceFrequency(&freq);
		gPerformanceCounterFrequency = freq.QuadPart;
	}

	LARGE_INTEGER counter;
	QueryPerformanceCounter(&counter);

	return counter.QuadPart * 1000000LLU / gPerformanceCounterFrequency;
#else
	auto duration = std::chrono::steady_clock::now().time_since_epoch();
	return std::chrono::duration_cast<std::chrono::microseconds>(duration).count();
#endif
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

inline vr::HmdQuaternion_t HmdQuaternion_Scale(vr::HmdQuaternion_t *q, double s)
{
	return HmdQuaternion_Init(q->w*s, q->x*s, q->y*s, q->z*s);
	
}

inline double HmdQuaternion_Norm(vr::HmdQuaternion_t *q) {
	return (q->w*q->w + q->x * q->x + q->y * q->y + q->z * q->z);
}

inline vr::HmdQuaternion_t HmdQuaternion_Conjugate(vr::HmdQuaternion_t *q)
{
	return HmdQuaternion_Init(q->w, -q->x, -q->y, -q->z);
}

inline vr::HmdQuaternion_t HmdQuaternion_Inverse(vr::HmdQuaternion_t *q)
{

	vr::HmdQuaternion_t res;
	res = HmdQuaternion_Conjugate(q);

	return HmdQuaternion_Scale(&res , 1 / HmdQuaternion_Norm(&res));
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

inline void QuaternionToEulerAngle(const vr::HmdQuaternion_t &q, double *yaw_pitch_roll)
{
	// roll (x-axis rotation)
	double sinr = +2.0 * (q.w * q.x + q.y * q.z);
	double cosr = +1.0 - 2.0 * (q.x * q.x + q.y * q.y);
	yaw_pitch_roll[2] = atan2(sinr, cosr);

	// pitch (y-axis rotation)
	double sinp = +2.0 * (q.w * q.y - q.z * q.x);
	if (fabs(sinp) >= 1)
		yaw_pitch_roll[1] = copysign(M_PI / 2, sinp); // use 90 degrees if out of range
	else
		yaw_pitch_roll[1] = asin(sinp);

	// yaw (z-axis rotation)
	double siny = +2.0 * (q.w * q.z + q.x * q.y);
	double cosy = +1.0 - 2.0 * (q.y * q.y + q.z * q.z);
	yaw_pitch_roll[0] = atan2(siny, cosy);
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

inline vr::HmdQuaternionf_t EulerAngleToQuaternionF(const float* yaw_pitch_roll)
{
	vr::HmdQuaternionf_t q;
	// Abbreviations for the various angular functions
	double cy = cos(yaw_pitch_roll[0] * 0.5);
	double sy = sin(yaw_pitch_roll[0] * 0.5);
	double cr = cos(yaw_pitch_roll[2] * 0.5);
	double sr = sin(yaw_pitch_roll[2] * 0.5);
	double cp = cos(yaw_pitch_roll[1] * 0.5);
	double sp = sin(yaw_pitch_roll[1] * 0.5);

	q.w = float(cy * cr * cp + sy * sr * sp);
	q.x = float(cy * sr * cp - sy * cr * sp);
	q.y = float(cy * cr * sp + sy * sr * cp);
	q.z = float(sy * cr * cp - cy * sr * sp);
	return q;
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

inline float Magnitude(const TrackingVector3& v) {
	return v.x * v.x + v.y * v.y + v.z * v.z;
}
// Magnitude already squared
inline float Shape(float x, float a) {
	return (x > a*a ? 1 - (a*a/x) : 0.);
}

#ifdef _WIN32
// Delay loading for Cuda driver API to correctly work on non-NVIDIA GPU.
inline bool LoadCudaDLL() {
	__try {
		return !FAILED(__HrLoadAllImportsForDll("nvcuda.dll"));
	} __except (EXCEPTION_EXECUTE_HANDLER) {
	}
	return false;
}

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
