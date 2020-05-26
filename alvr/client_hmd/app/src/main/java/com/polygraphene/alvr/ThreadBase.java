package com.polygraphene.alvr;

import android.util.Log;

public class ThreadBase {
    private Thread mThread;
    private boolean mStopped = false;

    protected final void startBase() {
        mThread = new MyThread();
        mStopped = false;
        mThread.start();
    }

    public void stopAndWait() {
        interrupt();
        while (mThread.isAlive()) {
            try {
                mThread.join();
            } catch (InterruptedException e) {
            }
        }
    }

    public void interrupt() {
        mStopped = true;
        mThread.interrupt();
    }

    protected void run() {
    }

    public boolean isStopped() {
        return mStopped;
    }

    private class MyThread extends Thread {
        @Override
        public void run() {
            String name = ThreadBase.this.getClass().getName();
            String[] split = name.split("\\.");
            setName(split[split.length - 1]);

            Utils.log("ThreadBase", () -> name +" has started.");

            ThreadBase.this.run();
        }
    }
}
