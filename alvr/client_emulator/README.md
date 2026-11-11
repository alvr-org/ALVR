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
- **Decodes and displays the video stream.** The toolbar switches between the decoded video
  (**Virtual**) and the local glTF scene (**Real**), which is useful for telling a decode problem
  apart from a rendering one.
- **Previews AR passthrough.** With a passthrough mode enabled on the server, including the 8 bit
  alpha stream, the **Mixed** source blends the video over the local scene exactly as the headset
  blends it over the real world; see [Passthrough](#passthrough).
- Renders a glTF scene from a first person camera. This is also what the capture endpoints return —
  they render the scene offscreen, not the video.
- **Emulates motion controllers.** Either controller can be enabled independently, posed in 6DoF,
  and driven through every button and axis the selected controller type has, from the mouse or from
  the HTTP API. The server treats them as real controllers; SteamVR renders and reacts to them.
- **Emulates hand tracking.** Either hand can be enabled instead of its controller, posed in 6DoF,
  put into a pose from a user-editable library, or made to perform a timed gesture built from those
  poses. The full 26-joint skeleton goes on the wire,
  so SteamVR sees hand tracking devices rather than controllers, and the emulator draws the hands
  as skinned models from the very joints it is sending.

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

A test room with baked lighting is committed as `models/environment.gltf`, and `build.rs` copies it
beside the executable (`target/debug/models/`), so a fresh build renders out of the box. It is a
custom 16 x 16 m studio, 3.9 m high: navy walls with a lime line along their foot and a blue
line-fan motif across each, grey carpet with a blue metre grid and a dark blue rug inside a lime
circle at the centre, and two windows looking out over an office district. Along the walls: two
bookshelves (a cat on one, a graphics card on the other), a display cabinet of headsets, model
vehicles, a graphics card, a memory stick and maple syrup, a desk under a screen showing a tactical
map with NATO unit symbols between two connected PCs (one with a graphics card mounted on its side
panel), a foosball table, a sofa, a kitchen counter with a cereal dispenser and a bowl of bananas,
potted plants and small trees, model planes and helicopters hanging from the ceiling, and a colour
chart.
The previous 8 x 8 m room is kept beside it as `models/environment-brown.gltf`; it is not loaded,
but can be copied into the user folder as `environment.gltf` to use it. To use a
room of your own, place it as `environment.gltf` in the emulator's user folder; that file takes
precedence over the bundled one.

**The user folder** is `%LOCALAPPDATA%\ALVR Client Emulator` on Windows and
`~/.local/share/ALVR Client Emulator` on Linux. It holds everything that belongs to the user rather
than the app: the settings files and a room of your own. It is used rather than the executable's
folder because an installed app cannot write to its own folder. Settings files found next to the
executable are copied into the user folder on first run.

Both rooms are generated by `make_environment.py`; rerun it after changing the script:

```sh
python alvr/client_emulator/tools/make_environment.py alvr/client_emulator/models/environment.gltf
python alvr/client_emulator/tools/make_environment.py --style brown alvr/client_emulator/models/environment-brown.gltf
```

Requires Pillow, and Blender for the studio's lighting (below). The generated file is
self-contained: geometry and textures are embedded.

The emulator draws rooms unlit, so a room's lighting has to be baked in: into its base colour
textures, or into vertex colours (`COLOR_0`), which multiply the material's colour as glTF defines.
The test room uses both. Its textures carry the colour and fine shading of each object, and its
vertex colours the light of the room, baked in Blender by `tools/bake_in_blender.py`: the generator
cuts the room's surfaces finely, hands them to Blender in the background with an area light at each
ceiling panel and window, and writes back the light Cycles works out for every vertex, direct and
bounced, with shadows from the real shapes. It takes about half a minute on a GPU.

The generator finds Blender on the `PATH`, in its standard install folder, or from `BLENDER`.
Without it the generator falls back to a simpler bake of its own (`--bake python` forces it): direct
light with shadows from boxes round the furniture, and an estimate of occlusion, on the floor and
walls only. Commit the room baked in Blender.

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
| `R` / `F`, or `Page Up` / `Page Down` | Move up / down |
| `Shift` | Move faster |

There is no collision: walking through walls is expected.

The window draws its own title bar, matching the toolbar under it: the app name and the connection
state on the left, and the window buttons on the right. Hover the connection state for the client
core's full status message, including the server address and any error. Drag the title bar to move
the window, double-click it to maximise, and drag any edge or corner to resize.

The toolbar is one row of icon groups, which wraps group by group onto further lines when the
window is too narrow. Hints on how to look around and drive the devices run along a thin status
bar at the bottom of the window. Hovering any icon says
what it does. The toolbar is otherwise monochrome, so the few colours each keep one meaning: light
blue marks what is selected, and green and red the connection state. The first group selects which
eye the window shows — **Left** (the default), **Right** or **Stereo**, drawn as a pair of eyes with the
shown ones open — and which source it draws. **Virtual** (a headset) is the decoded video, the
content the VR server renders; **Real** (a landscape) is the local glTF scene, which stands in for
what the headset's cameras would see; **Mixed** (the landscape seen through the headset), the
default, is what the headset shows — the video blended over the scene by the stream's passthrough
mode, which is plain video when passthrough is off.

**Stats** (the bar chart) toggles a readout over the top of the view (off at start): the camera position in metres,
and while streaming the stream's resolution and refresh rate, frame pacing, head speed, decoded
frame count and passthrough mode. It wraps onto more lines when the view is too narrow for one. The
same timing numbers are in `GET /api/state`; see [`frame_timing`](#frame_timing).

## Passthrough

Every passthrough mode in the server's `video.passthrough` setting is applied as the headset applies
it, in [`src/video.wgsl`](src/video.wgsl), ported from the headset's stream shader in
`alvr/graphics`: **Blend** (constant alpha), **RGB** and **HSV Chroma Key**, and **Alpha Stream (8
bit)**. The mode's real-time parameters, such as the key colour, apply mid-stream as they do on the
headset.

**Alpha Stream** is the mode from the `ar_mode` branch: the server keeps the application's own alpha
channel and sends it as a second, monochrome video stream. The emulator decodes it with a second
software decoder and pairs its frames with the color frames by timestamp, showing the newest alpha
frame no newer than the color frame on screen, and holding the previous mask if the matching one is
late. The stats line reads `alpha stream, N frames`; a count stuck at zero means the alpha decoder
never received a keyframe or the server could not create the second encoder (look for "alpha
stream" in the server log).

In **Mixed**, the scene behind the video is drawn from the eye poses the displayed frame was
rendered with and with the stream's projection, not from the live camera. That is the emulator's
stand-in for the headset reprojecting the layer onto the current pose: without it, content anchored
in the room would swim against the scene by the stream's latency whenever the camera moved.

Server side, the alpha stream mode needs an application that writes a meaningful alpha channel —
areas it leaves at zero are fully see-through, and with the mode on the server clears uncovered
areas to transparent instead of the usual blue. Whether the alpha encoder exists is decided when
the driver starts, so *switching to* Alpha Stream needs a SteamVR restart; after that, toggling
passthrough on and off, or changing its real-time parameters, applies live.

**The SteamVR dashboard is opaque.** While it is open — the panel with **Resume Game** — the alpha
plane comes through as solid 255 and Mixed looks exactly like Virtual. Close it (the left
controller's `menu/click` toggles it) and the application's own alpha returns. If Mixed ever looks
opaque, check this before suspecting the pairing.

The source can be switched from the API with [`POST /api/source`](#post-apisource), and
`GET /api/state` reports `source`, `passthrough` and `alpha_frames`.

## Controller emulation

The last two toolbar groups control the emulated controllers and emulated hands. The controller
section, marked by a gamepad: **L** and **R** enable each hand independently, the eye shows
the 3D models in the scene view, the axes select a pose to draw as a gizmo, the dropdown selects
which controller type to emulate, and the circular arrow returns poses and inputs to their
defaults.

A side is emulated as a controller **or** as a hand, never both, exactly as a real headset reports
one or the other. Switching a controller on therefore switches that side's hand off, and vice
versa. Switching one off does not bring the other back, so the toggle always means the same thing.

Enabled devices appear as **LC**/**RC** icons (**LH**/**RH** for hands) projected over the 3D view
at their position; when the device is outside the view the icon sticks to the edge with an arrow
pointing towards it. Hovering an icon — including an edge-clamped one, which is how an off-screen
device is brought back — opens the movement panel underneath, four drag pads that pose it:

| Pad | Drag | Effect |
|---|---|---|
| Move | 2D | Translate on the vertical plane facing the head (drag maps 1:1 to on-screen motion) |
| Depth | vertical | Reach out along where the device points, so aiming at something and dragging up touches it. Right-drag moves towards / away from the head instead |
| Roll | horizontal | Roll around the device's own forward axis |
| Aim | 2D | Yaw and pitch in head space, with configurable sensitivity |

Poses are head-relative, so devices ride along as the camera moves and turns. The axes follow
ALVR's convention: X right, Y up, -Z forward, origin at the head. For a hand the pose is the
**palm**, which is where OpenXR puts the hand's root joint.

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

`controllers.json` in the [user folder](#running) is created on first run and can be edited freely;
it is documented by its own content. Relative model paths in it are resolved against the
executable's folder, where the bundled models are. It holds:

- `rotation_sensitivity` — radians of controller rotation per pixel of drag on the rotation pads.
- `left_start_position` / `right_start_position` — head-relative default positions.
- `start_pitch_degrees` — upward pitch of the resting aim, the direction the pointer leaves in
  (default 10). Deliberately above the horizon: resting the controllers' laser on SteamVR's status
  panel makes the system UI capture input focus, which replaces the application's solid controller
  rendering with SteamVR's own laggy teal ghost silhouettes. Files written before the aim/grip
  split below carry 30, which only cancelled the old tilt; lower it to taste.
- `profiles` — the controller types offered for emulation. Each entry has a display `name`, the
  interaction profile `path`, the input path suffixes each hand supports (`left_inputs` /
  `right_inputs`, e.g. `"trigger/value"`), and optionally `left_model` / `right_model`, glTF files
  (relative to the executable's directory) shown when the controller is visible. The Quest profile
  uses the bundled Quest 2 models (see below), also in files written before they existed; set a
  model to `""` to get the placeholder instead. Without a model a small procedural placeholder is
  drawn, its orange nose tip on the aim.
  `grip_offset` places the grip relative to the aim; see below.

**What you steer is the aim; what ALVR is sent is the grip.** OpenXR gives a controller two poses:
the *aim* (`/input/aim/pose`), where it points and where the application's pointer starts, and the
*grip* (`/input/grip/pose`), how the hand holds it. A real client sends the grip, and the
application's aim is derived from it downstream, so the emulator does the same: the pads, the
API's `pose` and `start_pitch_degrees` all set the aim, and the grip sent is the aim composed with
the profile's `grip_offset` — `position` in metres in the aim's frame and `rotation_degrees` about
its X, Y, Z in that order, for the left controller, mirrored across X for the right the way ALVR's
server mirrors its own offsets. The Quest's built-in offset is taken from SteamVR's own model of
the Quest 2 controller (its `openxr_grip` and `openxr_aim` components): the grip sits 60° up from
the aim, 3 cm below it and 9.5 cm behind it. Without it the pointer left the controller about 40°
below where it was aimed, and rolling the controller swung that tilt off to the side. Other
profiles have no known offset yet and send the aim unchanged.

On top of the grip, a real client applies a per-headset correction before sending
(`alvr_client_core::controller_pose_offset`, shared with the headset client so the two cannot
drift). The emulator reports itself as a desktop, so it applies the correction of the headset the
profile belongs to: the Quest's for the Quest profile. That correction is itself a little short —
it turns the grip 15° where SteamVR's model implies 20.6° — so SteamVR's laser leaves about 5.6°
above where the emulator aims, exactly as it does for a real Quest. The gray placeholder's orange
nose shows the emulator's aim.

**Controller models are drawn at the grip**, the frame OpenXR and WebXR controller models are
authored in. The Quest profile's are committed as `models/quest_controller_{left,right}.glb` and
copied beside the executable by `build.rs`: the Quest 2 Touch controllers from the W3C
[WebXR Input Profiles](https://github.com/immersive-web/webxr-input-profiles), MIT licensed
(`models/quest_controller_LICENSE.md`; the licence does not extend to Meta's trademarks).
[`tools/fetch_controller_model.py`](tools/fetch_controller_model.py) is how they were obtained and
refreshes them. WebXR models are authored around Meta's grip, which is not quite the SteamVR grip
the emulator's is built on, so the script adds the measured difference as a parent node: fitting
the mesh onto SteamVR's own Quest 2 model gives 15.7° about X and 2.5 cm, to 1.6 mm RMS, and the
bundled models then sit within 1.3 mm (median) of where SteamVR draws its own.

SteamVR's own render models cannot be redistributed, but SteamVR ships them locally and
[`tools/convert_rendermodel.py`](tools/convert_rendermodel.py) converts one into a glTF the
emulator loads, for controllers without a bundled model:

```sh
python alvr/client_emulator/tools/convert_rendermodel.py \
  "C:/Program Files (x86)/Steam/steamapps/common/SteamVR/resources/rendermodels/oculus_quest2_controller_left" \
  target/debug/models/quest_left.gltf
```

Run it once per hand and point the profile's `left_model` / `right_model` at the outputs. Render
models are authored around the SteamVR device pose, so the converter moves them into the frame of
their own `openxr_grip` component. A render model without one falls back to ALVR's default
grip-to-device translation (`0, 0, -0.11`, or `--offset`), which only approximates the grip. The
models display in both the scene and
the video view — over the video they overlap the application's own controller rendering, which is
exactly the comparison the display toggle is for.

The default file is generated from ALVR's own interaction profile definitions, so the predefined
profiles emulate exactly the inputs ALVR accepts from each real controller: Quest, Index, Vive
Wand, Pico Neo3 / 4 / 4S / G3, PSVR2 Sense, Vive Focus 3 and YVR. New profiles can be added as long
as they use inputs from that set — unknown inputs are ignored with a warning. Note the emulator
reports inputs exactly as the real controller would; any remapping to the server's configured
emulation mode happens on the server, as with real hardware.

## Hand emulation

> Implementation notes, the measurements behind the server settings below, and the dead ends are in
> [`HAND-EMULATION.md`](HAND-EMULATION.md).

The hand section, marked by a hand, mirrors the controller one: **L** and **R**
enable each hand, the eye shows the 3D models, the axes select a pose gizmo, and the circular arrow
returns the pose and the gesture to their defaults. Hands are posed with the same icons and the same movement panel as controllers, and
excluded against them per side as described above.

What differs is what is being emulated. A controller sends buttons; a hand sends **the 26 joint
poses of OpenXR's `XR_EXT_hand_tracking`** and *no* device motion for that side, which is exactly
what `client_openxr` sends for a freely tracked hand. That absence is what the server keys on: it
runs its gesture recognition only for a hand whose device motion is missing, and the driver derives
the SteamVR device pose from the skeleton's palm joint. Verified against a running SteamVR: with
both hands on, the emulated controller devices go invalid and two hand-tracking devices appear,
posed by the emulator; switching a side back to a controller reverses it.

While a hand is enabled, a panel sits in the matching bottom corner holding two grids separated by
a rule:

- **Poses** — how the hand is held. Clicking one moves the hand into it over a configurable time
  (0.5 s by default), eased in and out.
- **Gestures** — what the hand *does*: a timed sequence of those poses. Clicking one plays it
  through and leaves the hand in whatever pose it ended on.

Gestures sit on the *inner* side of each panel, nearer the middle of the view, so the two hands'
panels mirror each other and what you reach for most often is closest to the scene. The running
pose change or gesture fills a bar along the bottom of its button.

Each button draws the articulation it selects, rendered from its own joints — so anything added to
`hands.json` gets a correct icon without artwork, and an icon cannot drift from what it does. A
gesture is drawn as the keyframe furthest from the one it starts in, since a click drawn at its
starting point would be indistinguishable from the pose it starts in, and carries a small play mark
to set it apart from a pose. Tooltips carry the name and description.

### Gestures

A pose alone cannot express a tap. Holding a pinch and releasing it by hand means getting the
timing right with the mouse, which is fiddly and not repeatable — so a gesture is a keyframed path
through the poses over a fixed duration:

```json
{
  "name": "Click",
  "description": "Pinch and release while pointing, which is a trigger click",
  "duration_seconds": 0.5,
  "keyframes": [
    { "pose": "Point",  "phase": 0.0 },
    { "pose": "Pinch",  "phase": 0.5 },
    { "pose": "Point",  "phase": 1.0 }
  ]
}
```

`phase` runs 0 at the start to 1 at the end, and each segment eases in and out so a tap accelerates
away from one pose and settles into the next the way a finger does. Playing a gesture that is
already running restarts it, which is what makes a click button repeatable without waiting.

Two are provided: **Click** and **Double**. Only momentary ones, deliberately — anything a gesture
would merely *arrive* at, such as a closed fist or a held pinch, is already a pose, and shipping it
as a gesture too would put the same thing in both grids. A gesture naming a pose that does not
exist is dropped at load with a warning rather than played as a flat hand.

### How a pose is described

Posing 26 joints directly would mean 26 free rotations, most combinations of which are not hands.
A pose is therefore seven numbers, all `0`..`1`, and the joint angles follow from them:

| Field | Meaning |
|---|---|
| `thumb`, `index`, `middle`, `ring`, `little` | Curl of each digit: 0 extended, 1 fully curled. Spread across that finger's knuckles the way the tendons do — roughly 85° at the knuckle, 100° at the middle joint, 68° at the fingertip joint |
| `spread` | 0 fingers together, 1 fully splayed. Fans the knuckles about the palm normal |
| `thumb_opposition` | 0 lies alongside the fingers in the plane of the palm, 1 is fully opposed across the palm with the pad facing the fingertips. Applies curled or not |

This is the design's parameterisation with two refinements. Splay is attenuated as the fingers
curl, since a fist cannot fan. And the thumb's base is built from two interpolated directions —
where the metacarpal points, and where flexing it takes the tip — rather than from angles about the
palm's axes: opposition is a rotation about no axis the palm has, and expressing it as Euler angles
makes a curling opposed thumb swing away from the fingers instead of towards them, which is what
stops it ever being able to pinch.

Curling the ring and little fingers also cups the palm at their carpometacarpal joints, which a
flat palm cannot fake.

Unit tests assert the geometry against the numbers the **server's** gesture recognition uses
(`hand_gestures.rs` plus the defaults in `HandTrackingInteractionConfig`): that Pinch really does
bring the fingertips within its click distance, that Grasp curls every finger inside the curl
distance while Point keeps the index out of it, and that the two hands are exact mirrors.

### Gestures and settings

`hands.json` in the [user folder](#running) is created on first run and can be edited freely. It
holds:

- `rotation_sensitivity` — radians of palm rotation per pixel of drag, as the controllers have.
- `left_start_position` / `right_start_position`, `start_pitch_degrees`, `start_roll_degrees` —
  the resting palm pose. The two hands roll inwards by the same angle in opposite directions.
- `hand_length` — wrist joint to middle fingertip, in metres. Scales the whole skeleton, so the
  gesture distances the server measures scale with it.
- `transition_seconds` — how long a change of pose takes, unless the pose overrides it.
- `model_color` — what the hand model is tinted with. A hand model usually carries no material, so
  without this it would draw plain white; set it to `[1, 1, 1]` for a model that is textured.
- `glove_color` — the bundled model is a *fingerless glove*, with a modelled
  cuff ridge at the second knuckle. Drawn in one colour that ridge reads as an unexplained seam
  across every finger, so the glove is tinted separately from the bare fingertips; the blend
  follows the skin weights, so the seam lands on the ridge. Set it to `null` for a plain hand
  model, which then draws entirely in `model_color`.
- `left_model` / `right_model` — glTF hand models, relative to the executable's directory.
- `joint_names` — maps a rig's own bone names onto the canonical joint names, for a model whose
  names are not recognisable. Rarely needed; see below.
- `poses` — the selectable articulations, each with a `name`, a `description` for the tooltip, the
  seven articulation fields (all defaulting to 0), and optionally `transition_seconds`. Idle,
  Grasp, Point, Pinch and Plank (a flat open hand) are provided; a file written before Plank
  existed lacks it, and gets it by adding the entry or deleting the file. Pinch was retuned when
  the thumb was reworked to curl across the palm: a file written before that has `"thumb": 0.45,
  "index": 0.52, "thumb_opposition": 0.72`, which no longer brings the tips together. Change those
  to `0.15`, `0.45` and `0.7`, or delete the file to have it regenerated.
- `gestures` — the timed sequences, described above.

Add as many of either as you like; the grids grow to fit. A settings file this cannot parse — one
written before the poses and gestures were separated, for instance — is moved aside to
`hands.json.old` and replaced with fresh defaults, rather than leaving you with built-in settings
that no edit can change.

### Hand models

A left and a right hand from [Godot XR Tools](https://github.com/GodotVR/godot-xr-tools) are
committed in `alvr/client_emulator/models/`, and the build copies them into
`target/<profile>/models/`, where `hands.json` looks for them. Nothing needs downloading. The glTF
files declare themselves CC0 Public Domain in their own `asset.copyright` (the surrounding project
is MIT licensed), and they suit this exactly: one skinned mesh each, rigged to twenty-six joints
named after the same OpenXR layout ALVR carries.

`tools/fetch_hand_model.py` is how they were obtained, and refreshes them: it pins the upstream
commit and checks a SHA-256, so a refresh yields the same art.

```sh
python alvr/client_emulator/tools/fetch_hand_model.py --force
```

Nothing depends on those particular files. Any rigged, skinned glTF hand works: joints are matched
by name, ignoring case, separators and a trailing `_L` / `_R`, and `joint_names` covers a rig whose
names are unrecognisable. Bones with no counterpart — a forearm, a decorative bone — ride along on
their parent using the offset they had in the bind pose.

**Rig conventions are worked out rather than assumed.** The model that ships here runs each bone
along its own +Y, a Blender armature default, while OpenXR joints run along -Z, and its bind pose
is a slightly relaxed hand rather than a flat one. So both skeletons are reduced to the same purely
geometric frame — bone along -Z, back of the hand along +Y — and the constant rotation between a
rig's own bone frame and that one is measured once from its bind pose.

Joints then take their **rotation** from the emulated skeleton and their **position** from the
model's own bind pose, with only the root pinned and the whole model uniformly scaled to
`hand_length`. Pinning every joint to the emulated positions instead — so that the drawn hand is
exactly the hand being sent — is the obvious thing to do and looks wrong: no two hands have the
same proportions, and against this model the emulator's phalanges run 0.84x to 1.16x of the
model's, alternating along each finger, so pinning squashes one segment while stretching the next
and a straight finger comes out visibly wavy. Keeping the model's proportions costs a few
millimetres between the drawn fingertip and the transmitted one, and the joint *angles* — which
are what a pose is — are exact either way.

The models are shaded from their normals rather than drawn unlit like the scene, because a hand
model carries no baked lighting and a flat fetch would draw it as a silhouette with no readable
curl.

### Pointing and clicking

Two things about hand tracking in SteamVR are the server's doing rather than the emulator's, and
both surprise people.

**The ray comes from the index knuckle, along the index finger.** SteamVR draws a hand's pointer
from the hand profile's `/pose/tip`, which ALVR's driver sets from the joints it is sent: at the
index knuckle, pointing along the index metacarpal with +Y out of the back of the hand. That is the
aim pose of OpenXR's `XR_EXT_hand_interaction` figure, and it is rigid with the palm, so a pinch
does not move the ray off its target. The emulator's **Aim** gizmo is the same pose, so the ray
should leave exactly from the gizmo's origin along its -Z. SteamVR's poke cursor uses the same
tip, so poking happens at the knuckle too.

The device pose itself is still the palm joint plus `left_hand_tracking_rotation_offset` (default
`[0, -45, -90]` degrees) and `left_hand_tracking_position_offset` (default `[0.04, -0.02, -0.13]`
m, both mirrored for the right hand). Those present a hand-tracked hand as a *held controller*, and
now only place the render model and the grip anchor applications attach held objects to; the ray no
longer depends on them. With the head and the palm both at identity, the device orientation SteamVR
reports is *bit for bit* the rotation offset, so the emulator is sending the palm exactly where
ALVR expects it. The palm is the OpenXR specification's: "at the center of the middle finger's
metacarpal bone", "+Z parallel to the middle finger's metacarpal bone, pointing away from the
finger tips", "+Y ... pointing towards the back of the hand", which a unit test asserts.

**Clicking needs one switch.** `headset.controllers.hand_tracking_interaction` is **off by
default**, and while it is off no hand gesture produces any button at all, so nothing can be
clicked at any distance. Turn it on and the server derives buttons from the joints it is being
sent, on both the controller and the hand-tracking device:

| Gesture | Left | Right |
|---|---|---|
| Thumb + index pinch | `trigger/value`, and `trigger/click` | the same |
| Thumb + middle pinch | `y/click` | `b/click` |
| Thumb + ring pinch | `x/click` | `a/click` |
| Thumb + little pinch | `menu/click` | — |
| Curling the last three fingers | `squeeze/value`, and `squeeze/click` | the same |
| Curling the thumb | `thumbstick/click` | the same |
| The thumb over the index finger | `thumbstick/x`, `thumbstick/y` | the same |

So the built-in **Pinch** pose is a trigger pull and **Grasp** is a grip squeeze, and the **Click**
gesture is a whole press-and-release — which is the right
model to emulate: a pinch is the select gesture on Quest and in OpenXR's own
`XR_EXT_hand_interaction`. (The air tap that moves the index finger down and up instead is
HoloLens'; nothing in this path uses it.) Their poses are built to land inside the recognition's
default distances, which the unit tests assert against the same numbers `hand_gestures.rs` uses.

> **A pinch does not click, and it is a driver limitation rather than anything the emulator can
> send.** With `hand_skeleton.steamvr_input_2_0` on — the default — the hands are presented as
> *separate hand-tracking devices*, and `props.rs` gives those devices SteamVR's
> `svl_hand_interaction_augmented` input profile. That profile declares `/input/index_pinch`,
> `/input/grip`, `/input/system`, `/input/index_point` and the skeleton; it has **no
> `/input/trigger`**. Meanwhile `register_buttons` maps the tracker back to the hand id and creates
> the *emulated controller's* components on it (`trigger/value`, `x/click`, ...), and the string
> `index_pinch` appears nowhere in ALVR's source. Applications bind against the advertised profile,
> so they wait on inputs nothing ever sets: the pose and the skeleton work, and no button ever
> does.
>
> **Turning `steamvr_input_2_0` off is not a good workaround**, though it does make clicking work.
> The hands then ride on the ordinary controller device, which brings the whole controller
> presentation with it: SteamVR shows controller icons and draws controller models, the pointer
> comes from the oculus_touch profile's own tip pose rather than from the palm — so the finger
> alignment above is undone — and applications request the skeleton in its "with controller" range,
> which curls the fingers around a controller that is not there. It also needs a SteamVR restart
> each way, since the option is `steamvr-restart` flagged.
>
> The fix belongs in the driver: either set the profile's own inputs on the hand-tracking devices
> (`/input/index_pinch` from the thumb-index pinch gesture, `/input/grip` from the grip curl), or
> stop advertising a profile whose inputs are never driven. Until then, hand emulation gives
> correct poses, skeletons and gestures, and no clicks.
>
> (The server log also shows `Received button not mapped: .../trigger/click`. That one is a red
> herring: when a gesture source has a value but no click, `automatic_bindings` derives the click
> from the value with a threshold, so that path is fine.)

### Server settings

Hand tracking reaches SteamVR through settings that are on by default, but worth knowing:

| Setting | Effect |
|---|---|
| `headset.controllers.hand_skeleton` | Must be enabled, or the skeleton is dropped before the driver. `steamvr_input_2_0` puts the hands on separate hand-tracking devices rather than on the controller devices — **turn it off if you want gestures to click**; see above |
| `headset.controllers.tracked` | Must be true, as for controllers |
| `headset.controllers.hand_tracking_interaction` | **Off by default.** Turn it on to have the server derive controller buttons from the gestures — pinches, curls, a joystick from the index finger. The gesture poses here are built to land inside its default thresholds |
| `left_hand_tracking_position_offset` / `rotation_offset` | The server offsets the SteamVR device pose from the palm by these (default `[0.04, -0.02, -0.13]` and `[0, -45, -90]`, mirrored for the right hand), so the device pose sits where a controller grip would. Real hand tracking gets the same treatment; the emulator does not compensate for it |

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
  "codec": null,
  "source": "mixed",
  "passthrough": null,
  "alpha_frames": 0,
  "frame_timing": {
    "publish_ms":    { "mean": 8.35,  "deviation": 4.34 },
    "sent_ms":       { "mean": 4.63,  "deviation": 0.16 },
    "sent_step_deg": { "mean": 0.277, "deviation": 0.156 },
    "world_ms":      { "mean": 13.89, "deviation": 0.21 },
    "step_deg":      { "mean": 0.833, "deviation": 0.155 },
    "step_mm":       { "mean": 0.0,   "deviation": 0.0 },
    "screen_ms":     { "mean": 13.89, "deviation": 0.72 },
    "repeated_view_ratio": 0.0
  }
}
```

`hud_message` carries the client core's own status text, which is where connection errors surface.

`codec` becomes the negotiated codec once the server announces it. Decoded frame count and frame
layout are shown in the stats overlay but are not yet exposed here.

`passthrough` is the stream's passthrough mode (`"blend"`, `"RGB chroma key"`, `"HSV chroma key"`
or `"alpha stream"`), `null` for an opaque stream; `alpha_frames` counts the alpha stream frames
shown. See [Passthrough](#passthrough).

#### `frame_timing`

How evenly the streamed world is advancing, averaged over the last ~150 displayed frames. The
**Stats** overlay shows a subset of the same numbers. Each entry is a `mean` with its mean absolute
`deviation`, and the deviations are the interesting half — they say *where* judder is coming from.

| Field | Measures | A large deviation means |
|---|---|---|
| `publish_ms` | UI thread handing poses to the tracking thread | uneven UI frame times |
| `sent_ms`, `sent_step_deg` | tracking packets leaving, and the head rotation between them | the emulator built an uneven signal |
| `world_ms`, `step_deg`, `step_mm` | tracking time and head movement between consecutive displayed frames | **the judder you can see** |
| `screen_ms` | real time each frame was on screen | this window presented them unevenly |
| `repeated_view_ratio` | frames that came back with the previous frame's pose | view parameter lookups are missing; should be `0` while moving |

The `sent_*` / `step_*` pair is the useful split: an even step going out with an uneven one coming
back means the signal left clean and something downstream resampled it.

Under steady motion `step_deg.mean` should equal the turn rate times `world_ms.mean`, with
`step_deg.deviation` near zero. Drive the camera with `POST /api/drive` and compare.

### `POST /api/source`

Selects what the window draws, as the toolbar's **Source** buttons do: `mixed`, `virtual` or `real`.

```sh
curl -X POST http://127.0.0.1:8080/api/source -d '{"source": "real"}'
```

### `POST /api/drive`

Holds a camera input until changed, as if keys were held down. Omitted fields reset to zero, so
`{}` stops.

```sh
curl -X POST http://127.0.0.1:8080/api/drive \
  -H "Content-Type: application/json" \
  -d '{"forward": 1, "yaw_rate": 60}'
```

`forward`, `right` and `height` are key-press amounts, not speeds; `yaw_rate`, `pitch_rate` and
`roll_rate` are degrees per second; `fast` is the shift modifier. `GET` returns the current value.

Use this rather than repeated `POST /api/move` calls whenever motion smoothness is what is being
measured. A rate is integrated against the real frame time down the same path the keyboard uses,
whereas posting individual poses puts the calling script's own scheduling into the tracking signal —
and no HTTP client paces itself to anywhere near a frame.

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

`position` and `orientation` are the head-relative **aim** pose (X right, Y up, -Z forward; an
XYZW quaternion): -Z is where the controller points. The grip actually sent is derived from it; see
[Controller profiles and settings](#controller-profiles-and-settings).
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

### `GET /api/hands`

Both hands' full state, plus the available gestures:

```json
{
  "poses":    [{ "name": "Idle",  "description": "Relaxed open hand, ..." }, ...],
  "gestures": [{ "name": "Click", "description": "Pinch and release ..." }, ...],
  "left": {
    "enabled": true,
    "visible": true,
    "position": [-0.18, -0.25, -0.35],
    "orientation": [0.172, 0.023, -0.129, 0.976],
    "pose": "Pinch",
    "gesture": null,
    "articulation": {
      "thumb": 0.15, "index": 0.45, "middle": 0.95, "ring": 1.0, "little": 1.0,
      "spread": 0.0, "thumb_opposition": 0.7
    },
    "moving": false
  },
  "right": { ... }
}
```

`position` and `orientation` are the head-relative palm pose, same axes as the controllers.
`articulation` is what is being sent *right now*, so mid-movement it is between two poses. `pose`
is `null` when the articulation was set directly rather than picked; `gesture` names the sequence
playing, if any; `moving` is true while either is still running.

### `POST /api/hands/{left|right}`

Enables the hand and shows its model. Every field is optional. Enabling a hand disables that side's
controller, as the toolbar toggles do:

```sh
curl -X POST http://127.0.0.1:8080/api/hands/left \
  -H "Content-Type: application/json" \
  -d '{"enabled": true, "visible": true}'
```

### `POST /api/hands/{left|right}/pose`

Sets the head-relative palm pose. Both fields optional; the quaternion is normalised on apply:

```sh
curl -X POST http://127.0.0.1:8080/api/hands/left/pose \
  -H "Content-Type: application/json" \
  -d '{"position": [-0.1, -0.05, -0.4], "orientation": [0.38, 0.0, 0.0, 0.92]}'
```

### `POST /api/hands/{left|right}/articulation`

Moves the hand into a named pose, into an articulation given outright, or into a pose with some
fields overridden. `transition_seconds` overrides the configured time; `0` applies immediately,
which is what a scripted test usually wants:

```sh
curl -X POST http://127.0.0.1:8080/api/hands/right/articulation \
  -H "Content-Type: application/json" \
  -d '{"pose": "Pinch"}'

curl -X POST http://127.0.0.1:8080/api/hands/right/articulation \
  -H "Content-Type: application/json" \
  -d '{"pose": "Grasp", "little": 0.0, "transition_seconds": 0}'

curl -X POST http://127.0.0.1:8080/api/hands/left/articulation \
  -H "Content-Type: application/json" \
  -d '{"index": 1.0, "spread": 0.5}'
```

Unknown pose names are rejected with a 400 listing what is available. Articulation fields given
without a pose modify what the hand is currently holding.

### `POST /api/hands/{left|right}/gesture`

Plays a timed gesture from its beginning, restarting it if it is already running:

```sh
curl -X POST http://127.0.0.1:8080/api/hands/left/gesture \
  -H "Content-Type: application/json" \
  -d '{"gesture": "Click"}'
```

`GET /api/hands` reports it as `gesture` while it runs, with `moving` true, and the hand is left
holding whatever pose the sequence ended on.

### `POST /api/hands/{left|right}/reset`

Returns the palm pose and the articulation to their defaults and stops any gesture. Emulation stays
enabled.

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
- **Hand models are loaded once**, when a hand's model is first shown. Editing `hands.json` to
  point at a different one takes a restart, unlike a controller profile change.
- A hand model may have at most 64 skin joints; beyond that the extras are not posed. A hand has
  26.

## Layout

| File | Purpose |
|---|---|
| `src/main.rs` | eframe app, toolbar, input handling, API request servicing |
| `src/camera.rs` | First person camera and eye/projection matrices |
| `src/scene.rs` | glTF loading into a plain geometry container |
| `src/skinned.rs` | Skinned glTF loading and retargeting an arbitrary hand rig onto the joints |
| `src/render.rs` | wgpu pipelines, on-screen views, offscreen capture, controller and hand models |
| `src/client.rs` | `ClientCoreContext` lifecycle, tracking thread, button and skeleton sync |
| `src/controllers.rs` | Controller state, profiles and the settings file |
| `src/hands.rs` | Hand articulation model, skeleton synthesis, gestures and the settings file |
| `src/overlay.rs` | Device icons over the view and the 6DoF movement panel, shared by both kinds |
| `src/controller_ui.rs` | Controller toolbar section and the corner input panels |
| `src/hand_ui.rs` | Hand toolbar section and the corner gesture panels |
| `src/icons.rs` | Painted toolbar icons and the icon buttons that show them |
| `src/api.rs` | HTTP server and shared state |
| `src/shader.wgsl` | Unlit vertex/fragment shader |
| `src/hand.wgsl` | Skinned, normal-shaded hand shader |
| `tools/make_environment.py` | Generates a test environment |
| `tools/bake_in_blender.py` | Bakes the test environment's lighting in Blender, run by `make_environment.py` |
| `tools/convert_rendermodel.py` | Converts a local SteamVR render model into a loadable glTF |
| `tools/fetch_hand_model.py` | Refreshes the committed CC0 hand models in `models/` |
| `tools/fetch_controller_model.py` | Refreshes the committed MIT Quest controller models in `models/`, aligned to the grip |

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
