# ALVR client emulator — handover

**Branch:** `emulator` · **Last commit:** `8f1c6a77 Fix video freeze`
**State:** Working. Connects to a real ALVR server, streams, decodes and displays video, no freezing
or corruption observed. Verified against SteamVR Home on Windows with an RTX 5090.

Read [`README.md`](README.md) for how to build, run and use it. This file is the "why is it like
this, and what next" companion — the reasoning and the hard-won failures that the code comments alone
do not convey.

---

## What it is

A desktop application that pretends to be a headset. It talks to an ALVR server over the real
protocol, sends head tracking from a mouse-and-keyboard camera, decodes the video stream, and exposes
an HTTP API so the emulated headset can be inspected and driven programmatically.

Built to develop ALVR *and* applications on top of it without hardware, and to eventually run several
emulated headsets at once.

It is **not** an Android emulator and involves no OpenXR runtime. It links `alvr_client_core`
directly, which is the same crate the real client is built on, so the protocol path is genuine.

## Scope decisions already made

These were settled deliberately; re-opening them needs a reason.

| Decision | Why |
|---|---|
| Own crate, `client_mock` untouched | `client_mock` stays a low-dependency smoke test; upstream still commits to it |
| Direct `alvr_client_core` client, no OpenXR | `client_core` has no OpenXR dependency; that lives only in `client_openxr` |
| No timewarp / reprojection | Deletes the subtlest part of the client render path; not needed for a debug tool |
| eframe/egui + wgpu | Matches the dashboard, launcher and `client_mock`; one dependency gives window, UI and 3D |
| CPU video decode via ffmpeg | Measured 38–45× realtime, ~0.3 core per stream, and **3× faster than d3d11va** for one stream |
| Unlit glTF rendering | Assumes baked lighting, as a photogrammetry capture would have |
| Backward-compatible ALVR changes only | Far likelier to be accepted upstream; real clients keep working unchanged |

## Architecture

```
main.rs           eframe app: toolbar, input, paint callback, services API requests
camera.rs         first person camera, per-eye view and projection matrices
client.rs         ClientCoreContext lifecycle, tracking thread, decoder wiring, statistics,
                  controller motion / button / interaction-profile sync
controllers.rs    emulated controller state, profiles, controllers.json settings file
controller_ui.rs  controller icons over the view, movement panel, corner input panels
decoder/          VideoDecoder trait + DecodedFrame enum; software.rs is the ffmpeg CPU implementation
video.rs          decoded frames to GPU, YUV to RGB in video.wgsl, per-eye region sampling
render.rs         glTF scene pipeline, controller models, on-screen views and offscreen capture
scene.rs          glTF loading into a plain geometry container
api.rs            HTTP control server
```

## Controller emulation notes

Added after the freeze work; see README.md for usage. Implementation points worth knowing:

- **Poses ride the existing tracking channel.** Controllers are extra `device_motions` entries in
  the same `TrackingData` as the head, at the same 3× refresh cadence. The head and the controllers
  are published by the UI thread as **one atomic snapshot** (`TrackedState`) — with separate slots,
  packets sometimes paired a fresh head pose with last frame's controllers, and head-relative
  controllers visibly flickered against the view while the camera moved.
- **Controller velocities are zero, like the head's.** Derived velocities (even low-pass filtered)
  made the controllers swim relative to the view while walking, because the server extrapolates
  poses by velocity and the head reports none. Both unpredicted keeps them locked together; revisit
  only together with head velocities.
- **Buttons are change-driven.** `EmulatedClient::sync_buttons` mirrors what was last sent and
  transmits diffs, like the real client; releases are sent for inputs that disappear from the
  desired set. Derived inputs (touch from press, click from full pull) are computed in
  `ControllerState::effective_entries`, filtered by the active profile.
- **One interaction profile announcement for both hands.** The server rebuilds its button mapping
  manager from every `ActiveInteractionProfile` packet and keeps only the last, so the emulator
  sends the union of both hands' input ids as one announcement rather than one per hand.
- **Profiles are generated from `alvr_common::CONTROLLER_PROFILE_INFO`** into `controllers.json` on
  first run, which keeps the emulated input sets identical to what ALVR accepts from real hardware.
- **egui Areas and DPI.** The corner panels are positioned with explicit coordinates from
  `ctx.content_rect()`; `Area::anchor`/`pivot` place by the area's remembered size, which is wrong
  on the first frame. When verifying the UI with `PrintWindow` screenshots, the capturing process
  must be DPI-aware or the capture is silently cropped to the top-left corner — this cost a long
  false hunt for a rendering bug that did not exist.

Verified end-to-end against SteamVR Home: SteamVR renders the emulated Quest controllers at the
emulated poses, and button presses arrive server-side (watched via the `/api/events` WebSocket with
`log_button_presses` enabled) including the derived touch inputs and timed click releases.

The decoder is behind a trait with a `DecoderKind::preferred()` selector so a platform-specific
zero-copy implementation can be added later without touching the renderer. `DecodedFrame` is an enum
for the same reason: a GPU variant would carry a texture instead of CPU planes.

wgpu is asked to prefer **DirectX on Windows** (`preferred_wgpu_setup` in `main.rs`) purely so a
future D3D11 decoder shares the DXGI family with the renderer, needing a shared handle rather than
cross-API interop. Note `Backends` is a *filter*, not a priority order — preferring a backend
requires the `native_adapter_selector` callback.

## ALVR changes, and why each exists

All additive, all keeping existing clients working against a new server. An old *server* cannot reach
a client that moved off the well-known ports, which is acceptable because both sides of an emulator
setup are under our control.

**`client_core/src/sockets.rs` — announcer shutdown.** `AnnouncerSocket` created an mdns-sd
`ServiceDaemon` and never shut it down, leaving a thread parked in a blocking receive that
`ClientCoreContext::drop` then joined forever. Closing the window hung the process indefinitely
(confirmed by native stack dump; not slow, never completes). Now shuts down on drop.

**Client control port** (`sockets`, `client_core`, `server_core`). Only one process per machine can
bind `CONTROL_PORT`. A client that cannot get it falls back to an OS-assigned port and advertises it
in a new `control_port` mDNS TXT entry; a client that advertises nothing is reached on the well-known
port exactly as before.

**Client stream port** (`sockets`, `client_core`, `packets`). Same problem for UDP, but the client
cannot simply take the port: it dials the *server* by port number, so the server's port is fixed by
the protocol while the client's is not. The client therefore yields when the server is on the same
machine, binds an OS-assigned port, and reports it via a new
`ClientControlPacket::StreamReadyOnPort`. That is a **new enum variant** rather than a field on
`StreamReady`, because these packets are bincode-encoded by variant index — appending leaves existing
indices untouched, while changing `StreamReady` would reinterpret every old client's packet.

**`.cargo/config.toml`** sets `FFMPEG_DIR` to the ffmpeg that `cargo xtask prepare-deps` already
downloads. `ffmpeg-sys-next` reads it from the process environment, so a build script cannot set it.

**The server driver (`server_openvr`) is deliberately untouched.** See "the freeze" below.

## The freeze, and what actually fixed it

This consumed most of the session and is the single most important thing to understand before
changing the statistics or tracking code.

**Symptom.** Video played, then froze on the last frame. It resumed only when the *view* moved, ran a
few seconds, and froze again. SteamVR's own preview kept updating throughout, so the server was
rendering. Client-side instrumentation showed **zero** frames arriving at the decoder callback, so
nothing was reaching the wire.

**Root cause: `report_frame_decoded` was called in the wrong place.** It ran on the render thread,
once per *displayed* frame. The real client (`client_openxr`) reports it from the decoder, for every
decoded frame, as the frame is produced.

ALVR's statistics are a strict chain, each stage measured from the previous one
(`client_core/src/statistics.rs`):

```
report_video_packet_received  ->  video_decode
report_frame_decoded          ->  video_decoder_queue
report_compositor_start       ->  rendering
report_submit                 ->  total_pipeline_latency  -> sent to the server
```

Break the chain and `summary()` finds nothing, so **no statistics packet is sent at all** — the
`Statistics summary not ready!` flood. The server uses `total_pipeline_latency` for its prediction
offset, which shifts the `target_timestamp` it stamps on frames (`server_openvr/src/lib.rs:306`).
Wrong offset means timestamps that no longer match `HEAD_POSE_QUEUE`, and `VideoSend`
(`server_openvr/src/lib.rs:591-598`) then **silently drops the frame** — no log line. The
`Latency is too high. Clamping prediction` warnings in the server log were this, visible.

Two supporting fixes landed with it, both real:

- **Codec parameter sets are refreshed on every `DecoderConfig` event.** The server repeats them
  whenever asked for a recovery keyframe. Discarding the repeat left recovery keyframes without a
  sequence header, so they could not decode and corruption looked permanent.
- **Parameter sets are prepended to every keyframe**, not consumed once on the first.

### Dead ends — do not repeat these

Recorded because each looked convincing and cost real time.

- **Plane row stride.** Theory: ffmpeg's last row lacks padding so the upload was skipped. Measured:
  buffers are exactly `stride * height`, stride equals width. Wrong.
- **Timestamp round-trip through ffmpeg.** Theory: PTS was not preserved. The tiny values in the log
  are `push_nal`'s *input* — ALVR's own timestamps — not something ffmpeg mangled. Wrong.
- **Timer-paced statistics reporting.** Made the warning flood far worse. The real client reports
  only when a new frame is available, never with a repeated timestamp.
- **`enforce_server_frame_pacing` and `max_queued_server_video_frames`.** Both red herrings. The
  server's `frame_interval` is write-once from the negotiated framerate
  (`server_core/src/statistics.rs:87`) and never driven by client reports, so starving reports cannot
  change its cadence.
- **Server-side `PoseHistory::GetBestPoseMatch` change.** `GetBestPoseMatch` reverse-engineers *which
  tracking sample* a rendered frame belongs to, by nearest-orientation search over a 3-second, 360-
  entry buffer — SteamVR passes the pose but no frame ID. It compares **rotation only** (position
  ignored) and keeps the **oldest** entry on ties, so identical static poses resolve to a stale
  sample. That analysis is correct and the weakness is genuine, but it was a *consequence* of the
  broken statistics, not the cause. Changing shared server code to accommodate a synthetic client was
  the wrong trade, and it was reverted. Real clients work; the bug was ours.
- **Tracking jitter to make poses distinguishable.** Fixed the freeze but caused visible shaking and
  out-of-order frames — because it was added *together with* derived velocities, which divide the
  jittered delta by a ~4.6 ms interval and amplify the noise ~216×. Both were reverted. Tracking
  reports zero velocities, as it did originally.

**Method note.** Five wrong theories in a row were broken by having subagents read the real client's
implementation and the server's frame path, instead of continuing to reason from symptoms. Do that
earlier next time.

## Known limitations

- **Single instance only.** The port collisions are solved, but `alvr_client_core` stores its hostname
  in one per-user config file, so concurrent instances present as the same device. Needs the
  `ALVR_CLIENT_CONFIG_DIR` / `ALVR_CLIENT_HOSTNAME` overrides prototyped on the `multiple_clients`
  branch (~30 additive lines in `client_core/src/storage.rs`).
- **Shared client identity** with any real client on this machine
  (`%APPDATA%\ALVR Client\session.json`). Note a UTF-8 BOM in that file makes the server reset all
  settings, and repackaging deletes it.
- **Capture endpoints render the scene, not the video.** `/api/view/color` and `/api/view/depth` are
  offscreen renders of the glTF scene. Capturing the decoded video is not implemented.
- **Depth values are not comparable between captures** — normalised across the range present in each
  image, because a small room occupies a tiny fraction of the 0.02–100 m frustum.
- **Statistics warnings may still appear.** `report_submit` fires only on new frames while the UI runs
  faster, so some are expected. If the freeze ever returns under load, look here first.
- **Only H.264 verified.** HEVC and AV1 paths exist in the decoder but are untested. AV1's keyframe
  detection is stubbed to always-true, since it is not Annex-B framed.
- Linux is intended but unverified; nothing in the crate is Windows-specific by design.

## Test setup that works

Server settings in `build/alvr_streamer_windows/session.json`:

| Setting | Value | Note |
|---|---|---|
| `stream_protocol` | `Udp` | |
| `preferred_codec` | `H264` | the only verified path |
| `passthrough.enabled` | `false` | its `variant` must be a value this branch knows; a stale `AlphaStream` from `ar_mode` breaks negotiation |
| `client_connections` | emulator hostname trusted | |

Repackaging with `cargo xtask package-streamer` **regenerates `session.json` with defaults**, losing
trust and any settings above. Trust can be restored live without a restart:

```sh
curl -H "X-ALVR: true" -H "Content-Type: application/json" \
  -X POST -d '["<hostname>","Trust"]' \
  http://127.0.0.1:8082/api/session/client-connections
```

Note `cargo xtask package-streamer` currently fails at the license-generation step
(`cargo about` errors); the driver DLL is copied before that, so the package is usable anyway.

**Operational gotchas.**

- Starting SteamVR is best done via the ALVR dashboard, which owns the driver lifecycle.
- **Do not force-kill `vrserver`.** It orphans IPC port 27062 and the next launch fails with
  "Port 27062 in use". Ask the user to close SteamVR; a programmatic close request raises a
  confirmation dialog only they can accept.
- Kill stray `alvr_client_emulator` processes before rebuilding, or the linker cannot replace the exe
  — and a stale instance also holds port 9943, which silently breaks the next run's discovery.

## Suggested next steps

1. **Multiple instances.** Take the `storage.rs` identity patch; the port work is already done.
2. **Capture the decoded video** through the API, so the emulator is useful to an automated harness
   rather than only to a human watching the window.
3. **MCP or richer control surface** over the existing HTTP API.
4. **3D Gaussian splat scenes** for realistic AR environments. The `Scene` type is deliberately a
   geometry container rather than a renderer so an alternative source can slot in.
5. **Zero-copy decode**, only if profiling demands it. ffmpeg's Vulkan decoder currently fails with
   `VK_ERROR_DEVICE_LOST` on the RTX 5090 tested, so the portable GPU path is not viable yet; d3d11va
   works but was slower than CPU for a single stream.
6. **Upstream the three ALVR changes** — they are useful beyond this emulator, particularly the
   announcer shutdown, which is a real leak.
