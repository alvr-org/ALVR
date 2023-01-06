/// @ref gtc_matrix_integer
/// @file glm/gtc/matrix_integer.hpp
///
/// @see core (dependence)
///
/// @defgroup gtc_matrix_integer GLM_GTC_matrix_integer
/// @ingroup gtc
///
/// Include <glm/gtc/matrix_integer.hpp> to use the features of this extension.
///
/// Defines a number of matrices with integer types.

#pragma once

// Dependency:
#include "../mat2x2.hpp"
#include "../mat2x3.hpp"
#include "../mat2x4.hpp"
#include "../mat3x2.hpp"
#include "../mat3x3.hpp"
#include "../mat3x4.hpp"
#include "../mat4x2.hpp"
#include "../mat4x3.hpp"
#include "../mat4x4.hpp"

#if GLM_MESSAGES == GLM_ENABLE && !defined(GLM_EXT_INCLUDED)
#	pragma message("GLM: GLM_GTC_matrix_integer extension included")
#endif

namespace glm
{
	/// @addtogroup gtc_matrix_integer
	/// @{

	/// High-qualifier signed integer 2x2 matrix.
	/// @see gtc_matrix_integer
	typedef mat<2, 2, int, highp>				highp_imat2;

	/// High-qualifier signed integer 3x3 matrix.
	/// @see gtc_matrix_integer
	typedef mat<3, 3, int, highp>				highp_imat3;

	/// High-qualifier signed integer 4x4 matrix.
	/// @see gtc_matrix_integer
	typedef mat<4, 4, int, highp>				highp_imat4;

	/// High-qualifier signed integer 2x2 matrix.
	/// @see gtc_matrix_integer
	typedef mat<2, 2, int, highp>				highp_imat2x2;

	/// High-qualifier signed integer 2x3 matrix.
	/// @see gtc_matrix_integer
	typedef mat<2, 3, int, highp>				highp_imat2x3;

	/// High-qualifier signed integer 2x4 matrix.
	/// @see gtc_matrix_integer
	typedef mat<2, 4, int, highp>				highp_imat2x4;

	/// High-qualifier signed integer 3x2 matrix.
	/// @see gtc_matrix_integer
	typedef mat<3, 2, int, highp>				highp_imat3x2;

	/// High-qualifier signed integer 3x3 matrix.
	/// @see gtc_matrix_integer
	typedef mat<3, 3, int, highp>				highp_imat3x3;

	/// High-qualifier signed integer 3x4 matrix.
	/// @see gtc_matrix_integer
	typedef mat<3, 4, int, highp>				highp_imat3x4;

	/// High-qualifier signed integer 4x2 matrix.
	/// @see gtc_matrix_integer
	typedef mat<4, 2, int, highp>				highp_imat4x2;

	/// High-qualifier signed integer 4x3 matrix.
	/// @see gtc_matrix_integer
	typedef mat<4, 3, int, highp>				highp_imat4x3;

	/// High-qualifier signed integer 4x4 matrix.
	/// @see gtc_matrix_integer
	typedef mat<4, 4, int, highp>				highp_imat4x4;


	/// Medium-qualifier signed integer 2x2 matrix.
	/// @see gtc_matrix_integer
	typedef mat<2, 2, int, mediump>			mediump_imat2;

	/// Medium-qualifier signed integer 3x3 matrix.
	/// @see gtc_matrix_integer
	typedef mat<3, 3, int, mediump>			mediump_imat3;

	/// Medium-qualifier signed integer 4x4 matrix.
	/// @see gtc_matrix_integer
	typedef mat<4, 4, int, mediump>			mediump_imat4;


	/// Medium-qualifier signed integer 2x2 matrix.
	/// @see gtc_matrix_integer
	typedef mat<2, 2, int, mediump>			mediump_imat2x2;

	/// Medium-qualifier signed integer 2x3 matrix.
	/// @see gtc_matrix_integer
	typedef mat<2, 3, int, mediump>			mediump_imat2x3;

	/// Medium-qualifier signed integer 2x4 matrix.
	/// @see gtc_matrix_integer
	typedef mat<2, 4, int, mediump>			mediump_imat2x4;

	/// Medium-qualifier signed integer 3x2 matrix.
	/// @see gtc_matrix_integer
	typedef mat<3, 2, int, mediump>			mediump_imat3x2;

	/// Medium-qualifier signed integer 3x3 matrix.
	/// @see gtc_matrix_integer
	typedef mat<3, 3, int, mediump>			mediump_imat3x3;

	/// Medium-qualifier signed integer 3x4 matrix.
	/// @see gtc_matrix_integer
	typedef mat<3, 4, int, mediump>			mediump_imat3x4;

	/// Medium-qualifier signed integer 4x2 matrix.
	/// @see gtc_matrix_integer
	typedef mat<4, 2, int, mediump>			mediump_imat4x2;

	/// Medium-qualifier signed integer 4x3 matrix.
	/// @see gtc_matrix_integer
	typedef mat<4, 3, int, mediump>			mediump_imat4x3;

	/// Medium-qualifier signed integer 4x4 matrix.
	/// @see gtc_matrix_integer
	typedef mat<4, 4, int, mediump>			mediump_imat4x4;


	/// Low-qualifier signed integer 2x2 matrix.
	/// @see gtc_matrix_integer
	typedef mat<2, 2, int, lowp>				lowp_imat2;

	/// Low-qualifier signed integer 3x3 matrix.
	/// @see gtc_matrix_integer
	typedef mat<3, 3, int, lowp>				lowp_imat3;

	/// Low-qualifier signed integer 4x4 matrix.
	/// @see gtc_matrix_integer
	typedef mat<4, 4, int, lowp>				lowp_imat4;


	/// Low-qualifier signed integer 2x2 matrix.
	/// @see gtc_matrix_integer
	typedef mat<2, 2, int, lowp>				lowp_imat2x2;

	/// Low-qualifier signed integer 2x3 matrix.
	/// @see gtc_matrix_integer
	typedef mat<2, 3, int, lowp>				lowp_imat2x3;

	/// Low-qualifier signed integer 2x4 matrix.
	/// @see gtc_matrix_integer
	typedef mat<2, 4, int, lowp>				lowp_imat2x4;

	/// Low-qualifier signed integer 3x2 matrix.
	/// @see gtc_matrix_integer
	typedef mat<3, 2, int, lowp>				lowp_imat3x2;

	/// Low-qualifier signed integer 3x3 matrix.
	/// @see gtc_matrix_integer
	typedef mat<3, 3, int, lowp>				lowp_imat3x3;

	/// Low-qualifier signed integer 3x4 matrix.
	/// @see gtc_matrix_integer
	typedef mat<3, 4, int, lowp>				lowp_imat3x4;

	/// Low-qualifier signed integer 4x2 matrix.
	/// @see gtc_matrix_integer
	typedef mat<4, 2, int, lowp>				lowp_imat4x2;

	/// Low-qualifier signed integer 4x3 matrix.
	/// @see gtc_matrix_integer
	typedef mat<4, 3, int, lowp>				lowp_imat4x3;

	/// Low-qualifier signed integer 4x4 matrix.
	/// @see gtc_matrix_integer
	typedef mat<4, 4, int, lowp>				lowp_imat4x4;


	/// High-qualifier unsigned integer 2x2 matrix.
	/// @see gtc_matrix_integer
	typedef mat<2, 2, uint, highp>				highp_umat2;

	/// High-qualifier unsigned integer 3x3 matrix.
	/// @see gtc_matrix_integer
	typedef mat<3, 3, uint, highp>				highp_umat3;

	/// High-qualifier unsigned integer 4x4 matrix.
	/// @see gtc_matrix_integer
	typedef mat<4, 4, uint, highp>				highp_umat4;

	/// High-qualifier unsigned integer 2x2 matrix.
	/// @see gtc_matrix_integer
	typedef mat<2, 2, uint, highp>				highp_umat2x2;

	/// High-qualifier unsigned integer 2x3 matrix.
	/// @see gtc_matrix_integer
	typedef mat<2, 3, uint, highp>				highp_umat2x3;

	/// High-qualifier unsigned integer 2x4 matrix.
	/// @see gtc_matrix_integer
	typedef mat<2, 4, uint, highp>				highp_umat2x4;

	/// High-qualifier unsigned integer 3x2 matrix.
	/// @see gtc_matrix_integer
	typedef mat<3, 2, uint, highp>				highp_umat3x2;

	/// High-qualifier unsigned integer 3x3 matrix.
	/// @see gtc_matrix_integer
	typedef mat<3, 3, uint, highp>				highp_umat3x3;

	/// High-qualifier unsigned integer 3x4 matrix.
	/// @see gtc_matrix_integer
	typedef mat<3, 4, uint, highp>				highp_umat3x4;

	/// High-qualifier unsigned integer 4x2 matrix.
	/// @see gtc_matrix_integer
	typedef mat<4, 2, uint, highp>				highp_umat4x2;

	/// High-qualifier unsigned integer 4x3 matrix.
	/// @see gtc_matrix_integer
	typedef mat<4, 3, uint, highp>				highp_umat4x3;

	/// High-qualifier unsigned integer 4x4 matrix.
	/// @see gtc_matrix_integer
	typedef mat<4, 4, uint, highp>				highp_umat4x4;


	/// Medium-qualifier unsigned integer 2x2 matrix.
	/// @see gtc_matrix_integer
	typedef mat<2, 2, uint, mediump>			mediump_umat2;

	/// Medium-qualifier unsigned integer 3x3 matrix.
	/// @see gtc_matrix_integer
	typedef mat<3, 3, uint, mediump>			mediump_umat3;

	/// Medium-qualifier unsigned integer 4x4 matrix.
	/// @see gtc_matrix_integer
	typedef mat<4, 4, uint, mediump>			mediump_umat4;


	/// Medium-qualifier unsigned integer 2x2 matrix.
	/// @see gtc_matrix_integer
	typedef mat<2, 2, uint, mediump>			mediump_umat2x2;

	/// Medium-qualifier unsigned integer 2x3 matrix.
	/// @see gtc_matrix_integer
	typedef mat<2, 3, uint, mediump>			mediump_umat2x3;

	/// Medium-qualifier unsigned integer 2x4 matrix.
	/// @see gtc_matrix_integer
	typedef mat<2, 4, uint, mediump>			mediump_umat2x4;

	/// Medium-qualifier unsigned integer 3x2 matrix.
	/// @see gtc_matrix_integer
	typedef mat<3, 2, uint, mediump>			mediump_umat3x2;

	/// Medium-qualifier unsigned integer 3x3 matrix.
	/// @see gtc_matrix_integer
	typedef mat<3, 3, uint, mediump>			mediump_umat3x3;

	/// Medium-qualifier unsigned integer 3x4 matrix.
	/// @see gtc_matrix_integer
	typedef mat<3, 4, uint, mediump>			mediump_umat3x4;

	/// Medium-qualifier unsigned integer 4x2 matrix.
	/// @see gtc_matrix_integer
	typedef mat<4, 2, uint, mediump>			mediump_umat4x2;

	/// Medium-qualifier unsigned integer 4x3 matrix.
	/// @see gtc_matrix_integer
	typedef mat<4, 3, uint, mediump>			mediump_umat4x3;

	/// Medium-qualifier unsigned integer 4x4 matrix.
	/// @see gtc_matrix_integer
	typedef mat<4, 4, uint, mediump>			mediump_umat4x4;


	/// Low-qualifier unsigned integer 2x2 matrix.
	/// @see gtc_matrix_integer
	typedef mat<2, 2, uint, lowp>				lowp_umat2;

	/// Low-qualifier unsigned integer 3x3 matrix.
	/// @see gtc_matrix_integer
	typedef mat<3, 3, uint, lowp>				lowp_umat3;

	/// Low-qualifier unsigned integer 4x4 matrix.
	/// @see gtc_matrix_integer
	typedef mat<4, 4, uint, lowp>				lowp_umat4;


	/// Low-qualifier unsigned integer 2x2 matrix.
	/// @see gtc_matrix_integer
	typedef mat<2, 2, uint, lowp>				lowp_umat2x2;

	/// Low-qualifier unsigned integer 2x3 matrix.
	/// @see gtc_matrix_integer
	typedef mat<2, 3, uint, lowp>				lowp_umat2x3;

	/// Low-qualifier unsigned integer 2x4 matrix.
	/// @see gtc_matrix_integer
	typedef mat<2, 4, uint, lowp>				lowp_umat2x4;

	/// Low-qualifier unsigned integer 3x2 matrix.
	/// @see gtc_matrix_integer
	typedef mat<3, 2, uint, lowp>				lowp_umat3x2;

	/// Low-qualifier unsigned integer 3x3 matrix.
	/// @see gtc_matrix_integer
	typedef mat<3, 3, uint, lowp>				lowp_umat3x3;

	/// Low-qualifier unsigned integer 3x4 matrix.
	/// @see gtc_matrix_integer
	typedef mat<3, 4, uint, lowp>				lowp_umat3x4;

	/// Low-qualifier unsigned integer 4x2 matrix.
	/// @see gtc_matrix_integer
	typedef mat<4, 2, uint, lowp>				lowp_umat4x2;

	/// Low-qualifier unsigned integer 4x3 matrix.
	/// @see gtc_matrix_integer
	typedef mat<4, 3, uint, lowp>				lowp_umat4x3;

	/// Low-qualifier unsigned integer 4x4 matrix.
	/// @see gtc_matrix_integer
	typedef mat<4, 4, uint, lowp>				lowp_umat4x4;

#if(defined(GLM_PRECISION_HIGHP_INT))
	typedef highp_imat2								imat2;
	typedef highp_imat3								imat3;
	typedef highp_imat4								imat4;
	typedef highp_imat2x2							imat2x2;
	typedef highp_imat2x3							imat2x3;
	typedef highp_imat2x4							imat2x4;
	typedef highp_imat3x2							imat3x2;
	typedef highp_imat3x3							imat3x3;
	typedef highp_imat3x4							imat3x4;
	typedef highp_imat4x2							imat4x2;
	typedef highp_imat4x3							imat4x3;
	typedef highp_imat4x4							imat4x4;
#elif(defined(GLM_PRECISION_LOWP_INT))
	typedef lowp_imat2								imat2;
	typedef lowp_imat3								imat3;
	typedef lowp_imat4								imat4;
	typedef lowp_imat2x2							imat2x2;
	typedef lowp_imat2x3							imat2x3;
	typedef lowp_imat2x4							imat2x4;
	typedef lowp_imat3x2							imat3x2;
	typedef lowp_imat3x3							imat3x3;
	typedef lowp_imat3x4							imat3x4;
	typedef lowp_imat4x2							imat4x2;
	typedef lowp_imat4x3							imat4x3;
	typedef lowp_imat4x4							imat4x4;
#else //if(defined(GLM_PRECISION_MEDIUMP_INT))

	/// Signed integer 2x2 matrix.
	/// @see gtc_matrix_integer
	typedef mediump_imat2							imat2;

	/// Signed integer 3x3 matrix.
	/// @see gtc_matrix_integer
	typedef mediump_imat3							imat3;

	/// Signed integer 4x4 matrix.
	/// @see gtc_matrix_integer
	typedef mediump_imat4							imat4;

	/// Signed integer 2x2 matrix.
	/// @see gtc_matrix_integer
	typedef mediump_imat2x2							imat2x2;

	/// Signed integer 2x3 matrix.
	/// @see gtc_matrix_integer
	typedef mediump_imat2x3							imat2x3;

	/// Signed integer 2x4 matrix.
	/// @see gtc_matrix_integer
	typedef mediump_imat2x4							imat2x4;

	/// Signed integer 3x2 matrix.
	/// @see gtc_matrix_integer
	typedef mediump_imat3x2							imat3x2;

	/// Signed integer 3x3 matrix.
	/// @see gtc_matrix_integer
	typedef mediump_imat3x3							imat3x3;

	/// Signed integer 3x4 matrix.
	/// @see gtc_matrix_integer
	typedef mediump_imat3x4							imat3x4;

	/// Signed integer 4x2 matrix.
	/// @see gtc_matrix_integer
	typedef mediump_imat4x2							imat4x2;

	/// Signed integer 4x3 matrix.
	/// @see gtc_matrix_integer
	typedef mediump_imat4x3							imat4x3;

	/// Signed integer 4x4 matrix.
	/// @see gtc_matrix_integer
	typedef mediump_imat4x4							imat4x4;
#endif//GLM_PRECISION

#if(defined(GLM_PRECISION_HIGHP_UINT))
	typedef highp_umat2								umat2;
	typedef highp_umat3								umat3;
	typedef highp_umat4								umat4;
	typedef highp_umat2x2							umat2x2;
	typedef highp_umat2x3							umat2x3;
	typedef highp_umat2x4							umat2x4;
	typedef highp_umat3x2							umat3x2;
	typedef highp_umat3x3							umat3x3;
	typedef highp_umat3x4							umat3x4;
	typedef highp_umat4x2							umat4x2;
	typedef highp_umat4x3							umat4x3;
	typedef highp_umat4x4							umat4x4;
#elif(defined(GLM_PRECISION_LOWP_UINT))
	typedef lowp_umat2								umat2;
	typedef lowp_umat3								umat3;
	typedef lowp_umat4								umat4;
	typedef lowp_umat2x2							umat2x2;
	typedef lowp_umat2x3							umat2x3;
	typedef lowp_umat2x4							umat2x4;
	typedef lowp_umat3x2							umat3x2;
	typedef lowp_umat3x3							umat3x3;
	typedef lowp_umat3x4							umat3x4;
	typedef lowp_umat4x2							umat4x2;
	typedef lowp_umat4x3							umat4x3;
	typedef lowp_umat4x4							umat4x4;
#else //if(defined(GLM_PRECISION_MEDIUMP_UINT))

	/// Unsigned integer 2x2 matrix.
	/// @see gtc_matrix_integer
	typedef mediump_umat2							umat2;

	/// Unsigned integer 3x3 matrix.
	/// @see gtc_matrix_integer
	typedef mediump_umat3							umat3;

	/// Unsigned integer 4x4 matrix.
	/// @see gtc_matrix_integer
	typedef mediump_umat4							umat4;

	/// Unsigned integer 2x2 matrix.
	/// @see gtc_matrix_integer
	typedef mediump_umat2x2							umat2x2;

	/// Unsigned integer 2x3 matrix.
	/// @see gtc_matrix_integer
	typedef mediump_umat2x3							umat2x3;

	/// Unsigned integer 2x4 matrix.
	/// @see gtc_matrix_integer
	typedef mediump_umat2x4							umat2x4;

	/// Unsigned integer 3x2 matrix.
	/// @see gtc_matrix_integer
	typedef mediump_umat3x2							umat3x2;

	/// Unsigned integer 3x3 matrix.
	/// @see gtc_matrix_integer
	typedef mediump_umat3x3							umat3x3;

	/// Unsigned integer 3x4 matrix.
	/// @see gtc_matrix_integer
	typedef mediump_umat3x4							umat3x4;

	/// Unsigned integer 4x2 matrix.
	/// @see gtc_matrix_integer
	typedef mediump_umat4x2							umat4x2;

	/// Unsigned integer 4x3 matrix.
	/// @see gtc_matrix_integer
	typedef mediump_umat4x3							umat4x3;

	/// Unsigned integer 4x4 matrix.
	/// @see gtc_matrix_integer
	typedef mediump_umat4x4							umat4x4;
#endif//GLM_PRECISION

	/// @}
}//namespace glm
