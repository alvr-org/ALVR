package com.polygraphene.alvr;

import android.app.Service;
import android.content.Intent;
import android.os.IBinder;

public class ChangeSettings extends Service {
    private static final String TAG = "ChangeSettings";

    @Override
    public int onStartCommand(Intent intent, int flags, int startId) {
        PersistentConfig.loadCurrentConfig(this, false);

        Utils.logi(TAG, () -> "Config setting " + PersistentConfig.KEY_TARGET_SERVERS + " has value: " + PersistentConfig.sTargetServers);

        String targetServers = intent.getStringExtra(PersistentConfig.KEY_TARGET_SERVERS);

        if (targetServers != null) {
            Utils.logi(TAG, () -> "Setting config setting " + PersistentConfig.KEY_TARGET_SERVERS + " to: " + targetServers);
            PersistentConfig.sTargetServers = targetServers;
        }

        PersistentConfig.saveCurrentConfig(false);
        stopSelf(startId);
        return Service.START_NOT_STICKY;
    }

    @Override
    public IBinder onBind(Intent intent) {
        return null;
    }
}
