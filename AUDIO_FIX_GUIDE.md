# Audio Dual Output Fix - Implementation Guide

## Problem Statement
When ALVR is connected via wired USB (Quest 2 wired), audio plays from BOTH the headset AND the laptop speakers simultaneously. This is issue #3325.

## Root Causes
1. `mute_when_streaming` setting may not work reliably on wired connections
2. Audio device selection may default to system speakers instead of isolated ALVR device
3. System audio routing isn't properly isolated when ALVR captures game audio

## Solution Architecture

### Current Code Flow
```
Settings (AudioConfig) 
    ↓
server_core/connection.rs (game_audio_thread)
    ↓
audio/lib.rs (record_audio_blocking)
    ↓
audio/windows.rs (set_mute_windows_device)
```

### Key Issues in Current Implementation

**File**: `alvr/server_core/src/connection.rs` (lines 894-960)

**Issue 1**: Audio device is selected for recording (loopback), but it may NOT be isolated from system speakers:
```rust
let device = match alvr_audio::new_output(config.device.as_ref()) {
    Ok(data) => data,
    Err(e) => {
        // Fallback to default - THIS CAN BE SYSTEM SPEAKERS!
        warn!("New audio device failed: {e:?}");
        continue;
    }
};
```

**Issue 2**: After muting, the device is reset to default, but the default might NOT be the original speaker:
```rust
if let Ok(id) = alvr_audio::new_output(None)
    .and_then(|d| alvr_audio::get_windows_device_id(&d))
{
    // Resetting to None = system default, but what if default is speakers?
```

### Solutions

## Solution 1: Validate Audio Device Before Use (HIGH PRIORITY)

**File**: `alvr/audio/src/windows.rs`

Add validation function:
```rust
/// Check if the device is a valid ALVR/virtual audio device, not system speakers
pub fn is_virtual_audio_device(device: &Device) -> Result<bool> {
    let name = device.name()?;
    
    // List of known virtual audio device names
    let virtual_devices = [
        "ALVR",
        "VB-Cable",
        "VoiceMeeter",
        "Loopback",
        "Stereo Mix",
        "What U Hear",
        "Wave Out Mix",
    ];
    
    Ok(virtual_devices.iter().any(|vd| 
        name.to_lowercase().contains(&vd.to_lowercase())
    ))
}

/// Find a suitable virtual audio device if custom one isn't configured
pub fn find_virtual_audio_device() -> Result<Device> {
    let host = cpal::default_host();
    
    let mut devices: Vec<_> = host.output_devices()?
        .filter(|d| is_virtual_audio_device(d).unwrap_or(false))
        .collect();
    
    // Sort by name to get consistent selection
    devices.sort_by(|a, b| {
        a.name().unwrap_or_default()
            .cmp(&b.name().unwrap_or_default())
    });
    
    devices.into_iter().next()
        .ok_or_else(|| anyhow::anyhow!(
            "No virtual audio device found. Please install VB-Cable or similar."
        ))
}
```

## Solution 2: Improve Audio Device Selection (MEDIUM PRIORITY)

**File**: `alvr/server_core/src/connection.rs`

Replace the audio device selection logic with better fallback:
```rust
let game_audio_thread = if let Switch::Enabled(config) = 
    initial_settings.audio.game_audio.clone() 
{
    #[cfg(not(target_os = "linux"))]
    {
        // Attempt to get configured device
        let device = if let Some(custom_config) = &config.device {
            match alvr_audio::new_output(Some(custom_config)) {
                Ok(d) => d,
                Err(e) => {
                    warn!("Could not open configured audio device: {e:?}");
                    warn!("Attempting to find virtual audio device automatically...");
                    
                    // Fallback: try to find a virtual device
                    match alvr_audio::find_virtual_audio_device() {
                        Ok(d) => {
                            info!("Found virtual audio device: {}", 
                                  d.name().unwrap_or("Unknown"));
                            d
                        }
                        Err(_) => {
                            error!("No audio device available. Audio streaming disabled.");
                            continue;
                        }
                    }
                }
            }
        } else {
            // No custom device specified, find virtual device
            match alvr_audio::find_virtual_audio_device() {
                Ok(d) => {
                    info!("Using virtual audio device: {}", 
                          d.name().unwrap_or("Unknown"));
                    d
                }
                Err(e) => {
                    error!("No audio device found: {e:?}");
                    warn!("To fix this, install VB-Cable or similar virtual audio device");
                    continue;
                }
            }
        };
        
        // Validate device is virtual (not system speakers)
        if let Ok(false) = alvr_audio::is_virtual_audio_device(&device) {
            error!("Selected device is not a virtual audio device!");
            warn!("Audio will play from both headset and speakers.");
            warn!("Please configure a virtual audio device like VB-Cable.");
        }
        
        // ... rest of streaming logic
    }
}
```

## Solution 3: Improve Mute Logic (MEDIUM PRIORITY)

**File**: `alvr/audio/src/lib.rs`

Enhance the mute logic to be more robust:
```rust
pub fn record_audio_blocking(
    is_running: Arc<dyn Fn() -> bool + Send + Sync>,
    mut sender: StreamSender<()>,
    device: &Device,
    channels_count: u16,
    mute: bool,
) -> Result<()> {
    // ... existing config setup ...
    
    let mute_success = if mute {
        #[cfg(windows)]
        {
            info!("Attempting to mute device: {}", 
                  device.name().unwrap_or("Unknown"));
            match crate::windows::set_mute_windows_device(device, true) {
                Ok(_) => {
                    info!("Device muted successfully");
                    true
                }
                Err(e) => {
                    warn!("Failed to mute device: {e:?}");
                    warn!("Audio may play from both headset and speakers");
                    false
                }
            }
        }
        #[cfg(not(windows))]
        false
    } else {
        false
    };
    
    // ... streaming logic ...
    
    // Always try to unmute, even if muting failed
    if mute {
        #[cfg(windows)]
        {
            if let Err(e) = crate::windows::set_mute_windows_device(device, false) {
                warn!("Failed to unmute device: {e:?}");
            }
        }
    }
    
    res
}
```

## Solution 4: Add Diagnostic Logging (LOW PRIORITY)

Add logging to help users troubleshoot:
```rust
pub fn log_audio_device_info(device: &Device) {
    if let Ok(name) = device.name() {
        info!("Audio Device: {}", name);
    }
    
    if let Ok(config) = device.default_input_config() {
        info!("  Input Config: {} channels, {} Hz", 
              config.channels(), config.sample_rate());
    }
    
    if let Ok(config) = device.default_output_config() {
        info!("  Output Config: {} channels, {} Hz", 
              config.channels(), config.sample_rate());
    }
    
    #[cfg(windows)]
    if let Ok(id) = crate::windows::get_windows_device_id(device) {
        info!("  Windows Device ID: {}", id);
    }
}
```

## Implementation Priority

### Phase 1 (Fix dual audio issue)
1. ✅ Add `is_virtual_audio_device()` function
2. ✅ Add `find_virtual_audio_device()` function  
3. ✅ Improve device selection in connection.rs
4. ✅ Add validation and error messages

### Phase 2 (Improve stability)
1. ⬜ Enhance mute/unmute logic
2. ⬜ Add diagnostic logging
3. ⬜ Add configuration validation

## Testing Plan

### Test Case 1: Virtual Audio Device Configured
- Setup: Configure VB-Cable as audio device in ALVR settings
- Expected: Audio plays ONLY from Quest 2, not from laptop speakers
- Verify: Check Windows Volume Mixer - only ALVR/VB-Cable showing activity

### Test Case 2: No Audio Device Configured
- Setup: Leave audio device as default
- Expected: ALVR auto-detects and uses VB-Cable if available
- Verify: Log shows "Found virtual audio device"

### Test Case 3: Mute When Streaming
- Setup: Enable "Mute desktop audio when streaming"
- Expected: System sounds muted, audio only from Quest
- Verify: Play system sound → verify it doesn't play during ALVR

### Test Case 4: Audio After Disconnect
- Setup: Disconnect ALVR, then play system sound
- Expected: System sound plays normally from laptop speakers
- Verify: Audio restored after disconnect

## Related Issues

- #3325: Audio playing out of both headset and laptop
- #3229: Audio coming from PC and not ALVR - Fixed
- Configuration in dashboard for audio device selection

## Environment Notes

- **Windows**: WASAPI loopback devices
- **Linux**: PipeWire/PulseAudio configuration
- **Android**: Not applicable (audio only via network)

