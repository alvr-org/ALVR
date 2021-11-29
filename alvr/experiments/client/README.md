# alvr_client

## Debugging

Start the application:

```console
cd alvr/experiments/client
cargo apk run
```

In another terminal:

```console
adb logcat -s RustStdoutStderr OpenXR:E
```
