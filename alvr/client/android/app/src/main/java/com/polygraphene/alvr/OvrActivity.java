package com.polygraphene.alvr;

import android.Manifest;
import android.app.Activity;
import android.content.Context;
import android.content.pm.PackageManager;
import android.content.res.AssetManager;
import android.graphics.SurfaceTexture;
import android.media.AudioManager;
import android.opengl.EGL14;
import android.opengl.EGLContext;
import android.os.Bundle;
import android.os.Handler;
import android.os.HandlerThread;
import android.os.Looper;
import android.os.SystemClock;
import android.support.v4.app.ActivityCompat;
import android.support.v4.content.ContextCompat;
import android.util.Log;
import android.view.KeyEvent;
import android.view.MotionEvent;
import android.view.Surface;
import android.view.SurfaceHolder;
import android.view.SurfaceView;
import android.view.ViewGroup;
import android.view.Window;
import android.view.WindowManager;
import android.widget.Toast;

import java.util.concurrent.TimeUnit;

public class OvrActivity extends Activity {
    static {
        System.loadLibrary("alvr_client");
    }

    private final static String TAG = "OvrActivity";
    public static final int WEBVIEW_WIDTH = 800;
    public static final int WEBVIEW_HEIGHT = 600;


    class RenderingCallbacks implements SurfaceHolder.Callback {
        @Override
        public void surfaceCreated(final SurfaceHolder holder) {
            mHandler.post(() -> onSurfaceCreatedNative(holder.getSurface()));
        }

        @Override
        public void surfaceChanged(SurfaceHolder holder, int format, int width, int height) {
            mHandler.post(() -> onSurfaceChangedNative(holder.getSurface()));
        }

        @Override
        public void surfaceDestroyed(SurfaceHolder holder) {
            mHandler.post(() -> onSurfaceDestroyedNative());
        }
    }

    private SurfaceWebView mWebView = null;

    private Handler mHandler;
    private HandlerThread mHandlerThread;

    private Handler mMainHandler;

    // Wrapper used to emulate pointer/pass by reference
    public class WebViewWrapper {
        public SurfaceWebView webView = null;
    }

    private WebViewWrapper mWebViewWrapper = null;

    private SurfaceTexture mSurfaceTexture;
    private Surface mSurface;
    private SurfaceTexture mWebViewSurfaceTexture;
    private Surface mWebViewSurface;

    private LoadingTexture mLoadingTexture = new LoadingTexture();

    // Worker threads
    private DecoderThread mDecoderThread;
    private ServerConnection mReceiverThread;

    private EGLContext mEGLContext;

    private boolean mVrMode = false;
    private boolean mDecoderPrepared = false;
    private int mRefreshRate = 72;

    private long mPreviousRender = 0;

    private Runnable mRenderRunnable = () -> render();
    private Runnable mIdleRenderRunnable = () -> render();

    @Override
    protected void onCreate(Bundle savedInstanceState) {
        super.onCreate(savedInstanceState);

        getWindow().addFlags(WindowManager.LayoutParams.FLAG_KEEP_SCREEN_ON);
        getWindow().addFlags(WindowManager.LayoutParams.FLAG_FULLSCREEN);
        requestWindowFeature(Window.FEATURE_NO_TITLE);

        setContentView(R.layout.activity_main);
        SurfaceView surfaceView = findViewById(R.id.surfaceview);

        mWebView = new SurfaceWebView(this);
        addContentView(mWebView, new ViewGroup.LayoutParams(WEBVIEW_WIDTH, WEBVIEW_HEIGHT));

        mHandlerThread = new HandlerThread("Rendering thread");
        mHandlerThread.start();
        mHandler = new Handler(mHandlerThread.getLooper());
        mHandler.post(() -> startup());

        mMainHandler = new Handler(this.getMainLooper());

        mWebViewWrapper = new WebViewWrapper();
        mWebViewWrapper.webView = mWebView;

        SurfaceHolder holder = surfaceView.getHolder();
        holder.addCallback(new RenderingCallbacks());


        requestAudioPermissions();
    }

    @Override
    protected void onResume() {
        super.onResume();

        // Sometimes previous decoder output remains not updated (when previous call of waitFrame() didn't call updateTexImage())
        // and onFrameAvailable won't be called after next output.
        // To avoid deadlock caused by it, we need to flush last output.
        mHandler.post(() -> {

            mReceiverThread = new ServerConnection(mUdpReceiverConnectionListener, this);

            // Sometimes previous decoder output remains not updated (when previous call of waitFrame() didn't call updateTexImage())
            // and onFrameAvailable won't be called after next output.
            // To avoid deadlock caused by it, we need to flush last output.
            mSurfaceTexture.updateTexImage();

            mDecoderThread = new DecoderThread(mSurface, this, mDecoderCallback);

            try {
                mDecoderThread.start();

                DeviceDescriptor deviceDescriptor = new DeviceDescriptor();
                getDeviceDescriptorNative(deviceDescriptor);
                mRefreshRate = deviceDescriptor.mRefreshRates[0];
                if (!mReceiverThread.start(mEGLContext, this, deviceDescriptor, 0, mDecoderThread)) {
                    Utils.loge(TAG, () -> "FATAL: Initialization of ReceiverThread failed.");
                    return;
                }
            } catch (IllegalArgumentException | IllegalStateException | SecurityException e) {
                Utils.loge(TAG, () -> e.toString());
            }

            onResumeNative();
        });
    }

    @Override
    protected void onPause() {
        super.onPause();

        // DecoderThread must be stopped before ReceiverThread and setting mResumed=false.
        mHandler.post(() -> {
            // DecoderThread must be stopped before ReceiverThread and setting mResumed=false.
            if (mDecoderThread != null) {
                mDecoderThread.stopAndWait();
            }
            if (mReceiverThread != null) {
                mReceiverThread.stopAndWait();
            }

            onPauseNative();
        });
    }

    @Override
    protected void onDestroy() {
        super.onDestroy();

        mHandler.post(() -> {
            mLoadingTexture.destroyTexture();
            Utils.logi(TAG, () -> "Destroying vrapi state.");
            destroyNative();
        });
        mHandlerThread.quitSafely();
    }

    @Override
    public boolean dispatchKeyEvent(KeyEvent event) {
        //Utils.log(TAG, () ->  "dispatchKeyEvent: " + event.getKeyCode());
        if(event.getAction() == KeyEvent.ACTION_DOWN || event.getAction() == KeyEvent.ACTION_UP) {
            if (event.getKeyCode() == KeyEvent.KEYCODE_VOLUME_UP)
            {
                adjustVolume(1);
                return true;
            }
            if (event.getKeyCode() == KeyEvent.KEYCODE_VOLUME_DOWN)
            {
                adjustVolume(-1);
                return true;
            }

            return true;
        }else{
            return super.dispatchKeyEvent(event);
        }
    }

    private void adjustVolume(int direction)
    {
        AudioManager audio = (AudioManager) getSystemService(Context.AUDIO_SERVICE);
        audio.adjustStreamVolume(AudioManager.STREAM_MUSIC, direction, 0);
    }

    //Create placeholder for user's consent to record_audio permission.
    //This will be used in handling callback
    private final int MY_PERMISSIONS_RECORD_AUDIO = 1;

    private void requestAudioPermissions() {
        if (ContextCompat.checkSelfPermission(this,
                Manifest.permission.RECORD_AUDIO)
                != PackageManager.PERMISSION_GRANTED) {

            //When permission is not granted by user, show them message why this permission is needed.
            if (ActivityCompat.shouldShowRequestPermissionRationale(this,
                    Manifest.permission.RECORD_AUDIO)) {
                Toast.makeText(this, "Please grant permissions to use microphone", Toast.LENGTH_LONG).show();

                //Give user option to still opt-in the permissions
                ActivityCompat.requestPermissions(this,
                        new String[]{Manifest.permission.RECORD_AUDIO},
                        MY_PERMISSIONS_RECORD_AUDIO);

            } else {
                // Show user dialog to grant permission to record audio
                ActivityCompat.requestPermissions(this,
                        new String[]{Manifest.permission.RECORD_AUDIO},
                        MY_PERMISSIONS_RECORD_AUDIO);
            }
        }
        //If permission is granted, then go ahead recording audio
        else if (ContextCompat.checkSelfPermission(this,
                Manifest.permission.RECORD_AUDIO)
                == PackageManager.PERMISSION_GRANTED) {

            //Go ahead with recording audio now
           // recordAudio();
        }
    }

    //Handling callback
    @Override
    public void onRequestPermissionsResult(int requestCode,
                                           String permissions[], int[] grantResults) {
        switch (requestCode) {
            case MY_PERMISSIONS_RECORD_AUDIO: {
                if (grantResults.length > 0
                        && grantResults[0] == PackageManager.PERMISSION_GRANTED) {
                    // permission was granted, yay!
                    //recordAudio();
                } else {
                    // permission denied, boo! Disable the
                    // functionality that depends on this permission.
                    Toast.makeText(this, "Permissions Denied to record audio", Toast.LENGTH_LONG).show();
                }
                return;
            }
        }
    }

    public void setupWebView(String url) {
        // We now have dashboard url, so we can post() to the main thread to set up our WebView.
        mMainHandler.post(() -> {
            SurfaceWebView webView = mWebViewWrapper.webView;
            webView.getSettings().setJavaScriptEnabled(true);
            webView.getSettings().setDomStorageEnabled(true);
            webView.setInitialScale(100);
            webView.loadUrl(url);
        });
    }



    //called from constructor
    public void startup() {

        initializeNative(this, this.getAssets(), this, false, 72);

        mSurfaceTexture = new SurfaceTexture(getSurfaceTextureIDNative());
        mSurfaceTexture.setOnFrameAvailableListener(surfaceTexture -> {
            mDecoderThread.onFrameAvailable();
            mHandler.removeCallbacks(mIdleRenderRunnable);
            mHandler.post(mRenderRunnable);
        }, new Handler(Looper.getMainLooper()));
        mSurface = new Surface(mSurfaceTexture);

        mWebViewSurfaceTexture = new SurfaceTexture(getWebViewSurfaceTextureNative());
        mWebViewSurfaceTexture.setDefaultBufferSize(WEBVIEW_WIDTH, WEBVIEW_HEIGHT);
        mWebViewSurface = new Surface(mWebViewSurfaceTexture);

        // Doesn't need to be posted to the main thread since it's our method.
        mWebViewWrapper.webView.setSurface(mWebViewSurface);

        mLoadingTexture.initializeMessageCanvas(getLoadingTextureNative());
        mLoadingTexture.drawMessage(Utils.getVersionName(this) + "\nLoading...");

        mEGLContext = EGL14.eglGetCurrentContext();
    }

    private void render() {
        if (mReceiverThread.isConnected())
        {
            long next = checkRenderTiming();
            if(next > 0) {
                mHandler.postDelayed(mRenderRunnable, next);
                return;
            }
            long renderedFrameIndex = mDecoderThread.clearAvailable(mSurfaceTexture);

            if (mWebViewSurfaceTexture != null){
                mWebViewSurfaceTexture.updateTexImage();
            }

            if (renderedFrameIndex != -1)
            {
                renderNative(renderedFrameIndex);
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
            if (!isVrModeNative())
                return;

            if (mReceiverThread.isConnected())
            {
                mLoadingTexture.drawMessage(Utils.getVersionName(this) + "\n \nConnected!\nStreaming will begin soon!");
            }
            else
            {
                mLoadingTexture.drawMessage(Utils.getVersionName(this) + "\n \nOpen ALVR on PC and\nclick on \"Trust\" next to\nthe client entry");
            }

            renderLoadingNative();
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
                setRefreshRateNative(refreshRate);
                setFFRParamsNative(foveationMode, foveationStrength, foveationShape, foveationVerticalOffset);
                setFrameGeometryNative(width, height);
                setStreamMicNative(streamMic);
                mDecoderThread.onConnect(codec, frameQueueSize);
            });
        }

        @Override
        public void onShutdown(String serverAddr, int serverPort) {
        }

        @Override
        public void onDisconnect() {
            mDecoderThread.onDisconnect();
        }

        @Override
        public void onTracking() {
            if (isVrModeNative()) {
                sendTrackingInfoNative(mReceiverThread);

                //TODO: maybe use own thread, but works fine with tracking
                sendMicDataNative(mReceiverThread);

                //TODO: same as above
                sendGuardianInfoNative(mReceiverThread);
            }
        }

        @Override
        public void onHapticsFeedback(long startTime, float amplitude, float duration, float frequency, boolean hand) {
            mHandler.post(() -> {
                if (isVrModeNative()) {
                    onHapticsFeedbackNative(startTime, amplitude, duration, frequency, hand);
                }
            });
        }

        @Override
        public void onGuardianSyncAck(long timestamp) {
            mHandler.post(() -> {
                onGuardianSyncAckNative(timestamp);
            });
        }

        @Override
        public void onGuardianSegmentAck(long timestamp, int segmentIndex) {
            mHandler.post(() -> {
                onGuardianSegmentAckNative(timestamp, segmentIndex);
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


    private native void initializeNative(Activity activity, AssetManager assetManager, Activity ovrThread, boolean ARMode, int initialRefreshRate);
    private native void destroyNative();

    private native void onResumeNative();
    private native void onPauseNative();

    private native void onSurfaceCreatedNative(Surface surface);
    private native void onSurfaceChangedNative(Surface surface);
    private native void onSurfaceDestroyedNative();
    private native void renderNative(long renderedFrameIndex);
    private native void renderLoadingNative();
    private native void sendTrackingInfoNative(ServerConnection serverConnection);
    private native void sendMicDataNative(ServerConnection serverConnection);
    private native void sendGuardianInfoNative(ServerConnection serverConnection);

    private native int getLoadingTextureNative();
    private native int getSurfaceTextureIDNative();
    private native int getWebViewSurfaceTextureNative();

    private native boolean isVrModeNative();
    private native void getDeviceDescriptorNative(DeviceDescriptor deviceDescriptor);

    private native void setFrameGeometryNative(int width, int height);
    private native void setRefreshRateNative(int refreshRate);
    private native void setStreamMicNative(boolean streamMic);
    private native void setFFRParamsNative(int foveationMode, float foveationStrength, float foveationShape, float foveationVerticalOffset);
    private native void onHapticsFeedbackNative(long startTime, float amplitude, float duration, float frequency, boolean hand);
    private native void onGuardianSyncAckNative(long timestamp);
    private native void onGuardianSegmentAckNative(long timestamp, int segmentIndex);

    @SuppressWarnings("unused")
    public void applyWebViewInteractionEvent(int type, float x, float y) {
        mMainHandler.post(() -> {
            if (mWebViewWrapper != null && mWebViewWrapper.webView != null) {
                long time = SystemClock.uptimeMillis();

                int action = 0;
                boolean touchEvent = false;
                switch (type) {
                    case 0:
                        action = MotionEvent.ACTION_HOVER_ENTER;
                        touchEvent = false;
                        break;
                    case 1:
                        action = MotionEvent.ACTION_HOVER_EXIT;
                        touchEvent = false;
                        break;
                    case 2:
                        action = MotionEvent.ACTION_HOVER_MOVE;
                        touchEvent = false;
                        break;
                    case 3:
                        action = MotionEvent.ACTION_MOVE;
                        touchEvent = true;
                        break;
                    case 4:
                        action = MotionEvent.ACTION_DOWN;
                        touchEvent = true;
                        break;
                    case 5:
                        action = MotionEvent.ACTION_UP;
                        touchEvent = true;
                        break;
                }

                float mx = x * WEBVIEW_WIDTH;
                float my = y * WEBVIEW_HEIGHT;

                MotionEvent ev = MotionEvent.obtain(time, time, action, mx, my, 0);
                if (touchEvent) {
                    mWebViewWrapper.webView.dispatchTouchEvent(ev);
                } else {
                    mWebViewWrapper.webView.dispatchGenericMotionEvent(ev);
                }
            }
        });
    }

}

