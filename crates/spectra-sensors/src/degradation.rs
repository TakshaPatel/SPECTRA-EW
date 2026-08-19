use serde::{Deserialize, Serialize};

/// Tracks health degradation of a sensor over time.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SensorHealth {
    /// 1.0 = perfect, 0.0 = completely dead
    pub integrity: f64,
    /// Accumulated drift in dB (adds to noise)
    pub drift_db: f64,
    /// Number of ticks since last calibration
    pub ticks_since_calibration: u64,
    /// Calibration resets drift and restores integrity partially
    pub calibration_interval: u64,
    /// How much integrity recovers per calibration
    pub calibration_recovery: f64,
    /// How fast integrity decays per tick
    pub decay_rate: f64,
    /// Maximum drift before sensor is considered unreliable
    pub max_drift_db: f64,
}

impl SensorHealth {
    pub fn perfect() -> Self {
        Self {
            integrity: 1.0,
            drift_db: 0.0,
            ticks_since_calibration: 0,
            calibration_interval: 100,
            calibration_recovery: 0.1,
            decay_rate: 0.001,
            max_drift_db: 10.0,
        }
    }

    pub fn degraded(integrity: f64, drift_db: f64) -> Self {
        Self {
            integrity: integrity.clamp(0.0, 1.0),
            drift_db,
            ticks_since_calibration: 0,
            calibration_interval: 100,
            calibration_recovery: 0.1,
            decay_rate: 0.001,
            max_drift_db: 10.0,
        }
    }

    pub fn failed() -> Self {
        Self {
            integrity: 0.0,
            drift_db: f64::MAX,
            ticks_since_calibration: 0,
            calibration_interval: 100,
            calibration_recovery: 0.0,
            decay_rate: 0.0,
            max_drift_db: 0.0,
        }
    }

    /// Advance one tick: decay integrity, increase drift.
    pub fn tick(&mut self) {
        self.ticks_since_calibration += 1;
        self.integrity = (self.integrity - self.decay_rate).max(0.0);
        self.drift_db += self.decay_rate * 0.5;
    }

    /// Calibrate: restore integrity, reduce drift.
    pub fn calibrate(&mut self) {
        self.integrity = (self.integrity + self.calibration_recovery).min(1.0);
        self.drift_db = (self.drift_db * 0.5).max(0.0);
        self.ticks_since_calibration = 0;
    }

    /// Auto-calibrate if interval has elapsed.
    pub fn maybe_calibrate(&mut self) {
        if self.ticks_since_calibration >= self.calibration_interval {
            self.calibrate();
        }
    }

    pub fn is_operational(&self) -> bool {
        self.integrity > 0.1
    }

    pub fn is_reliable(&self) -> bool {
        self.is_operational() && self.drift_db.abs() < self.max_drift_db
    }

    /// Effective noise addition from degradation (dB).
    pub fn degradation_noise(&self) -> f64 {
        self.drift_db + (1.0 - self.integrity) * 10.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn health_perfect() {
        let h = SensorHealth::perfect();
        assert_eq!(h.integrity, 1.0);
        assert!(h.is_operational());
        assert!(h.is_reliable());
    }

    #[test]
    fn health_failed() {
        let h = SensorHealth::failed();
        assert!(!h.is_operational());
        assert!(!h.is_reliable());
    }

    #[test]
    fn health_decay() {
        let mut h = SensorHealth::perfect();
        h.tick();
        assert!(h.integrity < 1.0);
        assert!(h.drift_db > 0.0);
    }

    #[test]
    fn health_calibration() {
        let mut h = SensorHealth::degraded(0.5, 5.0);
        h.calibrate();
        assert!(h.integrity > 0.5);
        assert!(h.drift_db < 5.0);
    }

    #[test]
    fn health_auto_calibrate() {
        let mut h = SensorHealth {
            calibration_interval: 3,
            ..SensorHealth::degraded(0.5, 5.0)
        };
        // Tick advances the counter, maybe_calibrate checks it
        for _ in 0..3 {
            h.tick();
            h.maybe_calibrate();
        }
        // After calibration at tick 3, integrity should recover
        assert!(h.integrity > 0.5);
    }

    #[test]
    fn health_degradation_noise() {
        let h = SensorHealth::perfect();
        assert!((h.degradation_noise()).abs() < f64::EPSILON);

        let h = SensorHealth::degraded(0.5, 3.0);
        let noise = h.degradation_noise();
        assert!(noise > 3.0);
    }
}
