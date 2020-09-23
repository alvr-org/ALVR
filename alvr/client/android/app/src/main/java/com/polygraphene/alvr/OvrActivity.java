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

public class OvrActivity extends Activity {
    static {
        System.loadLibrary("alvr_client");
    }

    private final static String TAG = "OvrActivity";

    //Create placeholder for user's consent to record_audio permission.
    //This will be used in handling callback
    private final int MY_PERMISSIONS_RECORD_AUDIO = 1;

    static class PrivateIdentity {
        String hostname;
        String certificatePEM;
        String privateKey;
    }

    static class OnCreateNativeOutParams {
        int streamSurfaceHandle;
        int webviewSurfaceHandle;
    }

    class RenderingCallbacks implements SurfaceHolder.Callback {

        @Override
        public void surfaceCreated(@NonNull SurfaceHolder holder) {
            mScreenSurface = holder.getSurface();
            maybeResume();
        }

        @Override
        public void surfaceChanged(@NonNull SurfaceHolder holder, int fmt, int w, int h) {
            maybePause();
            mScreenSurface = holder.getSurface();
            maybeResume();
        }

        @Override
        public void surfaceDestroyed(@NonNull SurfaceHolder surfaceHolder) {
            maybePause();
            mScreenSurface = null;
        }
    }

    static class DecoderCallbacks implements DecoderThread.DecoderCallback {
        @Override
        public void onFrameInput() {
            onFrameInputNative();
        }

        @Override
        public void onFrameOutput() {
            onFrameOutputNative();
        }
    }

    boolean mResumed = false;
    Handler mRenderingHandler = null;
    HandlerThread mRenderingHandlerThread = null;
    Surface mScreenSurface = null;
    SurfaceTexture mStreamSurfaceTexture = null;
    Surface mStreamSurface = null;
    SurfaceTexture mWebViewSurfaceTexture = null;
    OffscreenWebView mWebView = null;
    boolean mWebViewVisible = false;
    String mDashboardURL = "";
    DecoderThread mDecoderThread = null;
    DecoderThread.DecoderCallback mDecoderCallbacks = new DecoderCallbacks();
    int mCodec = 0;
    long mPreviousRender = 0;
    float mRefreshRate = 0;

    @Override
    protected void onCreate(Bundle savedInstanceState) {
        super.onCreate(savedInstanceState);

        initNativeLogging();

        getWindow().addFlags(WindowManager.LayoutParams.FLAG_KEEP_SCREEN_ON);
        getWindow().addFlags(WindowManager.LayoutParams.FLAG_FULLSCREEN);
        requestWindowFeature(Window.FEATURE_NO_TITLE);

        setContentView(R.layout.activity_main);
        SurfaceView surfaceView = findViewById(R.id.surfaceview);

        Handler webViewHandler = new Handler(this.getMainLooper());
        mWebView = new OffscreenWebView(this, webViewHandler);

        mRenderingHandlerThread = new HandlerThread("Rendering thread");
        mRenderingHandlerThread.start();
        mRenderingHandler = new Handler(mRenderingHandlerThread.getLooper());
        mRenderingHandler.post(this::startup);

        SurfaceHolder holder = surfaceView.getHolder();
        holder.addCallback(new RenderingCallbacks());

        requestAudioPermissions();
    }

    public void startup() {
        // this call initializes a GL context, and this must be done within the scope of the
        // rendering handler, so successive rendering calls don't fail.
        OnCreateNativeOutParams params = new OnCreateNativeOutParams();
        onCreateNative(this, this.getAssets(), params);

        mStreamSurfaceTexture = new SurfaceTexture(params.streamSurfaceHandle);
        mStreamSurfaceTexture.setOnFrameAvailableListener(surfaceTexture -> {
            if (mDecoderThread != null) {
                mDecoderThread.onFrameAvailable();
            }

            mRenderingHandler.removeCallbacks(this::render);
            mRenderingHandler.post(this::render);
        }, new Handler(Looper.getMainLooper()));
        mStreamSurface = new Surface(mStreamSurfaceTexture);

        mWebViewSurfaceTexture = new SurfaceTexture(params.webviewSurfaceHandle);
        mWebViewSurfaceTexture.setDefaultBufferSize(WEBVIEW_WIDTH, WEBVIEW_HEIGHT);
        Surface webViewSurface = new Surface(mWebViewSurfaceTexture);
        mWebView.setSurface(webViewSurface);
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
    public void onRequestPermissionsResult(int requestCode, @NonNull String[] permissions, @NonNull int[] grantResults) {
        if (requestCode == MY_PERMISSIONS_RECORD_AUDIO) {
            if (grantResults.length <= 0
                    || grantResults[0] != PackageManager.PERMISSION_GRANTED) {
                Toast.makeText(this, "Permissions Denied to record audio", Toast.LENGTH_LONG).show();
            }
        }
    }

    @Override
    public boolean dispatchKeyEvent(KeyEvent event) {
        if (event.getAction() == KeyEvent.ACTION_DOWN || event.getAction() == KeyEvent.ACTION_UP) {
            int direction = 0;
            if (event.getKeyCode() == KeyEvent.KEYCODE_VOLUME_UP) {
                direction = 1;
            } else if (event.getKeyCode() == KeyEvent.KEYCODE_VOLUME_DOWN) {
                direction = -1;
            }

            AudioManager audio = (AudioManager) getSystemService(Context.AUDIO_SERVICE);
            audio.adjustStreamVolume(AudioManager.STREAM_MUSIC, direction, 0);

            return true;
        } else {
            return super.dispatchKeyEvent(event);
        }
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

    @Override
    protected void onResume() {
        super.onResume();

        mResumed = true;
        maybeResume();
    }

    void maybeResume() {
        if (mResumed && mScreenSurface != null) {
            mRenderingHandler.post(() -> {
                mWebView.setMessage("Launch ALVR on PC and click on \"Trust\" next to the client entry");

                // Sometimes previous decoder output remains not updated (when previous call of waitFrame() didn't call updateTexImage())
                // and onFrameAvailable won't be called after next output.
                // To avoid deadlock caused by it, we need to flush last output.
                mStreamSurfaceTexture.updateTexImage();

                PrivateIdentity id = this.getCertificate();

                // initialize Ovr, enable vr mode, startup sockets
                mRefreshRate = onResumeNative(this, id.hostname, id.certificatePEM, id.privateKey, mScreenSurface);
            });
            mRenderingHandler.postDelayed(this::render, 13);
        }
    }

    private void render() {
        if (mResumed && mScreenSurface != null) {
            if (mWebViewVisible) {
                mWebViewSurfaceTexture.updateTexImage();
//                Utils.loge(TAG, () -> "updateTexImage");
            }

            if (mDecoderThread != null) {
                long next = checkRenderTiming();
                if (next > 0) {
                    mRenderingHandler.postDelayed(this::render, next);
                    return;
                }
                long renderedFrameIndex = mDecoderThread.clearAvailable(mStreamSurfaceTexture);

                if (renderedFrameIndex != -1) {
                    renderNative(true, renderedFrameIndex);
                    mPreviousRender = System.nanoTime();

                    mRenderingHandler.postDelayed(this::render, 5);
                } else {
                    mRenderingHandler.removeCallbacks(this::render);
                    mRenderingHandler.postDelayed(this::render, 50);
                }
            } else {
                renderNative(false, 0);

                mRenderingHandler.removeCallbacks(this::render);
                mRenderingHandler.postDelayed(this::render, (long) (1 / mRefreshRate));
            }
        }
    }

    private long checkRenderTiming() {
        long current = System.nanoTime();
        long threshold = (long) (TimeUnit.SECONDS.toNanos(1) / mRefreshRate) -
                TimeUnit.MILLISECONDS.toNanos(5);
        return TimeUnit.NANOSECONDS.toMillis(threshold - (current - mPreviousRender));
    }

    @Override
    protected void onPause() {
        maybePause();
        mResumed = false;

        super.onPause();
    }

    void maybePause() {
        // the check (mResumed && mSurfaceValid) is intended: either mResumed or mSurfaceValid will
        // be set to false once this method returns.
        if (mResumed && mScreenSurface != null) {
            onStreamStop(false);

            mRenderingHandler.post(OvrActivity::onPauseNative);
        }
    }

    @Override
    protected void onDestroy() {
        mRenderingHandler.post(() -> {
            Utils.logi(TAG, () -> "Destroying vrapi state.");
            onDestroyNative();
        });
        mRenderingHandlerThread.quitSafely();

        super.onDestroy();
    }


    // INTEROPERATION WITH RUST:

    // Java to Rust:

    static native void initNativeLogging();

    static native void createIdentity(PrivateIdentity id); // id fields are reset

    static native void onCreateNative(Activity activity, AssetManager assetManager, OnCreateNativeOutParams outParams);

    static native float onResumeNative(Activity activity, String hostname, String certificatePEM, String privateKey, Surface screenSurface); // returns default framerate

    // this callback is needed to call C++ code inside the gl context using the rendering handler
    static native float onStreamStartNative();

    static native void renderNative(boolean streaming, long frameIdx);

    static native void onFrameInputNative(); // only for statistics

    static native void onFrameOutputNative(); // only for statistics

    // this callback is needed to call C++ code inside the gl context using the rendering handler
    static native float onStreamStopNative();

    static native void onPauseNative();

    static native void onDestroyNative();

    // Rust to Java:

    @SuppressLint("SetJavaScriptEnabled")
    @SuppressWarnings("unused")
    public void onServerFound(boolean isCompatible, String url, int codec) {
        if (isCompatible) {
            mDashboardURL = url;
            mCodec = codec;

            mWebView.setMessage("Server found, the stream will begin shortly");
        } else {
            mWebView.setMessage("Found unsupported server. Make sure the client and the server are up to date.");
        }
    }

    @SuppressWarnings("unused")
    public void onStreamStart() {
        mRenderingHandler.post(OvrActivity::onStreamStartNative);

        if (mDecoderThread != null) {
            mDecoderThread.onDisconnect();
        }
        mDecoderThread = new DecoderThread(mStreamSurface, mDecoderCallbacks, mCodec);
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
    public void pushNAL(NAL nal) {
        if (mDecoderThread != null) {
            mDecoderThread.pushNAL(nal);
        }
    }

    public void onStreamStop(boolean restarting) {
        if (restarting) {
            mWebView.setMessage("Server is restarting, please wait.");
        } else {
            mWebView.setMessage("Server disconnected.");
        }

        if (mDecoderThread != null) {
            mDecoderThread.onDisconnect();
            mDecoderThread = null;
        }

        mRenderingHandler.post(OvrActivity::onStreamStopNative);
    }

    @SuppressWarnings("unused")
    public void applyWebViewInteractionEvent(int type, float x, float y) {
        mWebView.applyWebViewInteractionEvent(type, x, y);
    }

    @SuppressWarnings("unused")
    public void setWebViewVisibility(boolean visible) {
        // detach webview from view tree when not needed
        this.runOnUiThread(() -> {
            if (visible && !mWebViewVisible) {
                this.addContentView(mWebView, new ViewGroup.LayoutParams(WEBVIEW_WIDTH, WEBVIEW_HEIGHT));
            } else if (!visible && mWebViewVisible) {
                ((ViewGroup) mWebView.getParent()).removeView(mWebView);
            }

            mWebViewVisible = visible;
        });
    }
}