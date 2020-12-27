package com.polygraphene.alvr;

import android.content.Context;
import android.content.pm.PackageInfo;
import android.content.pm.PackageManager;
import android.util.Log;

public class Utils {
    public static boolean sEnableLog = false;

    public interface LogProvider
    {
        String obtain();
    }

    public static void frameLog(long frameIndex, LogProvider s) {
        if(sEnableLog)
        {
            Log.v("FrameTracking", "[Frame " + frameIndex + "] " + s.obtain());
        }
    }

    public static void log(LogProvider s) {
        if(sEnableLog)
        {
            Log.v("FrameTracking", s.obtain());
        }
    }

    public static void log(String tag, LogProvider s)
    {
        if(sEnableLog)
        {
            Log.v(tag, s.obtain());
        }
    }

    public static void logi(String tag, LogProvider s)
    {
        Log.i(tag, s.obtain());
    }

    public static void loge(String tag, LogProvider s)
    {
        Log.e(tag, s.obtain());
    }
}
