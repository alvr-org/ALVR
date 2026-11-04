# Hand emulation — handover

**Branch:** `emulator` · Companion to [`HANDOVER.md`](HANDOVER.md) (the emulator as a whole) and
[`README.md`](README.md) (how to use it). The original brief is
[`tasks/Hand-Emulation.md`](tasks/Hand-Emulation.md).

**State:** working and verified against a running SteamVR. Hands present as hand-tracking devices,
poses and gestures reach applications as a real hand-tracked headset's would, and the emulator
draws them from the joints it is sending. One change is **in flight**; see "Not finished" below.

---

## What it does

Either hand can be emulated instead of that side's controller: posed in 6DoF with the same icon and
drag pads the controllers use, put into a **pose** from a user-editable library, or made to perform
a **gesture** — a timed sequence of those poses. The full 26-joint `XR_EXT_hand_tracking` skeleton
goes on the wire, so the server runs its real gesture recognition and the driver registers real
hand-tracking devices. A skinned glTF hand is drawn from the same joints.

## Architecture

The path from a mouse click to an application seeing a pinch:

```
hand_ui.rs        pose / gesture panels in the corners        \
overlay.rs        icon over the view + 6DoF drag pads          |  UI thread, per frame
main.rs           applies API commands, advances animations   /

hands.rs          HandState -> HandPose (7 numbers)
                  skeleton() -> [Pose; 26] in the palm frame
                     |                              \
                     |                               \
main.rs             |                                 skinned.rs + render.rs
  composes with      |                                   retarget onto the model's rig,
  the palm pose      |                                   skinning matrices, hand.wgsl
                     v
client.rs         TrackedState.hands -> interpolated at the tracking rate
                  TrackingData { hand_skeletons: [Some([Pose; 26]); 2], .. }
                     |
                     v  the real ALVR protocol from here down
server_core       tracking/mod.rs        recentres, stores per timestamp
                  hand_gestures.rs       distances between fingertips -> gesture values
                  input_mapping.rs       gesture values -> emulated controller buttons
                     |
                     v
server_openvr     lib.rs                 skeleton -> to_openvr_ffi_hand_skeleton (26 -> 31 bones)
                  props.rs               device identity and input profile
                  Controller.cpp         device pose from the palm joint, components, buttons
                     |
                     v
                  SteamVR, and the application
```

Two properties are worth stating because a lot follows from them:

- **A hand replaces its controller on the wire; it does not accompany it.** An enabled hand sends
  `hand_skeletons[i]` and *no* `HAND_*_ID` device motion, which is what `client_openxr` does for a
  freely tracked hand. The absence is load-bearing at both ends: `server_core`'s `tracking_loop`
  runs `trigger_hand_gesture_actions` only for a hand whose device motion is missing, and
  `Controller.cpp` derives the device pose from `handSkeleton->jointPositions[0]` only when
  `controllerMotion` is null. Sending both is the *multimodal* case — a hand holding a controller,
  which ALVR supports through `headset.multimodal_tracking` and the
  `/user/detached_controller_meta/*` devices — and is not what the toggles mean. Hence the per-side
  exclusion in the UI and in the API.
- **Detection happens on the server, not here.** `client_openxr` reads no pinch, grasp or aim
  inputs from the OpenXR runtime; it sends joints, and `hand_gestures.rs` derives everything from
  them. The emulator does the same deliberately: detecting a pinch client-side would send something
  no real client sends, and would skip the very code a hand-tracked headset exercises. (A real
  Quest runtime *does* expose pinch through `XR_EXT_hand_interaction`; ALVR simply does not use it.
  If it ever does, the emulator will need to synthesise those values too.)

## Code layout

| File | Lines | Purpose |
|---|---|---|
| `src/hands.rs` | ~1590 | Pose model, 26-joint skeleton synthesis, poses and gestures, `hands.json`, the tests |
| `src/hand_ui.rs` | ~640 | Toolbar section, corner panels (pose grid + gesture grid), icons drawn from the poses |
| `src/overlay.rs` | ~520 | Device icon over the view and the 6DoF movement panel — **shared with the controllers** |
| `src/skinned.rs` | ~500 | Skinned glTF loading and retargeting any rig onto the emulated joints |
| `src/hand.wgsl` | 85 | Skinned, normal-shaded hand shader |
| `src/render.rs` | — | `HandRenderer`, `SkinPipeline`, `HandTint`, `DeviceModels` |
| `src/client.rs` | — | `TrackedHand`, skeleton interpolation, `hand_skeletons` on the wire |
| `src/api.rs`, `src/main.rs` | — | `HandCommand`, `HandsResponse`, state ownership and wiring |
| `models/hand_*.gltf` | — | The CC0 hand models, copied beside the executable by `build.rs` |
| `tools/fetch_hand_model.py` | — | Refreshes those models from upstream |

`overlay.rs` is shared on purpose: the brief called for hands to be posed identically to
controllers, and two copies would have drifted. It has four slots — a controller and a hand per
side — and `Slot::label()` is what distinguishes `LC`/`RC` from `LH`/`RH`.

## The pose model

Seven numbers, all 0..1: a `curl` per digit, one `spread`, one `thumb_opposition`. Not for
usability alone — a joint-angle representation makes most of its state space anatomically
impossible, so an API caller sweeping values would mostly produce non-hands.

Built in the **palm frame**, which is OpenXR's palm joint verbatim: origin at the centre of the
middle metacarpal, -Z towards the fingertips, +Y out of the back of the hand. The constants place
the middle metacarpal exactly on the palm's Z axis, which `palm_and_wrist_match_the_spec` asserts.
Being close is not the same as being right here: every offset downstream was tuned against the
real convention.

The wrist is the one deliberate departure. It is the WebXR generic hand's, a few millimetres off
the middle finger's axis, and oriented like the palm, *not* as the spec words it ("+Z parallel to
the line from wrist joint to middle finger metacarpal joint"). That wording puts the middle
metacarpal exactly on the wrist's Z axis, and SteamVR cannot handle that; see "the shaking wrist"
below. `the_middle_metacarpal_is_off_the_wrist_axis` keeps it about 6° off.

Two refinements over the brief:

- **Splay attenuates with curl** — a fist cannot fan.
- **The thumb's base frame is two directions, not three angles.** Opposition is a rotation about no
  axis the palm has: the metacarpal swings across in front of the palm *and* the digit rolls, so
  that flexing it afterwards carries the tip towards the fingers. As `fan × swing × roll` Euler
  angles the two interact, and the measured result was a thumb tip moving *further* from the index
  as opposition increased — 45 mm to 96 mm — so a pinch was unreachable at any curl. Interpolating
  a bone direction and a "where flexing takes the tip" direction and building an orthonormal frame
  from the pair gives 11 mm.

**The tests are calibrated against the server, not against taste.** `hand_gestures.rs` measures real
distances between fingertips and adds fixed finger radii to the configured thresholds; the tests use
those same numbers. A change to the anatomy that would stop Pinch registering as a pinch fails the
build rather than being discovered in SteamVR. 15 tests, all in `hands.rs` and `decoder`.

## Poses against gestures

A **pose** is how a hand is held — held indefinitely. A **gesture** is what a hand does — a
keyframed path through poses over a fixed duration, each segment eased in and out. The split
arrived after the first real click: holding a pinch by mouse and releasing it at the right moment
is fiddly and not repeatable, and no static pose fixes that, because the thing being emulated is a
movement.

Built in: poses Idle / Grasp / Point / Pinch, gestures Click and Double. Only momentary gestures
ship — anything a sequence would merely *arrive* at is already a pose, and shipping both puts the
same thing in two grids.

The older naming had "gesture" meaning the static thing, so a `hands.json` written before the split
will not parse; that file is moved to `hands.json.old` and replaced with defaults, rather than
leaving the user with built-ins no edit can change. The API renamed the pose selector to
`/articulation` and gave `/gesture` to the new concept.

## Rendering

The models are committed in `models/` and copied beside the executable by `build.rs`: two CC0
hands from Godot XR Tools, rigged to 26 joints named after the OpenXR layout.
`tools/fetch_hand_model.py` refreshes them (pinned commit, SHA-256 checked).

**Retargeting assumes nothing about the rig.** That model runs bones along +Y (Blender armature
default) where OpenXR runs along -Z, and its bind pose is a relaxed hand, not a flat one. Both
skeletons are reduced to a geometric frame derived only from joint positions
(`hands::canonical_frames`), and the constant offset between a rig's own bone frame and that one is
measured from its bind pose. A naive `inv(rest) * bind` correction would have baked the bind pose's
~20° of curl into the flat pose.

**Joints take rotation from the emulator and position from the model's own bind pose**, root pinned,
whole model scaled to `hand_length`. Pinning every joint to the transmitted positions is the obvious
thing and looks wrong: the emulator's phalanges are 0.84x to 1.16x of the model's and the ratio
*alternates* along each finger, so every segment was squashed or stretched in turn and a straight
finger rendered with an S-curve that read as bending backwards.

Other points: joint matrices go in a **uniform** buffer (`array<mat4x4<f32>, 64>`), because
read-only storage in the vertex stage is a downlevel capability not every backend offers, and
`glam::Mat4` is not `Pod` here so the block is packed column by column. Hands are **shaded** while
the scene is unlit — a bare mesh drawn unlit is a silhouette with no readable curl. `glove_color`
two-tones a fingerless-glove model; its cuff sits at the second knuckle, found by profiling the
bind pose's cross-sectional radius along each finger.

## The server side, and the settings that matter

| Setting | Effect |
|---|---|
| `headset.controllers.hand_skeleton` | Must be enabled or the skeleton is dropped before the driver |
| `...hand_skeleton.steamvr_input_2_0` | **The big one.** On: hands are separate devices with SteamVR's `svl_hand_interaction_augmented` profile, hand icons, full skeletal level. Off: hands ride the controller device and bring its whole presentation — controller icons and models, a ray from the oculus_touch tip pose, and the skeleton in its "with controller" range, which curls the fingers around a controller that is not there. `steamvr-restart` flagged |
| `...hand_tracking_interaction` | **Off by default.** While off, no gesture produces any button, so nothing can be clicked at any distance |
| `left_hand_tracking_position_offset` / `rotation_offset` | Place the device pose relative to the palm, which places the render model and the grip anchor applications attach held objects to. The pointer ray no longer follows them; see below |

The pointer ray and poke cursor come from the profile's `/pose/tip`, which the driver sets from
the joints (`tracking::hand_tip_offset`): at the index knuckle, pointing along the index metacarpal,
+Y out of the back of the hand. Both are rigid with the palm, so pinching does not move the ray, and
they are independent of the offsets above. Defaults `[0.04, -0.02, -0.13]` and `[0, -45, -90]`
therefore only present the hand as a *held controller* for the render model and the grip anchor.
Before the tip had its own orientation it inherited the device's, and the ray left 45° off the
fingers; the only cure then was a rotation offset that also moved the grip anchor.

## The driver change

**Deployed 2026-09-05, not committed.** With `steamvr_input_2_0` on, the hand-tracking
devices advertise `svl_hand_interaction_augmented`, whose inputs are the four pinches
(`index_pinch` and so on), `grip`, `index_point`, `system`, the skeleton and the poses — there is
**no `/input/trigger`**. But `register_buttons` maps the tracker id back to the hand id and creates the
*emulated controller's* components, and `index_pinch` appears nowhere in ALVR's source. Applications
bind against the advertised profile and wait on inputs nothing sets: pose and skeleton work, no
button ever does.

The fix adds a parallel mapping selected by `device_id`
(`alvr/server_openvr/cpp/alvr_server/{Paths.h,Paths.cpp,Controller.h,Controller.cpp}`, +73 lines):

| ALVR button id | Controller device | Hand-tracker device |
|---|---|---|
| `trigger/value` | `/input/trigger/value`, `/input/l2/value` | **`/input/index_pinch/value`** |
| `squeeze/value` | `/input/grip/value`, `/input/l1/value` | `/input/grip/value` (already correct) |
| `system/click` | `/input/system/click`, … | `/input/system/click` |

Ids absent from the tracker table create no component, which is right: a hand has no thumbstick.
Both `RegisterButton` and `SetButton` needed the parallel table, or the update would look up handles
the device never registered.

The mapping targets were checked against the profile SteamVR actually ships, not assumed:
`SteamVR/drivers/vrlink/resources/input/svl_hand_interaction_augmented_input_profile.json` declares
`index_pinch` and `grip` as `trigger` sources with `value`, and `system` with `click`.
`index_point` declares only `touch` and ALVR has no source for it, so it stays unmapped rather than
faked. Note the devices are not merely *compatible* with SteamVR's hand tracking — `props.rs:610`
impersonates it outright, serial `VRLINKQ_Hand_Left` and tracking system `vrlink`, so they enumerate
as VRLink hand trackers.

### Building and deploying it

The deployed DLL is a **`distribution`** build, not `release` — the profile adds LTO and the two
differ by more than a megabyte. Rebuild the profile that is actually deployed:

```sh
cargo build -p alvr_server_openvr --profile distribution
# with SteamVR closed (never force-kill vrserver; CloseMainWindow on vrmonitor, then wait
# for vrserver to exit, which takes about 30 s):
cp target/distribution/alvr_server_openvr.dll \
   build/alvr_streamer_windows/bin/win64/driver_alvr_server.dll
```

`build/alvr_streamer_windows/bin/win64/driver_alvr_server.dll.orig` is the stock build to revert to;
`git checkout alvr/server_openvr/cpp` plus a rebuild is the other direction. This deliberately avoids
`cargo xtask package-streamer`, which regenerates `session.json` with defaults and loses client trust
and every setting above.

**The deployed DLL also carries the `PoseHistory` tie-break fix** (`HANDOVER.md`, "the judder",
item 3). That one is not optional: without it the video freezes whenever the head holds still.
Rebuilding from a tree that lacks either change silently removes it — verify with
`GET /api/state`'s `frame_timing` and the server's `GraphStatistics` before calling a deploy done.

## Dead ends and near-misses

Recorded because each cost real time or looked convincing.

- **`steamvr_input_2_0 = false` "fixes" clicking.** It does, and it is the wrong trade — it brings
  the entire controller presentation with it (see the table above). Tried and reverted.
- **`Received button not mapped: .../trigger/click` is a red herring.** `get_click_bind_for_gesture`
  emits click ids absent from `HAND_GESTURE_BUTTON_SET`, so the log is real — but
  `map_button_pair_automatic` derives a destination click from a source *value* through a threshold
  whenever the source has no click of its own, so the click still arrives by that route. Reading the
  mapping code rather than trusting the log found the real cause one layer down.
- **Do not "fix" the 45° ray in the emulator.** It is the server's offset doing exactly what it was
  designed to do; compensating here would make the emulator differ from real hardware.
- **Pinch, not an air tap.** Pinch is the select gesture on Quest and in `XR_EXT_hand_interaction`.
  The air tap that flexes the index down and up is HoloLens'.
- **The inferred controller-held hand pose is a separate thing.** `Controller::SetButton`
  accumulates `m_currentThumbTouch` / `m_triggerValue` / `m_gripValue` to fake finger curl for a
  hand *holding* a controller (there is a `todo: move to rust` above it). Presentational only;
  bindings read the buttons.
- **The shaking wrist.** An OpenXR application's `XR_HAND_JOINT_WRIST_EXT` snapped between a fixed
  set of orientations up to 180° apart, constantly and worst while moving, with the palm and the
  fingers steady and every location flag valid. The trigger is the middle metacarpal base lying
  *exactly* on the wrist's Z axis: in the wrist's frame it sits at (0, 0, -d), with x and y down to
  float rounding, and whatever SteamVR's OpenXR runtime derives the wrist's orientation from has no
  stable answer there, so rounding of about 1e-7 picks a different one each frame. That is precisely
  the frame the spec describes for the wrist, which is why a careful emulator walks into it and a
  tracked hand never does. Pinned by three builds: wrist on the palm's axis and oriented like the
  palm (the original layout) shook; the WebXR wrist position, oriented like the palm (≈6° off the
  axis), was steady; the same position turned to face the metacarpal as the spec words it shook
  again (the first two measured in the app, the third by eye). We have not seen SteamVR's
  computation, only where it fails. What did *not* explain it, each checked: the application (it
  reads exactly what the runtime returns), the `/pose/tip` rotation (the old driver shook the same),
  the wrist bone sitting at a 180° quaternion (moved off it through the rotation offset; no change),
  and the arrangement of the other metacarpal bases (the third build had them arched, and shook).
  Noise also hides it (0.03 mm of random per-joint noise was enough), which is why a real headset
  never shows it, but noise would only mask the alignment, not remove it.

## How to verify

- **Geometry:** `cargo test -p alvr_client_emulator` — 26 tests, including the palm/wrist spec
  conformance, mirroring, and that Click's midpoint lands inside the recogniser's pinch distance.
- **What SteamVR actually sees:** pyopenvr is installed. Read
  `getDeviceToAbsoluteTrackingPose` and compare device orientation against the head to isolate the
  driver's offsets; devices 1–2 are the controllers and 3–4 the hand trackers. Enabling both hands
  should invalidate 1–2 and validate 3–4.
- **Server-side gestures:** set `extra.logging.log_button_presses`, then read the events WebSocket
  at `ws://127.0.0.1:8082/api/events` (a ~50-line raw-socket client is enough; no library needed).
- **Settings:** `POST /api/session/values` with `[{"path":[{"Name":"session_settings"},…],"value":…}]`
  changes one value without rewriting `session.json`.
- **The UI:** the capturing process must be DPI-aware or `PrintWindow` silently crops to the
  top-left. Use `GetWindowRect`, not `GetClientRect`, or the content is offset by the title bar.
  The window sometimes starts minimised; restore it by handle before capturing.

## Open items

1. **Upstream the two driver changes** — both are real gaps rather than emulator accommodations.
   The input mapping is a gap for any hand-tracked headset; the `PoseHistory` tie-break costs a real
   user a 1.67-second-stale pose whenever they hold still. Neither is committed.
2. **Aim and grip need separate poses.** Aim is now `/pose/tip`. The grip is still the offset
   device pose; publishing the profile's `/pose/grip` at the spec's grip (`interaction.rs`) would
   free the offsets from doing that job.
3. **`middle/ring/pinky_pinch` are unpublished.** The gesture recogniser binds those to face-button
   *clicks* (binary) while the profile wants scalar values, so they were left out of the mapping.
4. **Velocities are zero**, as for the controllers; see `HANDOVER.md` for why that is deliberate.
5. **Hand models load once**, when a hand's model is first shown; changing `hands.json` to point
   elsewhere needs a restart.
