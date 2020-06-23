package com.polygraphene.alvr;

import android.content.Context;
import android.graphics.Canvas;
import android.view.Surface;
import android.view.ViewGroup;
import android.webkit.WebChromeClient;
import android.webkit.WebView;
import android.webkit.WebViewClient;

public class OffscreenWebView extends WebView {
    private static final String TAG = "WebView";

    private static final int TEXTURE_WIDTH = 800;
    private static final int TEXTURE_HEIGHT = 600;

    public Surface surface = null;

    public OffscreenWebView(Context context) {
        super(context);

        setWebChromeClient(new WebChromeClient(){});
        setWebViewClient(new WebViewClient());

        setLayoutParams(new ViewGroup.LayoutParams(TEXTURE_WIDTH, TEXTURE_HEIGHT));
    }

    @Override
    protected void onDraw(Canvas _canvas) {
        if (surface != null){
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
