# ALVR Resolved Issues & Improvements Log

This document tracks issues resolved in this fork, their root causes, and implementation details.

---

## 1. Resolved Issues

### [Issue #3380](https://github.com/alvr-org/ALVR/issues/3380): OS Error 10048 (WSAEADDRINUSE) Panic on Port Conflict
- **Symptoms**: When port 8082 (Web Server / API) or UDP Discovery socket was occupied or when restarting ALVR quickly, the server panicked with `Only one usage of each socket address is normally permitted (os error 10048)`.
- **Root Cause**:
  - `alvr/server_core/src/web_server.rs` called `TcpListener::bind(...).await.unwrap()`, causing an unhandled panic.
  - `alvr/server_core/src/connection.rs` completely exited `handshake_loop` if `WelcomeSocket::new()` failed.
- **Resolution**:
  - Replaced `.unwrap()` in `web_server.rs` with graceful error logging and returning `Err` so `show_err` handles it cleanly.
  - Made `WelcomeSocket` optional in `connection.rs` with automatic lazy recreation and retries during the handshake loop, allowing wired/manual IP connections to remain functional even if mDNS temporarily fails.

---

### [Issue #3375](https://github.com/alvr-org/ALVR/issues/3375): ALVR Launcher Can't Download Latest Nightly
- **Symptoms**: ALVR Launcher showed months-old nightly versions (e.g. `v21.0.0-dev13+nightly.2026.06.06`) instead of the true latest nightly (e.g. `v21.0.0-dev12+nightly.2026.08.23`).
- **Root Cause**:
  - `alvr/launcher/src/actions.rs` did not sort GitHub API `/releases` responses, relying on API response order or naive string comparison where `dev13` superseded `dev12` despite `dev12` having a newer nightly build date.
- **Resolution**:
  - Added timestamp-based sorting (`published_at` / `created_at` descending) to `fetch_releases_for_repo`, guaranteeing that the newest release is always indexed first regardless of dev tag number anomalies.

---

### [Issue #2368](https://github.com/alvr-org/ALVR/issues/2368): VRChat Thumb Tracking & Button Sensor Ambiguity
- **Symptoms**: In VRChat with locked gestures, thumb movement was jittery or unresponsive. Touching B/Y button produced the identical avatar pose as touching the thumbstick or A/X button, and releasing one button often turned off the thumb touch state entirely.
- **Root Cause**:
  - In `alvr/server_openvr/cpp/alvr_server/Controller.cpp`, `m_currentThumbTouch` was a single boolean shared across all buttons (A, B, X, Y, Thumbstick, Thumbrest, Trackpad). Releasing one button sent `value.binary = 0`, immediately overwriting `m_currentThumbTouch` to false even when thumbstick was still touched.
  - No distinction was made in skeletal transforms between button touches and thumbstick touches.
- **Resolution**:
  - Added individual per-sensor touch states (`m_thumbstickTouch`, `m_button1Touch`, `m_button2Touch`, `m_trackpadTouch`, `m_thumbrestTouch`) in `Controller.h`.
  - Updated `Controller::SetButton` so `m_currentThumbTouch` is computed as a combined logical OR, preventing release events on one sensor from clearing other touches.
  - Implemented differentiated thumb skeletal bone offsets when touching buttons vs thumbstick vs thumbrest, enabling VRChat models to accurately reflect thumb placement.

---

### [Issue #3365](https://github.com/alvr-org/ALVR/issues/3365) & [#3373](https://github.com/alvr-org/ALVR/issues/3373): Microphone / Audio Handshake Errors & Audio Routing
- **Symptoms**: Enabling microphone in ALVR caused handshake connection failure if virtual audio devices (VB-CABLE, VoiceMeeter) were missing or unconfigured. Game audio in VRChat was silent or misrouted.
- **Root Cause**:
  - `alvr/server_core/src/connection.rs` aborted the entire VR streaming connection pipeline with `to_con()?` if `new_virtual_microphone_pair` failed to locate a virtual audio device.
- **Resolution**:
  - Changed microphone thread initialization to catch device errors gracefully, logging a clear warning while allowing video streaming and game audio to continue without disruption.

---

### [Issue #3340](https://github.com/alvr-org/ALVR/issues/3340): Driver Persistence After Uninstall on Windows 11
- **Symptoms**: Uninstalling ALVR or removing drivers left orphaned driver entries in SteamVR's `openvrpaths.vrpath`, causing SteamVR initialization errors on subsequent boots.
- **Root Cause**:
  - In `alvr/server_io/src/openvr_drivers.rs`, `paths.remove(path)` performed exact, case-sensitive `PathBuf` matching, which failed on Windows when paths differed in casing or slash styles (`/` vs `\`).
- **Resolution**:
  - Added normalized path canonicalization and case-insensitive string comparison for Windows driver registration removal.

---

## 2. Roadmap for Additional Issues

1. **NVENC / Vulkan Layer Compatibility ([#3393](https://github.com/alvr-org/ALVR/issues/3393), [#3386](https://github.com/alvr-org/ALVR/issues/3386))**:
   - Tracking upstream driver fixes and adding fallback software/VPL layers for problematic driver revisions.
2. **Quest Wired USB API Streaming ([#3389](https://github.com/alvr-org/ALVR/issues/3389))**:
   - Integrating Meta Quest USB streaming protocol alongside standard ADB reverse forwarding.
3. **v21.0.0 Protocol Finalization ([#1722](https://github.com/alvr-org/ALVR/issues/1722))**:
   - Packet protocol unification and migration to `bincode 2.0`.
