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

    interface ConnectionListener {
        void onConnected(int width, int height, int codec, boolean realtimeDecoder, int frameQueueSize, int refreshRate, boolean streamMic, int foveationMode, float foveationStrength, float foveationShape, float foveationVerticalOffset, int trackingSpaceType);

        void onDisconnect();

        void onTracking();

        void onHapticsFeedback(long startTime, float amplitude, float duration, float frequency, boolean hand);

        void onGuardianSyncAck(long timestamp);

        void onGuardianSegmentAck(long timestamp, int segmentIndex);
    }

    public interface NALCallback {
        NAL obtainNAL(int length);

        void pushNAL(NAL nal);
    }

    private TrackingThread mTrackingThread;

    private boolean mInitialized = false;

    private final OvrActivity mParent;

    private final ConnectionListener mConnectionListener;

    private final Object mWaiter = new Object();

    ServerConnection(ConnectionListener connectionListener, OvrActivity parent) {
        mConnectionListener = connectionListener;
        mParent = parent;
    }

    private String getDeviceName() {
        String manufacturer = android.os.Build.MANUFACTURER;
        String model = android.os.Build.MODEL;
        if (model.toLowerCase().startsWith(manufacturer.toLowerCase())) {
            return model;
        } else {
            return manufacturer + " " + model;
        }
    }

    public void setSinkPrepared(boolean prepared) {
        synchronized (mWaiter) {
            setSinkPreparedNative(prepared);
        }
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
                if (isConnectedNative()) {
                    mConnectionListener.onTracking();
                }
            });
        }
        return !initializeFailed;
    }

    @Override
    public void stopAndWait() {
        mTrackingThread.stopAndWait();
        synchronized (mWaiter) {
            interruptNative();
        }
        super.stopAndWait();
    }

    @Override
    public void run() {
        try {
            initializeSocket();
            synchronized (this) {
                mInitialized = true;
                notifyAll();
            }
            Utils.logi(TAG, () -> "ServerConnection initialized.");

            runLoop();
        } finally {
            closeSocket();
        }

        Utils.logi(TAG, () -> "ServerConnection stopped.");
    }

    public boolean isConnected() {
        return isConnectedNative();
    }

    // called from native
    @SuppressWarnings("unused")
    public void onConnected(int width, int height, int codec, boolean realtimeDecoder, int frameQueueSize, int refreshRate, boolean streamMic, int foveationMode, float foveationStrength, float foveationShape, float foveationVerticalOffset, int trackingSpaceType) {
        Utils.logi(TAG, () -> "onConnected is called.");
        mConnectionListener.onConnected(width, height, codec, realtimeDecoder, frameQueueSize, refreshRate, streamMic, foveationMode, foveationStrength, foveationShape, foveationVerticalOffset, trackingSpaceType);
    }

    @SuppressWarnings("unused")
    public void onDisconnected() {
        Utils.logi(TAG, () -> "onDisconnected is called.");
        mConnectionListener.onDisconnect();
    }

    @SuppressWarnings("unused")
    public void onHapticsFeedback(long startTime, float amplitude, float duration, float frequency, boolean hand) {
        mConnectionListener.onHapticsFeedback(startTime, amplitude, duration, frequency, hand);
    }

    @SuppressWarnings("unused")
    public void onGuardianSyncAck(long timestamp) {
        mConnectionListener.onGuardianSyncAck(timestamp);
    }

    @SuppressWarnings("unused")
    public void onGuardianSegmentAck(long timestamp, int segmentIndex) {
        mConnectionListener.onGuardianSegmentAck(timestamp, segmentIndex);
    }

    @SuppressWarnings("unused")
    public void send(long nativeBuffer, int bufferLength) {
        synchronized (mWaiter) {
            sendNative(nativeBuffer, bufferLength);
        }
    }

    @SuppressWarnings("unused")
    public NAL obtainNAL(int length) {
        return mParent.obtainNAL(length);
    }

    @SuppressWarnings("unused")
    public void pushNAL(NAL nal) {
        mParent.pushNAL(nal);
    }

    @SuppressWarnings("unused")
    public void setWebViewURL(String url) {
        mParent.setDashboardURL(url);
    }


    private native void initializeSocket();

    private native void closeSocket();

    private native void runLoop();

    private native void interruptNative();

    private native void sendNative(long nativeBuffer, int bufferLength);

    public native boolean isConnectedNative();

    private native void setSinkPreparedNative(boolean prepared);
}