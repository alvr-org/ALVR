package com.polygraphene.alvr;

import android.annotation.SuppressLint;
import android.content.Context;
import android.graphics.Canvas;
import android.support.annotation.NonNull;
import android.view.Surface;
import android.webkit.WebView;

public class OffscreenWebView extends WebView {
    private static final String TAG = "SurfaceWebView";
    private static final String MSG_TEMPLATE = "<!doctype html>" +
            "<html>" +
            "<head>" +
            "</head>" +
            "<body>" +
            "   <h1> %s </h1>" +
            "   <h4> %s </h4>" +
            "</body>" +
            "</html>";

    public static final int WEBVIEW_WIDTH = 800;
    public static final int WEBVIEW_HEIGHT = 600;

    private Surface mSurface;
    private String mMsgTitle;

    @SuppressLint("SetJavaScriptEnabled")
    public OffscreenWebView(@NonNull Context context) {
        super(context);

        this.getSettings().setJavaScriptEnabled(true);
        this.getSettings().setDomStorageEnabled(true);
        this.setInitialScale(100);

        mMsgTitle = Utils.getVersionName(context);
    }

    public void setSurface(Surface surface) {
        this.mSurface = surface;
    }

    public void setMessage(String msg) {
        this.loadData(String.format(MSG_TEMPLATE, mMsgTitle, msg), "text/html; charset=utf-8", "UTF-8");
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
