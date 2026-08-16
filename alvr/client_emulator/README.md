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
- **Emulates motion controllers.** Either controller can be enabled independently, posed in 6DoF,
  and driven through every button and axis the selected controller type has, from the mouse or from
  the HTTP API. The server treats them as real controllers; SteamVR renders and reacts to them.

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

Then start SteamVR — via the **ALVR dashboard**, which owns the driver lifecycle — and click
**Trust** next to the device entry that appears.

A few things that will otherwise cost you time:

- **Kill any stray `alvr_client_emulator` before rebuilding.** The linker cannot replace a running
  exe, and a stale instance also holds the control port, which silently breaks the next run's
  discovery.
- **Do not force-kill `vrserver`.** It orphans SteamVR's IPC port 27062 and the next launch fails with
  "Port 27062 in use". Close SteamVR normally instead.
- **`cargo xtask package-streamer` regenerates the server's `session.json` with defaults**, losing
  client trust and any settings. Trust can be restored without restarting:
  ```sh
  curl -H "X-ALVR: true" -H "Content-Type: application/json" \
    -X POST -d '["<hostname>","Trust"]' \
    http://127.0.0.1:8082/api/session/client-connections
  ```
- The server's `passthrough` `variant` must be one this branch knows. A stale `AlphaStream` value left
  by the `ar_mode` branch breaks stream negotiation.

See [`HANDOVER.md`](HANDOVER.md) for the design reasoning, the freeze diagnosis, and the dead ends
worth not repeating.

## Controls

| Input | Action |
|---|---|
| Left-click the view | Capture the mouse for look control (cursor hidden and confined); click again or press `Esc` to release |
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

## Controller emulation

The **Inputs** toolbar row controls the emulated controllers: **L** and **R** enable each hand
independently, the dropdown selects which controller type to emulate, **Display** shows the 3D
models in the scene view, and **Reset** returns poses and inputs to their defaults.

Enabled controllers appear as **L**/**R** icons projected over the 3D view at their position; when
the controller is outside the view the icon sticks to the edge with an arrow pointing towards it.
Hovering an icon — including an edge-clamped one, which is how an off-screen controller is brought
back — opens the movement panel underneath, four drag pads that pose the controller:

| Pad | Drag | Effect |
|---|---|---|
| Move | 2D | Translate on the vertical plane facing the head (drag maps 1:1 to on-screen motion) |
| Depth | vertical | Move towards / away from the head |
| Roll | horizontal | Roll around the controller's own forward axis |
| Aim | 2D | Yaw and pitch in head space, with configurable sensitivity |

Poses are head-relative, so controllers ride along as the camera moves and turns. The axes follow
ALVR's convention: X right, Y up, -Z forward, origin at the head.

While a controller is enabled, a panel mimicking its physical layout sits in the matching bottom
corner: trigger on top filling downwards as it is pulled, the grip as a bar on the inner edge
filling from the screen centre outwards, thumbstick or trackpad in the middle, menu / face buttons
/ system / thumbrest along the bottom. The right panel mirrors the left, rows a profile lacks
collapse away, face buttons take the traditional gamepad colours, and the border matches the hand's
icon colour. The buttons follow one scheme so held inputs can be combined with moving the
controller or the headset: the left button is momentary, a middle-button drag or click sets state
that persists, and a right click toggles the press until clicked again.

| Control | Left button | Middle button | Right button |
|---|---|---|---|
| Trigger / grip | Hold a full pull | Drag: analog value, kept on release | Click: toggle full pull |
| Thumbstick | Drag to deflect (springs back); click: recentre a held deflection | Click: toggle touch | Drag: deflect, kept; click: toggle stick click |
| Face / menu / system buttons | Hold the press | Click: toggle touch | Click: toggle press |
| Trackpad | Place the contact point | Drag: force (kept); click: toggle touch | Click: toggle pad click |
| Thumbrest | Hold the touch | Click: toggle touch | Click: toggle touch |

Every control shows a tooltip naming the input paths it drives and its mouse actions.

Inputs are kept consistent the way a physical controller would report them: pulling a trigger also
reports its touch, a full pull reports the click, deflecting a stick reports its touch, and so on.
Values set through the API are held until changed and show up in the panels; interacting with a
control in the UI overrides it. Haptic feedback from the server shows on the indicator in the
panel's top corner — brightness follows the amplitude and the blink hints at the frequency — and as
a flashing trackpad border.

Emulation works alongside streaming: SteamVR sees the controllers as real devices and applications
render and react to them.

### Controller profiles and settings

`controllers.json` next to the executable is created on first run and can be edited freely; it is
documented by its own content. It holds:

- `rotation_sensitivity` — radians of controller rotation per pixel of drag on the rotation pads.
- `left_start_position` / `right_start_position` — head-relative default positions.
- `profiles` — the controller types offered for emulation. Each entry has a display `name`, the
  interaction profile `path`, the input path suffixes each hand supports (`left_inputs` /
  `right_inputs`, e.g. `"trigger/value"`), and optionally `left_model` / `right_model`, glTF files
  (relative to the executable's directory) shown when the controller is visible. Without a model a
  small procedural placeholder is drawn instead.

Real controller models cannot be redistributed, but SteamVR ships them locally and
[`tools/convert_rendermodel.py`](tools/convert_rendermodel.py) converts one into a glTF the
emulator loads:

```sh
python alvr/client_emulator/tools/convert_rendermodel.py \
  "C:/Program Files (x86)/Steam/steamapps/common/SteamVR/resources/rendermodels/oculus_quest2_controller_left" \
  target/debug/models/quest_left.gltf
```

Run it once per hand and point the profile's `left_model` / `right_model` at the outputs. Render
models are authored around the SteamVR device pose while the emulator poses the grip, so the
converter bakes in ALVR's default grip-to-device translation (`0, 0, -0.11`); pass `--offset` if
the server's controller position offset was customised. The models display in both the scene and
the video view — over the video they overlap the application's own controller rendering, which is
exactly the comparison the display toggle is for.

The default file is generated from ALVR's own interaction profile definitions, so the predefined
profiles emulate exactly the inputs ALVR accepts from each real controller: Quest, Index, Vive
Wand, Pico Neo3 / 4 / 4S / G3, PSVR2 Sense, Vive Focus 3 and YVR. New profiles can be added as long
as they use inputs from that set — unknown inputs are ignored with a warning. Note the emulator
reports inputs exactly as the real controller would; any remapping to the server's configured
emulation mode happens on the server, as with real hardware.

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

`codec` becomes the negotiated codec once the server announces it. Decoded frame count and frame
layout are shown in the toolbar but are not yet exposed here.

### `GET /api/view/color`

Both eyes side by side as a PNG (left eye first). Rendered offscreen at the negotiated stream
resolution when streaming, so captures do not change with window size, and at 960x916 per eye
otherwise.

**Renders the local glTF scene, not the decoded video**, whatever the toolbar is showing. Capturing
the video stream is not implemented.

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

### `GET /api/controllers`

Both controllers' full state, plus the available profiles:

```json
{
  "profiles": [{ "name": "Quest", "path": "/interaction_profiles/oculus/touch_controller" }, ...],
  "left": {
    "enabled": true,
    "profile": "Quest",
    "visible": false,
    "position": [-0.15, -0.25, -0.35],
    "orientation": [0.0, 0.0, 0.0, 1.0],
    "inputs": { "trigger/value": 0.6 },
    "supported_inputs": ["menu/click", "x/click", ...]
  },
  "right": { ... }
}
```

`position` is head-relative (X right, Y up, -Z forward), `orientation` is an XYZW quaternion.
`inputs` lists the explicitly held inputs; derived ones (touch from a press, and so on) are added
when sending. `supported_inputs` is what the current profile accepts for that hand, which is also
what input requests are validated against.

### `POST /api/controllers/{left|right}`

Configures one controller. Every field is optional:

```sh
curl -X POST http://127.0.0.1:8080/api/controllers/left \
  -H "Content-Type: application/json" \
  -d '{"enabled": true, "profile": "Index", "visible": true}'
```

`profile` takes a display name (case-insensitive) or an interaction profile path.

### `POST /api/controllers/{left|right}/pose`

Sets the head-relative pose. Both fields optional; the quaternion is normalised on apply:

```sh
curl -X POST http://127.0.0.1:8080/api/controllers/left/pose \
  -H "Content-Type: application/json" \
  -d '{"position": [0.1, -0.2, -0.4], "orientation": [0.0, 0.383, 0.0, 0.924]}'
```

### `POST /api/controllers/{left|right}/inputs`

Sets button and axis states, held until changed or reset. Keys are input path suffixes, values are
booleans or numbers; inputs the current profile does not support are rejected with a 400 listing
what is available:

```sh
curl -X POST http://127.0.0.1:8080/api/controllers/right/inputs \
  -H "Content-Type: application/json" \
  -d '{"trigger/value": 0.7, "a/click": true, "thumbstick/x": -0.5}'
```

Setting a value to `0` / `false` releases it.

### `POST /api/controllers/{left|right}/inputs/click`

Presses an input and releases it after `duration` seconds (default 0.1):

```sh
curl -X POST http://127.0.0.1:8080/api/controllers/right/inputs/click \
  -H "Content-Type: application/json" \
  -d '{"input": "a/click", "duration": 0.25}'
```

### `POST /api/controllers/{left|right}/reset`

Returns the pose and all inputs to their defaults. Emulation stays enabled and the profile
selection is kept.

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
| `src/render.rs` | wgpu pipeline, on-screen views, offscreen capture, controller models |
| `src/client.rs` | `ClientCoreContext` lifecycle, tracking thread, button sync |
| `src/controllers.rs` | Controller state, profiles and the settings file |
| `src/controller_ui.rs` | Controller icons, movement panel and the corner input panels |
| `src/api.rs` | HTTP server and shared state |
| `src/shader.wgsl` | Unlit vertex/fragment shader |
| `tools/make_environment.py` | Generates a test environment |
| `tools/convert_rendermodel.py` | Converts a local SteamVR render model into a loadable glTF |

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
