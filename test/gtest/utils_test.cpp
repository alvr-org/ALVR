#include <gtest/gtest.h>

#include "../../alvr_server/Utils.h"
#include "test-common.h"

void normalize(vr::HmdQuaternion_t &q) {
	double mag = sqrt(q.x * q.x + q.y * q.y + q.z * q.z + q.w * q.w);
	q.x /= mag;
	q.y /= mag;
	q.z /= mag;
	q.w /= mag;
}

vr::HmdQuaternion_t randomQuat() {
	vr::HmdQuaternion_t q;
	q.x = rand() * 1.0 / RAND_MAX;
	q.y = rand() * 1.0 / RAND_MAX;
	q.z = rand() * 1.0 / RAND_MAX;
	q.w = rand() * 1.0 / RAND_MAX;
	normalize(q);
	return q;
}

TEST(utils_test, quaternion_rand) {
	srand(1);

	for (int i = 0; i < 1000; i++) {
		auto q = randomQuat();
		double euler[3];
		QuaternionToEulerAngle(q, euler);
		auto p = EulerAngleToQuaternion(euler);

		ASSERT_TRUE(abs(q.x - p.x) < EPS);
		ASSERT_TRUE(abs(q.y - p.y) < EPS);
		ASSERT_TRUE(abs(q.z - p.z) < EPS);
		ASSERT_TRUE(abs(q.w - p.w) < EPS);
	}
}

TEST(utils_test, quaternion_1) {
	vr::HmdQuaternion_t q;
	double euler[3];

	q.x = 1;
	q.y = 0;
	q.z = 0;
	q.w = 1;
	normalize(q);

	QuaternionToEulerAngle(q, euler);

	ASSERT_EQ_F(euler[0], 0);
	ASSERT_EQ_F(euler[1], 0);
	ASSERT_EQ_F(euler[2], M_PI / 2);

	q.x = 0;
	q.y = 1;
	q.z = 0;
	q.w = 1;
	normalize(q);

	QuaternionToEulerAngle(q, euler);

	ASSERT_EQ_F(euler[0], 0);
	ASSERT_EQ_F(euler[1], M_PI / 2);
	ASSERT_EQ_F(euler[2], 0);

	q.x = 0;
	q.y = 0;
	q.z = 1;
	q.w = 1;
	normalize(q);

	QuaternionToEulerAngle(q, euler);

	ASSERT_EQ_F(euler[0], M_PI / 2);
	ASSERT_EQ_F(euler[1], 0);
	ASSERT_EQ_F(euler[2], 0);
}