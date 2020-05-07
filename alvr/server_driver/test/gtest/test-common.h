#pragma once

#define EPS 1e-6
#define ASSERT_EQ_F(a, b) ASSERT_LE(abs(a - b), EPS)
