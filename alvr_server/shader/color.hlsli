//*********************************************************
//
// Copyright (c) Microsoft. All rights reserved.
// This code is licensed under the MIT License (MIT).
// THIS CODE IS PROVIDED *AS IS* WITHOUT WARRANTY OF
// ANY KIND, EITHER EXPRESS OR IMPLIED, INCLUDING ANY
// IMPLIED WARRANTIES OF FITNESS FOR A PARTICULAR
// PURPOSE, MERCHANTABILITY, OR NON-INFRINGEMENT.
//
//*********************************************************

// These values must match the DisplayCurve enum in D3D12HDR.h.
#define DISPLAY_CURVE_SRGB      0
#define DISPLAY_CURVE_ST2084    1
#define DISPLAY_CURVE_LINEAR    2

float3 xyYToRec709(float2 xy, float Y = 1.0)
{
    // https://github.com/ampas/aces-dev/blob/v1.0.3/transforms/ctl/README-MATRIX.md
    static const float3x3 XYZtoRGB =
    {
        3.2409699419, -1.5373831776, -0.4986107603,
        -0.9692436363, 1.8759675015, 0.0415550574,
        0.0556300797, -0.2039769589, 1.0569715142
    };
    float3 XYZ = Y * float3(xy.x / xy.y, 1.0, (1.0 - xy.x - xy.y) / xy.y);
    float3 RGB = mul(XYZtoRGB, XYZ);
    float maxChannel = max(RGB.r, max(RGB.g, RGB.b));
    return RGB / max(maxChannel, 1.0);
}

float3 xyYToRec2020(float2 xy, float Y = 1.0)
{
    // https://github.com/ampas/aces-dev/blob/v1.0.3/transforms/ctl/README-MATRIX.md
    static const float3x3 XYZtoRGB =
    {
        1.7166511880, -0.3556707838, -0.2533662814,
        -0.6666843518, 1.6164812366, 0.0157685458,
        0.0176398574, -0.0427706133, 0.9421031212
    };
    float3 XYZ = Y * float3(xy.x / xy.y, 1.0, (1.0 - xy.x - xy.y) / xy.y);
    float3 RGB = mul(XYZtoRGB, XYZ);
    float maxChannel = max(RGB.r, max(RGB.g, RGB.b));
    return RGB / max(maxChannel, 1.0);
}

float3 LinearToSRGB(float3 color)
{
    // Approximately pow(color, 1.0 / 2.2)
    return color < 0.0031308 ? 12.92 * color : 1.055 * pow(abs(color), 1.0 / 2.4) - 0.055;
}

float3 SRGBToLinear(float3 color)
{
    // Approximately pow(color, 2.2)
    return color < 0.04045 ? color / 12.92 : pow(abs(color + 0.055) / 1.055, 2.4);
}

float3 Rec709ToRec2020(float3 color)
{
    static const float3x3 conversion =
    {
        0.627402, 0.329292, 0.043306,
        0.069095, 0.919544, 0.011360,
        0.016394, 0.088028, 0.895578
    };
    return mul(conversion, color);
}

float3 Rec2020ToRec709(float3 color)
{
    static const float3x3 conversion =
    {
        1.660496, -0.587656, -0.072840,
        -0.124547, 1.132895, -0.008348,
        -0.018154, -0.100597, 1.118751
    };
    return mul(conversion, color);
}

float3 LinearToST2084(float3 color)
{
    float m1 = 2610.0 / 4096.0 / 4;
    float m2 = 2523.0 / 4096.0 * 128;
    float c1 = 3424.0 / 4096.0;
    float c2 = 2413.0 / 4096.0 * 32;
    float c3 = 2392.0 / 4096.0 * 32;
    float3 cp = pow(abs(color), m1);
    return pow((c1 + c2 * cp) / (1 + c3 * cp), m2);
}
