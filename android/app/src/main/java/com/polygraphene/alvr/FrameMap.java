package com.polygraphene.alvr;

import java.util.concurrent.atomic.AtomicLongArray;

// Stores mapping of presentationTime to frameIndex for tracking frameIndex on decoding.
public class FrameMap
{
    private final AtomicLongArray mFakeFrameHashMap = new AtomicLongArray(4096);

    public void put(long presentationTime, long frameIndex) {
        mFakeFrameHashMap.set((int)(presentationTime & (4096 - 1)), frameIndex);
    }

    public long find(long presentationTime) {
        return mFakeFrameHashMap.getAndSet((int)(presentationTime & (4096 - 1)), -1);
    }
}
