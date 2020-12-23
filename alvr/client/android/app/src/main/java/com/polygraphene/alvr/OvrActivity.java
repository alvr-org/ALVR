package com.polygraphene.alvr;

import android.Manifest;
import android.app.Activity;
import android.content.BroadcastReceiver;
import android.content.Context;
import android.content.Intent;
import android.content.IntentFilter;
import android.content.SharedPreferences;
import android.content.pm.PackageManager;
import android.content.res.AssetManager;
import android.graphics.SurfaceTexture;
import android.media.AudioManager;
import android.net.Uri;
import android.opengl.EGL14;
import android.opengl.EGLContext;
import android.os.BatteryManager;
import android.os.Bundle;
import android.os.Handler;
import android.os.HandlerThread;
import android.os.Looper;
import android.support.annotation.NonNull;
import android.support.v4.app.ActivityCompat;
import android.support.v4.content.ContextCompat;
import android.view.KeyEvent;
import android.view.Surface;
import android.view.SurfaceHolder;
import android.view.SurfaceView;
import android.view.ViewGroup;
import android.view.Window;
import android.view.WindowManager;
import android.widget.Toast;

import java.util.Objects;
import java.util.concurrent.TimeUnit;

public class OvrActivity extends Activity {
    static {
        System.loadLibrary("alvr_client");
    }

    final static String TAG = "OvrActivity";
    final static String TRUST_MESSAGE = "Open ALVR on PC and\nclick on \"Trust\" next to\nthe client entry";

    //Create placeholder for user's consent to record_audio permission.
    //This will be used in handling callback
    final int MY_PERMISSIONS_RECORD_AUDIO = 1;

    static class PrivateIdentity {
        String hostname;
        String certificatePEM;
        String privateKey;
    }

    public static class OnCreateResult {
        public int streamSurfaceHandle;
        public int loadingSurfaceHandle;
        public int refreshRate;
        public int renderWidth;
        public int renderHeight;
    }

    class RenderingCallbacks implements SurfaceHolder.Callback {
        @Override
        public void surfaceCreated(@NonNull final SurfaceHolder holder) {
            mScreenSurface = holder.getSurface();
            maybeResume();
        }

        @Override
        public void surfaceChanged(@NonNull SurfaceHolder holder, int _fmt, int _w, int _h) {
            maybePause();
            mScreenSurface = holder.getSurface();
            maybeResume();
        }

        @Override
        public void surfaceDestroyed(@NonNull SurfaceHolder holder) {
            maybePause();
            mScreenSurface = null;
        }
    }

    final BroadcastReceiver mBatInfoReceiver = new BroadcastReceiver() {
        @Override
        public void onReceive(Context ctxt, Intent intent) {
            onBatteryChangedNative(intent.getIntExtra(BatteryManager.EXTRA_LEVEL, 0));
        }
    };

    OnCreateResult deviceDescriptor = null;
    boolean mResumed = false;
    Handler mRenderingHandler;
    HandlerThread mRenderingHandlerThread;
    Surface mScreenSurface;
    SurfaceTexture mStreamSurfaceTexture;
    Surface mStreamSurface;
    final LoadingTexture mLoadingTexture = new LoadingTexture();
    DecoderThread mDecoderThread = null;
    ServerConnection mReceiverThread;
    EGLContext mEGLContext;
    boolean mVrMode = false;
    boolean mDecoderPrepared = false;
    int mRefreshRate = 72;
    long mPreviousRender = 0;
    String mDashboardURL = null;
    String mLoadingMessage = TRUST_MESSAGE;
    public final Object mWaiter = new Object();
    public OvrActivity self = this;

    // Cache method references for performance reasons
    final Runnable mRenderRunnable = this::render;

    @Override
    protected void onCreate(Bundle savedInstanceState) {
        super.onCreate(savedInstanceState);

        initNativeLogging();

        getWindow().addFlags(WindowManager.LayoutParams.FLAG_KEEP_SCREEN_ON);
        getWindow().addFlags(WindowManager.LayoutParams.FLAG_FULLSCREEN);
        requestWindowFeature(Window.FEATURE_NO_TITLE);

        setContentView(R.layout.activity_main);
        SurfaceView surfaceView = findViewById(R.id.surfaceview);

        mRenderingHandlerThread = new HandlerThread("Rendering thread");
        mRenderingHandlerThread.start();
        mRenderingHandler = new Handler(mRenderingHandlerThread.getLooper());
        mRenderingHandler.post(this::startup);

        SurfaceHolder holder = surfaceView.getHolder();
        holder.addCallback(new RenderingCallbacks());

        requestAudioPermissions();
        this.registerReceiver(this.mBatInfoReceiver, new IntentFilter(Intent.ACTION_BATTERY_CHANGED));
    }

    // This method initializes a GL context, and must be called within the scope of the rendering
    // handler, so successive rendering calls don't fail.
    public void startup() {
        deviceDescriptor = new OnCreateResult();
        onCreateNative(this.getAssets(), deviceDescriptor);

        mStreamSurfaceTexture = new SurfaceTexture(deviceDescriptor.streamSurfaceHandle);
        mStreamSurfaceTexture.setOnFrameAvailableListener(surfaceTexture -> {
            if (mDecoderThread != null) {
                mDecoderThread.onFrameAvailable();
            }
            mRenderingHandler.removeCallbacks(mRenderRunnable);
            mRenderingHandler.post(mRenderRunnable);
        }, new Handler(Looper.getMainLooper()));
        mStreamSurface = new Surface(mStreamSurfaceTexture);

        mLoadingTexture.initializeMessageCanvas(deviceDescriptor.loadingSurfaceHandle);
        mLoadingTexture.drawMessage(Utils.getVersionName(this) + "\nLoading...");

        mEGLContext = EGL14.eglGetCurrentContext();
    }

    PrivateIdentity getPrivateIdentity() {
        PrivateIdentity id = new PrivateIdentity();

        SharedPreferences prefs = this.getSharedPreferences("pref", Context.MODE_PRIVATE);

        id.hostname = prefs.getString("hostname", "");
        id.certificatePEM = prefs.getString("certificate", "");
        id.privateKey = prefs.getString("private-key", "");

        if (Objects.equals(id.hostname, "") || Objects.equals(id.certificatePEM, "") || Objects.equals(id.privateKey, "")) {
            createIdentity(id);

            SharedPreferences.Editor editor = prefs.edit();
            editor.putString("hostname", id.hostname);
            editor.putString("certificate", id.certificatePEM);
            editor.putString("private-key", id.privateKey);

            editor.apply();
        }

        return id;
    }

    @Override
    protected void onResume() {
        super.onResume();

        mResumed = true;
        maybeResume();
    }

    void maybeResume() {
        if (mResumed && mScreenSurface != null) {
            mRenderingHandler.post(() -> {

                mReceiverThread = new ServerConnection(this);

                // Sometimes previous decoder output remains not updated (when previous call of waitFrame() didn't call updateTexImage())
                // and onFrameAvailable won't be called after next output.
                // To avoid deadlock caused by it, we need to flush last output.
                mStreamSurfaceTexture.updateTexImage();

                mDecoderThread = new DecoderThread(mStreamSurface, mDecoderCallback);

                try {
                    mDecoderThread.start();
                    mReceiverThread.start();
                } catch (IllegalArgumentException | IllegalStateException | SecurityException e) {
                    Utils.loge(TAG, e::toString);
                }

                PrivateIdentity id = this.getPrivateIdentity();

                onResumeNative(id.hostname, id.certificatePEM, id.privateKey, mScreenSurface);

                onVrModeChanged(true);
            });
        }
    }

    @Override
    protected void onPause() {
        maybePause();
        mResumed = false;

        super.onPause();
    }

    void maybePause() {
        // the check (mResumed && mScreenSurface != null) is intended: either mResumed or
        // mScreenSurface != null will be false after this method returns.
        if (mResumed && mScreenSurface != null) {
            // DecoderThread must be stopped before ReceiverThread and setting mResumed=false.
            mRenderingHandler.post(() -> {
                // DecoderThread must be stopped before ReceiverThread and setting mResumed=false.
                if (mDecoderThread != null) {
                    mDecoderThread.stopAndWait();
                }
                if (mReceiverThread != null) {
                    mReceiverThread.stopAndWait();
                }

                onVrModeChanged(false);

                onPauseNative();
            });
        }
    }

    @Override
    protected void onDestroy() {
        super.onDestroy();

        mRenderingHandler.post(() -> {
            mLoadingTexture.destroyTexture();
            Utils.logi(TAG, () -> "Destroying vrapi state.");
            destroyNative();
        });
        mRenderingHandlerThread.quitSafely();
    }

    @Override
    public boolean dispatchKeyEvent(KeyEvent event) {
        //Utils.log(TAG, () ->  "dispatchKeyEvent: " + event.getKeyCode());
        if (event.getAction() == KeyEvent.ACTION_DOWN || event.getAction() == KeyEvent.ACTION_UP) {
            if (event.getKeyCode() == KeyEvent.KEYCODE_VOLUME_UP) {
                adjustVolume(1);
                return true;
            }
            if (event.getKeyCode() == KeyEvent.KEYCODE_VOLUME_DOWN) {
                adjustVolume(-1);
                return true;
            }

            return true;
        } else {
            return super.dispatchKeyEvent(event);
        }
    }

    private void adjustVolume(int direction) {
        AudioManager audio = (AudioManager) getSystemService(Context.AUDIO_SERVICE);
        audio.adjustStreamVolume(AudioManager.STREAM_MUSIC, direction, 0);
    }

    private void requestAudioPermissions() {
        if (ContextCompat.checkSelfPermission(this,
                Manifest.permission.RECORD_AUDIO)
                != PackageManager.PERMISSION_GRANTED) {

            //When permission is not granted by user, show them message why this permission is needed.
            if (ActivityCompat.shouldShowRequestPermissionRationale(this,
                    Manifest.permission.RECORD_AUDIO)) {
                Toast.makeText(this, "Please grant permissions to use microphone", Toast.LENGTH_LONG).show();
            }

            ActivityCompat.requestPermissions(this,
                    new String[]{Manifest.permission.RECORD_AUDIO},
                    MY_PERMISSIONS_RECORD_AUDIO);
        } else {
            ContextCompat.checkSelfPermission(this,
                    Manifest.permission.RECORD_AUDIO);//Go ahead with recording audio now
        }
    }

    //Handling callback
    @Override
    public void onRequestPermissionsResult(int requestCode, @NonNull String[] permissions, @NonNull int[] grantResults) {
        if (requestCode == MY_PERMISSIONS_RECORD_AUDIO) {
            if (grantResults.length <= 0 || grantResults[0] != PackageManager.PERMISSION_GRANTED) {
                Toast.makeText(this, "Permissions Denied to record audio", Toast.LENGTH_LONG).show();
            }
        }
    }

    private void render() {
        if (mResumed && mScreenSurface != null) {
            if (isConnectedNative()) {
                long next = checkRenderTiming();
                if (next > 0) {
                    mRenderingHandler.postDelayed(mRenderRunnable, next);
                    return;
                }
                long renderedFrameIndex = mDecoderThread.clearAvailable(mStreamSurfaceTexture);

                if (renderedFrameIndex != -1) {
                    renderNative(renderedFrameIndex);
                    mPreviousRender = System.nanoTime();

                    mRenderingHandler.postDelayed(mRenderRunnable, 5);
                } else {
                    mRenderingHandler.removeCallbacks(mRenderRunnable);
                    mRenderingHandler.postDelayed(mRenderRunnable, 50);
                }
            } else {
                mLoadingTexture.drawMessage(Utils.getVersionName(this) + "\n\n" + mLoadingMessage);

                renderLoadingNative();
                mRenderingHandler.removeCallbacks(mRenderRunnable);
                mRenderingHandler.postDelayed(mRenderRunnable, 13); // 72Hz = 13.8888ms
            }
        }
    }

    private long checkRenderTiming() {
        long current = System.nanoTime();
        long threshold = TimeUnit.SECONDS.toNanos(1) / mRefreshRate -
                TimeUnit.MILLISECONDS.toNanos(5);
        return TimeUnit.NANOSECONDS.toMillis(threshold - (current - mPreviousRender));
    }

    public void onVrModeChanged(boolean enter) {
        mVrMode = enter;
        Utils.logi(TAG, () -> "onVrModeChanged. mVrMode=" + mVrMode + " mDecoderPrepared=" + mDecoderPrepared);
        if (mReceiverThread != null) {
            if (mVrMode) {
                mRenderingHandler.post(mRenderRunnable);
            }
        }
    }

    private final DecoderThread.DecoderCallback mDecoderCallback = new DecoderThread.DecoderCallback() {
        @Override
        public void onPrepared() {
            mDecoderPrepared = true;
            Utils.logi(TAG, () -> "DecoderCallback.onPrepared. mVrMode=" + mVrMode + " mDecoderPrepared=" + mDecoderPrepared);
        }

        @Override
        public void onDestroy() {
            mDecoderPrepared = false;
            Utils.logi(TAG, () -> "DecoderCallback.onDestroy. mVrMode=" + mVrMode + " mDecoderPrepared=" + mDecoderPrepared);
        }

        @Override
        public void onFrameDecoded() {
            if (mDecoderThread != null) {
                mDecoderThread.releaseBuffer();
            }
        }
    };

    static native void initNativeLogging();

    static native void createIdentity(PrivateIdentity id); // id fields are reset

    native void onCreateNative(AssetManager assetManager, OnCreateResult outResult);

    native void destroyNative();

    native void onResumeNative(String hostname, String certificatePEM, String privateKey, Surface screenSurface);

    native void onPauseNative();

    native void renderNative(long renderedFrameIndex);

    native void renderLoadingNative();

    native void onTrackingNative();

    native boolean isVrModeNative();

    native void onStreamStartNative(int width, int height, int refreshRate, boolean streamMic, int foveationMode, float foveationStrength, float foveationShape, float foveationVerticalOffset, int trackingSpaceType);

    native void onHapticsFeedbackNative(long startTime, float amplitude, float duration, float frequency, boolean hand);

    native void onGuardianSyncAckNative(long timestamp);

    native void onGuardianSegmentAckNative(long timestamp, int segmentIndex);

    native void onBatteryChangedNative(int battery);

    native void initializeSocket();

    native void closeSocket();

    native void runLoop();

    native void interruptNative();

    native void sendNative(long nativeBuffer, int bufferLength);

    native boolean isConnectedNative();

    @SuppressWarnings("unused")
    public void openDashboard() {
        if (mDashboardURL != null) {
            Intent browserIntent = new Intent(Intent.ACTION_VIEW, Uri.parse(mDashboardURL));
            startActivity(browserIntent);
        }
    }

    @SuppressWarnings("unused")
    public void setLoadingMessage(String message) {
        mLoadingMessage = message;
    }

    @SuppressWarnings("unused")
    public void onServerConnected(int width, int height, int codec, boolean realtimeDecoder,
                                  int refreshRate, boolean streamMic, int foveationMode,
                                  float foveationStrength, float foveationShape,
                                  float foveationVerticalOffset, int trackingSpaceType,
                                  String dashboardURL) {
        mDashboardURL = dashboardURL;
        mRenderingHandler.post(() -> {
            onStreamStartNative(width, height, refreshRate, streamMic, foveationMode, foveationStrength, foveationShape, foveationVerticalOffset, trackingSpaceType);
            mDecoderThread.onConnect(codec, realtimeDecoder);
        });
    }

    @SuppressWarnings("unused")
    public void onDisconnected() {
        Utils.logi(TAG, () -> "onDisconnected is called.");
        if (mDecoderThread != null) {
            mDecoderThread.onDisconnect();
        }
    }

    @SuppressWarnings("unused")
    public void onHapticsFeedback(long startTime, float amplitude, float duration, float frequency, boolean hand) {
        mRenderingHandler.post(() -> {
            if (mResumed && mScreenSurface != null) {
                onHapticsFeedbackNative(startTime, amplitude, duration, frequency, hand);
            }
        });
    }

    @SuppressWarnings("unused")
    public void onGuardianSyncAck(long timestamp) {
        mRenderingHandler.post(() -> onGuardianSyncAckNative(timestamp));
    }

    @SuppressWarnings("unused")
    public void onGuardianSegmentAck(long timestamp, int segmentIndex) {
        mRenderingHandler.post(() -> onGuardianSegmentAckNative(timestamp, segmentIndex));
    }

    @SuppressWarnings("unused")
    public void send(long nativeBuffer, int bufferLength) {
        synchronized (mWaiter) {
            sendNative(nativeBuffer, bufferLength);
        }
    }

    @SuppressWarnings("unused")
    public NAL obtainNAL(int length) {
        if (mDecoderThread != null) {
            return mDecoderThread.obtainNAL(length);
        } else {
            NAL nal = new NAL();
            nal.length = length;
            nal.buf = new byte[length];
            return nal;
        }
    }

    @SuppressWarnings("unused")
    public void pushNAL(NAL nal) {
        if (mDecoderThread != null) {
            mDecoderThread.pushNAL(nal);
        }
    }
}
