# ALVR Project Improvements - Implementation Guide

This document provides comprehensive guidance for implementing critical bug fixes and high-value features for the ALVR project.

## Executive Summary

This guide covers the implementation of:
1. **Critical Bug Fixes** (3 issues)
2. **High-Impact Features** (4 features)
3. **Medium-Priority Improvements** (3 fixes)

---

## PART 1: CRITICAL BUG FIXES

### Bug Fix #1: SteamVR Crash on Exit (#3322) ✅ ALREADY FIXED

**Status**: MERGED in commit `e9b8e3ac`  
**Location**: [alvr/server_openvr/src/lib.rs](alvr/server_openvr/src/lib.rs#L682-L692)

```rust
pub extern "C" fn shutdown_driver() {
    SERVER_CORE_CONTEXT.write().take();

    // join driver threads so the dll isn't unloaded while they're still running
    if let Some(handle) = EVENT_LOOP_HANDLE.lock().take() {
        handle.join().ok();
    }
    if let Some(handle) = IDLE_INIT_HANDLE.lock().take() {
        handle.join().ok();
    }
}
```

**What it fixes**: Prevents SteamVR crashes during driver unload by properly joining background threads before DLL unmapping.

---

### Bug Fix #2: SteamVR 2.16.7 ExpectWirelessHeadset Race Condition (#3320)

**Problem**: SteamVR reads `ExpectWirelessHeadset` property before ALVR driver sets it, causing SteamVR to treat wireless headset as wired.

**Root Cause**: Race condition between device activation and property initialization.

**Solution**: Ensure all critical OpenVR properties are set BEFORE device activation.

**Implementation Steps**:

1. **File**: [alvr/server_core/src/connection.rs](alvr/server_core/src/connection.rs)
   - Add synchronous property initialization during device setup
   - Move all property setting BEFORE device is reported as active
   - Ensure `ExpectWirelessHeadset` is set to `true` for wireless headsets immediately after device creation

2. **File**: [alvr/server_openvr/src/props.rs](alvr/server_openvr/src/props.rs)
   - Add property validation function to ensure critical properties are set
   - Create a `initialize_critical_properties()` function that runs synchronously

3. **Code Location**: In the device initialization flow (likely in OpenVR C++ layer), synchronously set:
   ```
   ExpectWirelessHeadset = true
   DeviceIsWirelessBool = true  // Already set, verify it's before activation
   ```

4. **Testing**:
   - Connect Meta Quest 2 via ALVR
   - Verify `ExpectWirelessHeadset: 1` in vrmonitor.txt immediately after device activation
   - Confirm SteamVR does NOT transition to Restart→Shutdown sequence

---

### Bug Fix #3: Dual Audio Output - Headset AND Laptop Speaker (#3325)

**Problem**: Audio plays from BOTH the headset (USB/wired) AND the laptop speaker simultaneously.

**Root Cause**: When `mute_when_streaming` is enabled, the muting logic may not work correctly for all device configurations, or audio is being duplicated at the system level.

**Solution**: Implement proper audio device isolation and fallback logic.

**Implementation Steps**:

1. **File**: [alvr/audio/src/windows.rs](alvr/audio/src/windows.rs)
   
   **Add function to validate audio device selection**:
   ```rust
   pub fn validate_audio_device_isolation(
       alvr_device: &Device,
       system_default: &Device,
   ) -> Result<()> {
       // Ensure ALVR device is different from system default
       if is_same_device(alvr_device, system_default) {
           bail!("ALVR audio device cannot be the same as system default device")
       }
       Ok(())
   }
   ```

2. **File**: [alvr/server_core/src/connection.rs](alvr/server_core/src/connection.rs)
   
   **Improve audio device selection logic** (lines 894-960):
   - Add explicit check that `mute_when_streaming` only mutes when audio is ACTIVELY being recorded
   - Add validation to ensure audio device is NOT the default speakers
   - Implement fallback if no custom audio device is specified: use the FIRST non-default output device

   **Code change**:
   ```rust
   let game_audio_thread = if let Switch::Enabled(config) = 
       initial_settings.audio.game_audio.clone() 
   {
       let device = match alvr_audio::new_output(config.device.as_ref()) {
           Ok(d) => d,
           Err(e) => {
               // FALLBACK: if no device specified, find first ALVR/VB-Audio device
               if config.device.is_none() {
                   if let Ok(d) = find_virtual_audio_device() {
                       d
                   } else {
                       bail!("Could not find ALVR audio device")
                   }
               } else {
                   bail!("Audio device configuration failed: {e}")
               }
           }
       };

       // VALIDATION: ensure device is isolated
       if let Ok(system_default) = alvr_audio::new_output(None) {
           alvr_audio::validate_audio_device_isolation(&device, &system_default)?;
       }
       // ... rest of thread logic
   }
   ```

3. **Add helper function to find virtual audio device**:
   ```rust
   fn find_virtual_audio_device() -> Result<Device> {
       use cpal::traits::HostTrait;
       let host = cpal::default_host();
       
       // Look for common ALVR virtual audio device names
       let virtual_names = vec!["ALVR", "VB-Cable", "VoiceMeeter", "Loopback"];
       
       for name in virtual_names {
           if let Ok(device) = host.output_devices()?
               .find(|d| d.name()
                   .map(|n| n.to_lowercase().contains(&name.to_lowercase()))
                   .unwrap_or(false))
           {
               return Ok(device);
           }
       }
       
       bail!("No virtual audio device found")
   }
   ```

4. **Testing**:
   - Connect wired Quest 2
   - Enable "Mute desktop audio when streaming" in settings
   - Launch game via ALVR
   - Verify audio ONLY comes from headset, NOT laptop speaker
   - Verify system audio is restored after ALVR disconnects

---

## PART 2: HIGH-IMPACT FEATURES

### Feature #1: ARM64 Linux Build Support

**Target**: Add `aarch64-unknown-linux-gnu` build target (for Raspberry Pi, NVIDIA Jetson, other ARM64 boards)

**Locations**:
- [Cargo.toml](Cargo.toml) - workspace root
- [alvr/xtask/src/build.rs](alvr/xtask/src/build.rs) - build configuration
- [alvr/xtask/src/dependencies.rs](alvr/xtask/src/dependencies.rs) - dependency cross-compilation

**Implementation Steps**:

1. **Add target to Cargo workspace** - [Cargo.toml](Cargo.toml)
   ```toml
   [profile.release]
   lto = "thin"
   codegen-units = 1
   # ... existing config
   
   # Add ARM64 specific section if needed
   ```

2. **Add ARM64 build profile** - [alvr/xtask/src/build.rs](alvr/xtask/src/build.rs)
   ```rust
   pub fn build_for_arm64_linux() {
       let target = "aarch64-unknown-linux-gnu";
       
       // Verify cross-compiler is installed
       // rustup target add aarch64-unknown-linux-gnu
       
       // Build with proper flags
       std::process::Command::new("cargo")
           .args(&[
               "build",
               "--release",
               "--target", target,
               "--package", "alvr_server_core",
           ])
           .output()
           .expect("Failed to build for ARM64");
   }
   ```

3. **Add dependency cross-compilation** - [alvr/xtask/src/dependencies.rs](alvr/xtask/src/dependencies.rs)
   - Download FFmpeg ARM64 binaries or build from source
   - Download x264 ARM64 libraries
   - Handle Vulkan/OpenGL ARM64 headers

4. **Docker build support** - [docker/Dockerfile.linux-build](docker/Dockerfile.linux-build)
   ```dockerfile
   # Add ARM64 build stage
   FROM arm64v8/ubuntu:24.04 AS build-arm64
   RUN apt-get install -y \
       build-essential \
       pkg-config \
       libavformat-dev \
       libavcodec-dev \
       # ... other dependencies
   ```

5. **Testing**:
   - Cross-compile for ARM64 Linux target
   - Deploy to Raspberry Pi 5 or NVIDIA Jetson
   - Verify ALVR server starts and accepts client connections

---

### Feature #2: Software Black Frame Insertion

**Purpose**: Reduce motion blur and flicker by inserting black frames between rendered frames.

**Locations**:
- [alvr/graphics/src/stream.rs](alvr/graphics/src/stream.rs) - video encoding
- [alvr/graphics/src/staging.rs](alvr/graphics/src/staging.rs) - frame pipeline
- [alvr/session/src/settings.rs](alvr/session/src/settings.rs) - add setting

**Implementation Steps**:

1. **Add setting** - [alvr/session/src/settings.rs](alvr/session/src/settings.rs)
   ```rust
   #[derive(SettingsSchema, Serialize, Deserialize, Clone)]
   pub struct PostProcessingConfig {
       #[schema(strings(display_name = "Black Frame Insertion"))]
       #[schema(flag = "real-time")]
       pub enable_black_frame_insertion: bool,
   }
   ```

2. **Implement frame insertion** - [alvr/graphics/src/stream.rs](alvr/graphics/src/stream.rs)
   ```rust
   pub fn insert_black_frames(
       frame: &TextureFrame,
       enable: bool,
   ) -> Vec<TextureFrame> {
       if !enable {
           return vec![frame.clone()];
       }
       
       vec![
           frame.clone(),                    // Original frame
           TextureFrame::black(frame.dims)   // Black frame (50ms duration)
       ]
   }
   ```

3. **Integrate into encoding pipeline**:
   - Before encoder receives frame, check if BFI is enabled
   - If enabled, send both original and black frame
   - Adjust frame timing to ensure 120Hz input → 240Hz output

4. **Testing**:
   - Enable Black Frame Insertion in settings
   - Launch game
   - Verify reduced motion blur and flicker
   - Monitor latency impact (should be minimal)

---

### Feature #3: FPS Limiter / Frame Rate Control

**Purpose**: Allow users to cap framerate to reduce latency and improve latency stability.

**Locations**:
- [alvr/session/src/settings.rs](alvr/session/src/settings.rs) - add FPS setting
- [alvr/server_core/src/bitrate.rs](alvr/server_core/src/bitrate.rs) - implement limiter
- [alvr/graphics/src/stream.rs](alvr/graphics/src/stream.rs) - apply timing

**Implementation Steps**:

1. **Add FPS setting** - [alvr/session/src/settings.rs](alvr/session/src/settings.rs)
   ```rust
   pub struct BitrateConfig {
       // ... existing fields
       
       #[schema(strings(display_name = "Maximum FPS"))]
       #[schema(flag = "real-time")]
       #[schema(gui(slider(min = 30, max = 120, step = 1)), suffix = "fps")]
       pub max_fps_limiter: Option<u32>,  // None = unlimited
   }
   ```

2. **Implement FPS limiter** - [alvr/server_core/src/bitrate.rs](alvr/server_core/src/bitrate.rs)
   ```rust
   pub struct FrameRateLimiter {
       max_fps: u32,
       last_frame_time: Instant,
   }

   impl FrameRateLimiter {
       pub fn new(max_fps: u32) -> Self {
           Self {
               max_fps,
               last_frame_time: Instant::now(),
           }
       }

       pub fn should_drop_frame(&mut self) -> bool {
           let elapsed = self.last_frame_time.elapsed();
           let min_interval = Duration::from_secs_f32(1.0 / self.max_fps as f32);
           
           if elapsed < min_interval {
               return true;
           }
           
           self.last_frame_time = Instant::now();
           false
       }
   }
   ```

3. **Apply in encoding pipeline**:
   - Before encoding each frame, check FPS limiter
   - If FPS limit reached, skip frame
   - Update bitrate calculation to account for reduced frame count

4. **Testing**:
   - Set FPS limit to 60fps
   - Launch game
   - Verify framerate stays at ~60fps (check logs)
   - Measure latency reduction

---

### Feature #4: Auto-Firewall Configuration (Linux)

**Purpose**: Automatically configure UFW/firewalld on Linux to allow ALVR traffic.

**Location**: [alvr/server_io/src/firewall.rs](alvr/server_io/src/firewall.rs)

**Implementation Steps**:

1. **Detect firewall type**:
   ```rust
   pub enum FirewallType {
       UFW,
       Firewalld,
       None,
   }

   pub fn detect_firewall() -> FirewallType {
       // Check if ufw is active: sudo ufw status | grep active
       // Check if firewalld is active: sudo firewall-cmd --state
   }
   ```

2. **Implement UFW rules**:
   ```rust
   pub fn configure_ufw_rules(enable: bool) -> Result<()> {
       let alvr_port = 9943; // ALVR default port
       
       if enable {
           Command::new("sudo")
               .args(&["ufw", "allow", &format!("{}/tcp", alvr_port)])
               .output()?;
       } else {
           Command::new("sudo")
               .args(&["ufw", "delete", "allow", &format!("{}/tcp", alvr_port)])
               .output()?;
       }
       Ok(())
   }
   ```

3. **Implement firewalld rules**:
   ```rust
   pub fn configure_firewalld_rules(enable: bool) -> Result<()> {
       let cmd = if enable { "add-port" } else { "remove-port" };
       
       Command::new("sudo")
           .args(&["firewall-cmd", "--permanent", &format!("--{}", cmd), "9943/tcp"])
           .output()?;
       
       Command::new("sudo")
           .args(&["firewall-cmd", "--reload"])
           .output()?;
       
       Ok(())
   }
   ```

4. **Add to settings**:
   - Add option to dashboard: "Automatically configure firewall"
   - Default: enabled for Linux, shows password prompt if needed

---

## PART 3: MEDIUM-PRIORITY IMPROVEMENTS

### Improvement #1: Hand Tracking Settings Button Fix (#3336)

**Problem**: Settings (Steam) button doesn't work when hand tracking is enabled.

**Locations**:
- [alvr/server_openvr/src/tracking.rs](alvr/server_openvr/src/tracking.rs)
- [alvr/common/src/lib.rs](alvr/common/src/lib.rs) - button definitions

**Implementation**:
- Find button mapping for hand tracked controllers
- Ensure Settings button (menu button) is properly mapped
- Test with hand tracking enabled

---

### Improvement #2: Windows 11 Uninstall Device Cleanup (#3340)

**Problem**: Uninstalling ALVR on Windows 11 leaves orphaned VR device drivers.

**Implementation**:
- Create uninstall script to clean up registry entries
- Remove virtual audio device registrations
- Clean up OpenVR driver registry

---

### Improvement #3: Linux Firewall Rules Automation (#3302)

**Problem**: ALVR can't set firewall rules automatically on Linux.

**Implementation**:
- See Feature #4 above for full implementation

---

## IMPLEMENTATION PRIORITY & EFFORT

### Quick Wins (< 2 hours each)
1. ✅ Bug #1: Already fixed
2. 🔧 Bug #3: Audio dual output (1-2 hours)
3. 🎯 Improvement #1: Hand tracking button (1 hour)

### Medium Effort (2-4 hours each)
1. 🔨 Bug #2: ExpectWirelessHeadset race (3-4 hours)
2. 💾 Feature #3: FPS limiter (2-3 hours)
3. 🖨️ Feature #4: Auto-firewall (2 hours)

### High Effort (4+ hours each)
1. 🏗️ Feature #1: ARM64 build support (4-6 hours)
2. 🎨 Feature #2: Black frame insertion (6-8 hours)

---

## TESTING CHECKLIST

### Critical Path
- [ ] SteamVR connects without errors
- [ ] Audio plays only from headset, not laptop
- [ ] SteamVR doesn't crash on disconnect
- [ ] Game frame rate is stable

### Feature Verification
- [ ] FPS limiter caps frame rate correctly
- [ ] Black frame insertion reduces blur
- [ ] Firewall rules are applied automatically
- [ ] ARM64 build completes successfully
- [ ] Hand tracking settings button works

---

## DEPLOYMENT

### Phase 1: Bug Fixes (Priority)
1. Fix audio dual output
2. Fix ExpectWirelessHeadset race
3. Merge fixes to master

### Phase 2: Features (Next Release)
1. FPS limiter
2. Auto-firewall configuration
3. Black frame insertion
4. ARM64 Linux support
5. Hand tracking button fix

### Release Strategy
- Branch: `feature/alvr-improvements-audio-fps-arm64`
- Create PR with detailed testing results
- Merge to master after review and testing
- Tag release version

---

## References

- ALVR GitHub: https://github.com/alvr-org/ALVR
- Issue #3322: Steam/SteamVR crash on driver unload
- Issue #3320: SteamVR 2.16.7 fails to initialize
- Issue #3325: Audio playing from both headset and laptop
- Issue #3336: Hand tracking settings button
- Issue #3340: Windows 11 uninstall device cleanup
- Issue #3302: Linux firewall rules

