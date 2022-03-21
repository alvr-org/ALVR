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

## Control flow pseudocode

```
main {
    init resources
    spawn connection thread
    render loop {
        if should render check {
            standby = false
            if streaming {
                dequeue decoder frame
                render frame
            } else {
                render lobby
            }
            render HUD
            present
        } else {
            standby = true
        }
    }
}

connection thread {
    loop {
        handshake with server
        init streaming resources
        run concurrently {
            control socket listen loop,
            video receive loop {
                if init packet {
                    create decoder
                    streaming = true
                } else if streaming and not standby {
                    enqueue decoder frame
                }
            },
            ... (all other input/output streams)
        }
        streaming = false
    }
}
```
