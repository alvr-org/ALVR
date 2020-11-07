package com.polygraphene.alvr;

import android.app.Activity;
import android.content.res.AssetManager;
import android.os.Handler;
import android.os.SystemClock;
import android.view.InputDevice;
import android.view.MotionEvent;
import android.view.Surface;
import android.webkit.WebView;

public class OvrContext {

    static {
        System.loadLibrary("alvr_client");
    }

    private Handler mMainHandler;

    public OvrThread.WebViewWrapper mWebViewWrapper = null;

    public void initialize(Activity activity, AssetManager assetManager, OvrThread ovrThread, boolean ARMode, int initialRefreshRate) {
        initializeNative(activity, assetManager, ovrThread, ARMode, initialRefreshRate);

        // Grab the activity's looper into a handler so that we can post() to the main thread to
        // interact with our WebView.
        mMainHandler = new Handler(activity.getMainLooper());
    }

    public void destroy() {
        destroyNative();
    }

    public void onResume() {
        onResumeNative();
    }

    public void onPause() {
        onPauseNative();
    }

    public void onSurfaceCreated(Surface surface) {
        onSurfaceCreatedNative(surface);
    }

    public void onSurfaceChanged(Surface surface) {
        onSurfaceChangedNative(surface);
    }

    public void onSurfaceDestroyed() {
        onSurfaceDestroyedNative();
    }

    public void render(long renderedFrameIndex) {
        renderNative(renderedFrameIndex);
    }

    public void renderLoading() {
        renderLoadingNative();
    }

    public void sendTrackingInfo(ServerConnection serverConnection) {
        sendTrackingInfoNative(serverConnection);
    }

    public void sendMicData(ServerConnection serverConnection) {
        sendMicDataNative(serverConnection);
    }

    public void onChangeSettings(int suspend) {
        onChangeSettingsNative(suspend);
    }

    public void sendGuardianInfo(ServerConnection serverConnection) {
        sendGuardianInfoNative(serverConnection);
    }

    public int getLoadingTexture() {
        return getLoadingTextureNative();
    }

    public int getSurfaceTextureID() {
        return getSurfaceTextureIDNative();
    }

    public int getWebViewSurfaceTexture() {
        return getWebViewSurfaceTextureNative();
    }

    public int getCameraTexture() {
        return 0;
    }

    public boolean isVrMode() {
        return isVrModeNative();
    }

    public void getDeviceDescriptor(DeviceDescriptor deviceDescriptor) {
        getDeviceDescriptorNative(deviceDescriptor);
    }

    public void setFrameGeometry(int width, int height) {
        setFrameGeometryNative(width, height);
    }

    public void setRefreshRate(int refreshRate) {
        setRefreshRateNative(refreshRate);
    }

    public void setStreamMic(boolean streamMic) {
        setStreamMicNative(streamMic);
    }

    public void setFFRParams(int foveationMode, float foveationStrength, float foveationShape, float foveationVerticalOffset) {
        setFFRParamsNative(foveationMode, foveationStrength, foveationShape,  foveationVerticalOffset);
    }

    public void onHapticsFeedback(long startTime, float amplitude, float duration, float frequency, boolean hand) {
        onHapticsFeedbackNative(startTime, amplitude, duration, frequency, hand);
    }

    public void onGuardianSyncAck(long timestamp) {
        onGuardianSyncAckNative(timestamp);
    }

    public void onGuardianSegmentAck(long timestamp, int segmentIndex) {
        onGuardianSegmentAckNative(timestamp, segmentIndex);
    }

    private native void initializeNative(Activity activity, AssetManager assetManager, OvrThread ovrThread, boolean ARMode, int initialRefreshRate);
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

    private native void onChangeSettingsNative(int suspend);

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

                float mx = x * OvrThread.WEBVIEW_WIDTH;
                float my = y * OvrThread.WEBVIEW_HEIGHT;

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
