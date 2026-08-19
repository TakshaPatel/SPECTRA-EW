use serde::{Deserialize, Serialize};

use crate::id::{EntityId, SignalId};
use crate::world::Position;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ObservationQuality {
    Clear,
    Noisy,
    Degraded,
    Unreliable,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Observation {
    pub observer_id: EntityId,
    pub estimated_source_id: Option<EntityId>,
    pub signal_id: Option<SignalId>,
    pub estimated_channel: Option<u32>,
    pub estimated_frequency_mhz: Option<f64>,
    pub signal_strength_db: f64,
    pub noise_level_db: f64,
    pub confidence: f64,
    pub quality: ObservationQuality,
    pub timestamp: u64,
    pub position_estimate: Option<Position>,
    pub uncertainty_position: f64,
}

impl Observation {
    pub fn new(observer_id: EntityId, timestamp: u64) -> Self {
        Self {
            observer_id,
            estimated_source_id: None,
            signal_id: None,
            estimated_channel: None,
            estimated_frequency_mhz: None,
            signal_strength_db: 0.0,
            noise_level_db: -100.0,
            confidence: 0.0,
            quality: ObservationQuality::Unreliable,
            timestamp,
            position_estimate: None,
            uncertainty_position: f64::MAX,
        }
    }

    pub fn signal_to_noise_ratio(&self) -> f64 {
        self.signal_strength_db - self.noise_level_db
    }

    pub fn is_usable(&self) -> bool {
        self.confidence > 0.3 && self.quality != ObservationQuality::Unreliable
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn observation_creation() {
        let observer = EntityId::from_raw(1);
        let obs = Observation::new(observer, 0);
        assert_eq!(obs.timestamp, 0);
        assert!(!obs.is_usable());
    }

    #[test]
    fn observation_snr() {
        let observer = EntityId::from_raw(1);
        let mut obs = Observation::new(observer, 0);
        obs.signal_strength_db = -60.0;
        obs.noise_level_db = -90.0;
        assert!((obs.signal_to_noise_ratio() - 30.0).abs() < f64::EPSILON);
    }

    #[test]
    fn observation_usability() {
        let observer = EntityId::from_raw(1);
        let mut obs = Observation::new(observer, 0);
        obs.confidence = 0.8;
        obs.quality = ObservationQuality::Clear;
        assert!(obs.is_usable());

        obs.quality = ObservationQuality::Unreliable;
        assert!(!obs.is_usable());

        obs.quality = ObservationQuality::Clear;
        obs.confidence = 0.1;
        assert!(!obs.is_usable());
    }
}
