package com.polygraphene.alvr;

import android.app.Activity;
import android.graphics.SurfaceTexture;
import android.opengl.EGL14;
import android.opengl.EGLContext;
import android.os.Handler;
import android.os.HandlerThread;
import android.os.Looper;
import android.util.Log;
import android.view.Surface;
import android.view.SurfaceHolder;

import java.util.concurrent.TimeUnit;

class OvrThread implements SurfaceHolder.Callback {
    private static final String TAG = "OvrThread";

    private Activity mActivity;

    private OvrContext mOvrContext = new OvrContext();
    private Handler mHandler;
    private HandlerThread mHandlerThread;

    private SurfaceTexture mSurfaceTexture;
    private Surface mSurface;

    private LoadingTexture mLoadingTexture = new LoadingTexture();

    // Worker threads
    private DecoderThread mDecoderThread;
    private ServerConnection mReceiverThread;
    private LauncherSocket mLauncherSocket;

    private EGLContext mEGLContext;

    private boolean mVrMode = false;
    private boolean mDecoderPrepared = false;
    private int mRefreshRate = 72;

    private long mPreviousRender = 0;

    private Runnable mRenderRunnable = () -> render();
    private Runnable mIdleRenderRunnable = () -> render();


    public OvrThread(Activity activity) {
        this.mActivity = activity;

        mHandlerThread = new HandlerThread("OvrThread");
        mHandlerThread.start();
        mHandler = new Handler(mHandlerThread.getLooper());
        mHandler.post(() -> startup());
    }

    //SurfaceHolder Callbacks

    @Override
    public void surfaceCreated(final SurfaceHolder holder) {
        Utils.logi(TAG, () -> "OvrThread.onSurfaceCreated.");
        mHandler.post(() -> mOvrContext.onSurfaceCreated(holder.getSurface()));
    }

    @Override
    public void surfaceChanged(SurfaceHolder holder, int format, int width, int height) {
        Utils.logi(TAG, () -> "OvrThread.onSurfaceChanged.");
        mHandler.post(() -> mOvrContext.onSurfaceChanged(holder.getSurface()));
    }

    @Override
    public void surfaceDestroyed(SurfaceHolder holder) {
        Utils.logi(TAG, () -> "OvrThread.onSurfaceDestroyed.");
        mHandler.post(() -> mOvrContext.onSurfaceDestroyed());
    }


    //Activity callbacks
    public void onResume() {
        Utils.logi(TAG, () -> "OvrThread.onResume: Starting worker threads.");
        // Sometimes previous decoder output remains not updated (when previous call of waitFrame() didn't call updateTexImage())
        // and onFrameAvailable won't be called after next output.
        // To avoid deadlock caused by it, we need to flush last output.
        mHandler.post(() -> {
            mLauncherSocket = new LauncherSocket(mLauncherSocketCallback);
            mLauncherSocket.listen();

            mReceiverThread = new ServerConnection(mUdpReceiverConnectionListener);

            PersistentConfig.ConnectionState connectionState = new PersistentConfig.ConnectionState();
            PersistentConfig.loadConnectionState(mActivity, connectionState);

            if (connectionState.serverAddr != null && connectionState.serverPort != 0) {
                Utils.logi(TAG, () -> "Load connection state: " + connectionState.serverAddr + " " + connectionState.serverPort);
                mReceiverThread.recoverConnectionState(connectionState.serverAddr, connectionState.serverPort);
            }

            // Sometimes previous decoder output remains not updated (when previous call of waitFrame() didn't call updateTexImage())
            // and onFrameAvailable won't be called after next output.
            // To avoid deadlock caused by it, we need to flush last output.
            mSurfaceTexture.updateTexImage();

            mDecoderThread = new DecoderThread(mSurface, mActivity, mDecoderCallback);

            try {
                mDecoderThread.start();

                DeviceDescriptor deviceDescriptor = new DeviceDescriptor();
                mOvrContext.getDeviceDescriptor(deviceDescriptor);
                mRefreshRate = deviceDescriptor.mRefreshRates[0];
                if (!mReceiverThread.start(mEGLContext, mActivity, deviceDescriptor, mOvrContext.getCameraTexture(), mDecoderThread)) {
                    Utils.loge(TAG, () -> "FATAL: Initialization of ReceiverThread failed.");
                    return;
                }
            } catch (IllegalArgumentException | IllegalStateException | SecurityException e) {
                e.printStackTrace();
            }

            Utils.logi(TAG, () -> "OvrThread.onResume: mOvrContext.onResume().");
            mOvrContext.onResume();
        });
    }

    public void onPause() {
        Utils.logi(TAG, () -> "OvrThread.onPause: Stopping worker threads.");
        // DecoderThread must be stopped before ReceiverThread and setting mResumed=false.
        mHandler.post(() -> {
            if (mLauncherSocket != null) {
                mLauncherSocket.close();
                mLauncherSocket = null;
            }

            // DecoderThread must be stopped before ReceiverThread and setting mResumed=false.
            if (mDecoderThread != null) {
                Utils.log(TAG, () -> "OvrThread.onPause: Stopping DecoderThread.");
                mDecoderThread.stopAndWait();
            }
            if (mReceiverThread != null) {
                Utils.log(TAG, () -> "OvrThread.onPause: Stopping ReceiverThread.");
                mReceiverThread.stopAndWait();
            }

            mOvrContext.onPause();
        });
    }

    public void onDestroy() {
        mHandler.post(() -> {
            mLoadingTexture.destroyTexture();
            Utils.logi(TAG, () -> "Destroying vrapi state.");
            mOvrContext.destroy();
        });
        mHandlerThread.quitSafely();
    }



    //called from constructor
    public void startup() {
        Utils.logi(TAG, () -> "OvrThread started.");

        mOvrContext.initialize(mActivity, mActivity.getAssets(), this, false, 72);

        mSurfaceTexture = new SurfaceTexture(mOvrContext.getSurfaceTextureID());
        mSurfaceTexture.setOnFrameAvailableListener(surfaceTexture -> {
            Utils.log(TAG, () -> "OvrThread: waitFrame: onFrameAvailable is called.");
            mDecoderThread.onFrameAvailable();
            mHandler.removeCallbacks(mIdleRenderRunnable);
            mHandler.post(mRenderRunnable);
        }, new Handler(Looper.getMainLooper()));
        mSurface = new Surface(mSurfaceTexture);

        mLoadingTexture.initializeMessageCanvas(mOvrContext.getLoadingTexture());
        mLoadingTexture.drawMessage(Utils.getVersionName(mActivity) + "\nLoading...");

        mEGLContext = EGL14.eglGetCurrentContext();
    }

    private void render() {
        if (mReceiverThread.isConnected())
        {
            /*if (mDecoderThread.discartStaleFrames(mSurfaceTexture)) {
                Utils.log(TAG, () ->  "Discard stale frame. Wait next onFrameAvailable.");
                mHandler.removeCallbacks(mIdleRenderRunnable);
                mHandler.postDelayed(mIdleRenderRunnable, 50);
                return;
            }*/
            long next = checkRenderTiming();
            if(next > 0) {
                mHandler.postDelayed(mRenderRunnable, next);
                return;
            }
            long renderedFrameIndex = mDecoderThread.clearAvailable(mSurfaceTexture);
            if (renderedFrameIndex != -1)
            {
                mOvrContext.render(renderedFrameIndex);
                mPreviousRender = System.nanoTime();

                mHandler.postDelayed(mRenderRunnable, 5);
            }
            else
            {
                mHandler.removeCallbacks(mIdleRenderRunnable);
                mHandler.postDelayed(mIdleRenderRunnable, 50);
            }
        }
        else
        {
            if (!mOvrContext.isVrMode())
                return;

            if (mReceiverThread.isConnected())
            {
                mLoadingTexture.drawMessage(Utils.getVersionName(mActivity) + "\n \nConnected!\nStreaming will begin soon!");
            }
            else if(mLauncherSocket != null && mLauncherSocket.isConnected())
            {
                mLoadingTexture.drawMessage(Utils.getVersionName(mActivity) + "\n \nConnected!\nPress Trigger\nto start SteamVR.");
                if (mOvrContext.getButtonDown())
                {
                    mLauncherSocket.sendCommand("StartServer");
                }
            }
            else
            {
                mLoadingTexture.drawMessage(Utils.getVersionName(mActivity) + "\n \nPress CONNECT button\non ALVR server.");
            }

            mOvrContext.renderLoading();
            mHandler.removeCallbacks(mIdleRenderRunnable);
            mHandler.postDelayed(mIdleRenderRunnable, 13); // 72Hz = 13.8888ms
        }
    }

    private long checkRenderTiming() {
        long current = System.nanoTime();
        long threashold = TimeUnit.SECONDS.toNanos(1) / mRefreshRate -
                TimeUnit.MILLISECONDS.toNanos(5);
        return TimeUnit.NANOSECONDS.toMillis(threashold - (current - mPreviousRender));
    }

    // Called on OvrThread.
    public void onVrModeChanged(boolean enter) {
        mVrMode = enter;
        Utils.logi(TAG, () -> "onVrModeChanged. mVrMode=" + mVrMode + " mDecoderPrepared=" + mDecoderPrepared);
        mReceiverThread.setSinkPrepared(mVrMode && mDecoderPrepared);
        if (mVrMode) {
            mHandler.post(mRenderRunnable);
        }
    }

    private ServerConnection.ConnectionListener mUdpReceiverConnectionListener = new ServerConnection.ConnectionListener() {
        @Override
        public void onConnected(final int width, final int height, final int codec, final int frameQueueSize,
                                final int refreshRate, final boolean streamMic, final int foveationMode,
                                final float foveationStrength, final float foveationShape,
                                final float foveationVerticalOffset) {

            // We must wait completion of notifyGeometryChange
            // to ensure the first video frame arrives after notifyGeometryChange.
            mHandler.post(() -> {
                mOvrContext.setRefreshRate(refreshRate);
                mOvrContext.setFFRParams(foveationMode, foveationStrength, foveationShape, foveationVerticalOffset);
                mOvrContext.setFrameGeometry(width, height);
                mOvrContext.setStreamMic(streamMic);
                mDecoderThread.onConnect(codec, frameQueueSize);
            });
        }

        @Override
        public void onChangeSettings(int suspend, int frameQueueSize) {
            mOvrContext.onChangeSettings(suspend);
        }

        @Override
        public void onShutdown(String serverAddr, int serverPort) {
            Log.v(TAG, "save connection state: " + serverAddr + " " + serverPort);
            PersistentConfig.saveConnectionState(mActivity, serverAddr, serverPort);
        }

        @Override
        public void onDisconnect() {
            mDecoderThread.onDisconnect();
        }

        @Override
        public void onTracking() {
            if (mOvrContext.isVrMode()) {
                mOvrContext.sendTrackingInfo(mReceiverThread);

                //TODO: maybe use own thread, but works fine with tracking
                mOvrContext.sendMicData(mReceiverThread);
            }
        }

        @Override
        public void onHapticsFeedback(long startTime, float amplitude, float duration, float frequency, boolean hand) {
            mHandler.post(() -> {
                if (mOvrContext.isVrMode()) {
                    mOvrContext.onHapticsFeedback(startTime, amplitude, duration, frequency, hand);
                }
            });
        }
    };



    private DecoderThread.DecoderCallback mDecoderCallback = new DecoderThread.DecoderCallback() {
        @Override
        public void onPrepared() {
            mDecoderPrepared = true;
            Utils.logi(TAG, () -> "DecoderCallback.onPrepared. mVrMode=" + mVrMode + " mDecoderPrepared=" + mDecoderPrepared);
            mReceiverThread.setSinkPrepared(mVrMode && mDecoderPrepared);
        }

        @Override
        public void onDestroy() {
            mDecoderPrepared = false;
            Utils.logi(TAG, () -> "DecoderCallback.onDestroy. mVrMode=" + mVrMode + " mDecoderPrepared=" + mDecoderPrepared);
            mReceiverThread.setSinkPrepared(mVrMode && mDecoderPrepared);
        }

        @Override
        public void onFrameDecoded() {
            mDecoderThread.releaseBuffer();
        }
    };

    private LauncherSocket.LauncherSocketCallback mLauncherSocketCallback = new LauncherSocket.LauncherSocketCallback() {
        @Override
        public void onConnect() {
        }
    };
}
