package com.polygraphene.alvr;

import android.annotation.SuppressLint;
import android.content.Context;
import android.graphics.Canvas;
import android.os.Handler;
import android.os.SystemClock;
import android.support.annotation.NonNull;
import android.view.MotionEvent;
import android.view.Surface;
import android.webkit.WebView;

public class OffscreenWebView extends WebView {
    private static final String TAG = "OffscreenWebView";

    public static final int WEBVIEW_WIDTH = 800;
    public static final int WEBVIEW_HEIGHT = 600;

    Surface mSurface;
    Handler mHandler;

    @SuppressLint("SetJavaScriptEnabled")
    public OffscreenWebView(@NonNull Context context, Handler handler) {
        super(context);
        mHandler = handler;
        mHandler.post(() -> {
            this.getSettings().setJavaScriptEnabled(true);
            this.getSettings().setDomStorageEnabled(true);
            this.setInitialScale(100);
        });
    }

    public void setSurface(Surface surface) {
        mSurface = surface;
    }

    public void setURL(String url) {
        mHandler.post(() -> this.loadUrl(url));
    }

    public void applyWebViewInteractionEvent(int type, float x, float y) {
        mHandler.post(() -> {
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
                this.dispatchTouchEvent(ev);
            } else {
                this.dispatchGenericMotionEvent(ev);
            }
        });
    }

    @Override
    protected void onDraw(Canvas canvas) {
        if (mSurface != null) {
            try {
                final Canvas surfaceCanvas = mSurface.lockCanvas(null);
                super.onDraw(surfaceCanvas);
                mSurface.unlockCanvasAndPost(surfaceCanvas);
            } catch (Surface.OutOfResourcesException e) {
                Utils.loge(TAG, e::toString);
            }
        }
    }
}
