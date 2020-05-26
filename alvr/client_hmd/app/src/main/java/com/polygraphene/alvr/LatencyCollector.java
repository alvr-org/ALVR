package com.polygraphene.alvr;

public class LatencyCollector {
    static {
        System.loadLibrary("native-lib");
    }
    public static native void DecoderInput(long frameIndex);
    public static native void DecoderOutput(long frameIndex);
    public static native void Submit(long frameIndex);
}
