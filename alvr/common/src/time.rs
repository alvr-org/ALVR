use std::collections::VecDeque;

const WINDOW_SIZE: usize = 16;
const EMA_ALPHA: f64 = 0.1;
const SKEW_ALPHA: f64 = 0.05;
const MAX_SKEW_PPM: f64 = 200.0 * 1e-6; // Clamp to +/- 200 ppm
const STEP_THRESHOLD_NS: f64 = 250_000_000.0; // 250ms threshold to trigger a hard baseline reset

pub fn monotonic_ns() -> u64 {
    #[cfg(any(target_os = "linux", target_os = "android"))]
    {
        let mut ts = libc::timespec {
            tv_sec: 0,
            tv_nsec: 0,
        };
        unsafe { libc::clock_gettime(libc::CLOCK_BOOTTIME, &mut ts) };
        (ts.tv_sec as u64) * 1_000_000_000 + (ts.tv_nsec as u64)
    }
    #[cfg(all(unix, not(any(target_os = "linux", target_os = "android"))))]
    {
        let mut ts = libc::timespec {
            tv_sec: 0,
            tv_nsec: 0,
        };
        unsafe { libc::clock_gettime(libc::CLOCK_MONOTONIC, &mut ts) };
        (ts.tv_sec as u64) * 1_000_000_000 + (ts.tv_nsec as u64)
    }
    #[cfg(windows)]
    {
        // On Windows: QueryPerformanceCounter
        unsafe extern "system" {
            fn QueryPerformanceCounter(lpPerformanceCount: *mut i64) -> i32;
            fn QueryPerformanceFrequency(lpFrequency: *mut i64) -> i32;
        }
        static FREQ: std::sync::LazyLock<i64> = std::sync::LazyLock::new(|| {
            let mut freq = 0;
            unsafe { QueryPerformanceFrequency(&mut freq) };
            freq
        });
        let mut count = 0;
        unsafe { QueryPerformanceCounter(&mut count) };
        ((count as u128 * 1_000_000_000) / (*FREQ as u128)) as u64
    }
}

struct Sample {
    rtt_ns: i128,
    offset_ns: i64,
    client_time_ns: u64,
}
pub struct TimeSyncEstimator {
    samples: VecDeque<Sample>,
    base_client_time_ns: u64,
    base_offset_ns: Option<f64>,
    skew: f64, // ppm ratio
    last_skew_client_time_ns: u64,
    last_skew_offset_ns: f64,
}

impl Default for TimeSyncEstimator {
    fn default() -> Self {
        Self::new()
    }
}

impl TimeSyncEstimator {
    pub fn new() -> Self {
        Self {
            samples: VecDeque::with_capacity(WINDOW_SIZE),
            base_client_time_ns: 0,
            base_offset_ns: None,
            skew: 0.0,
            last_skew_client_time_ns: 0,
            last_skew_offset_ns: 0.0,
        }
    }

    pub fn push_sample(
        &mut self,
        client_send_ns: u64,
        server_recv_ns: u64,
        server_send_ns: u64,
        client_recv_ns: u64,
    ) {
        let rtt_ns = (client_recv_ns as i128 - client_send_ns as i128)
            - (server_send_ns as i128 - server_recv_ns as i128);
        let sample_offset = (((server_recv_ns as i128 - client_send_ns as i128)
            + (server_send_ns as i128 - client_recv_ns as i128))
            / 2) as i64;

        // If a sudden step-change in clock offset is detected (e.g. system resume,
        // clock discontinuity, or reconnect after pause), reset the baseline immediately
        // instead of slowly filtering over dozens of minutes with EMA.
        if let Some(prev_offset) = self.base_offset_ns {
            let dt = (client_send_ns as f64) - (self.base_client_time_ns as f64);
            let projected_offset = prev_offset + self.skew * dt;
            if ((sample_offset as f64) - projected_offset).abs() > STEP_THRESHOLD_NS {
                log::warn!(
                    "[TIME_SYNC] Clock step-change detected ({:+.3}ms error). Resetting baseline offset to {:+.3}ms",
                    ((sample_offset as f64) - projected_offset) / 1e6,
                    sample_offset as f64 / 1e6
                );
                self.samples.clear();
                self.samples.push_back(Sample {
                    rtt_ns,
                    offset_ns: sample_offset,
                    client_time_ns: client_send_ns,
                });
                self.base_offset_ns = Some(sample_offset as f64);
                self.base_client_time_ns = client_send_ns;
                self.last_skew_client_time_ns = client_send_ns;
                self.last_skew_offset_ns = sample_offset as f64;
                self.skew = 0.0;
                return;
            }
        }

        if self.samples.len() >= WINDOW_SIZE {
            self.samples.pop_front();
        }
        self.samples.push_back(Sample {
            rtt_ns,
            offset_ns: sample_offset,
            client_time_ns: client_send_ns,
        });

        if let Some(best) = self.samples.iter().min_by_key(|s| s.rtt_ns) {
            match self.base_offset_ns {
                None => {
                    self.base_offset_ns = Some(best.offset_ns as f64);
                    self.base_client_time_ns = best.client_time_ns;
                    self.last_skew_client_time_ns = best.client_time_ns;
                    self.last_skew_offset_ns = best.offset_ns as f64;
                }
                Some(prev_offset) => {
                    let dt = (best.client_time_ns as f64) - (self.base_client_time_ns as f64);
                    // Project previous offset forward by current skew rate
                    let projected_offset = prev_offset + self.skew * dt;
                    // Blend residual error with EMA
                    let updated_offset =
                        projected_offset + EMA_ALPHA * ((best.offset_ns as f64) - projected_offset);

                    // Skew estimation over multi-second baseline
                    let skew_dt = best
                        .client_time_ns
                        .saturating_sub(self.last_skew_client_time_ns);
                    if skew_dt >= 2_000_000_000 {
                        let raw_skew =
                            ((best.offset_ns as f64) - self.last_skew_offset_ns) / (skew_dt as f64);
                        let clamped_skew = raw_skew.clamp(-MAX_SKEW_PPM, MAX_SKEW_PPM);
                        if self.skew == 0.0 {
                            self.skew = clamped_skew;
                        } else {
                            self.skew = SKEW_ALPHA * clamped_skew + (1.0 - SKEW_ALPHA) * self.skew;
                        }
                        self.last_skew_client_time_ns = best.client_time_ns;
                        self.last_skew_offset_ns = best.offset_ns as f64;
                    }

                    self.base_offset_ns = Some(updated_offset);
                    self.base_client_time_ns = best.client_time_ns;
                }
            }
        }
    }

    pub fn client_to_server_ns(&self, client_time_ns: u64) -> Option<u64> {
        self.base_offset_ns.map(|theta_0| {
            let dt = client_time_ns as f64 - self.base_client_time_ns as f64;
            let current_offset = theta_0 + self.skew * dt;
            client_time_ns.saturating_add_signed(current_offset.round() as i64)
        })
    }

    pub fn get_offset(&self) -> Option<i64> {
        self.base_offset_ns.map(|o| o.round() as i64)
    }

    pub fn get_skew_ppm(&self) -> f64 {
        self.skew * 1e6
    }

    /// Predicts what the offset should be at `client_time_ns` based on current base offset and skew
    /// For allowing extrapolation of the offset of the server clock without having to ping the server(debugging/tooling)
    pub fn predict_offset_at(&self, client_time_ns: u64) -> Option<i64> {
        self.base_offset_ns.map(|theta_0| {
            let dt = client_time_ns as f64 - self.base_client_time_ns as f64;
            (theta_0 + self.skew * dt).round() as i64
        })
    }
}

#[test]
fn test_time_sync_ground_truth() {
    let mut estimator = TimeSyncEstimator::new();
    let true_offset: i128 = 15_000_000_000; // Server is 15s ahead
    let one_way_delay: i128 = 2_000_000; // 2ms network flight time
    let server_hold: i128 = 500_000; // 0.5ms turnaround

    for i in 0..16 {
        let t1 = 1_000_000_000 + i * 200_000_000; // Ping every 200ms
        let t2 = t1 + true_offset + one_way_delay;
        let t3 = t2 + server_hold;
        let t4 = t3 - true_offset + one_way_delay;

        estimator.push_sample(t1 as u64, t2 as u64, t3 as u64, t4 as u64);
    }

    let estimated_offset = estimator.get_offset().unwrap();
    // Must converge to within 1 microsecond of ground truth
    assert!((estimated_offset - true_offset as i64).abs() < 1_000);
}

#[test]
fn test_time_sync_negative_offset() {
    let mut estimator = TimeSyncEstimator::new();
    // Client booted 24h ago, Server booted 1h ago: offset is -23 hours
    let true_offset: i128 = -23 * 3600 * 1_000_000_000;
    let delay: i128 = 3_000_000; // 3ms

    let t1 = 86_400_000_000_000u64; // 24 hours
    let t2 = (t1 as i128 + true_offset + delay) as u64;
    let t3 = t2 + 100_000;
    let t4 = (t3 as i128 - true_offset + delay) as u64;

    estimator.push_sample(t1, t2, t3, t4);
    assert_eq!(estimator.get_offset().unwrap(), true_offset as i64);
}

#[test]
fn test_wi_fi_spike_rejection() {
    let mut estimator = TimeSyncEstimator::new();
    let true_offset: i128 = 5_000_000_000;

    for i in 0..20 {
        let t1 = 10_000_000_000 + i * 200_000_000;
        // Inject 60ms spikes on samples 5 and 12, 2ms baseline otherwise
        let delay = if i == 5 || i == 12 {
            30_000_000
        } else {
            1_000_000
        };
        let t2 = t1 + true_offset + delay;
        let t3 = t2 + 200_000;
        let t4 = t3 - true_offset + delay;

        estimator.push_sample(t1 as u64, t2 as u64, t3 as u64, t4 as u64);
    }

    let diff = (estimator.get_offset().unwrap() - true_offset as i64).abs();
    // Offset must not get pulled by the 60ms spikes (error must stay under 0.1ms)
    assert!(
        diff < 100_000,
        "Spike leaked into estimator! Error: {diff}ns"
    );
}

#[test]
fn test_time_sync_skew_correction() {
    let mut estimator = TimeSyncEstimator::new();
    let initial_offset: i128 = 10_000_000_000; // 10s
    let skew_ppm: f64 = 40.0; // Server ticks 40 ppm faster
    let skew_ratio = skew_ppm * 1e-6;
    let delay: i128 = 1_500_000; // 1.5ms one-way

    // Simulate 30 pings every 500ms (15 seconds total)
    for i in 0..30 {
        let t1 = 1_000_000_000 + i * 500_000_000;
        let drift = (t1 as f64 * skew_ratio) as i128;
        let current_true_offset = initial_offset + drift;

        let t2 = t1 + current_true_offset + delay;
        let t3 = t2 + 100_000;
        let t4 = t3 - current_true_offset + delay;

        estimator.push_sample(t1 as u64, t2 as u64, t3 as u64, t4 as u64);
    }

    // Extrapolate 3 seconds into the future without any new pings
    let future_client_time = 1_000_000_000u64 + 18 * 1_000_000_000u64;
    let future_true_offset = initial_offset + (future_client_time as f64 * skew_ratio) as i128;
    let true_server_time = (future_client_time as i128 + future_true_offset) as u64;

    let predicted_server_time = estimator
        .client_to_server_ns(future_client_time as u64)
        .unwrap();
    let error_ns = (predicted_server_time as i128 - true_server_time as i128).abs();

    // With skew correction, error stays under 50 microseconds (0.05ms) even when extrapolating!
    assert!(error_ns < 50_000, "Error was too large: {error_ns}ns");
}

#[test]
fn test_time_sync_step_change_recovery() {
    let mut estimator = TimeSyncEstimator::new();
    let initial_offset: i128 = 10_000_000_000; // 10s
    let delay: i128 = 2_000_000; // 2ms

    // Settle estimator on initial 10s offset
    for i in 0..16 {
        let t1 = 1_000_000_000 + i * 200_000_000;
        let t2 = t1 + initial_offset + delay;
        let t3 = t2 + 100_000;
        let t4 = t3 - initial_offset + delay;
        estimator.push_sample(t1 as u64, t2 as u64, t3 as u64, t4 as u64);
    }
    assert_eq!(estimator.get_offset().unwrap(), initial_offset as i64);

    // Simulate an 18-second jump in clock offset (e.g. device sleep or step change)
    let jumped_offset: i128 = 28_000_000_000; // 28s (+18s jump)
    let t1 = 1_000_000_000 + 16 * 200_000_000;
    let t2 = t1 + jumped_offset + delay;
    let t3 = t2 + 100_000;
    let t4 = t3 - jumped_offset + delay;

    estimator.push_sample(t1 as u64, t2 as u64, t3 as u64, t4 as u64);

    // Must snap immediately to the new offset without taking dozens of EMA cycles
    let new_offset = estimator.get_offset().unwrap();
    assert!(
        (new_offset - jumped_offset as i64).abs() < 1_000,
        "Failed to snap to new offset, got: {new_offset}ns"
    );
}

#[test]
fn test_time_sync_step_change_recovery() {
    let mut estimator = TimeSyncEstimator::new();
    let initial_offset: i128 = 10_000_000_000; // 10s
    let delay: i128 = 2_000_000; // 2ms

    // Settle estimator on initial 10s offset
    for i in 0..16 {
        let t1 = 1_000_000_000 + i * 200_000_000;
        let t2 = t1 + initial_offset + delay;
        let t3 = t2 + 100_000;
        let t4 = t3 - initial_offset + delay;
        estimator.push_sample(t1 as u64, t2 as u64, t3 as u64, t4 as u64);
    }
    assert_eq!(estimator.get_offset().unwrap(), initial_offset as i64);

    // Simulate an 18-second jump in clock offset (e.g. device sleep or step change)
    let jumped_offset: i128 = 28_000_000_000; // 28s (+18s jump)
    let t1 = 1_000_000_000 + 16 * 200_000_000;
    let t2 = t1 + jumped_offset + delay;
    let t3 = t2 + 100_000;
    let t4 = t3 - jumped_offset + delay;

    estimator.push_sample(t1 as u64, t2 as u64, t3 as u64, t4 as u64);

    // Must snap immediately to the new offset without taking dozens of EMA cycles
    let new_offset = estimator.get_offset().unwrap();
    assert!(
        (new_offset - jumped_offset as i64).abs() < 1_000,
        "Failed to snap to new offset, got: {new_offset}ns"
    );
}
