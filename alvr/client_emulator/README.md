# ALVR client emulator

A desktop application that connects to an ALVR server as if it were a headset. It renders a glTF
environment from a first person camera and exposes an HTTP API so the emulated headset can be
inspected and driven programmatically.

Unlike `client_openxr`, no OpenXR runtime is involved: this is a direct ALVR client built on
`alvr_client_core`. Unlike `client_mock`, it renders a real 3D view.

**No ALVR crates are modified.** The emulator only consumes the public `alvr_client_core` API.

## Scope of this iteration

- Renders **its own** glTF scene. The video stream from the server is **not decoded** — no decoder is
  registered, so `client_core` receives and discards video packets.
- Connects to the server, sends head tracking, and reports frame pacing so the server's latency
  estimate and adaptive bitrate behave normally.
- Single instance only. See "Multiple instances" below.

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

The toolbar selects which eye the window shows — **Left**, **Right** or **Stereo** — and displays the
connection state, stream resolution and current position.

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
- **Multiple instances do not work** without changing ALVR. `CONTROL_PORT` is a hardcoded constant,
  so a second instance fails to bind it (`os error 10048`), and both instances would read the same
  hostname anyway. Enabling this needs a small additive patch to `client_core`'s storage plus a
  control-port parameter — both prototyped on the `multiple_clients` branch.
- **Unlit rendering.** The shader samples the base colour texture and nothing else, on the assumption
  that lighting and shadows are baked in, as they are in a photogrammetry capture. Models relying on
  real-time lighting will look flat.
- **The client core is deliberately leaked on exit.** `alvr_client_core`'s `AnnouncerSocket` creates
  an mdns-sd `ServiceDaemon` and never calls its `shutdown()`. That daemon owns a thread parked in a
  blocking `recv()` with no timeout, so it never exits, and `ClientCoreContext::drop` — which joins
  the connection thread that owns the announcer — blocks forever. Closing the window therefore used
  to hang the process indefinitely (confirmed by native stack dump, and not a slow shutdown: it never
  completes). `EmulatedClient::shutdown` calls `pause()` so the server sees a clean disconnect, then
  reports that the context must not be dropped, and `on_exit` leaks it. The process is exiting, so
  the OS reclaims everything. Fixing this properly means an upstream change to `client_core`, which
  this crate deliberately does not make.
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
