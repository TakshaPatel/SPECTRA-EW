use serde::{Deserialize, Serialize};

use crate::id::EntityId;
use crate::world::Position;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Receiver {
    pub id: EntityId,
    pub owner_id: EntityId,
    pub sensitivity_db: f64,
    pub noise_figure_db: f64,
    pub bandwidth_mhz: f64,
    pub position: Position,
    pub monitored_channels: Vec<u32>,
}

impl Receiver {
    pub fn new(id: EntityId, owner_id: EntityId, sensitivity_db: f64, position: Position) -> Self {
        Self {
            id,
            owner_id,
            sensitivity_db,
            noise_figure_db: 3.0,
            bandwidth_mhz: 1.0,
            position,
            monitored_channels: Vec::new(),
        }
    }

    pub fn with_noise_figure(mut self, nf: f64) -> Self {
        self.noise_figure_db = nf;
        self
    }

    pub fn with_monitored_channels(mut self, channels: Vec<u32>) -> Self {
        self.monitored_channels = channels;
        self
    }

    pub fn noise_floor_db(&self, ambient_noise_db: f64) -> f64 {
        ambient_noise_db + self.noise_figure_db + 10.0 * (self.bandwidth_mhz * 1_000_000.0).log10()
    }

    pub fn can_detect(&self, power_at_receiver_db: f64, ambient_noise_db: f64) -> bool {
        let noise_floor = self.noise_floor_db(ambient_noise_db);
        power_at_receiver_db > noise_floor
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn receiver_creation() {
        let id = EntityId::from_raw(20);
        let owner = EntityId::from_raw(1);
        let rx = Receiver::new(id, owner, -90.0, Position::new(0.0, 0.0));
        assert_eq!(rx.sensitivity_db, -90.0);
        assert!(rx.monitored_channels.is_empty());
    }

    #[test]
    fn receiver_noise_floor() {
        let id = EntityId::from_raw(21);
        let owner = EntityId::from_raw(1);
        let rx = Receiver::new(id, owner, -90.0, Position::new(0.0, 0.0));
        let nf = rx.noise_floor_db(-100.0);
        assert!(nf > -100.0);
    }

    #[test]
    fn receiver_detection_threshold() {
        let id = EntityId::from_raw(22);
        let owner = EntityId::from_raw(1);
        let rx = Receiver::new(id, owner, -90.0, Position::new(0.0, 0.0));
        let noise_floor = rx.noise_floor_db(-100.0);
        assert!(!rx.can_detect(noise_floor - 10.0, -100.0));
        assert!(rx.can_detect(noise_floor + 10.0, -100.0));
    }
}
