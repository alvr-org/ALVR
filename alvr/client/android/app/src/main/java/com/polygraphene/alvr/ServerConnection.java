package com.polygraphene.alvr;

import android.app.Activity;
import android.opengl.EGLContext;
import android.util.Log;

import java.net.InterfaceAddress;
import java.net.NetworkInterface;
import java.net.SocketException;
import java.util.ArrayList;
import java.util.Enumeration;
import java.util.List;

class ServerConnection extends ThreadBase {
    private static final String TAG = "ServerConnection";

    static {
        System.loadLibrary("alvr_client");
    }

    public interface NALCallback {
        NAL obtainNAL(int length);

        void pushNAL(NAL nal);
    }

    private TrackingThread mTrackingThread;

    private boolean mInitialized = false;

    private final OvrActivity mParent;

    ServerConnection(OvrActivity parent) {
        mParent = parent;
    }

    public boolean start() {
        mTrackingThread = new TrackingThread();

        super.startBase();

        boolean initializeFailed = false;
        synchronized (this) {
            while (!mInitialized && !initializeFailed) {
                try {
                    wait();
                } catch (InterruptedException e) {
                    e.printStackTrace();
                }
            }
        }

        if (!initializeFailed) {
            mTrackingThread.start(() -> {
                if (mParent.isConnectedNative()) {
                    mParent.onTrackingNative(mParent);
                }
            });
        }
        return !initializeFailed;
    }

    @Override
    public void stopAndWait() {
        mTrackingThread.stopAndWait();
        synchronized (mParent.mWaiter) {
            mParent.interruptNative();
        }
        super.stopAndWait();
    }

    @Override
    public void run() {
        try {
            mParent.initializeSocket();
            synchronized (this) {
                mInitialized = true;
                notifyAll();
            }
            Utils.logi(TAG, () -> "ServerConnection initialized.");

            mParent.runLoop();
        } finally {
            mParent.closeSocket();
        }

        Utils.logi(TAG, () -> "ServerConnection stopped.");
    }
}