package com.polygraphene.alvr;

import android.graphics.SurfaceTexture;
import android.media.MediaCodec;
import android.support.annotation.NonNull;
import android.util.Log;

import java.util.LinkedList;
import java.util.Queue;

public class OutputFrameQueue {
    private static final String TAG = "OutputFrameQueue";

    private boolean mStopped = false;

    private class Element {
        public int index;
        public long frameIndex;
    }

    private Queue<Element> mQueue = new LinkedList<>();
    private Queue<Element> mUnusedList = new LinkedList<>();
    private MediaCodec mCodec;
    private FrameMap mFrameMap = new FrameMap();
    private final int mQueueSize = 1;
    private Element mSurface = new Element();

    private enum SurfaceState {
        Idle, Rendering, Available
    }

    SurfaceState mState = SurfaceState.Idle;

    OutputFrameQueue()
    {
        for (int i = 0; i < mQueueSize; i++) {
            mUnusedList.add(new Element());
        }
    }

    public void setCodec(MediaCodec codec)
    {
        mCodec = codec;
    }

    public void pushInputBuffer(long presentationTimeUs, long frameIndex)
    {
        mFrameMap.put(presentationTimeUs, frameIndex);
    }

    synchronized public void pushOutputBuffer(int index, @NonNull MediaCodec.BufferInfo info)
    {
        if (mStopped)
        {
            Utils.loge(TAG, () -> "Ignore output buffer because queue has been already stopped. index=" + index);
            mCodec.releaseOutputBuffer(index, false);
            return;
        }
        long foundFrameIndex = mFrameMap.find(info.presentationTimeUs);

        if (foundFrameIndex < 0)
        {
            Utils.loge(TAG, () -> "Ignore output buffer because unknown frameIndex. index=" + index);
            mCodec.releaseOutputBuffer(index, false);
            return;
        }

        Element elem = mUnusedList.poll();
        if (elem == null)
        {
            Log.e(TAG, "FrameQueue is full. Discard old frame.");

            elem = mQueue.poll();
            mCodec.releaseOutputBuffer(elem.index, false);
        }
        elem.index = index;
        elem.frameIndex = foundFrameIndex;
        mQueue.add(elem);

        LatencyCollector.DecoderOutput(foundFrameIndex);
        Utils.frameLog(foundFrameIndex, () -> "Current queue state=" + mQueue.size() + "/" + mQueueSize + " pushed index=" + index);

        render();
    }

    synchronized public long render()
    {
        if (mStopped) {
            return -1;
        }
        if (mState != SurfaceState.Idle) {
            // It will conflict with current rendering frame.
            // Defer processing until current frame is rendered.
            Utils.log(TAG, () -> "Conflict with current rendering frame. Defer processing.");
            return -1;
        }
        Element elem = mQueue.poll();
        if (elem == null) {
            return -1;
        }
        mUnusedList.add(elem);

        Utils.frameLog(elem.frameIndex, () -> "Calling releaseOutputBuffer(). index=" + elem.index);

        mState = SurfaceState.Rendering;
        mSurface.index = elem.index;
        mSurface.frameIndex = elem.frameIndex;
        mCodec.releaseOutputBuffer(elem.index, true);
        return elem.frameIndex;
    }

    synchronized public void onFrameAvailable()
    {
        if (mStopped)
        {
            return;
        }
        if (mState != SurfaceState.Rendering)
        {
            return;
        }
        Utils.frameLog(mSurface.frameIndex, () -> "onFrameAvailable().");
        mState = SurfaceState.Available;
    }

    synchronized public long clearAvailable(SurfaceTexture surfaceTexture)
    {
        if (mStopped) {
            return -1;
        }
        if (mState != SurfaceState.Available)
        {
            return -1;
        }
        Utils.frameLog(mSurface.frameIndex, () -> "clearAvailable().");
        long frameIndex = mSurface.frameIndex;
        mState = SurfaceState.Idle;

        if (surfaceTexture != null) {
            surfaceTexture.updateTexImage();
        }

        // Render deferred frame.
        render();

        return frameIndex;
    }

    synchronized public boolean discardStaleFrames(SurfaceTexture surfaceTexture)
    {
        if (mStopped) {
            return false;
        }
        if (mQueue.size() == 0 || mState == SurfaceState.Rendering) {
            return false;
        }
        if (mState == SurfaceState.Available) {
            mState = SurfaceState.Idle;
            if (surfaceTexture != null) {
                surfaceTexture.updateTexImage();
            }
        }

        while (true) {
            if (mQueue.size() > 1) {
                // Discard because this elem is not latest frame.
                Element elem = mQueue.poll();
                Utils.frameLog(elem.frameIndex, () -> "discardStaleFrames: releaseOutputBuffer(false)");
                mCodec.releaseOutputBuffer(elem.index, false);
                mUnusedList.add(elem);
            } else {
                // Latest frame.
                Element elem = mQueue.peek();
                Utils.frameLog(elem.frameIndex, () -> "discardStaleFrames: releaseOutputBuffer(true)");
                render();
                return true;
            }
        }
    }

    synchronized public void stop() {
        if (mStopped) {
            return;
        }
        Utils.logi(TAG, () -> "Stopping.");
        mStopped = true;
        mUnusedList.addAll(mQueue);
        mQueue.clear();
    }

    synchronized public void reset() {
        Utils.logi(TAG, () -> "Resetting.");
        mStopped = false;
        mUnusedList.addAll(mQueue);
        mQueue.clear();
    }
}
