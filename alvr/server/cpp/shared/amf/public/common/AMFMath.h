//
// Notice Regarding Standards.  AMD does not provide a license or sublicense to
// any Intellectual Property Rights relating to any standards, including but not
// limited to any audio and/or video codec technologies such as MPEG-2, MPEG-4;
// AVC/H.264; HEVC/H.265; AAC decode/FFMPEG; AAC encode/FFMPEG; VC-1; and MP3
// (collectively, the "Media Technologies"). For clarity, you will pay any
// royalties due for such third party technologies, which may include the Media
// Technologies that are owed as a result of AMD providing the Software to you.
//
// MIT license
//
//
// Copyright (c) 2018 Advanced Micro Devices, Inc. All rights reserved.
//
// Permission is hereby granted, free of charge, to any person obtaining a copy
// of this software and associated documentation files (the "Software"), to deal
// in the Software without restriction, including without limitation the rights
// to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
// copies of the Software, and to permit persons to whom the Software is
// furnished to do so, subject to the following conditions:
//
// The above copyright notice and this permission notice shall be included in
// all copies or substantial portions of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
// IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT.  IN NO EVENT SHALL THE
// AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
// LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
// OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN
// THE SOFTWARE.
//

#pragma once

#include <cmath>

namespace amf
{
    // right-handed system
    // +y is up
    // +x is to the right
    // -z is forward

    const float AMF_PI         = 3.141592654f;
    const float AMF_1DIV2PI    = 0.159154943f;
    const float AMF_2PI        = 6.283185307f;
    const float AMF_PIDIV2     = 1.570796327f;

    const uint32_t AMF_PERMUTE_0X        = 0;
    const uint32_t AMF_PERMUTE_0Y        = 1;
    const uint32_t AMF_PERMUTE_0Z        = 2;
    const uint32_t AMF_PERMUTE_0W        = 3;
    const uint32_t AMF_PERMUTE_1X        = 4;
    const uint32_t AMF_PERMUTE_1Y        = 5;
    const uint32_t AMF_PERMUTE_1Z        = 6;
    const uint32_t AMF_PERMUTE_1W        = 7;

    const uint32_t AMF_SWIZZLE_X         = 0;
    const uint32_t AMF_SWIZZLE_Y         = 1;
    const uint32_t AMF_SWIZZLE_Z         = 2;
    const uint32_t AMF_SWIZZLE_W         = 3;

    //---------------------------------------------------------------------------------------------
    class VectorPOD
    {
    public:
        float x;
        float y;
        float z;
        float w;

//        Vector():x(0),y(0),z(0),w(0){}
//        Vector(float _x, float _y, float _z, float _w ):x(_x),y(_y),z(_z),w(_w){}

        void Assign(float _x, float _y, float _z, float _w)
        {
            x= _x; y = _y; z = _z; w = _w;
        }

        inline VectorPOD& operator-=(const VectorPOD& other)
        {
            x -=other.x; y -=other.y; z -=other.z; w -=other.w;
            return *this;
        }
        inline VectorPOD operator-(const VectorPOD& other) const
        {
            VectorPOD vector;
            vector.x = x - other.x;
            vector.y = y - other.y;
            vector.z = z - other.z;
            vector.w = w - other.w;
            return vector;
        }
        inline VectorPOD& operator+=(const VectorPOD& other)
        {
            x +=other.x;
            y +=other.y;
            z +=other.z;
            w +=other.w;
            return *this;
        }
        inline VectorPOD operator+(const VectorPOD& other)  const
        {
            VectorPOD vector;
            vector.x = x + other.x;
            vector.y = y + other.y;
            vector.z = z + other.z;
            vector.w = w + other.w;
            return vector;
        }

        inline VectorPOD operator*(const VectorPOD& other)  const
        {
            VectorPOD vector;
            vector.x = x * other.x;
            vector.y = y * other.y;
            vector.z = z * other.z;
            vector.w = w * other.w;
            return vector;
        }
        inline VectorPOD operator*=(const VectorPOD& other)
        {
            x*=other.x;
            y*=other.y;
            z*=other.z;
            w*=other.w;
            return *this;
        }

        inline VectorPOD Swizzle(uint32_t E0, uint32_t E1, uint32_t E2, uint32_t E3) const
        {
            const uint32_t *aPtr = (const uint32_t* )(this);

            VectorPOD Result;
            uint32_t *pWork = (uint32_t*)(&Result);

            pWork[0] = aPtr[E0];
            pWork[1] = aPtr[E1];
            pWork[2] = aPtr[E2];
            pWork[3] = aPtr[E3];

            return Result;
        }

        /*
        inline VectorPOD& operator=(const VectorPOD& other)
        {
            Assign(other.x, other.y, other.z, other.w);
            return *this;
        }
        */
        inline bool operator==(const VectorPOD& other) const
        {
            return x == other.x && y == other.y && z == other.z && w == other.w;
        }
        inline bool operator!=(const VectorPOD& other) const { return !operator==(other); }
        inline VectorPOD Dot3(const VectorPOD& vec) const
        {
            float fValue = x * vec.x + y * vec.y + z * vec.z;
            VectorPOD Result;
            Result.Assign(fValue, fValue, fValue, fValue);
            return Result;
        }
        inline VectorPOD Dot4(const VectorPOD& vec) const
        {
            float fValue = x * vec.x + y * vec.y + z * vec.z + w * vec.w;
            VectorPOD Result;
            Result.Assign(fValue, fValue, fValue, fValue);
            return Result;
        }

        inline VectorPOD LengthSq3()  const
        {
            return Dot3(*this);
        }

        inline VectorPOD LengthSq4()  const
        {
            return Dot4(*this);
        }

        inline VectorPOD Sqrt()  const
        {
            VectorPOD Result;
            Result.x = sqrtf(x);
            Result.y = sqrtf(y);
            Result.z = sqrtf(z);
            Result.w = sqrtf(w);
            return Result;
        }
        inline VectorPOD Length3()  const
        {
            VectorPOD Result;
            Result = LengthSq3();
            Result = Result.Sqrt();
            return Result;
        }
        inline VectorPOD Length4()  const
        {
            VectorPOD Result;
            Result = LengthSq4();
            Result = Result.Sqrt();
            return Result;
        }
        inline VectorPOD Normalize3()  const
        {
            float fLength;
            VectorPOD vResult;

            vResult = Length3();
            fLength = vResult.x;

            // Prevent divide by zero
            if (fLength > 0)
            {
                fLength = 1.0f / fLength;
            }

            vResult.x = x * fLength;
            vResult.y = y * fLength;
            vResult.z = z * fLength;
            vResult.w = w * fLength;
            return vResult;
        }

        inline VectorPOD Cross3(const VectorPOD& vec) const
        {
            VectorPOD vResult;
            vResult.Assign(
                (y * vec.z) - (z * vec.y),
                (z * vec.x) - (x * vec.z),
                (x * vec.y) - (y * vec.x),
                0.0f);
            return vResult;
        }
        inline VectorPOD Negate() const
        {
            VectorPOD Result;
            Result.x = -x;
            Result.y = -y;
            Result.z = -z;
            Result.w = -w;
            return Result;
        }

		inline VectorPOD operator-() const
		{
			return Negate();
		}

        inline VectorPOD MergeXY(const VectorPOD& vec) const
        {
            VectorPOD Result;
            Result.x = x;
            Result.y = vec.x;
            Result.z = y;
            Result.w = vec.y;
            return Result;
        }
        inline VectorPOD MergeZW(const VectorPOD& vec) const
        {
            VectorPOD Result;
            Result.x = z;
            Result.y = vec.z;
            Result.z = w;
            Result.w = vec.w;
            return Result;
        }
        inline VectorPOD VectorPermute(const VectorPOD& vec, uint32_t PermuteX, uint32_t PermuteY, uint32_t PermuteZ, uint32_t PermuteW ) const
        {
            const uint32_t *aPtr[2];
            aPtr[0] = (const uint32_t* )(this);
            aPtr[1] = (const uint32_t* )(&vec);

            VectorPOD Result;
            uint32_t *pWork = (uint32_t*)(&Result);

            const uint32_t i0 = PermuteX & 3;
            const uint32_t vi0 = PermuteX >> 2;
            pWork[0] = aPtr[vi0][i0];

            const uint32_t i1 = PermuteY & 3;
            const uint32_t vi1 = PermuteY >> 2;
            pWork[1] = aPtr[vi1][i1];

            const uint32_t i2 = PermuteZ & 3;
            const uint32_t vi2 = PermuteZ >> 2;
            pWork[2] = aPtr[vi2][i2];

            const uint32_t i3 = PermuteW & 3;
            const uint32_t vi3 = PermuteW >> 2;
            pWork[3] = aPtr[vi3][i3];

            return Result;
        }
        inline VectorPOD Reciprocal()
        {
            VectorPOD Result;
            Result.x = 1.f / x;
            Result.y = 1.f / y;
            Result.z = 1.f / z;
            Result.w = 1.f / w;
            return Result;
        }
    };

    class Vector : public VectorPOD
    {
    public:
        Vector()
        {
            x = 0;
            y = 0;
            z = 0;
            w = 0;
        }
        Vector(float _x, float _y, float _z, float _w )
        {
            x = _x;
            y = _y;
            z = _z;
            w = _w;
        }
        Vector(const VectorPOD& other)
        {
            operator=(other);
        }
        Vector& operator=(const VectorPOD& other)
        {
            x = other.x;
            y = other.y;
            z = other.z;
            w = other.w;
            return *this;
        }
        //---------------------------------------------------------------------------------------------

    };


    //---------------------------------------------------------------------------------------------
    class Quaternion : public Vector
    {
    public:
        Quaternion(){}
        Quaternion(const Quaternion& other){Assign(other.x, other.y, other.z, other.w);}
        Quaternion(float pitch, float yaw, float roll) {FromEuler(pitch, yaw, roll);}
        Quaternion(float _x, float _y, float _z, float _w) {Assign(_x, _y, _z, _w);}

        inline void FromEuler(float pitch, float yaw, float roll)
        {
            float cy = cosf(yaw * 0.5f);
            float sy = sinf(yaw * 0.5f);
            float cr = cosf(roll * 0.5f);
            float sr = sinf(roll * 0.5f);
            float cp = cosf(pitch * 0.5f);
            float sp = sinf(pitch * 0.5f);

            w = cp * cr * cy + sp * sr * sy;
            x = cp * sr * cy - sp * cr * sy;
            y = cp * cr * sy + sp * sr * cy;
            z = sp * cr * cy - cp * sr * sy;

        }


        inline Quaternion& operator=(const Quaternion& other)
        {
            Assign(other.x, other.y, other.z, other.w);
            return *this;
        }

        inline bool operator==(const Quaternion& other) const
        {
            return x == other.x && y == other.y && z == other.z && w == other.w;
        }

        inline bool operator!=(const Quaternion& other) const { return !operator==(other); }


        inline Quaternion operator*(const Quaternion& other) const
        {
            return Quaternion(
            other.w * x + other.x * w + other.y * z - other.z * y,
            other.w * y - other.x * z + other.y * w + other.z * x,
            other.w * z + other.x * y - other.y * x + other.z * w,
            other.w * w - other.x * x - other.y * y - other.z * z
            );
        }

        inline const Quaternion& RotateBy(const Quaternion& rotator)
        {
            *this = rotator * (*this);
            return *this;
        }

        inline Vector ToEulerAngles() const
        {
            float yaw, pitch, roll;
#if 0
        // roll (x-axis rotation)
            float sinr = 2.0f * (w * x + y * z);
            float cosr = 1.0f - 2.0f * (x * x + y * y);
            roll = atan2f(sinr, cosr);

            // pitch (y-axis rotation)
            float sinp = 2.0f * (w * y - z * x);
            if (fabsf(sinp) >= 1)
                pitch = copysignf(AMF_PIDIV2, sinp); // use 90 degrees if out of range
            else
                pitch = asinf(sinp);

            // yaw (z-axis rotation)
            float siny = 2.0f * (w * z + x * y);
            float cosy = 1.0f - 2.0f * (y * y + z * z);
            yaw = atan2f(siny, cosy);

#else
            float sqw = w*w;
            float sqx = x*x;
            float sqy = y*y;
            float sqz = z*z;
            float unit = sqx + sqy + sqz + sqw; // if normalised is one, otherwise is correction factor
            float test = x*y + z*w;
            if (test > 0.499f * unit) { // singularity at north pole
                yaw = 2.0f * atan2(x, w);
                pitch = AMF_PIDIV2;
                roll = 0;
            }else if (test < -0.499f*unit) { // singularity at south pole
                yaw = -2.0f * atan2(x, w);
                pitch = -AMF_PIDIV2;
                roll = 0;
            }
            else
            {
                yaw = atan2(2.0f * (y*w - x*z), sqx - sqy - sqz + sqw);
                pitch = asin(2.0f * test / unit);
                roll = atan2(2.0f * (x*w - y * z), -sqx + sqy - sqz + sqw);
            }
#endif
            return Vector(pitch, yaw, roll, 0);
        }
        inline Quaternion& operator-=(const Quaternion& other)
        {
            x -= other.x; y -= other.y; z -= other.z; w -= other.w;
            return *this;
        }
        inline Quaternion operator-(const Quaternion& other) const
        {
            Quaternion vector;
            vector.x = x - other.x;
            vector.y = y - other.y;
            vector.z = z - other.z;
            vector.w = w - other.w;
            return vector;
        }
        inline Quaternion& operator+=(const Quaternion& other)
        {
            x += other.x;
            y += other.y;
            z += other.z;
            w += other.w;
            return *this;
        }
        inline Quaternion operator+(const Quaternion& other)  const
        {
            Quaternion vector;
            vector.x = x + other.x;
            vector.y = y + other.y;
            vector.z = z + other.z;
            vector.w = w + other.w;
            return vector;
        }
        inline Quaternion Conjugate()  const
        {
            Quaternion result(-x, -y, -z, w);
            return result;
        }
        inline Vector DistanceAngles(const Quaternion& newValue) const
        {
            Vector diff;
            amf::Quaternion diffQ = newValue * Conjugate();
            float len = diffQ.Length4().x;

            if (len <= 0.0005f)
            {
                diff = amf::Vector(2.0f * diffQ.x, 2.0f * diffQ.y, 2.0f * diffQ.z, 0);
            }
            else
            {
                float angle = 2.0f * atan2(len, diffQ.w);
                diff = amf::Vector(diffQ.x * angle / len, diffQ.y * angle / len, diffQ.z * angle / len, 0);
            }
            return diff;
        }
    };
    inline void ScalarSinCos(float* pSin, float* pCos, float  Value)
    {
        // Map Value to y in [-pi,pi], x = 2*pi*quotient + remainder.
        float quotient = AMF_1DIV2PI *Value;
        if (Value >= 0.0f)
        {
            quotient = (float)((int)(quotient + 0.5f));
        }
        else
        {
            quotient = (float)((int)(quotient - 0.5f));
        }
        float y = Value - AMF_2PI * quotient;

        // Map y to [-pi/2,pi/2] with sin(y) = sin(Value).
        float sign;
        if (y > AMF_PIDIV2)
        {
            y = AMF_PI - y;
            sign = -1.0f;
        }
        else if (y < -AMF_PIDIV2)
        {
            y = -AMF_PI - y;
            sign = -1.0f;
        }
        else
        {
            sign = +1.0f;
        }

        float y2 = y * y;

        // 11-degree minimax approximation
        *pSin = ( ( ( ( (-2.3889859e-08f * y2 + 2.7525562e-06f) * y2 - 0.00019840874f ) * y2 + 0.0083333310f ) * y2 - 0.16666667f ) * y2 + 1.0f ) * y;

        // 10-degree minimax approximation
        float p = ( ( ( ( -2.6051615e-07f * y2 + 2.4760495e-05f ) * y2 - 0.0013888378f ) * y2 + 0.041666638f ) * y2 - 0.5f ) * y2 + 1.0f;
        *pCos = sign*p;
    }

    //---------------------------------------------------------------------------------------------
    class Matrix
    {
    public:
        union
        {
            float       m[4][4];
            VectorPOD   r[4];
            float       k[16];
        };

        Matrix() {Identity();}

        Matrix(float *_m) { memcpy(m, _m, sizeof(m));}
        Matrix(float i0, float i1, float i2, float i3, float i4, float i5, float i6, float i7,
               float i8, float i9, float i10, float i11, float i12, float i13, float i14, float i15)
                {
                    k[0] = i0;   k[1] = i1;   k[2] = i2;   k[3] = i3;
                    k[4] = i4;   k[5] = i5;   k[6] = i6;   k[7] = i7;
                    k[8] = i8;   k[9] = i9;   k[10] = i10; k[11] = i11;
                    k[12] = i12; k[13] = i13; k[14] = i14; k[15] = i15;
                }

        inline void Identity()
        {
            k[0] = k[5] = k[10] = k[15] = 1.0f;
            k[1] = k[2] = k[3] = k[4] = k[6] = k[7] = k[8] = k[9] = k[11] = k[12] = k[13] = k[14] = 0.0f;

        }
        inline Matrix &operator=(const Matrix & other)
        {
            memcpy(m, other.m, sizeof(m));
            return *this;
        }

        inline Matrix operator*(const Matrix& n) const
        {
            return Matrix(
                     k[0]*n.k[0]  + k[4]*n.k[1]  + k[8]*n.k[2]  + k[12]*n.k[3],   k[1]*n.k[0]  + k[5]*n.k[1]  + k[9]*n.k[2]  + k[13]*n.k[3],   k[2]*n.k[0]  + k[6]*n.k[1]  + k[10]*n.k[2]  + k[14]*n.k[3],   k[3]*n.k[0]  + k[7]*n.k[1]  + k[11]*n.k[2]  + k[15]*n.k[3],
                     k[0]*n.k[4]  + k[4]*n.k[5]  + k[8]*n.k[6]  + k[12]*n.k[7],   k[1]*n.k[4]  + k[5]*n.k[5]  + k[9]*n.k[6]  + k[13]*n.k[7],   k[2]*n.k[4]  + k[6]*n.k[5]  + k[10]*n.k[6]  + k[14]*n.k[7],   k[3]*n.k[4]  + k[7]*n.k[5]  + k[11]*n.k[6]  + k[15]*n.k[7],
                     k[0]*n.k[8]  + k[4]*n.k[9]  + k[8]*n.k[10] + k[12]*n.k[11],  k[1]*n.k[8]  + k[5]*n.k[9]  + k[9]*n.k[10] + k[13]*n.k[11],  k[2]*n.k[8]  + k[6]*n.k[9]  + k[10]*n.k[10] + k[14]*n.k[11],  k[3]*n.k[8]  + k[7]*n.k[9]  + k[11]*n.k[10] + k[15]*n.k[11],
                     k[0]*n.k[12] + k[4]*n.k[13] + k[8]*n.k[14] + k[12]*n.k[15],  k[1]*n.k[12] + k[5]*n.k[13] + k[9]*n.k[14] + k[13]*n.k[15],  k[2]*n.k[12] + k[6]*n.k[13] + k[10]*n.k[14] + k[14]*n.k[15],  k[3]*n.k[12] + k[7]*n.k[13] + k[11]*n.k[14] + k[15]*n.k[15]);
        }
        inline Matrix operator*=(const Matrix& other)
        {
            *this = *this * other;
            return *this;
        }

        inline bool operator==(const Matrix& other) const
        {
            return memcmp(this, &other, sizeof(*this)) == 0;
        }
        inline bool operator!=(const Matrix& other) const
        {
            return memcmp(this, &other, sizeof(*this)) != 0;
        }

        inline Vector operator*(const Vector& v) const
        {
            Vector Z(v.z, v.z, v.z, v.z);
            Vector Y(v.y, v.y, v.y, v.y);
            Vector X(v.x, v.x, v.x, v.x);

            Vector ret;
            ret = Z * r[2] + r[3];
            ret = Y * r[1] + ret;
            ret = X * r[0] + ret;

            return ret;
        }

        void MatrixAffineTransformation(const Vector &Scaling, const Vector &RotationOrigin, const Vector &RotationQuaternion, const Vector &Translation)
        {
            // M = MScaling * Inverse(MRotationOrigin) * MRotation * MRotationOrigin * MTranslation;

            MatrixScalingFromVector(Scaling);
            Vector VRotationOrigin (RotationOrigin.x,RotationOrigin.y,RotationOrigin.z, 0);
            Matrix MRotation;
            MRotation.MatrixRotationQuaternion(RotationQuaternion);
            Vector VTranslation (Translation.x, Translation.y, Translation.z, 0);

            r[3] -= VRotationOrigin;
            *this *= MRotation;
            r[3] += VRotationOrigin;
            r[3] += VTranslation;
        }
        inline void MatrixScalingFromVector(const Vector& Scale)
        {
            m[0][0] = Scale.x;
            m[1][1] = Scale.y;
            m[2][2] = Scale.z;
            m[3][3] = 1.0f;

        }
        void MatrixRotationQuaternion(const Vector& Quaternion)
        {
            static const Vector Constant1110 = {1.0f, 1.0f, 1.0f, 0.0f};

            Vector Q0 = Quaternion + Quaternion;
            Vector Q1 = Quaternion * Q0;

            Vector V0 = Q1.VectorPermute(Constant1110, AMF_PERMUTE_0Y, AMF_PERMUTE_0X, AMF_PERMUTE_0X, AMF_PERMUTE_1W);
            Vector V1 = Q1.VectorPermute(Constant1110, AMF_PERMUTE_0Z, AMF_PERMUTE_0Z, AMF_PERMUTE_0Y, AMF_PERMUTE_1W);
            Vector R0 = Constant1110 - V0;
            R0 = R0 - V1;

            V0 = Quaternion.Swizzle(AMF_SWIZZLE_X, AMF_SWIZZLE_X, AMF_SWIZZLE_Y, AMF_SWIZZLE_W);
            V1 = Q0.Swizzle(AMF_SWIZZLE_Z, AMF_SWIZZLE_Y, AMF_SWIZZLE_Z, AMF_SWIZZLE_W);
            V0 = V0 * V1;

            V1 = Vector(Quaternion.w, Quaternion.w, Quaternion.w, Quaternion.w);
            Vector V2 = Q0.Swizzle(AMF_SWIZZLE_Y, AMF_SWIZZLE_Z, AMF_SWIZZLE_X, AMF_SWIZZLE_W);
            V1 = V1 * V2;

            Vector R1 = V0 + V1;
            Vector R2 = V0 - V1;

            V0 = R1.VectorPermute(R2, AMF_PERMUTE_0Y, AMF_PERMUTE_1X, AMF_PERMUTE_1Y, AMF_PERMUTE_0Z);
            V1 = R1.VectorPermute(R2, AMF_PERMUTE_0X, AMF_PERMUTE_1Z, AMF_PERMUTE_0X, AMF_PERMUTE_1Z);

            r[0] = R0.VectorPermute(V0, AMF_PERMUTE_0X, AMF_PERMUTE_1X, AMF_PERMUTE_1Y, AMF_PERMUTE_0W);
            r[1] = R0.VectorPermute(V0, AMF_PERMUTE_1Z, AMF_PERMUTE_0Y, AMF_PERMUTE_1W, AMF_PERMUTE_0W);
            r[2] = R0.VectorPermute(V1, AMF_PERMUTE_1X, AMF_PERMUTE_1Y, AMF_PERMUTE_0Z, AMF_PERMUTE_0W);
            r[3] = Vector(0.0f, 0.0f, 0.0f, 1.0f);
        }
        inline void LookToLH(const Vector& EyePosition, const Vector& EyeDirection, const Vector& UpDirection)
        {
            Vector R2 = EyeDirection.Normalize3();

            Vector R0 = UpDirection.Cross3(R2);
            R0 = R0.Normalize3();

            Vector R1 = R2.Cross3(R0);

            Vector NegEyePosition = EyePosition.Negate();

            Vector D0 = R0.Dot3(NegEyePosition);
            Vector D1 = R1.Dot3(NegEyePosition);
            Vector D2 = R2.Dot3(NegEyePosition);

            Matrix M;
            M.r[0] = Vector(R0.x, R0.y, R0.z, D0.w);
            M.r[1] = Vector(R1.x, R1.y, R1.z, D1.w);
            M.r[2] = Vector(R2.x, R2.y, R2.z, D2.w);
            M.r[3] = Vector(0.0f, 0.0f, 0.0f, 1.0f);

            *this = M.Transpose();
        }
        inline Matrix Transpose() const
        {

            // Original matrix:
            //
            //     m00m01m02m03
            //     m10m11m12m13
            //     m20m21m22m23
            //     m30m31m32m33

            Matrix P;
            P.r[0] = r[0].MergeXY(r[2]); // m00m20m01m21
            P.r[1] = r[1].MergeXY(r[3]); // m10m30m11m31
            P.r[2] = r[0].MergeZW(r[2]); // m02m22m03m23
            P.r[3] = r[1].MergeZW(r[3]); // m12m32m13m33

            Matrix MT;
            MT.r[0] = P.r[0].MergeXY(P.r[1]); // m00m10m20m30
            MT.r[1] = P.r[0].MergeZW(P.r[1]); // m01m11m21m31
            MT.r[2] = P.r[2].MergeXY(P.r[3]); // m02m12m22m32
            MT.r[3] = P.r[2].MergeZW(P.r[3]); // m03m13m23m33
            return MT;
        }
        inline void LookAtLH(Vector& EyePosition, Vector& FocusPosition, Vector& UpDirection)
        {
            Vector EyeDirection = FocusPosition - EyePosition;
            LookToLH(EyePosition, EyeDirection, UpDirection);
        }
        inline void PerspectiveFovLH(float FovAngleY, float AspectRatio, float NearZ, float FarZ)
        {
            float    SinFov;
            float    CosFov;
            ScalarSinCos(&SinFov, &CosFov, 0.5f * FovAngleY);

            float Height = CosFov / SinFov;
            float Width = Height / AspectRatio;
            float fRange = FarZ / (FarZ-NearZ);

            m[0][0] = Width;
            m[0][1] = 0.0f;
            m[0][2] = 0.0f;
            m[0][3] = 0.0f;

            m[1][0] = 0.0f;
            m[1][1] = Height;
            m[1][2] = 0.0f;
            m[1][3] = 0.0f;

            m[2][0] = 0.0f;
            m[2][1] = 0.0f;
            m[2][2] = fRange;
            m[2][3] = 1.0f;

            m[3][0] = 0.0f;
            m[3][1] = 0.0f;
            m[3][2] = -fRange * NearZ;
            m[3][3] = 0.0f;
        }
        inline void RotationRollPitchYaw(float Pitch, float Yaw, float Roll)
        {
            Quaternion Q;
            Q.FromEuler( Pitch, Yaw, Roll);
            MatrixAffineTransformation(Vector(1.f, 1.f, 1.f, 0.f), Vector(), Q, Vector());
        }
        inline Vector Determinant()
        {
            static const Vector Sign (1.0f, -1.0f, 1.0f, -1.0f);

            Vector V0(r[2].y, r[2].x, r[2].x, r[2].x);
            Vector V1(r[3].z, r[3].z, r[3].y, r[3].y);
            Vector V2(r[2].y, r[2].x, r[2].x, r[2].x);
            Vector V3(r[3].w, r[3].w, r[3].w, r[3].z);
            Vector V4(r[2].z, r[2].z, r[2].y, r[2].y);
            Vector V5(r[3].w, r[3].w, r[3].w, r[3].z);

            Vector P0 = V0 * V1;
            Vector P1 = V2 * V3;
            Vector P2 = V4 * V5;

            V0 = Vector(r[2].z, r[2].z, r[2].y, r[2].y);
            V1 = Vector(r[3].y, r[3].x, r[3].x, r[3].x);
            V2 = Vector(r[2].w, r[2].w, r[2].w, r[2].z);
            V3 = Vector(r[3].y, r[3].x, r[3].x, r[3].x);
            V4 = Vector(r[2].w, r[2].w, r[2].w, r[2].z);
            V5 = Vector(r[3].z, r[3].z, r[3].y, r[3].y);

            P0 -= V0 * V1;
            P1 -= V2 * V3;
            P2 -= V4 * V5;

            V0 = Vector(r[1].w, r[1].w, r[1].w, r[1].z);
            V1 = Vector(r[1].z, r[1].z, r[1].y, r[1].y);
            V2 = Vector(r[1].y, r[1].x, r[1].x, r[1].x);


            Vector S = r[0] * Sign;
            Vector R = V0 * P0;
            R -= V1 * P1;
            R += V2 * P2;

            return S.Dot4(R);
        }
#define XM3RANKDECOMPOSE(a, b, c, x, y, z)      \
    if((x) < (y))                   \
    {                               \
        if((y) < (z))               \
        {                           \
            (a) = 2;                \
            (b) = 1;                \
            (c) = 0;                \
        }                           \
        else                        \
        {                           \
            (a) = 1;                \
                                    \
            if((x) < (z))           \
            {                       \
                (b) = 2;            \
                (c) = 0;            \
            }                       \
            else                    \
            {                       \
                (b) = 0;            \
                (c) = 2;            \
            }                       \
        }                           \
    }                               \
    else                            \
    {                               \
        if((x) < (z))               \
        {                           \
            (a) = 2;                \
            (b) = 0;                \
            (c) = 1;                \
        }                           \
        else                        \
        {                           \
            (a) = 0;                \
                                    \
            if((y) < (z))           \
            {                       \
                (b) = 2;            \
                (c) = 1;            \
            }                       \
            else                    \
            {                       \
                (b) = 1;            \
                (c) = 2;            \
            }                       \
        }                           \
    }

#define XM3_DECOMP_EPSILON 0.0001f

        inline amf::Quaternion ConvertMatrixToQuat()
        {
            amf::Quaternion q;
            float r22 = m[2][2];
            if (r22 <= 0.f)  // x^2 + y^2 >= z^2 + w^2
            {
                float dif10 = m[1][1] - m[0][0];
                float omr22 = 1.f - r22;
                if (dif10 <= 0.f)  // x^2 >= y^2
                {
                    float fourXSqr = omr22 - dif10;
                    float inv4x = 0.5f / sqrtf(fourXSqr);
                    q.x = fourXSqr*inv4x;
                    q.y = (m[0][1] + m[1][0])*inv4x;
                    q.z = (m[0][2] + m[2][0])*inv4x;
                    q.w = (m[1][2] - m[2][1])*inv4x;
                }
                else  // y^2 >= x^2
                {
                    float fourYSqr = omr22 + dif10;
                    float inv4y = 0.5f / sqrtf(fourYSqr);
                    q.x = (m[0][1] + m[1][0])*inv4y;
                    q.y = fourYSqr*inv4y;
                    q.z = (m[1][2] + m[2][1])*inv4y;
                    q.w = (m[2][0] - m[0][2])*inv4y;
                }
            }
            else  // z^2 + w^2 >= x^2 + y^2
            {
                float sum10 = m[1][1] + m[0][0];
                float opr22 = 1.f + r22;
                if (sum10 <= 0.f)  // z^2 >= w^2
                {
                    float fourZSqr = opr22 - sum10;
                    float inv4z = 0.5f / sqrtf(fourZSqr);
                    q.x = (m[0][2] + m[2][0])*inv4z;
                    q.y = (m[1][2] + m[2][1])*inv4z;
                    q.z = fourZSqr*inv4z;
                    q.w = (m[0][1] - m[1][0])*inv4z;
                }
                else  // w^2 >= z^2
                {
                    float fourWSqr = opr22 + sum10;
                    float inv4w = 0.5f / sqrtf(fourWSqr);
                    q.x = (m[1][2] - m[2][1])*inv4w;
                    q.y = (m[2][0] - m[0][2])*inv4w;
                    q.z = (m[0][1] - m[1][0])*inv4w;
                    q.w = fourWSqr*inv4w;
                }
            }
            return q;
        }
        inline bool DecomposeMatrix(amf::Quaternion &q, amf::Vector &p, amf::Vector &s)
        {
            static amf::Vector amfXMIdentityR0( 1.0f, 0.0f, 0.0f, 0.0f );
            static amf::Vector amfXMIdentityR1( 0.0f, 1.0f, 0.0f, 0.0f );
            static amf::Vector amfXMIdentityR2( 0.0f, 0.0f, 1.0f, 0.0f );
            static const amf::VectorPOD *pvCanonicalBasis[3] = {
                    &amfXMIdentityR0,
                    &amfXMIdentityR1,
                    &amfXMIdentityR2
            };

//    p.Assign(-m.m[0][3], -m.m[1][3], -m.m[2][3], 0);
            p = r[3];

            amf::VectorPOD *ppvBasis[3];
            amf::Matrix matTemp;
            ppvBasis[0] = &matTemp.r[0];
            ppvBasis[1] = &matTemp.r[1];
            ppvBasis[2] = &matTemp.r[2];

            matTemp.r[0] = r[0];
            matTemp.r[1] = r[1];
            matTemp.r[2] = r[2];
            matTemp.r[3] = amf::Vector(0.0f, 0.0f, 0.0f, 1.0f );


            float *pfScales = (float*)&s;

            size_t a, b, c;
            pfScales[0] = ppvBasis[0][0].Length3().x;
            pfScales[1] = ppvBasis[1][0].Length3().x;
            pfScales[2] = ppvBasis[2][0].Length3().x;
            pfScales[3] = 0.f;

            XM3RANKDECOMPOSE(a, b, c, pfScales[0], pfScales[1], pfScales[2])

            if(pfScales[a] < XM3_DECOMP_EPSILON)
            {
                ppvBasis[a][0] = pvCanonicalBasis[a][0];
            }
            ppvBasis[a][0] = ppvBasis[a][0].Normalize3();

            if(pfScales[b] < XM3_DECOMP_EPSILON)
            {
                size_t aa, bb, cc;
                float fAbsX, fAbsY, fAbsZ;

                fAbsX = fabsf(ppvBasis[a][0].x);
                fAbsY = fabsf(ppvBasis[a][0].y);
                fAbsZ = fabsf(ppvBasis[a][0].z);

                XM3RANKDECOMPOSE(aa, bb, cc, fAbsX, fAbsY, fAbsZ)

                ppvBasis[b][0] = ppvBasis[a][0].Cross3(pvCanonicalBasis[cc][0]);
            }

            ppvBasis[b][0] = ppvBasis[b][0].Normalize3();

            if(pfScales[c] < XM3_DECOMP_EPSILON)
            {
                ppvBasis[c][0] = ppvBasis[a][0].Cross3(ppvBasis[b][0]);
            }

            ppvBasis[c][0] = ppvBasis[c][0].Normalize3();


            float fDet = matTemp.Determinant().x;

            // use Kramer's rule to check for handedness of coordinate system
            if(fDet < 0.0f)
            {
                // switch coordinate system by negating the scale and inverting the basis vector on the x-axis
                pfScales[a] = -pfScales[a];
                ppvBasis[a][0] = ppvBasis[a][0].Negate();

                fDet = -fDet;
            }

            fDet -= 1.0f;
            fDet *= fDet;

            if(XM3_DECOMP_EPSILON < fDet)
            {
                // Non-SRT matrix encountered
                return false;
            }

            q = matTemp.ConvertMatrixToQuat();
            return true;
        }
        inline Matrix Inverse(Vector *pDeterminant)
        {

            float A2323 = m[2][2] * m[3][3] - m[2][3] * m[3][2];
            float A1323 = m[2][1] * m[3][3] - m[2][3] * m[3][1];
            float A1223 = m[2][1] * m[3][2] - m[2][2] * m[3][1];
            float A0323 = m[2][0] * m[3][3] - m[2][3] * m[3][0];
            float A0223 = m[2][0] * m[3][2] - m[2][2] * m[3][0];
            float A0123 = m[2][0] * m[3][1] - m[2][1] * m[3][0];
            float A2313 = m[1][2] * m[3][3] - m[1][3] * m[3][2];
            float A1313 = m[1][1] * m[3][3] - m[1][3] * m[3][1];
            float A1213 = m[1][1] * m[3][2] - m[1][2] * m[3][1];
            float A2312 = m[1][2] * m[2][3] - m[1][3] * m[2][2];
            float A1312 = m[1][1] * m[2][3] - m[1][3] * m[2][1];
            float A1212 = m[1][1] * m[2][2] - m[1][2] * m[2][1];
            float A0313 = m[1][0] * m[3][3] - m[1][3] * m[3][0];
            float A0213 = m[1][0] * m[3][2] - m[1][2] * m[3][0];
            float A0312 = m[1][0] * m[2][3] - m[1][3] * m[2][0];
            float A0212 = m[1][0] * m[2][2] - m[1][2] * m[2][0];
            float A0113 = m[1][0] * m[3][1] - m[1][1] * m[3][0];
            float A0112 = m[1][0] * m[2][1] - m[1][1] * m[2][0];

            float det =
              m[0][0] * (m[1][1] * A2323 - m[1][2] * A1323 + m[1][3] * A1223)
            - m[0][1] * (m[1][0] * A2323 - m[1][2] * A0323 + m[1][3] * A0223)
            + m[0][2] * (m[1][0] * A1323 - m[1][1] * A0323 + m[1][3] * A0123)
            - m[0][3] * (m[1][0] * A1223 - m[1][1] * A0223 + m[1][2] * A0123);
            det = 1.0f / det;
            Matrix ret;
            ret.m[0][0] = det *  (m[1][1] * A2323 - m[1][2] * A1323 + m[1][3] * A1223);
            ret.m[0][1] = det * -(m[0][1] * A2323 - m[0][2] * A1323 + m[0][3] * A1223);
            ret.m[0][2] = det *  (m[0][1] * A2313 - m[0][2] * A1313 + m[0][3] * A1213);
            ret.m[0][3] = det * -(m[0][1] * A2312 - m[0][2] * A1312 + m[0][3] * A1212);
            ret.m[1][0] = det * -(m[1][0] * A2323 - m[1][2] * A0323 + m[1][3] * A0223);
            ret.m[1][1] = det *  (m[0][0] * A2323 - m[0][2] * A0323 + m[0][3] * A0223);
            ret.m[1][2] = det * -(m[0][0] * A2313 - m[0][2] * A0313 + m[0][3] * A0213);
            ret.m[1][3] = det *  (m[0][0] * A2312 - m[0][2] * A0312 + m[0][3] * A0212);
            ret.m[2][0] = det *  (m[1][0] * A1323 - m[1][1] * A0323 + m[1][3] * A0123);
            ret.m[2][1] = det * -(m[0][0] * A1323 - m[0][1] * A0323 + m[0][3] * A0123);
            ret.m[2][2] = det *  (m[0][0] * A1313 - m[0][1] * A0313 + m[0][3] * A0113);
            ret.m[2][3] = det * -(m[0][0] * A1312 - m[0][1] * A0312 + m[0][3] * A0112);
            ret.m[3][0] = det * -(m[1][0] * A1223 - m[1][1] * A0223 + m[1][2] * A0123);
            ret.m[3][1] = det *  (m[0][0] * A1223 - m[0][1] * A0223 + m[0][2] * A0123);
            ret.m[3][2] = det * -(m[0][0] * A1213 - m[0][1] * A0213 + m[0][2] * A0113);
            ret.m[3][3] = det *  (m[0][0] * A1212 - m[0][1] * A0212 + m[0][2] * A0112);

            if (pDeterminant != nullptr)
            {
                *pDeterminant = Vector(det,det,det,det);
            }
            return ret;
        }

    };

    class Pose
    {
    public:
        typedef enum ValidityFlagBits
        {
            // validity flags
            PF_NONE                     = 0x0000,
            PF_ORIENTATION              = 0x0001,
            PF_POSITION                 = 0x0002,
            PF_ORIENTATION_VELOCITY     = 0x0004,
            PF_POSITION_VELOCITY        = 0x0008,
            PF_ORIENTATION_ACCELERATION = 0x0010,
            PF_POSITION_ACCELERATION    = 0x0020,
            PF_COORDINATES              = PF_ORIENTATION | PF_POSITION,
            PF_VELOCITY                 = PF_ORIENTATION_VELOCITY | PF_POSITION_VELOCITY,
            PF_ACCELERATION             = PF_ORIENTATION_ACCELERATION | PF_POSITION_ACCELERATION,
        } ValidityFlagBits;
        typedef uint32_t ValidityFlags;

        Pose() : m_ValidityFlags(PF_NONE){}
        Pose(const Pose &other) { *this = other; }

        Pose(const amf::Quaternion& orientation, const amf::Vector& position)
        {
            Set(orientation, position);
        }
        Pose(const amf::Quaternion& orientation, const amf::Vector& position,
            const amf::Vector& orientationVelocity, const amf::Vector& positionVelocity)
        {
            Set(orientation, position, orientationVelocity, positionVelocity);
        }
        Pose(const amf::Quaternion& orientation, const amf::Vector& position,
            const amf::Vector& orientationVelocity, const amf::Vector& positionVelocity,
            const amf::Vector& orientationAcceleration, const amf::Vector& positionAcceleration)
        {
            Set(orientation, position, orientationVelocity, positionVelocity, orientationAcceleration, positionAcceleration);
        }
        void Set(const amf::Quaternion& orientation, const amf::Vector& position)
        {
            m_Orientation = orientation;
            m_Position = position;
            m_ValidityFlags = PF_COORDINATES;
        }
        void Set(const amf::Quaternion& orientation, const amf::Vector& position,
            const amf::Vector& orientationVelocity, const amf::Vector& positionVelocity)
        {
            m_Orientation = orientation;
            m_Position = position;
            m_OrientationVelocity = orientationVelocity;
            m_PositionVelocity = positionVelocity;
            m_ValidityFlags = PF_COORDINATES | PF_VELOCITY;
        }

        void Set(const amf::Quaternion& orientation, const amf::Vector& position,
            const amf::Vector& orientationVelocity, const amf::Vector& positionVelocity,
            const amf::Vector& orientationAcceleration, const amf::Vector& positionAcceleration)
        {
            m_Orientation = orientation;
            m_Position = position;
            m_OrientationVelocity = orientationVelocity;
            m_PositionVelocity = positionVelocity;
            m_OrientationAcceleration = orientationAcceleration;
            m_PositionAcceleration = positionAcceleration;
            m_ValidityFlags = PF_COORDINATES | PF_VELOCITY | PF_ACCELERATION;
        }


        inline const amf::Quaternion&   GetOrientation() const { return m_Orientation; }
        inline const amf::Vector&   GetPosition() const { return m_Position; }
        inline const amf::Vector&   GetOrientationVelocity() const { return m_OrientationVelocity; }
        inline const amf::Vector&   GetPositionVelocity() const { return m_PositionVelocity; }
        inline const amf::Vector&   GetOrientationAcceleration() const { return m_OrientationAcceleration; }
        inline const amf::Vector&   GetPositionAcceleration() const { return m_PositionAcceleration; }
        inline ValidityFlags        GetValidityFlags() const { return m_ValidityFlags; }

        inline void   SetOrientation(const amf::Quaternion& orienation)
        {
            m_Orientation = orienation;
            m_ValidityFlags |= PF_ORIENTATION;
        }
        inline void       SetPosition(const amf::Vector& position)
        {
            m_Position = position;
            m_ValidityFlags |= PF_POSITION;
        }
        inline void       SetOrientationVelocity(const amf::Vector& orientationVelocity)
        {
            m_OrientationVelocity = orientationVelocity;
            m_ValidityFlags |= PF_ORIENTATION_VELOCITY;
        }
        inline void       SetPositionVelocity(const amf::Vector& positionVelocity)
        {
            m_PositionVelocity = positionVelocity;
            m_ValidityFlags |= PF_POSITION_VELOCITY;
        }
        inline void       SetOrientationAcceleration(const amf::Vector& orientationAcceleration)
        {
            m_OrientationAcceleration = orientationAcceleration;
            m_ValidityFlags |= PF_ORIENTATION_ACCELERATION;
        }
        inline void       SetPositionAcceleration(const amf::Vector& positionAcceleration)
        {
            m_PositionAcceleration = positionAcceleration;
            m_ValidityFlags |= PF_POSITION_ACCELERATION;
        }

    protected:
        amf::Quaternion                 m_Orientation;
        amf::Vector                     m_Position;
        amf::Vector                     m_OrientationVelocity;
        amf::Vector                     m_PositionVelocity;
        amf::Vector                     m_OrientationAcceleration;
        amf::Vector                     m_PositionAcceleration;
        ValidityFlags                   m_ValidityFlags;
    };

	//-------------------------------------------------------------------------------------------------
	template <typename T>
	class AlphaFilter
	{
	public:
		AlphaFilter(T alpha) :
			m_Alpha(alpha),
			m_FilteredValue(0)
		{
		}

		T Apply(T value)
		{
			m_FilteredValue = m_FilteredValue + m_Alpha * (value - m_FilteredValue);
			return m_FilteredValue;
		}

	private:
		T m_Alpha;
		T m_FilteredValue;
	};

	//-------------------------------------------------------------------------------------------------
	template <typename T>
	class AlphaBetaFilter
	{
	public:
		AlphaBetaFilter(T alpha, T beta) :
			m_Alpha(alpha),
			m_Beta(beta),
			m_Value(0),
			m_PrevValue(0),
			m_Velocity(0)
		{
		}

		T Apply(T value, T dt)
		{
			m_PrevValue = m_Value;

			m_Value += m_Velocity * dt;
			T rk = value - m_Value;
			m_Value += m_Alpha * rk;
			m_Velocity += (m_Beta * rk) / dt;

			return m_Value;
		}

		inline T GetVelocity() const { return m_Velocity; }

	private:
		T	m_Alpha,
			m_Beta;
		T	m_Value,
			m_PrevValue,
			m_Velocity;
	};

	//-------------------------------------------------------------------------------------------------
	template <typename T>
	class ThresholdFilter
	{
	public:
		ThresholdFilter(T threshold) :
			m_Threshold(threshold)
		{
		}

		T Apply(T value) const
		{
			T result = value;
			if (std::abs(value) < m_Threshold)
			{
				result = T(0);
			}
			return result;
		}

	private:
		T	m_Threshold;
	};

	//-------------------------------------------------------------------------------------------------
	class Derivative
	{
	public:
		Derivative() {}

		inline static float Calculate(float newVal, float oldVal, float dt)
		{
			return (newVal - oldVal) / dt;
		}

		inline static float Calculate(float dx, float dt)
		{
			return dx / dt;
		}

		static amf::Vector Calculate(const amf::Vector& newVal, const amf::Vector& oldVal, float dt)
		{
			amf::Vector result;
			result.w = Calculate(newVal.w, oldVal.w, dt);
			result.x = Calculate(newVal.x, oldVal.x, dt);
			result.y = Calculate(newVal.y, oldVal.y, dt);
			result.z = Calculate(newVal.z, oldVal.z, dt);
			return result;
		}

		static amf::Vector Calculate(const amf::Vector& dx, float dt)
		{
			amf::Vector result;
			result.w = Calculate(dx.w, dt);
			result.x = Calculate(dx.x, dt);
			result.y = Calculate(dx.y, dt);
			result.z = Calculate(dx.z, dt);
			return result;
		}
	};



    //---------------------------------------------------------------------------------------------
} // namespace amf