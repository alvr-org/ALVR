package com.polygraphene.alvr;

import android.Manifest;
import android.annotation.SuppressLint;
import android.app.Activity;
import android.content.Context;
import android.content.SharedPreferences;
import android.content.pm.PackageManager;
import android.content.res.AssetManager;
import android.graphics.SurfaceTexture;
import android.media.AudioManager;
import android.os.Bundle;
import android.os.Handler;
import android.os.HandlerThread;
import android.os.Looper;
import android.os.SystemClock;
import android.support.annotation.NonNull;
import android.support.v4.app.ActivityCompat;
import android.support.v4.content.ContextCompat;
import android.view.KeyEvent;
import android.view.MotionEvent;
import android.view.Surface;
import android.view.SurfaceHolder;
import android.view.SurfaceView;
import android.view.ViewGroup;
import android.view.Window;
import android.view.WindowManager;
import android.widget.Toast;

import java.util.Objects;
import java.util.concurrent.TimeUnit;

import static com.polygraphene.alvr.OffscreenWebView.WEBVIEW_HEIGHT;
import static com.polygraphene.alvr.OffscreenWebView.WEBVIEW_WIDTH;

public class OvrActivity extends BaseActivity {
    private final static String TAG = "OvrActivity";

    //Create placeholder for user's consent to record_audio permission.
    //This will be used in handling callback
    private final int MY_PERMISSIONS_RECORD_AUDIO = 1;

    static class PrivateIdentity {
        String hostname;
        String certificatePEM;
        String privateKey;
    }

    static class OnCreateNativeParams {
        // input:
        Activity javaParent;
        AssetManager assetManager;

        //output:
        int streamSurfaceHandle;
        int webviewSurfaceHandle;
    }

    static class OnResumeNativeParams {
        String hostname;
        String certificatePEM;
        String privateKey;
    }

    class RenderingCallbacks implements SurfaceHolder.Callback {

        @Override
        public void surfaceCreated(@NonNull SurfaceHolder holder) {
            mRenderingHandler.post(() -> surfaceCreatedNative(holder.getSurface()));
        }

        @Override
        public void surfaceChanged(@NonNull SurfaceHolder holder, int fmt, int w, int h) {
            mRenderingHandler.post(() -> surfaceChangedNative(holder.getSurface()));
        }

        @Override
        public void surfaceDestroyed(@NonNull SurfaceHolder surfaceHolder) {
            mRenderingHandler.post(OvrActivity::surfaceDestroyedNative);
        }
    }

    class DecoderCallbacks implements DecoderThread.DecoderCallback {
        @Override
        public void onPrepared() {
            mDecoderPrepared = true;
            mReceiverThread.setSinkPrepared(mVrMode && mDecoderPrepared);
        }

        @Override
        public void onDestroy() {
            mDecoderPrepared = false;
            mReceiverThread.setSinkPrepared(mVrMode && mDecoderPrepared);
        }

        @Override
        public void onFrameDecoded() {
            mDecoderThread.releaseBuffer();
        }
    };

    OffscreenWebView mWebView = null;
    Handler mMainHandler = null;
    Handler mRenderingHandler = null;
    SurfaceTexture mStreamSurfaceTexture = null;
    Surface mStreamSurface = null;
    SurfaceTexture mWebViewSurfaceTexture = null;
    Surface mWebViewSurface = null;
    DecoderThread mDecoderThread = null;
    DecoderThread.DecoderCallback mDecoderCallbacks = new DecoderCallbacks();
    boolean mDecoderPrepared = false;
    boolean mConnected = false;
    boolean mStreaming = false;
    boolean mShowingWebView = true;
    String mDashboardURL = "";

    @Override
    protected void onCreate(Bundle savedInstanceState) {
        super.onCreate(savedInstanceState);

        initNativeRuntime();

        getWindow().addFlags(WindowManager.LayoutParams.FLAG_KEEP_SCREEN_ON);
        getWindow().addFlags(WindowManager.LayoutParams.FLAG_FULLSCREEN);
        requestWindowFeature(Window.FEATURE_NO_TITLE);

        setContentView(R.layout.activity_main);
        SurfaceView surfaceView = findViewById(R.id.surfaceview);

        mWebView = new OffscreenWebView(this);
        mWebView.setMessage("Launch ALVR on PC and click on \"Trust\" next to the client entry");
        addContentView(mWebView, new ViewGroup.LayoutParams(WEBVIEW_WIDTH, WEBVIEW_HEIGHT));

        mMainHandler = new Handler(this.getMainLooper());

        HandlerThread handlerThread = new HandlerThread("Rendering thread");
        handlerThread.start();
        mRenderingHandler = new Handler(handlerThread.getLooper());
        mRenderingHandler.post(this::startup);

        SurfaceHolder holder = surfaceView.getHolder();
        holder.addCallback(new RenderingCallbacks());

        requestAudioPermissions();
    }

    PrivateIdentity getCertificate() {
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

    public void startup() {
        OnCreateNativeParams params = new OnCreateNativeParams();
        params.javaParent = this;
        params.assetManager = this.getAssets();

        // this call initializes a GL context, and this must be done within the scope of the
        // rendering handler, so successive rendering calls don't fail.
        onCreateNative(params);

        mStreamSurfaceTexture = new SurfaceTexture(params.streamSurfaceHandle);
        mStreamSurfaceTexture.setOnFrameAvailableListener(surfaceTexture -> {
            mDecoderThread.onFrameAvailable();
            mRenderingHandler.removeCallbacks(this::render);
            mRenderingHandler.post(this::render);
        }, new Handler(Looper.getMainLooper()));
        mStreamSurface = new Surface(mStreamSurfaceTexture);

        mWebViewSurfaceTexture = new SurfaceTexture(params.webviewSurfaceHandle);
        mWebViewSurfaceTexture.setDefaultBufferSize(WEBVIEW_WIDTH, WEBVIEW_HEIGHT);
        mWebViewSurface = new Surface(mWebViewSurfaceTexture);
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

    @Override
    protected void onResume() {
        super.onResume();

        mRenderingHandler.post(() -> {
            PrivateIdentity id = this.getCertificate();

            OnResumeNativeParams params = new OnResumeNativeParams();
            params.hostname = id.hostname;
            params.certificatePEM = id.certificatePEM;
            params.privateKey = id.privateKey;

            onResumeNative(params);

            // Sometimes previous decoder output remains not updated (when previous call of waitFrame() didn't call updateTexImage())
            // and onFrameAvailable won't be called after next output.
            // To avoid deadlock caused by it, we need to flush last output.
            mStreamSurfaceTexture.updateTexImage();

            mDecoderThread = new DecoderThread(mStreamSurface, this, mDecoderCallbacks);

            try {
                mDecoderThread.start();
            } catch (IllegalArgumentException | IllegalStateException | SecurityException e) {
                Utils.loge(TAG, e::toString);
            }
        });
    }

    private void render() {
        if (mShowingWebView) {
            mWebViewSurfaceTexture.updateTexImage();
        }

        if (mStreaming)
        {
            long next = checkRenderTiming();
            if(next > 0) {
                mRenderingHandler.postDelayed(this::render, next);
                return;
            }
            long renderedFrameIndex = mDecoderThread.clearAvailable(mStreamSurfaceTexture);

            if (renderedFrameIndex != -1)
            {
                renderNative(false, renderedFrameIndex);
                mPreviousRender = System.nanoTime();

                mRenderingHandler.postDelayed(mRenderRunnable, 5);
            }
            else
            {
                mRenderingHandler.removeCallbacks(this::render);
                mRenderingHandler.postDelayed(this::render, 50);
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
            else
            {
                mLoadingTexture.drawMessage(Utils.getVersionName(mActivity) + "\n \nOpen ALVR on PC and\nclick on \"Trust\" next to\nthe client entry");
            }
            mHandler.removeCallbacks(mIdleRenderRunnable);
            mHandler.postDelayed(mIdleRenderRunnable, 13); // 72Hz = 13.8888ms
        }

        renderNative(mStreaming, );
    }

    private long checkRenderTiming() {
        long current = System.nanoTime();
        long threashold = TimeUnit.SECONDS.toNanos(1) / mRefreshRate -
                TimeUnit.MILLISECONDS.toNanos(5);
        return TimeUnit.NANOSECONDS.toMillis(threashold - (current - mPreviousRender));
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

            mOvrContext.onPause();
        });
    }

    @Override
    protected void onDestroy() {
        super.onDestroy();

        mHandler.post(() -> {
            mLoadingTexture.destroyTexture();
            Utils.logi(TAG, () -> "Destroying vrapi state.");
            mOvrContext.destroy();
        });
        mHandlerThread.quitSafely();
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

    // INTEROPERATION WITH RUST:


    // Java to Rust:

    static native void initNativeRuntime();

    static native void createIdentity(PrivateIdentity id); // id fields are reset

    static native void onCreateNative(OnCreateNativeParams data);

    static native void surfaceCreatedNative(Surface surface);

    static native void surfaceChangedNative(Surface surface);

    static native void surfaceDestroyedNative();

    static native void initSockets();

    static native void onResumeNative(OnResumeNativeParams params);

    static native void renderNative(boolean streaming, long frameIdx);

    static native void onPauseNative();

    static native void onDestroyNative();


    // Rust to Java:

    @SuppressLint("SetJavaScriptEnabled")
    @SuppressWarnings("unused")
    public void onServerFound(boolean isCompatible, String url, String incompatibleMessage) {
        if (isCompatible) {
            mDashboardURL = url;
        } else {
            mMainHandler.post(() -> mWebView.setMessage("Server found, the stream will begin shortly"));
        }
    }

    @SuppressWarnings("unused")
    public void onServerConnected() {
        // We now have dashboard url, so we can post() to the main thread to set up our WebView.
        mMainHandler.post(() -> mWebView.setMessage("Server found, the stream will begin shortly"));

        if (mDecoderThread != null) {
            mDecoderThread.onDisconnect();
        }
        mDecoderThread = new DecoderThread();


//        mStreaming = true;
//        mShowingWebView = false;
    }

    @SuppressWarnings("unused")
    public NAL getNALBuffer(int bufLength) {
        if (mDecoderThread != null) {
            return mDecoderThread.obtainNAL(bufLength);
        } else {
            NAL nal = new NAL();
            nal.buf = new byte[bufLength];
            return nal;
        }
    }

    @SuppressWarnings("unused")
    public NAL pushNAL(NAL nal) {
        if (mDecoderThread != null) {
            mDecoderThread.pushNAL(nal);
        }
    }


    @SuppressWarnings("unused")
    public void onServerDisconnected(boolean restarting) {
        if (restarting) {
            mMainHandler.post(() -> mWebView.setMessage("Server is restarting, please wait."));
        } else {
            mMainHandler.post(() -> mWebView.setMessage("Server disconnected."));
        }
        mStreaming = true;
        mShowingWebView = false;

        if (mDecoderThread != null) {
            mDecoderThread.onDisconnect();
            mDecoderThread = null;
        }
    }

//    @SuppressWarnings("unused")
//    public void onVrModeChanged(boolean enter) {
//        mVrMode = enter;
//        Utils.logi(TAG, () -> "onVrModeChanged. mVrMode=" + mVrMode + " mDecoderPrepared=" + mDecoderPrepared);
//        mReceiverThread.setSinkPrepared(mVrMode && mDecoderPrepared);
//        if (mVrMode) {
//            mHandler.post(mRenderRunnable);
//        }
//    }

    @SuppressWarnings("unused")
    public void applyWebViewInteractionEvent(int type, float x, float y) {
        mMainHandler.post(() -> {
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
                mWebView.dispatchTouchEvent(ev);
            } else {
                mWebView.dispatchGenericMotionEvent(ev);
            }
        });
    }
}

