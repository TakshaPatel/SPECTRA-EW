use serde::{Deserialize, Serialize};

use crate::id::{EntityId, SignalId};
use crate::world::Position;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SignalCategory {
    Communication,
    Radar,
    Navigation,
    DataLink,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum EmitterState {
    Transmitting,
    Silent,
    Degraded,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Emitter {
    pub id: EntityId,
    pub owner_id: EntityId,
    pub channel: u32,
    pub frequency_mhz: f64,
    pub bandwidth_mhz: f64,
    pub power_db: f64,
    pub category: SignalCategory,
    pub state: EmitterState,
    pub position: Position,
}

impl Emitter {
    pub fn new(
        id: EntityId,
        owner_id: EntityId,
        channel: u32,
        frequency_mhz: f64,
        category: SignalCategory,
        position: Position,
    ) -> Self {
        Self {
            id,
            owner_id,
            channel,
            frequency_mhz,
            bandwidth_mhz: 1.0,
            power_db: 30.0,
            category,
            state: EmitterState::Transmitting,
            position,
        }
    }

    pub fn with_power_db(mut self, power: f64) -> Self {
        self.power_db = power;
        self
    }

    pub fn with_bandwidth(mut self, bw: f64) -> Self {
        self.bandwidth_mhz = bw;
        self
    }

    pub fn is_transmitting(&self) -> bool {
        self.state == EmitterState::Transmitting
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActiveSignal {
    pub signal_id: SignalId,
    pub source_emitter_id: EntityId,
    pub channel: u32,
    pub frequency_mhz: f64,
    pub power_at_receiver_db: f64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn emitter_creation() {
        let id = EntityId::from_raw(10);
        let owner = EntityId::from_raw(1);
        let emitter = Emitter::new(
            id,
            owner,
            1,
            2400.0,
            SignalCategory::Communication,
            Position::new(0.0, 0.0),
        );
        assert_eq!(emitter.channel, 1);
        assert!(emitter.is_transmitting());
    }

    #[test]
    fn emitter_with_config() {
        let id = EntityId::from_raw(11);
        let owner = EntityId::from_raw(1);
        let emitter = Emitter::new(
            id,
            owner,
            5,
            9000.0,
            SignalCategory::Radar,
            Position::new(10.0, 10.0),
        )
        .with_power_db(50.0)
        .with_bandwidth(5.0);
        assert_eq!(emitter.power_db, 50.0);
        assert_eq!(emitter.bandwidth_mhz, 5.0);
    }

    #[test]
    fn emitter_silent_state() {
        let id = EntityId::from_raw(12);
        let owner = EntityId::from_raw(1);
        let mut emitter = Emitter::new(
            id,
            owner,
            1,
            2400.0,
            SignalCategory::Communication,
            Position::new(0.0, 0.0),
        );
        emitter.state = EmitterState::Silent;
        assert!(!emitter.is_transmitting());
    }
}
