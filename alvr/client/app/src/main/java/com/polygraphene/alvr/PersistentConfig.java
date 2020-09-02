package com.polygraphene.alvr;

import android.content.Context;
import android.content.SharedPreferences;

/**
 * Hold previous connection state to recover connection after resume app.
 */
public class PersistentConfig {
    private static final String KEY_SERVER_ADDRESS = "serverAddress";
    private static final String KEY_SERVER_PORT = "serverPort";
    private static final String KEY_DEBUG_FLAGS = "debugFlags";
    public static final String KEY_TARGET_SERVERS = "targetServers";

    public static class ConnectionState {
        public String serverAddr;
        public int serverPort;
    }

    private static Context sAppContext = null;
    public static long sDebugFlags = 0;
    public static String sTargetServers = null;

    // Save current configs for next startup.
    public static void saveCurrentConfig(boolean reloadDebugFlags) {
        SharedPreferences pref = sAppContext.getSharedPreferences("pref", Context.MODE_PRIVATE);
        SharedPreferences.Editor edit = pref.edit();
        edit.putLong(KEY_DEBUG_FLAGS, sDebugFlags);
        edit.putString(KEY_TARGET_SERVERS, sTargetServers);
        edit.apply();
        if (reloadDebugFlags) {
            Utils.setDebugFlags(sDebugFlags);
        }
    }

    // Load previous saved config when startup app.
    public static void loadCurrentConfig(Context context, boolean reloadDebugFlags) {
        sAppContext = context.getApplicationContext();

        SharedPreferences pref = sAppContext.getSharedPreferences("pref", Context.MODE_PRIVATE);
        sDebugFlags = pref.getLong(KEY_DEBUG_FLAGS, 0);
        sTargetServers = pref.getString(KEY_TARGET_SERVERS, null);
        if (reloadDebugFlags) {
            Utils.setDebugFlags(sDebugFlags);
        }
    }

    public static void saveConnectionState(Context context, String serverAddr, int serverPort) {
        SharedPreferences pref = context.getSharedPreferences("pref", Context.MODE_PRIVATE);
        SharedPreferences.Editor edit = pref.edit();
        // If server address is NULL, it means no preserved connection.
        edit.putString(KEY_SERVER_ADDRESS, serverAddr);
        edit.putInt(KEY_SERVER_PORT, serverPort);
        edit.apply();
    }

    public static void loadConnectionState(Context context, ConnectionState connectionState) {
        SharedPreferences pref = context.getSharedPreferences("pref", Context.MODE_PRIVATE);
        connectionState.serverAddr = pref.getString(KEY_SERVER_ADDRESS, null);
        connectionState.serverPort = pref.getInt(KEY_SERVER_PORT, 0);

        saveConnectionState(context, null, 0);
    }
}
