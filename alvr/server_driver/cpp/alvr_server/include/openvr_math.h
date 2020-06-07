#pragma once

#include <cmath>


inline vr::HmdQuaternion_t operator+(const vr::HmdQuaternion_t& lhs, const vr::HmdQuaternion_t& rhs) {
	return {
		lhs.w + rhs.w,
		lhs.x + rhs.x,
		lhs.y + rhs.y,
		lhs.z + rhs.z
	};
}


inline vr::HmdQuaternion_t operator-(const vr::HmdQuaternion_t& lhs, const vr::HmdQuaternion_t& rhs) {
	return{
		lhs.w - rhs.w,
		lhs.x - rhs.x,
		lhs.y - rhs.y,
		lhs.z - rhs.z
	};
}


inline vr::HmdQuaternion_t operator*(const vr::HmdQuaternion_t& lhs, const vr::HmdQuaternion_t& rhs) {
	return {
		(lhs.w * rhs.w) - (lhs.x * rhs.x) - (lhs.y * rhs.y) - (lhs.z * rhs.z),
		(lhs.w * rhs.x) + (lhs.x * rhs.w) + (lhs.y * rhs.z) - (lhs.z * rhs.y),
		(lhs.w * rhs.y) + (lhs.y * rhs.w) + (lhs.z * rhs.x) - (lhs.x * rhs.z),
		(lhs.w * rhs.z) + (lhs.z * rhs.w) + (lhs.x * rhs.y) - (lhs.y * rhs.x)
	};
}


inline vr::HmdVector3d_t operator+(const vr::HmdVector3d_t& lhs, const vr::HmdVector3d_t& rhs) {
	return {
		lhs.v[0] + rhs.v[0],
		lhs.v[1] + rhs.v[1],
		lhs.v[2] + rhs.v[2]
	};
}

inline vr::HmdVector3d_t operator+(const vr::HmdVector3d_t& lhs, const double(&rhs)[3]) {
	return{
		lhs.v[0] + rhs[0],
		lhs.v[1] + rhs[1],
		lhs.v[2] + rhs[2]
	};
}

inline vr::HmdVector3d_t operator-(const vr::HmdVector3d_t& lhs, const vr::HmdVector3d_t& rhs) {
	return{
		lhs.v[0] - rhs.v[0],
		lhs.v[1] - rhs.v[1],
		lhs.v[2] - rhs.v[2]
	};
}

inline vr::HmdVector3d_t operator-(const vr::HmdVector3d_t& lhs, const double (&rhs)[3]) {
	return{
		lhs.v[0] - rhs[0],
		lhs.v[1] - rhs[1],
		lhs.v[2] - rhs[2]
	};
}


inline vr::HmdVector3d_t operator*(const vr::HmdVector3d_t& lhs, const double rhs) {
	return{
		lhs.v[0] * rhs,
		lhs.v[1] * rhs,
		lhs.v[2] * rhs
	};
}


inline vr::HmdVector3d_t operator/(const vr::HmdVector3d_t& lhs, const double rhs) {
	return{
		lhs.v[0] / rhs,
		lhs.v[1] / rhs,
		lhs.v[2] / rhs
	};
}


namespace vrmath {

	template<typename T> int signum(T v) {
		return (v > (T)0) ? 1 : ((v < (T)0) ? -1 : 0);
	}

	inline vr::HmdQuaternion_t quaternionFromRotationAxis(double rot, double ux, double uy, double uz) {
		auto ha = rot / 2;
		return{
			std::cos(ha),
			ux * std::sin(ha),
			uy * std::sin(ha),
			uz * std::sin(ha)
		};
	}

	inline vr::HmdQuaternion_t quaternionFromRotationX(double rot) {
		auto ha = rot / 2;
		return{
			std::cos(ha),
			std::sin(ha),
			0.0f,
			0.0f
		};
	}

	inline vr::HmdQuaternion_t quaternionFromRotationY(double rot) {
		auto ha = rot / 2;
		return{
			std::cos(ha),
			0.0f,
			std::sin(ha),
			0.0f
		};
	}

	inline vr::HmdQuaternion_t quaternionFromRotationZ(double rot) {
		auto ha = rot / 2;
		return{
			std::cos(ha),
			0.0f,
			0.0f,
			std::sin(ha)
		};
	}

	inline vr::HmdQuaternion_t quaternionFromYawPitchRoll(double yaw, double pitch, double roll) {
		return quaternionFromRotationY(yaw) * quaternionFromRotationX(pitch) * quaternionFromRotationZ(roll);
	}

	inline vr::HmdQuaternion_t quaternionFromRotationMatrix(const vr::HmdMatrix34_t& mat) {
		auto a = mat.m;
		vr::HmdQuaternion_t q;
		double trace = a[0][0] + a[1][1] + a[2][2];
		if (trace > 0) {
			double s = 0.5 / sqrt(trace + 1.0);
			q.w = 0.25 / s;
			q.x = (a[1][2] - a[2][1]) * s;
			q.y = (a[2][0] - a[0][2]) * s;
			q.z = (a[0][1] - a[1][0]) * s;
		} else {
			if (a[0][0] > a[1][1] && a[0][0] > a[2][2]) {
				double s = 2.0 * sqrt(1.0 + a[0][0] - a[1][1] - a[2][2]);
				q.w = (a[1][2] - a[2][1]) / s;
				q.x = 0.25 * s;
				q.y = (a[1][0] + a[0][1]) / s;
				q.z = (a[2][0] + a[0][2]) / s;
			} else if (a[1][1] > a[2][2]) {
				double s = 2.0 * sqrt(1.0 + a[1][1] - a[0][0] - a[2][2]);
				q.w = (a[2][0] - a[0][2]) / s;
				q.x = (a[1][0] + a[0][1]) / s;
				q.y = 0.25 * s;
				q.z = (a[2][1] + a[1][2]) / s;
			} else {
				double s = 2.0 * sqrt(1.0 + a[2][2] - a[0][0] - a[1][1]);
				q.w = (a[0][1] - a[1][0]) / s;
				q.x = (a[2][0] + a[0][2]) / s;
				q.y = (a[2][1] + a[1][2]) / s;
				q.z = 0.25 * s;
			}
		}
		q.x = -q.x;
		q.y = -q.y;
		q.z = -q.z;
		return q;
	}

	inline vr::HmdQuaternion_t quaternionConjugate(const vr::HmdQuaternion_t& quat) {
		return {
			quat.w,
			-quat.x,
			-quat.y,
			-quat.z,
		};
	}

	inline vr::HmdVector3d_t quaternionRotateVector(const vr::HmdQuaternion_t& quat, const vr::HmdVector3d_t& vector, bool reverse = false) {
		if (reverse) {
			vr::HmdQuaternion_t pin = { 0.0, vector.v[0], vector.v[1] , vector.v[2] };
			auto pout = vrmath::quaternionConjugate(quat) * pin * quat;
			return {pout.x, pout.y, pout.z};
		} else {
			vr::HmdQuaternion_t pin = { 0.0, vector.v[0], vector.v[1] , vector.v[2] };
			auto pout = quat * pin * vrmath::quaternionConjugate(quat);
			return { pout.x, pout.y, pout.z };
		}
	}

	inline vr::HmdVector3d_t quaternionRotateVector(const vr::HmdQuaternion_t& quat, const vr::HmdQuaternion_t& quatInv, const vr::HmdVector3d_t& vector, bool reverse = false) {
		if (reverse) {
			vr::HmdQuaternion_t pin = { 0.0, vector.v[0], vector.v[1] , vector.v[2] };
			auto pout = quatInv * pin * quat;
			return{ pout.x, pout.y, pout.z };
		} else {
			vr::HmdQuaternion_t pin = { 0.0, vector.v[0], vector.v[1] , vector.v[2] };
			auto pout = quat * pin * quatInv;
			return{ pout.x, pout.y, pout.z };
		}
	}

	inline vr::HmdVector3d_t quaternionRotateVector(const vr::HmdQuaternion_t& quat, const double (&vector)[3], bool reverse = false) {
		if (reverse) {
			vr::HmdQuaternion_t pin = { 0.0, vector[0], vector[1] , vector[2] };
			auto pout = vrmath::quaternionConjugate(quat) * pin * quat;
			return{ pout.x, pout.y, pout.z };
		} else {
			vr::HmdQuaternion_t pin = { 0.0, vector[0], vector[1] , vector[2] };
			auto pout = quat * pin * vrmath::quaternionConjugate(quat);
			return{ pout.x, pout.y, pout.z };
		}
	}

	inline vr::HmdVector3d_t quaternionRotateVector(const vr::HmdQuaternion_t& quat, const vr::HmdQuaternion_t& quatInv, const double(&vector)[3], bool reverse = false) {
		if (reverse) {
			vr::HmdQuaternion_t pin = { 0.0, vector[0], vector[1] , vector[2] };
			auto pout = quatInv * pin * quat;
			return{ pout.x, pout.y, pout.z };
		} else {
			vr::HmdQuaternion_t pin = { 0.0, vector[0], vector[1] , vector[2] };
			auto pout = quat * pin * quatInv;
			return{ pout.x, pout.y, pout.z };
		}
	}

	inline vr::HmdMatrix34_t matMul33(const vr::HmdMatrix34_t& a, const vr::HmdMatrix34_t& b) {
		vr::HmdMatrix34_t result;
		for (unsigned i = 0; i < 3; i++) {
			for (unsigned j = 0; j < 3; j++) {
				result.m[i][j] = 0.0f;
				for (unsigned k = 0; k < 3; k++) {
					result.m[i][j] += a.m[i][k] * b.m[k][j];
				}
			}
		}
		return result;
	}

	inline vr::HmdVector3_t matMul33(const vr::HmdMatrix34_t& a, const vr::HmdVector3_t& b) {
		vr::HmdVector3_t result;
		for (unsigned i = 0; i < 3; i++) {
			result.v[i] = 0.0f;
			for (unsigned k = 0; k < 3; k++) {
				result.v[i] += a.m[i][k] * b.v[k];
			};
		}
		return result;
	}

	inline vr::HmdVector3d_t matMul33(const vr::HmdMatrix34_t& a, const vr::HmdVector3d_t& b) {
		vr::HmdVector3d_t result;
		for (unsigned i = 0; i < 3; i++) {
			result.v[i] = 0.0f;
			for (unsigned k = 0; k < 3; k++) {
				result.v[i] += a.m[i][k] * b.v[k];
			};
		}
		return result;
	}

	inline vr::HmdVector3_t matMul33(const vr::HmdVector3_t& a, const vr::HmdMatrix34_t& b) {
		vr::HmdVector3_t result;
		for (unsigned i = 0; i < 3; i++) {
			result.v[i] = 0.0f;
			for (unsigned k = 0; k < 3; k++) {
				result.v[i] += a.v[k] * b.m[k][i];
			};
		}
		return result;
	}

	inline vr::HmdVector3d_t matMul33(const vr::HmdVector3d_t& a, const vr::HmdMatrix34_t& b) {
		vr::HmdVector3d_t result;
		for (unsigned i = 0; i < 3; i++) {
			result.v[i] = 0.0f;
			for (unsigned k = 0; k < 3; k++) {
				result.v[i] += a.v[k] * b.m[k][i];
			};
		}
		return result;
	}

	inline vr::HmdMatrix34_t transposeMul33(const vr::HmdMatrix34_t& a) {
		vr::HmdMatrix34_t result;
		for (unsigned i = 0; i < 3; i++) {
			for (unsigned k = 0; k < 3; k++) {
				result.m[i][k] = a.m[k][i];
			}
		}
		result.m[0][3] = a.m[0][3];
		result.m[1][3] = a.m[1][3];
		result.m[2][3] = a.m[2][3];
		return result;
	}
}

