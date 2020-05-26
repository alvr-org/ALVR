package com.polygraphene.alvr;

import android.app.Activity;
import android.opengl.EGLContext;
import android.util.Log;

import java.util.concurrent.TimeUnit;

class TrackingThread extends ThreadBase
{
    private static final String TAG = "TrackingThread";
    private int mRefreshRate = 72*3;
    interface TrackingCallback {
        void onTracking();
    }

    private TrackingCallback mCallback;
    //private ArThread mArThread;

    public TrackingThread() {
    }

    public void setCallback(TrackingCallback callback) {
        mCallback = callback;
    }

    void changeRefreshRate(int refreshRate) {
        mRefreshRate = refreshRate * 3;
    }

    public void start(EGLContext mEGLContext, Activity activity, int cameraTexture) {
//        mArThread = new ArThread(mEGLContext);
//        mArThread.initialize((BaseActivity) activity);
//        mArThread.setCameraTexture(cameraTexture);

        super.startBase();
        //mArThread.start();
    }

    public void onConnect() {
        //mArThread.onConnect();
    }

    public void onDisconnect() {
        //mArThread.onDisconnect();
    }

    @Override
    public void stopAndWait() {
        //mArThread.stopAndWait();
        super.stopAndWait();
    }

    @Override
    public void run() {
        long previousFetchTime = System.nanoTime();
        while (!isStopped()) {

            if(Utils.gDebugFlags!= 10)
                mCallback.onTracking();

            try {
                previousFetchTime += 1000 * 1000 * 1000 / mRefreshRate;
                long next = previousFetchTime - System.nanoTime();
                if (next < 0) {
                    // Exceed time!
                    previousFetchTime = System.nanoTime();
                } else {
                    TimeUnit.NANOSECONDS.sleep(next);
                }
            } catch (InterruptedException e) {
            }
        }
        Utils.logi(TAG, () -> "TrackingThread has stopped.");
    }

//    public boolean onRequestPermissionsResult(BaseActivity activity) {
//        return mArThread.onRequestPermissionsResult(activity);
//    }

//    public String getErrorMessage() {
//        return mArThread.getErrorMessage();
//    }
}
