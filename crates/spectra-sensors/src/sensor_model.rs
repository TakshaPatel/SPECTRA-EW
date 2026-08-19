use serde::{Deserialize, Serialize};

use spectra_core::id::EntityId;
use spectra_core::observation::{Observation, ObservationQuality};

use crate::degradation::SensorHealth;

/// Configuration for sensor noise behavior.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NoiseProfile {
    /// Standard deviation of signal strength noise (dB)
    pub signal_noise_std: f64,
    /// Standard deviation of position estimate noise (meters)
    pub position_noise_std: f64,
    /// Standard deviation of frequency estimate noise (MHz)
    pub frequency_noise_std: f64,
    /// Additional noise floor increase (dB)
    pub noise_floor_addition: f64,
}

impl Default for NoiseProfile {
    fn default() -> Self {
        Self {
            signal_noise_std: 2.0,
            position_noise_std: 10.0,
            frequency_noise_std: 0.5,
            noise_floor_addition: 0.0,
        }
    }
}

/// A single sensor with configurable noise, detection, and false positive characteristics.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SensorModel {
    pub id: EntityId,
    pub owner_id: EntityId,
    /// Probability of detecting a signal that is actually present (sensitivity)
    pub detection_probability: f64,
    /// Probability of reporting a signal that is NOT present
    pub false_positive_rate: f64,
    /// SNR threshold below which detection probability drops sharply (dB)
    pub detection_threshold_snr: f64,
    /// Noise profile for this sensor
    pub noise: NoiseProfile,
    /// Health/degradation state
    pub health: SensorHealth,
    /// Monitored channels (empty = all)
    pub monitored_channels: Vec<u32>,
}

impl SensorModel {
    pub fn new(id: EntityId, owner_id: EntityId) -> Self {
        Self {
            id,
            owner_id,
            detection_probability: 0.95,
            false_positive_rate: 0.01,
            detection_threshold_snr: 0.0,
            noise: NoiseProfile::default(),
            health: SensorHealth::perfect(),
            monitored_channels: Vec::new(),
        }
    }

    pub fn with_detection_probability(mut self, dp: f64) -> Self {
        self.detection_probability = dp;
        self
    }

    pub fn with_false_positive_rate(mut self, fpr: f64) -> Self {
        self.false_positive_rate = fpr;
        self
    }

    pub fn with_threshold_snr(mut self, snr: f64) -> Self {
        self.detection_threshold_snr = snr;
        self
    }

    pub fn with_noise(mut self, noise: NoiseProfile) -> Self {
        self.noise = noise;
        self
    }

    pub fn with_health(mut self, health: SensorHealth) -> Self {
        self.health = health;
        self
    }

    pub fn with_monitored_channels(mut self, channels: Vec<u32>) -> Self {
        self.monitored_channels = channels;
        self
    }

    pub fn tick(&mut self) {
        self.health.tick();
        self.health.maybe_calibrate();
    }

    /// Compute effective detection probability accounting for SNR and health.
    pub fn effective_detection_probability(&self, snr_db: f64) -> f64 {
        let base = self.detection_probability;

        // SNR-dependent detection curve: sigmoid centered at threshold
        let x = (snr_db - self.detection_threshold_snr) / 10.0;
        let snr_factor = 1.0 / (1.0 + (-x).exp());

        // Health reduces detection probability
        let health_factor = self.health.integrity;

        (base * snr_factor * health_factor).clamp(0.0, 1.0)
    }

    /// Apply sensor noise and imperfections to a ground-truth observation.
    /// Returns the processed observation (may be unusable).
    pub fn process_observation(
        &self,
        mut obs: Observation,
        _snr_db: f64,
        rng: &mut impl rand::Rng,
    ) -> Observation {
        // 1. Add signal strength noise
        let signal_noise: f64 = rng.gen_range(-1.0..1.0) * self.noise.signal_noise_std;
        obs.signal_strength_db += signal_noise;

        // 2. Add noise floor degradation from sensor health
        obs.noise_level_db += self.noise.noise_floor_addition + self.health.degradation_noise();

        // 3. Add position estimate noise
        if let Some(ref mut pos) = obs.position_estimate {
            let dx: f64 = rng.gen_range(-1.0..1.0) * self.noise.position_noise_std;
            let dy: f64 = rng.gen_range(-1.0..1.0) * self.noise.position_noise_std;
            pos.x += dx;
            pos.y += dy;
            obs.uncertainty_position += self.noise.position_noise_std;
        }

        // 4. Add frequency estimate noise
        if let Some(ref mut freq) = obs.estimated_frequency_mhz {
            let freq_noise: f64 = rng.gen_range(-1.0..1.0) * self.noise.frequency_noise_std;
            *freq += freq_noise;
        }

        // 5. Recompute SNR after noise injection
        let noisy_snr = obs.signal_strength_db - obs.noise_level_db;

        // 6. Determine detection outcome
        let effective_dp = self.effective_detection_probability(noisy_snr);
        let roll: f64 = rng.gen();

        let detected = if noisy_snr > self.detection_threshold_snr {
            // Real signal — check if sensor misses it (false negative)
            roll < effective_dp
        } else {
            // No real signal above threshold — check for false positive
            roll < self.false_positive_rate
        };

        if detected {
            // Signal detected — update quality and confidence
            obs.confidence =
                spectra_core::propagation::confidence_from_snr(noisy_snr) * self.health.integrity;
            obs.quality = if noisy_snr > 20.0 {
                ObservationQuality::Clear
            } else if noisy_snr > 10.0 {
                ObservationQuality::Noisy
            } else {
                ObservationQuality::Degraded
            };
        } else {
            // Signal missed or rejected
            obs.confidence = 0.0;
            obs.quality = ObservationQuality::Unreliable;
            obs.estimated_source_id = None;
            obs.signal_id = None;
        }

        obs
    }

    /// Check if this sensor monitors a given channel.
    pub fn monitors_channel(&self, channel: u32) -> bool {
        self.monitored_channels.is_empty() || self.monitored_channels.contains(&channel)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::SeedableRng;

    #[test]
    fn sensor_creation() {
        let s = SensorModel::new(EntityId::from_raw(1), EntityId::from_raw(10));
        assert_eq!(s.detection_probability, 0.95);
        assert_eq!(s.false_positive_rate, 0.01);
        assert!(s.health.is_operational());
    }

    #[test]
    fn sensor_detection_snr_curve() {
        let s = SensorModel::new(EntityId::from_raw(1), EntityId::from_raw(10));
        // High SNR -> high detection probability
        let dp_high = s.effective_detection_probability(30.0);
        assert!(dp_high > 0.8);
        // Very low SNR -> low detection probability
        let dp_low = s.effective_detection_probability(-30.0);
        assert!(dp_low < 0.1);
    }

    #[test]
    fn sensor_detection_health_effect() {
        let s_perfect = SensorModel::new(EntityId::from_raw(1), EntityId::from_raw(10));
        let s_degraded = SensorModel::new(EntityId::from_raw(2), EntityId::from_raw(10))
            .with_health(SensorHealth::degraded(0.3, 0.0));
        let dp_perfect = s_perfect.effective_detection_probability(20.0);
        let dp_degraded = s_degraded.effective_detection_probability(20.0);
        assert!(dp_perfect > dp_degraded);
    }

    #[test]
    fn sensor_process_observation_noise() {
        let s = SensorModel::new(EntityId::from_raw(1), EntityId::from_raw(10));
        let mut rng = rand_chacha::ChaCha8Rng::seed_from_u64(42);
        let mut obs = Observation::new(EntityId::from_raw(1), 0);
        obs.signal_strength_db = -50.0;
        obs.noise_level_db = -80.0;
        obs.confidence = 0.8;
        obs.quality = ObservationQuality::Clear;
        obs.position_estimate = Some(spectra_core::world::Position::new(100.0, 200.0));

        let processed = s.process_observation(obs, 30.0, &mut rng);
        // Signal strength should have noise added
        assert!((processed.signal_strength_db - (-50.0)).abs() > 0.0);
        // Position should have noise added
        let pos = processed.position_estimate.unwrap();
        assert!((pos.x - 100.0).abs() > 0.0 || (pos.y - 200.0).abs() > 0.0);
    }

    #[test]
    fn sensor_false_positive() {
        let s = SensorModel::new(EntityId::from_raw(1), EntityId::from_raw(10))
            .with_false_positive_rate(1.0); // Always false positive
        let mut rng = rand_chacha::ChaCha8Rng::seed_from_u64(42);
        let mut obs = Observation::new(EntityId::from_raw(1), 0);
        obs.signal_strength_db = -90.0;
        obs.noise_level_db = -80.0; // SNR = -10, below threshold
        obs.confidence = 0.0;

        let processed = s.process_observation(obs, -10.0, &mut rng);
        // False positive should produce a detection
        assert!(processed.confidence > 0.0);
        assert!(processed.quality != ObservationQuality::Unreliable);
    }

    #[test]
    fn sensor_false_negative() {
        let s = SensorModel::new(EntityId::from_raw(1), EntityId::from_raw(10))
            .with_detection_probability(0.0); // Always miss
        let mut rng = rand_chacha::ChaCha8Rng::seed_from_u64(42);
        let mut obs = Observation::new(EntityId::from_raw(1), 0);
        obs.signal_strength_db = -50.0;
        obs.noise_level_db = -80.0; // SNR = 30, well above threshold
        obs.confidence = 0.9;
        obs.quality = ObservationQuality::Clear;
        obs.estimated_source_id = Some(EntityId::from_raw(20));

        let processed = s.process_observation(obs, 30.0, &mut rng);
        // False negative should reject the signal
        assert_eq!(processed.confidence, 0.0);
        assert_eq!(processed.quality, ObservationQuality::Unreliable);
        assert!(processed.estimated_source_id.is_none());
    }

    #[test]
    fn sensor_channel_filter() {
        let s = SensorModel::new(EntityId::from_raw(1), EntityId::from_raw(10))
            .with_monitored_channels(vec![1, 3]);
        assert!(s.monitors_channel(1));
        assert!(!s.monitors_channel(2));
        assert!(s.monitors_channel(3));
    }

    #[test]
    fn sensor_tick_decay() {
        let mut s = SensorModel::new(EntityId::from_raw(1), EntityId::from_raw(10));
        let initial_integrity = s.health.integrity;
        s.tick();
        assert!(s.health.integrity < initial_integrity);
    }
}
