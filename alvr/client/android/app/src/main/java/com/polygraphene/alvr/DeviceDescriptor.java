package com.polygraphene.alvr;

public class DeviceDescriptor {
    public static final int REFRESH_RATE_COUNT = 4;

    public static final int ALVR_DEVICE_TYPE_UNKNOWN = 0;
    public static final int ALVR_DEVICE_TYPE_OCULUS_MOBILE = 1;
    public static final int ALVR_DEVICE_TYPE_DAYDREAM = 2;
    public static final int ALVR_DEVICE_TYPE_CARDBOARD = 3;


    public static final int ALVR_DEVICE_SUBTYPE_OCULUS_MOBILE_GEARVR = 1;
    public static final int ALVR_DEVICE_SUBTYPE_OCULUS_MOBILE_GO = 2;
    public static final int ALVR_DEVICE_SUBTYPE_OCULUS_MOBILE_QUEST = 3;

    public static final int ALVR_DEVICE_SUBTYPE_DAYDREAM_GENERIC = 1;
    public static final int ALVR_DEVICE_SUBTYPE_DAYDREAM_MIRAGE_SOLO = 2;

    public static final int ALVR_DEVICE_SUBTYPE_CARDBOARD_GENERIC = 1;

    public static final int ALVR_DEVICE_CAPABILITY_FLAG_HMD_6DOF = 1 << 0;

    public static final int ALVR_CONTROLLER_CAPABILITY_FLAG_ONE_CONTROLLER = 1 << 0;
    public static final int ALVR_CONTROLLER_CAPABILITY_FLAG_TWO_CONTROLLERS = 1 << 1;
    public static final int ALVR_CONTROLLER_CAPABILITY_FLAG_6DOF = 1 << 2;

    public int[] mRefreshRates = new int[REFRESH_RATE_COUNT];
    public int mRenderWidth;
    public int mRenderHeight;
    public float[] mFov = new float[8]; // [left, right, top, bottom] * 2
    public int mDeviceType;
    public int mDeviceSubType;
    public int mDeviceCapabilityFlags;
    public int mControllerCapabilityFlags;
}
