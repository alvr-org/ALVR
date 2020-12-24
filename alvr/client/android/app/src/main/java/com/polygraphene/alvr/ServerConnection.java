package com.polygraphene.alvr;

class ServerConnection extends ThreadBase {
    private static final String TAG = "ServerConnection";

    private boolean mInitialized = false;

    private final OvrActivity mParent;

    ServerConnection(OvrActivity parent) {
        mParent = parent;
    }

    public void start() {
        super.startBase();

        synchronized (this) {
            while (!mInitialized) {
                try {
                    wait();
                } catch (InterruptedException e) {
                    e.printStackTrace();
                }
            }
        }
    }

    @Override
    public void stopAndWait() {
        synchronized (mParent.mWaiter) {
            mParent.interruptNative();
        }
        super.stopAndWait();
    }

    @Override
    public void run() {
        mParent.initializeSocket();
        synchronized (this) {
            mInitialized = true;
            notifyAll();
        }
        Utils.logi(TAG, () -> "ServerConnection initialized.");

        mParent.runLoop();

        mParent.closeSocket();

        Utils.logi(TAG, () -> "ServerConnection stopped.");
    }
}