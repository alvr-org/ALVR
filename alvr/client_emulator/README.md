# ALVR client emulator

A desktop application that connects to an ALVR server as if it were a headset. It renders a glTF
environment from a first person camera and exposes an HTTP API so the emulated headset can be
inspected and driven programmatically.

Unlike `client_openxr`, no OpenXR runtime is involved: this is a direct ALVR client built on
`alvr_client_core`. Unlike `client_mock`, it renders a real 3D view.

The emulator needs three small, backward compatible changes in ALVR itself, all of which keep
existing clients working unchanged; see "ALVR changes" below.

## What it does

- Connects to the server, sends head tracking, and reports frame pacing so the server's latency
  estimate and adaptive bitrate behave normally.
- **Decodes and displays the video stream.** The toolbar switches between the decoded video and the
  local glTF scene, which is useful for telling a decode problem apart from a rendering one.
- Renders a glTF scene from a first person camera. This is also what the capture endpoints return —
  they render the scene offscreen, not the video.

### Video decoding

Decoding goes through the `VideoDecoder` trait in [`src/decoder`](src/decoder/), so a platform
specific implementation can be added without touching the renderer. Only `SoftwareDecoder` exists
today: ffmpeg on the CPU, then a per-frame upload of the YUV planes, with the conversion to RGB done
in a shader.

That is not a placeholder. Measured on this hardware, CPU decode of a 1920x1792 72 fps H.264 stream
runs at **38-45x realtime**, about 0.3 of a core per stream, and it was **three times faster than
d3d11va** hardware decode for a single stream. It is also the only option that behaves the same on
every platform: ffmpeg's Vulkan decoder, which would be the portable GPU path, currently fails with
`VK_ERROR_DEVICE_LOST` on the RTX 5090 tested.

A zero-copy path is still worth adding for many simultaneous streams. wgpu is therefore asked to
prefer DirectX on Windows (see `preferred_wgpu_setup` in [`src/main.rs`](src/main.rs)), so that a
future D3D11 decoder shares the DXGI family with the renderer and needs only a shared handle rather
than cross-API interop.

## Running

```sh
cargo run -p alvr_client_emulator
```

Debug builds keep a console window so log output is visible. Release builds hide it
(`windows_subsystem = "windows"`, as `alvr_dashboard` does), so use `--release` for a clean
windowed app.

Place `environment.gltf` next to the executable (`target/debug/environment.gltf`). Without it the
window explains what is missing instead of rendering.

To generate a test room with baked lighting:

```sh
python alvr/client_emulator/tools/make_environment.py target/debug/environment.gltf
```

Requires Pillow. The generated file is self-contained: geometry and textures are embedded.

ffmpeg comes from `cargo xtask prepare-deps`, the same copy the server uses; `FFMPEG_DIR` in
`.cargo/config.toml` points at it and `build.rs` copies the DLLs next to the executable. Note that
build is GPL licensed.

Then start SteamVR with ALVR and click **Trust** next to the device entry that appears.

## Controls

| Input | Action |
|---|---|
| Left-click the view | Capture the mouse for look control until `Esc` (cursor hidden and confined) |
| Hold right button | Look around only while held; releasing the button ends it |
| `Esc` | Release a left-click capture |
| `W` / `A` / `S` / `D` | Move horizontally (never vertically, regardless of pitch) |
| Mouse | Yaw and pitch |
| `Q` / `E` | Roll |
| `Page Up` / `Page Down` | Change height |
| `Shift` | Move faster |

There is no collision: walking through walls is expected.

The toolbar selects which eye the window shows — **Left**, **Right** or **Stereo** — and whether the
source is the decoded **Video** or the local **Scene**. It also shows the connection state, stream
resolution, decoded frame count and current position.

## HTTP API

Listens on `127.0.0.1:8080` by default. Override with `ALVR_EMULATOR_API_PORT`.

Localhost only, and unauthenticated: it is a debugging interface and must not be exposed.

### `GET /api/state`

```json
{
  "connected": false,
  "streaming": false,
  "hud_message": "ALVR v21.0.0-dev12\nhostname: 1439.client.local...",
  "position": [0.0, 1.6, 0.0],
  "yaw": 0.0,
  "pitch": 0.0,
  "roll": 0.0,
  "environment_file": "F:\\code\\ALVR\\target\\debug\\environment.gltf",
  "environment_loaded": true,
  "view_resolution": [0, 0],
  "refresh_rate": 0.0,
  "codec": null
}
```

`hud_message` carries the client core's own status text, which is where connection errors surface.

### `GET /api/view/color`

Both eyes side by side as a PNG (left eye first). Rendered offscreen at the negotiated stream
resolution when streaming, so captures do not change with window size, and at 960x916 per eye
otherwise.

### `GET /api/view/depth`

The same stereo framing as a greyscale PNG. Near surfaces are bright, distant ones dark, and pixels
where nothing was drawn are `0`.

Depth is normalised across the range present in the image, not across the clip range: a small room
occupies a tiny fraction of the 0.02..100 m frustum, so a clip-range mapping collapses the whole
scene into a few near-white values. This means **values are not comparable between captures** taken
from different positions. It is a visualisation, not a measurement.

### `POST /api/move`

Every field is optional; omitted fields keep their current value.

```sh
curl -X POST http://127.0.0.1:8080/api/move \
  -H "Content-Type: application/json" \
  -d '{"position": [1.0, 1.7, 2.0], "yaw": 0.5}'
```

Angles are radians. Applied on the next frame.

## Notes and limitations

- **Shared client identity.** `alvr_client_core` stores its hostname in a per-user config file
  (`%APPDATA%\ALVR Client\session.json` on Windows), resolved through a Win32 known-folder call with
  no override. The emulator therefore shares that identity with any real client on the same machine.
- **Multiple instances still share one identity.** The port collisions are solved (see below), but
  `alvr_client_core` stores its hostname in a single per-user config file, so concurrent instances
  would present themselves as the same device. That needs the `ALVR_CLIENT_CONFIG_DIR` /
  `ALVR_CLIENT_HOSTNAME` overrides prototyped on the `multiple_clients` branch.
- **Unlit rendering.** The shader samples the base colour texture and nothing else, on the assumption
  that lighting and shadows are baked in, as they are in a photogrammetry capture. Models relying on
  real-time lighting will look flat.
- Closing the window used to hang the process forever, because `AnnouncerSocket` created an mdns-sd
  `ServiceDaemon` and never shut it down, leaving a thread parked in a blocking receive that
  `ClientCoreContext::drop` then waited on (confirmed by native stack dump). The announcer now shuts
  its daemon down on drop, so exit is clean.
- `easy-gltf` 1.1.5 panics on spec-compliant embedded image `data:` URIs: it decodes them with a
  URL-safe base64 alphabet where the glTF spec requires standard base64. Embed textures as buffer
  views instead, which is what the generator script does.
- Only triangle-mode primitives are drawn; point and line modes are skipped with a warning.
- Back-face culling is disabled, since room scans are frequently inconsistently wound.

## Layout

| File | Purpose |
|---|---|
| `src/main.rs` | eframe app, toolbar, input handling, API request servicing |
| `src/camera.rs` | First person camera and eye/projection matrices |
| `src/scene.rs` | glTF loading into a plain geometry container |
| `src/render.rs` | wgpu pipeline, on-screen views, offscreen capture |
| `src/client.rs` | `ClientCoreContext` lifecycle and the tracking thread |
| `src/api.rs` | HTTP server and shared state |
| `src/shader.wgsl` | Unlit vertex/fragment shader |
| `tools/make_environment.py` | Generates a test environment |

`Scene` is deliberately a geometry container rather than a renderer, so an alternative source such as
a Gaussian splat capture can be added without changing the code that consumes it.

## ALVR changes

Three changes outside this crate, all additive and all keeping existing clients working against a
new server. The reverse is not true: an old server cannot reach a client that moved off the
well-known ports, which is fine because both sides of an emulator setup are under your control.

**Announcer shutdown** — `client_core/src/sockets.rs`. `AnnouncerSocket` now shuts its mdns-sd
daemon down on drop. Without this the daemon thread never exits and process exit hangs.

**Client control port** — `sockets`, `client_core`, `server_core`. Only one process per machine can
bind the well-known `CONTROL_PORT`, so a client that cannot get it falls back to an OS-assigned port
and advertises it in a new `control_port` mDNS TXT entry. A client that does not advertise one is
reached on the well-known port exactly as before, which is what keeps real headsets working. The
server side is `WelcomeSocket::recv_all` returning the port alongside the address, threaded through
`try_connect`.

**Client stream port** — `sockets`, `client_core`, `packets`. The UDP stream port has the same
problem, but the client cannot simply take it: the client dials the server by port number, so the
server's port is fixed by the protocol while the client's is not. The client therefore yields the
configured port whenever the server is on the same machine, binds an OS-assigned one, and reports it
with a new `ClientControlPacket::StreamReadyOnPort`.

That is a new enum variant rather than a field on the existing `StreamReady`, because these packets
are bincode encoded by variant index: appending leaves existing indices untouched, while changing
`StreamReady` itself would reinterpret every old client's packet. Old clients keep sending
`StreamReady` and are handled exactly as before.
