package com.polygraphene.alvr;

public class LatencyCollector {
    static {
        System.loadLibrary("alvr_client");
    }
    public static native void DecoderInput(long frameIndex);
    public static native void DecoderOutput(long frameIndex);
}
