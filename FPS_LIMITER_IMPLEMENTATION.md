# FPS Limiter Implementation Guide

## Overview
This document describes how to implement the FPS limiter feature in ALVR.

## Setting Addition (DONE ✓)
- **File**: `alvr/session/src/settings.rs`
- **Location**: Added `max_framerate_limiter: u32` field to `BitrateConfig` struct
- **Default Value**: 0 (unlimited)
- **Range**: 0-120 fps

## Implementation Points

### 1. BitrateManager Integration
**File**: `alvr/server_core/src/bitrate.rs`

The `BitrateManager` needs to respect the FPS limiter setting:

```rust
impl BitrateManager {
    pub fn update(
        &mut self,
        config: &BitrateConfig,
        // ... other params
    ) {
        let max_fps = if config.max_framerate_limiter > 0 {
            config.max_framerate_limiter as f32
        } else {
            1000.0  // effectively unlimited
        };
        
        // Apply FPS limit to framerate calculations
        let limited_framerate = self.get_recommended_framerate().min(max_fps);
        
        // Continue with rest of update logic using limited_framerate
        // ...
    }
}
```

### 2. Video Encoder Integration
**File**: `alvr/graphics/src/stream.rs`

The graphics pipeline should skip frames if the FPS limit is exceeded:

```rust
pub fn encode_frame(
    frame: TextureFrame,
    fps_limiter: &mut FrameRateLimiter,
) -> Option<EncodedFrame> {
    if fps_limiter.should_skip_frame() {
        return None;  // Skip this frame
    }
    
    // Proceed with encoding
    encode_video_frame(frame)
}
```

### 3. Frame Rate Limiter Struct
This should be added to a new module or utility file:

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

    pub fn should_skip_frame(&mut self) -> bool {
        if self.max_fps == 0 {
            return false;  // Unlimited
        }
        
        let elapsed = self.last_frame_time.elapsed();
        let min_interval = Duration::from_secs_f32(1.0 / self.max_fps as f32);
        
        if elapsed < min_interval {
            return true;  // Skip this frame
        }
        
        self.last_frame_time = Instant::now();
        false
    }

    pub fn update_fps_limit(&mut self, new_max_fps: u32) {
        self.max_fps = new_max_fps;
    }
}
```

### 4. Integration Points

#### In server connection initialization
**File**: `alvr/server_core/src/connection.rs`

```rust
let fps_limiter = FrameRateLimiter::new(
    initial_settings.video.bitrate.max_framerate_limiter
);
```

#### Frame encoding loop
Check FPS limiter before encoding each frame and skip if needed.

### 5. Dashboard UI Integration
The setting will automatically appear in the dashboard UI due to the `SettingsSchema` derive macro.

## Testing Checklist

- [ ] Setting appears in dashboard UI
- [ ] Setting value persists across sessions
- [ ] FPS limiter limits framerate correctly (use logs to verify)
- [ ] Latency improves with lower FPS cap
- [ ] Bitrate calculation accounts for dropped frames
- [ ] No visual stuttering when FPS is limited

## Performance Impact

- **Positive**: Reduced latency, more stable framerate, lower CPU/GPU load
- **Negative**: Reduced visual smoothness if set too low

## Recommended Defaults

- Unlimited (0) for high-end PCs
- 60 fps for mid-range systems
- 45 fps for low-end systems
- 30 fps for wireless/unstable networks

## Related Settings

- Bitrate mode (adaptive vs. constant)
- Encoder quality preset
- Network bandwidth limits

