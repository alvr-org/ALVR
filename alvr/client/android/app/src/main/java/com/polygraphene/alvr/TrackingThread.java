package com.polygraphene.alvr;

import android.app.Activity;
import android.opengl.EGLContext;
import android.util.Log;

import java.util.concurrent.TimeUnit;

class TrackingThread extends ThreadBase
{
    private static final String TAG = "TrackingThread";

    interface TrackingCallback {
        void onTracking();
    }

    private TrackingCallback mCallback;

    public void start(TrackingCallback callback) {
        mCallback = callback;
        super.startBase();
    }

    @Override
    public void run() {
        long previousFetchTime = System.nanoTime();
        while (!isStopped()) {

            if(Utils.gDebugFlags!= 10)
                mCallback.onTracking();

            try {
                previousFetchTime += 1000 * 1000 * 1000 / 72 * 3;
                long next = previousFetchTime - System.nanoTime();
                if (next < 0) {
                    // Exceed time!
                    previousFetchTime = System.nanoTime();
                } else {
                    TimeUnit.NANOSECONDS.sleep(next);
                }
            } catch (InterruptedException ignored) {
            }
        }
        Utils.logi(TAG, () -> "TrackingThread has stopped.");
    }
}
