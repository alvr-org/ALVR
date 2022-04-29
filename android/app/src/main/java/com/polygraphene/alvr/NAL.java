package com.polygraphene.alvr;

public class NAL {
    public int length;
    public long frameIndex;
    public byte[] buf;
    public int type;

    public NAL(int length) {
        this.length = length;
        this.buf = new byte[length];
    }
}
