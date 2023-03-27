use alvr_session::HapticsConfig;
use alvr_sockets::Haptics;
use std::time::Duration;

pub struct HapticsManager {
    config: HapticsConfig,
}

impl HapticsManager {
    pub fn new(config: HapticsConfig) -> Self {
        Self { config }
    }

    pub fn map(&self, haptics: Haptics) -> Haptics {
        Haptics {
            duration: Duration::max(
                haptics.duration,
                Duration::from_secs_f32(self.config.min_duration_s),
            ),
            amplitude: self.config.intensity_multiplier
                * f32::powf(haptics.amplitude, self.config.amplitude_curve),
            ..haptics
        }
    }
}
