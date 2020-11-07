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

class ServerConnection extends ThreadBase
{
    private static final String TAG = "ServerConnection";

    static {
        System.loadLibrary("alvr_client");
    }

    private static final String BROADCAST_ADDRESS = "255.255.255.255";
    private static final int HELLO_PORT = 9943;
    private static final int PORT = 9944;

    private TrackingThread mTrackingThread;

    private DeviceDescriptor mDeviceDescriptor;

    private boolean mInitialized = false;
    private boolean mInitializeFailed = false;

    private String mPreviousServerAddress;
    private int mPreviousServerPort;

    private OvrThread mParent;

    interface ConnectionListener {
        void onConnected(int width, int height, int codec, int frameQueueSize, int refreshRate, boolean streamMic, int foveationMode, float foveationStrength, float foveationShape, float foveationVerticalOffset);

        void onChangeSettings(int suspend, int frameQueueSize);

        void onShutdown(String serverAddr, int serverPort);

        void onDisconnect();

        void onTracking();

        void onHapticsFeedback(long startTime, float amplitude, float duration, float frequency, boolean hand);

        void onGuardianSyncAck(long timestamp);

        void onGuardianSegmentAck(long timestamp, int segmentIndex);
    }

    private ConnectionListener mConnectionListener;

    public interface NALCallback {
        NAL obtainNAL(int length);
        void pushNAL(NAL nal);
    }

    private NALCallback mNALCallback;

    private final Object mWaiter = new Object();

    ServerConnection(ConnectionListener connectionListener, OvrThread parent)
    {
        mConnectionListener = connectionListener;
        mParent = parent;
    }

    private String getDeviceName()
    {
        String manufacturer = android.os.Build.MANUFACTURER;
        String model = android.os.Build.MODEL;
        if (model.toLowerCase().startsWith(manufacturer.toLowerCase())) {
            return model;
        } else {
            return manufacturer + " " + model;
        }
    }

    public void recoverConnectionState(String serverAddress, int serverPort)
    {
        mPreviousServerAddress = serverAddress;
        mPreviousServerPort = serverPort;
    }

    public void setSinkPrepared(boolean prepared)
    {
        synchronized (mWaiter) {
            setSinkPreparedNative(prepared);
        }
    }

    public boolean start(EGLContext mEGLContext, Activity activity, DeviceDescriptor deviceDescriptor, int cameraTexture, NALCallback nalCallback)
    {
        mTrackingThread = new TrackingThread();
        mTrackingThread.setCallback(() -> {
            if (isConnectedNative()) {
                mConnectionListener.onTracking();
            }
        });

        mDeviceDescriptor = deviceDescriptor;

        mNALCallback = nalCallback;

        super.startBase();

        synchronized (this)
        {
            while (!mInitialized && !mInitializeFailed)
            {
                try {
                    wait();
                } catch (InterruptedException e) {
                    e.printStackTrace();
                }
            }
        }

        if(!mInitializeFailed)
        {
            mTrackingThread.start(mEGLContext, activity, cameraTexture);
        }
        return !mInitializeFailed;
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
    public void run()
    {
        try {
            String[] targetList = getTargetAddressList();

            for (String target: targetList) {
                Utils.logi(TAG, () -> "Target IP address for hello datagrams: " + target);
            }

            initializeSocket(HELLO_PORT, PORT, getDeviceName(), targetList,
                    mDeviceDescriptor.mRefreshRates, mDeviceDescriptor.mRenderWidth, mDeviceDescriptor.mRenderHeight, mDeviceDescriptor.mFov,
                    mDeviceDescriptor.mDeviceType, mDeviceDescriptor.mDeviceSubType, mDeviceDescriptor.mDeviceCapabilityFlags,
                    mDeviceDescriptor.mControllerCapabilityFlags, mDeviceDescriptor.mIpd
            );
            synchronized (this) {
                mInitialized = true;
                notifyAll();
            }
            Utils.logi(TAG, () -> "ServerConnection initialized.");

            runLoop(mPreviousServerAddress, mPreviousServerPort);
        } finally {
            mConnectionListener.onShutdown(getServerAddress(), getServerPort());
            closeSocket();
        }

        Utils.logi(TAG, () -> "ServerConnection stopped.");
    }

    // List addresses where discovery datagrams will be sent to reach ALVR server.
    private String[] getTargetAddressList()
    {
        // List addresses from targetServers setting (if present).
        if (PersistentConfig.sTargetServers != null && PersistentConfig.sTargetServers.length() > 6) {
            String[] addrs = PersistentConfig.sTargetServers.split("[^0-9.]+");
            Utils.logi(TAG, () -> addrs.length + " target server IP addresses were found in config setting " + PersistentConfig.KEY_TARGET_SERVERS + " value: " + PersistentConfig.sTargetServers);
            return addrs;
        }

        // List broadcast address from all interfaces except for mobile network.
        // We should send all broadcast address to use USB tethering or VPN.
        List<String> ret = new ArrayList<>();
        try {
            Enumeration<NetworkInterface> networkInterfaces = NetworkInterface.getNetworkInterfaces();

            while (networkInterfaces.hasMoreElements()) {
                NetworkInterface networkInterface = networkInterfaces.nextElement();

                if (networkInterface.getName().startsWith("rmnet")) {
                    // Ignore mobile network interfaces.
                    Utils.log(TAG, () -> "Ignore interface. Name=" + networkInterface.getName());
                    continue;
                }

                List<InterfaceAddress> interfaceAddresses = networkInterface.getInterfaceAddresses();

                String address = "";
                for (InterfaceAddress interfaceAddress : interfaceAddresses) {
                    address += interfaceAddress.toString() + ", ";
                    // getBroadcast() return non-null only when ipv4.
                    if (interfaceAddress.getBroadcast() != null) {
                        ret.add(interfaceAddress.getBroadcast().getHostAddress());
                    }
                }
                String finalAddress = address;
                Utils.logi(TAG, () -> "Interface: Name=" + networkInterface.getName() + " Address=" + finalAddress);
            }
            Utils.logi(TAG, () -> ret.size() + " broadcast addresses were found.");
            for (String address : ret) {
                Log.v(TAG, address);
            }
        } catch (SocketException e) {
            e.printStackTrace();
        }
        if (ret.size() == 0) {
            ret.add(BROADCAST_ADDRESS);
        }
        return ret.toArray(new String[]{});
    }


//    public String getErrorMessage() {
//        return mTrackingThread.getErrorMessage();
//    }

    public boolean isConnected() {
        return isConnectedNative();
    }

    // called from native
    @SuppressWarnings("unused")
    public void onConnected(int width, int height, int codec, int frameQueueSize, int refreshRate, boolean streamMic, int foveationMode, float foveationStrength, float foveationShape, float foveationVerticalOffset) {
        Utils.logi(TAG, () -> "onConnected is called.");
        mConnectionListener.onConnected(width, height, codec, frameQueueSize, refreshRate, streamMic, foveationMode, foveationStrength, foveationShape, foveationVerticalOffset);
        mTrackingThread.onConnect();
    }

    @SuppressWarnings("unused")
    public void onDisconnected() {
        Utils.logi(TAG, () -> "onDisconnected is called.");
        mConnectionListener.onDisconnect();
        mTrackingThread.onDisconnect();
    }

    @SuppressWarnings("unused")
    public void onChangeSettings(long debugFlags, int suspend, int frameQueueSize) {
        PersistentConfig.sDebugFlags = debugFlags;
        PersistentConfig.saveCurrentConfig(true);
        mConnectionListener.onChangeSettings(suspend, frameQueueSize);
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
        return mNALCallback.obtainNAL(length);
    }

    @SuppressWarnings("unused")
    public void pushNAL(NAL nal) {
        mNALCallback.pushNAL(nal);
    }

    @SuppressWarnings("unused")
    public void setWebViewURL(String url) {
        mParent.setupWebView(url);
    }


    private native void initializeSocket(int helloPort, int port, String deviceName, String[] broadcastAddrList,
                                         int[] refreshRates, int renderWidth, int renderHeight, float[] fov,
                                         int deviceType, int deviceSubType, int deviceCapabilityFlags, int controllerCapabilityFlags, float ipd);
    private native void closeSocket();
    private native void runLoop(String serverAddress, int serverPort);
    private native void interruptNative();

    private native void sendNative(long nativeBuffer, int bufferLength);

    public native boolean isConnectedNative();

    private native String getServerAddress();
    private native int getServerPort();
    private native void setSinkPreparedNative(boolean prepared);
}