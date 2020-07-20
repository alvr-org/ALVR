package com.polygraphene.alvr;

import android.content.Context;
import android.graphics.Canvas;
import android.support.annotation.NonNull;
import android.view.Surface;
import android.webkit.WebView;

public class SurfaceWebView extends WebView {
    private static final String TAG = "SurfaceWebView";

    private Surface surface;

    public SurfaceWebView(@NonNull Context context) {
        super(context);
    }

    public void setSurface(Surface surface) {
        this.surface = surface;
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
