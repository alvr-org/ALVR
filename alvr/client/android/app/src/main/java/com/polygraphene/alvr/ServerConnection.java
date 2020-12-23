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

    private TrackingThread mTrackingThread;

    private boolean mInitialized = false;

    private final OvrActivity mParent;

    ServerConnection(OvrActivity parent) {
        mParent = parent;
    }

    public void start() {
        mTrackingThread = new TrackingThread();

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

        mTrackingThread.start(() -> {
            if (mParent.isConnectedNative()) {
                mParent.onTrackingNative();
            }
        });
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