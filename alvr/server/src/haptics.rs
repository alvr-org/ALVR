use alvr_packets::Haptics;
use alvr_session::HapticsConfig;
use std::time::Duration;

pub fn map_haptics(config: &HapticsConfig, haptics: Haptics) -> Haptics {
    Haptics {
        duration: Duration::max(
            haptics.duration,
            Duration::from_secs_f32(config.min_duration_s),
        ),
        amplitude: config.intensity_multiplier
            * f32::powf(haptics.amplitude, config.amplitude_curve),
        ..haptics
    }
}
