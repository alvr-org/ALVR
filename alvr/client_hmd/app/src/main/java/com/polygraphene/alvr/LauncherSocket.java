package com.polygraphene.alvr;

import com.google.gson.Gson;
import com.koushikdutta.async.AsyncNetworkSocket;
import com.koushikdutta.async.AsyncServer;
import com.koushikdutta.async.AsyncServerSocket;
import com.koushikdutta.async.AsyncSocket;
import com.koushikdutta.async.ByteBufferList;
import com.koushikdutta.async.DataEmitter;
import com.koushikdutta.async.callback.DataCallback;
import com.koushikdutta.async.callback.ListenCallback;

import java.net.InetAddress;
import java.net.UnknownHostException;
import java.nio.ByteBuffer;
import java.nio.ByteOrder;
import java.nio.charset.StandardCharsets;
import java.util.concurrent.TimeUnit;

public class LauncherSocket {
    private static final String TAG = "LauncherSocket";
    private boolean mConnected = false;
    private AsyncSocket mSocket;
    private AsyncServerSocket mServerSocket;
    private Gson mGson = new Gson();
    private int mRequestId = 1;
    private int mReadState = 0;
    private int mLength = 0;
    private int mRemaining = 0;
    private long mLastActivity = 0;
    private byte[] mReadBuffer;

    private class Command {
        public int requestId;
        public String command;
        public String result;

        Command(int requestId, String command) {
            this.requestId = requestId;
            this.command = command;
        }
    }

    public interface LauncherSocketCallback {
        void onConnect();
    }

    LauncherSocketCallback mCallback;

    public LauncherSocket(LauncherSocketCallback callback) {
        mCallback = callback;
    }

    public void listen() {
        try {
            AsyncServer.getDefault().listen(InetAddress.getByName("0.0.0.0"), 9944, new ListenCallback() {
                @Override
                public void onAccepted(AsyncSocket socket) {
                    AsyncNetworkSocket networkSocket = (AsyncNetworkSocket) socket;
                    if (mConnected) {
                        Utils.logi(TAG, () -> "Ignored connection request while connected. Address=" + networkSocket.getRemoteAddress().toString());
                        socket.end();
                        socket.close();
                        return;
                    }
                    Utils.logi(TAG, () -> "Connected. Address=" + networkSocket.getRemoteAddress().toString());
                    mConnected = true;
                    mSocket = socket;
                    socket.setDataCallback(LauncherSocket.this::onDataAvailable);
                    socket.setClosedCallback(LauncherSocket.this::onClosedCallback);
                    mCallback.onConnect();
                }

                @Override
                public void onListening(AsyncServerSocket socket) {
                    mServerSocket = socket;
                    mLastActivity = System.nanoTime();
                    AsyncServer.getDefault().postDelayed(LauncherSocket.this::checkAlive, 1000);
                }

                @Override
                public void onCompleted(Exception ex) {
                    Utils.logi(TAG, () -> "onCompleted. Address=" + ex.getMessage());
                }
            });
        } catch (UnknownHostException ignored) {
        }
    }

    public void close() {
        Utils.logi(TAG, () -> "Close.");
        closeClient();
        if (mServerSocket != null) {
            mServerSocket.stop();
            mServerSocket = null;
        }
    }

    private void closeClient() {
        if (mSocket != null) {
            sendCommand("Close");
            mSocket.end();
            mSocket.close();
            mSocket = null;
        }
        mConnected = false;
        mReadState = 0;
    }


    public void sendCommand(String commandName) {
        send(new Command(mRequestId, commandName));
    }

    public void sendReply(int requestId, String result) {
        send(new Command(requestId, result));
    }

    public void send(Command command) {
        String json = mGson.toJson(command);
        ByteBufferList byteBufferList = new ByteBufferList();

        byte[] buffer = json.getBytes(StandardCharsets.UTF_8);
        byte[] length = new byte[]{(byte) buffer.length, (byte) (buffer.length >> 8), (byte) (buffer.length >> 16), (byte) (buffer.length >> 24)};

        byteBufferList.add(ByteBuffer.wrap(length));
        byteBufferList.add(ByteBuffer.wrap(buffer));
        mSocket.write(byteBufferList);
    }

    public boolean isConnected() {
        return mConnected;
    }

    private void onDataAvailable(DataEmitter emitter, ByteBufferList bb) {
        bb.order(ByteOrder.LITTLE_ENDIAN);

        while (bb.remaining() > 0) {
            if (mReadState == 0) {
                mRemaining = mLength = bb.getInt();
                mReadBuffer = new byte[mLength];

                mReadState = 1;
            } else if (mReadState == 1) {
                int r = bb.remaining();
                if (r < mRemaining) {
                    // No sufficient buffer.
                    bb.get(mReadBuffer, mLength - mRemaining, r);
                    mRemaining -= r;
                    return;
                }
                bb.get(mReadBuffer, mLength - mRemaining, mRemaining);

                onReceive();
                mReadBuffer = new byte[0];
                mReadState = 0;
            }
        }
    }

    private void onClosedCallback(Exception e) {
        Utils.logi(TAG, () -> "onClosedCallback. Exception=" + (e == null ? "null" : e.getMessage()));
    }

    private void onReceive() {
        mLastActivity = System.nanoTime();

        String json = new String(mReadBuffer, StandardCharsets.UTF_8);
        //Utils.log(TAG, () -> "onReceive: " + json);
        Command command = mGson.fromJson(json, Command.class);
        if (command.result != null) {
            // Reply message.
            return;
        }
        if (command.command.equals("Close")) {
            Utils.logi(TAG, () -> "Connection closed by server.");
            closeClient();
        } else if (command.command.equals("Ping")) {
            sendReply(command.requestId, "Pong");
        } else {
            Utils.loge(TAG, () -> "Unknown command received. command=" + command.command + " requestId=" + command.requestId);
        }
    }

    private void checkAlive() {
        if (mServerSocket == null) {
            // Exit periodic call.
            return;
        }
        if (mSocket != null) {
            sendCommand("Ping");
            if (System.nanoTime() - mLastActivity > TimeUnit.SECONDS.toNanos(5)) {
                Utils.logi(TAG, () -> "Close socket because of inactivity.");
                closeClient();
            }
        }

        AsyncServer.getDefault().postDelayed(this::checkAlive, 1000);
    }
}
