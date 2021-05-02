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
import android.view.KeyEvent;
import android.view.Surface;
import android.view.SurfaceHolder;
import android.view.SurfaceView;
import android.view.Window;
import android.view.WindowManager;
import android.widget.Toast;

import androidx.annotation.NonNull;
import androidx.core.app.ActivityCompat;
import androidx.core.content.ContextCompat;

import java.util.Objects;
import java.util.concurrent.Semaphore;

public class OvrActivity extends Activity {
    static {
        System.loadLibrary("alvr_client");
    }

    final static String TAG = "OvrActivity";

    //Create placeholder for user's consent to record_audio permission.
    //This will be used in handling callback
    final int MY_PERMISSIONS_RECORD_AUDIO = 1;

    static class Preferences {
        String hostname;
        String certificatePEM;
        String privateKey;
        boolean darkMode;
    }

    public static class OnCreateResult {
        public int streamSurfaceHandle;
        public int loadingSurfaceHandle;
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
    EGLContext mEGLContext;
    boolean mVrMode = false;
    float mRefreshRate = 60f;
    String mDashboardURL = null;
    String mLoadingMessage = "";

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

        mEGLContext = EGL14.eglGetCurrentContext();
    }

    Preferences getPreferences() {
        Preferences p = new Preferences();

        SharedPreferences prefs = this.getSharedPreferences("pref", Context.MODE_PRIVATE);

        p.hostname = prefs.getString("hostname", "");
        p.certificatePEM = prefs.getString("certificate", "");
        p.privateKey = prefs.getString("private-key", "");
        p.darkMode = prefs.getBoolean("dark-mode", false);

        if (Objects.equals(p.hostname, "") || Objects.equals(p.certificatePEM, "") || Objects.equals(p.privateKey, "")) {
            createIdentity(p);

            SharedPreferences.Editor editor = prefs.edit();
            editor.putString("hostname", p.hostname);
            editor.putString("certificate", p.certificatePEM);
            editor.putString("private-key", p.privateKey);

            editor.apply();
        }

        return p;
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
                // Sometimes previous decoder output remains not updated (when previous call of waitFrame() didn't call updateTexImage())
                // and onFrameAvailable won't be called after next output.
                // To avoid deadlock caused by it, we need to flush last output.
                mStreamSurfaceTexture.updateTexImage();

                mDecoderThread = new DecoderThread(mStreamSurface, mDecoderCallback);

                try {
                    mDecoderThread.start();
                } catch (IllegalArgumentException | IllegalStateException | SecurityException e) {
                    Utils.loge(TAG, e::toString);
                }

                Preferences p = this.getPreferences();
                onResumeNative(NAL.class, p.hostname, p.certificatePEM, p.privateKey, mScreenSurface, p.darkMode);

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

                onVrModeChanged(false);

                onPauseNative();
            });
        }
    }

    @Override
    protected void onDestroy() {
        super.onDestroy();
        Semaphore sem = new Semaphore(1);
        try {
            sem.acquire();
        } catch (InterruptedException e) {
            e.printStackTrace();
        }
        mRenderingHandler.post(() -> {
            mLoadingTexture.destroyTexture();
            Utils.logi(TAG, () -> "Destroying vrapi state.");
            destroyNative();
            sem.release();
        });
        mRenderingHandlerThread.quitSafely();
        try {
            // Wait until destroyNative() is finished. Can't use Thread.join here, because
            // the posted lambda might not run, so wait on an object instead.
            sem.acquire();
            sem.release();
        } catch (InterruptedException e) {
            e.printStackTrace();
        }
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
                long renderedFrameIndex = mDecoderThread.clearAvailable(mStreamSurfaceTexture);

                if (renderedFrameIndex != -1) {
                    renderNative(renderedFrameIndex);
                }

                mRenderingHandler.removeCallbacks(mRenderRunnable);
                mRenderingHandler.postDelayed(mRenderRunnable, 1);
            } else {
                mLoadingTexture.drawMessage(mLoadingMessage);

                renderLoadingNative();
                mRenderingHandler.removeCallbacks(mRenderRunnable);
                mRenderingHandler.postDelayed(mRenderRunnable, (long) (1f / mRefreshRate));
            }
        }
    }

    public void onVrModeChanged(boolean enter) {
        mVrMode = enter;
        if (mVrMode) {
            mRenderingHandler.post(mRenderRunnable);
        }
    }

    private final DecoderThread.DecoderCallback mDecoderCallback = new DecoderThread.DecoderCallback() {
        @Override
        public void onPrepared() {
            requestIDR();
        }

        @Override
        public void onFrameDecoded() {
            if (mDecoderThread != null) {
                mDecoderThread.releaseBuffer();
            }
        }
    };

    static native void initNativeLogging();

    static native void createIdentity(Preferences p); // id fields are reset

    native void onCreateNative(AssetManager assetManager, OnCreateResult outResult);

    native void destroyNative();

    // nal_class is needed to access NAL objects fields in native code without access to a Java thread
    native void onResumeNative(Class<?> nal_class, String hostname, String certificatePEM, String privateKey, Surface screenSurface, boolean darkMode);

    native void onPauseNative();

    native void renderNative(long renderedFrameIndex);

    native void renderLoadingNative();

    native boolean isVrModeNative();

    native void onStreamStartNative();

    native void onHapticsFeedbackNative(long startTime, float amplitude, float duration, float frequency, boolean hand);

    native void onBatteryChangedNative(int battery);

    native boolean isConnectedNative();

    native void requestIDR();

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
    public void onServerConnected(float fps, int codec, boolean realtimeDecoder, String dashboardURL) {
        mRefreshRate = fps;
        mDashboardURL = dashboardURL;
        mRenderingHandler.post(() -> {
            onStreamStartNative();
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

    @SuppressWarnings("unused")
    void setDarkMode(boolean mode) {
        SharedPreferences.Editor editor = this.getSharedPreferences("pref", Context.MODE_PRIVATE).edit();
        editor.putBoolean("dark-mode", mode);
        editor.apply();
    }
}
