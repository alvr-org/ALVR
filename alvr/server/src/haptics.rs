use alvr_session::HapticsConfig;
use alvr_sockets::Haptics;
use std::time::Duration;

// todo: explain the algorithm (for @sctanf)
pub struct HapticsManager {
    d_min: f32,
    a_multiplier: f32,
    a_alpha_s: f32,
    a_beta_s2: f32,
    a_gamma: f32,
    a_delta_s: f32,
    d_alpha_s2: f32,
}

impl HapticsManager {
    pub fn new(config: HapticsConfig) -> Self {
        Self {
            d_min: config.min_duration_s * 0.5,
            a_multiplier: config.intensity_multiplier,
            a_alpha_s: (config.low_duration_amplitude_multiplier - 1.0)
                * config.min_duration_s
                * config.low_duration_range_multiplier,
            a_beta_s2: f32::powi(
                config.min_duration_s * config.low_duration_range_multiplier,
                2,
            ) * 0.25,
            a_gamma: 1.0 - config.amplitude_curve,
            a_delta_s: 0.5 * config.min_duration_s * (1.0 - config.low_duration_range_multiplier),
            d_alpha_s2: f32::powi(config.min_duration_s, 2) * 0.25,
        }
    }

    pub fn map(&self, haptics: Haptics) -> Haptics {
        let duration_s = f32::max(haptics.duration.as_secs_f32(), self.d_min);

        let amplitude = self.a_multiplier
            * f32::powf(
                haptics.amplitude
                    * (1.0
                        + self.a_alpha_s
                            / (self.a_beta_s2 / (duration_s - self.a_delta_s)
                                + (duration_s - self.a_delta_s))),
                self.a_gamma,
            );

        let duration_s = self.d_alpha_s2 / duration_s + duration_s;

        Haptics {
            duration: Duration::from_secs_f32(duration_s),
            amplitude,
            ..haptics
        }
    }
}
