package com.polygraphene.alvr;

import android.app.Activity;
import android.content.BroadcastReceiver;
import android.content.Context;
import android.content.Intent;
import android.content.IntentFilter;
import android.content.res.AssetManager;
import android.net.Uri;
import android.opengl.EGL14;
import android.opengl.EGLContext;
import android.os.BatteryManager;
import android.os.Bundle;
import android.os.Handler;
import android.os.HandlerThread;
import android.os.Looper;
import android.view.Surface;
import android.view.SurfaceHolder;
import android.view.SurfaceView;
import android.view.Window;
import android.view.WindowManager;

import androidx.annotation.NonNull;

import java.util.concurrent.Semaphore;

public class OvrActivity extends Activity {
    static {
        System.loadLibrary("alvr_client");
    }

    final static String TAG = "OvrActivity";

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
            onBatteryChangedNative(intent.getIntExtra(BatteryManager.EXTRA_LEVEL, 0), intent.getIntExtra(BatteryManager.EXTRA_PLUGGED, 0));
        }
    };

    boolean mResumed = false;
    Handler mRenderingHandler;
    HandlerThread mRenderingHandlerThread;
    Surface mScreenSurface;
    DecoderThread mDecoderThread = null;
    float mRefreshRate = 60f;
    String mDashboardURL = null;
    int mStreamSurfaceHandle;

    // Cache method references for performance reasons
    final Runnable mRenderRunnable = this::render;

    @Override
    protected void onCreate(Bundle savedInstanceState) {
        super.onCreate(savedInstanceState);

        getWindow().addFlags(WindowManager.LayoutParams.FLAG_KEEP_SCREEN_ON);
        getWindow().addFlags(WindowManager.LayoutParams.FLAG_FULLSCREEN);
        requestWindowFeature(Window.FEATURE_NO_TITLE);

        setContentView(R.layout.activity_main);
        SurfaceView surfaceView = findViewById(R.id.surfaceview);

        mRenderingHandlerThread = new HandlerThread("Rendering thread");
        mRenderingHandlerThread.start();
        mRenderingHandler = new Handler(mRenderingHandlerThread.getLooper());
        mRenderingHandler.post(this::initializeNative);

        SurfaceHolder holder = surfaceView.getHolder();
        holder.addCallback(new RenderingCallbacks());

        this.registerReceiver(this.mBatInfoReceiver, new IntentFilter(Intent.ACTION_BATTERY_CHANGED));
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
                mDecoderThread = new DecoderThread();
                onResumeNative(mScreenSurface, mDecoderThread);

                // bootstrap the rendering loop
                mRenderingHandler.post(mRenderRunnable);
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
            mRenderingHandler.post(() -> {
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

    private void render() {
        if (mResumed && mScreenSurface != null) {
            if (isConnectedNative()) {
                renderNative();

                mRenderingHandler.removeCallbacks(mRenderRunnable);
                mRenderingHandler.postDelayed(mRenderRunnable, 1);
            } else {
                renderLoadingNative();
                mRenderingHandler.removeCallbacks(mRenderRunnable);
                mRenderingHandler.postDelayed(mRenderRunnable, (long) (1f / mRefreshRate));
            }
        }
    }

    native void initializeNative();

    native void destroyNative();

    // nal_class is needed to access NAL objects fields in native code without access to a Java thread
    native void onResumeNative(Surface screenSurface, DecoderThread decoder);

    native void onPauseNative();

    native void renderNative();

    native void renderLoadingNative();

    native void onStreamStartNative(int codec, boolean realtimeDecoder);

    native void onBatteryChangedNative(int battery, int plugged);

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
    public void onServerConnected(float fps, int codec, boolean realtimeDecoder, String dashboardURL) {
        mRefreshRate = fps;
        mDashboardURL = dashboardURL;
        mRenderingHandler.post(() -> {
            onStreamStartNative(codec, realtimeDecoder);
        });
    }

    @SuppressWarnings("unused")
    public void restartRenderCycle() {
        mRenderingHandler.removeCallbacks(mRenderRunnable);
        mRenderingHandler.post(mRenderRunnable);
    }
}
