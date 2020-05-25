package com.polygraphene.alvr;

import android.app.Activity;
import android.os.Bundle;
import android.support.annotation.NonNull;

abstract class BaseActivity extends Activity {
    static {
        System.loadLibrary("native-lib");
    }

    @Override
    protected void onCreate(Bundle savedInstanceState) {
        PersistentConfig.loadCurrentConfig(this, true);
        super.onCreate(savedInstanceState);
        //ArThread.requestPermissions(this);
    }

    @Override
    protected void onResume() {
        super.onResume();
    }

    @Override
    protected void onPause() {
        super.onPause();
    }

    @Override
    protected void onDestroy() {
        super.onDestroy();
    }

    @Override
    public void onRequestPermissionsResult(int requestCode, @NonNull String[] permissions, @NonNull int[] grantResults) {
//        if (!ArThread.onRequestPermissionsResult(this)) {
//            finish();
//        }
    }
}
