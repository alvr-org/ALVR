package com.polygraphene.alvr;

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

    private Surface surface;
    private Context context;

    public OffscreenWebView(@NonNull Context context) {
        super(context);

        this.getSettings().setJavaScriptEnabled(true);
        this.getSettings().setDomStorageEnabled(true);
        this.setInitialScale(100);
    }

    public void setSurface(Surface surface) {
        this.surface = surface;
    }

    public void setMessage(String msg) {
        String title = Utils.getVersionName(context);
        this.loadData(String.format(MSG_TEMPLATE, title, msg), "text/html; charset=utf-8", "UTF-8");
    }

    @Override
    protected void onDraw(Canvas canvas) {
        if (surface != null) {
            try {
                final Canvas surfaceCanvas = surface.lockCanvas(null);
                super.onDraw(surfaceCanvas);
                surface.unlockCanvasAndPost(surfaceCanvas);
            } catch (Surface.OutOfResourcesException e) {
                Utils.loge(TAG, () -> e.toString());
            }
        }
    }
}
