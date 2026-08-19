use serde::{Deserialize, Serialize};

use crate::id::EntityId;
use crate::world::Position;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum InterferenceKind {
    NoiseJamming,
    DeceptionJamming,
    ChannelFlooding,
    CrossChannelInterference,
    EnvironmentalNoise,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Interference {
    pub id: EntityId,
    pub source_id: EntityId,
    pub kind: InterferenceKind,
    pub affected_center: Position,
    pub affected_radius: f64,
    pub intensity_db: f64,
    pub duration_ticks: u64,
    pub remaining_ticks: u64,
    pub affected_channels: Vec<u32>,
}

impl Interference {
    pub fn new(
        id: EntityId,
        source_id: EntityId,
        kind: InterferenceKind,
        center: Position,
        radius: f64,
        intensity_db: f64,
        duration_ticks: u64,
    ) -> Self {
        Self {
            id,
            source_id,
            kind,
            affected_center: center,
            affected_radius: radius,
            intensity_db,
            duration_ticks,
            remaining_ticks: duration_ticks,
            affected_channels: Vec::new(),
        }
    }

    pub fn with_affected_channels(mut self, channels: Vec<u32>) -> Self {
        self.affected_channels = channels;
        self
    }

    pub fn is_active(&self) -> bool {
        self.remaining_ticks > 0
    }

    pub fn tick(&mut self) {
        if self.remaining_ticks > 0 {
            self.remaining_ticks -= 1;
        }
    }

    pub fn affects_position(&self, pos: Position) -> bool {
        self.affected_center.distance_to(pos) <= self.affected_radius
    }

    pub fn affects_channel(&self, channel: u32) -> bool {
        self.affected_channels.is_empty() || self.affected_channels.contains(&channel)
    }

    pub fn attenuation_at(&self, pos: Position) -> f64 {
        if !self.affects_position(pos) {
            return 0.0;
        }
        let dist = self.affected_center.distance_to(pos);
        if dist < f64::EPSILON {
            return self.intensity_db;
        }
        let falloff = 20.0 * (dist / self.affected_radius).log10().max(0.0);
        (self.intensity_db - falloff).max(0.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn interference_creation() {
        let id = EntityId::from_raw(30);
        let source = EntityId::from_raw(1);
        let ix = Interference::new(
            id,
            source,
            InterferenceKind::NoiseJamming,
            Position::new(50.0, 50.0),
            100.0,
            60.0,
            10,
        );
        assert!(ix.is_active());
        assert_eq!(ix.remaining_ticks, 10);
    }

    #[test]
    fn interference_tick() {
        let id = EntityId::from_raw(31);
        let source = EntityId::from_raw(1);
        let mut ix = Interference::new(
            id,
            source,
            InterferenceKind::NoiseJamming,
            Position::new(0.0, 0.0),
            50.0,
            40.0,
            3,
        );
        ix.tick();
        assert_eq!(ix.remaining_ticks, 2);
        ix.tick();
        assert_eq!(ix.remaining_ticks, 1);
        ix.tick();
        assert_eq!(ix.remaining_ticks, 0);
        assert!(!ix.is_active());
    }

    #[test]
    fn interference_affects_position() {
        let id = EntityId::from_raw(32);
        let source = EntityId::from_raw(1);
        let ix = Interference::new(
            id,
            source,
            InterferenceKind::NoiseJamming,
            Position::new(0.0, 0.0),
            100.0,
            50.0,
            5,
        );
        assert!(ix.affects_position(Position::new(50.0, 0.0)));
        assert!(!ix.affects_position(Position::new(200.0, 0.0)));
    }

    #[test]
    fn interference_channel_filter() {
        let id = EntityId::from_raw(33);
        let source = EntityId::from_raw(1);
        let ix = Interference::new(
            id,
            source,
            InterferenceKind::NoiseJamming,
            Position::new(0.0, 0.0),
            100.0,
            50.0,
            5,
        )
        .with_affected_channels(vec![1, 2]);
        assert!(ix.affects_channel(1));
        assert!(!ix.affects_channel(3));
    }

    #[test]
    fn interference_channel_filter_empty() {
        let id = EntityId::from_raw(34);
        let source = EntityId::from_raw(1);
        let ix = Interference::new(
            id,
            source,
            InterferenceKind::NoiseJamming,
            Position::new(0.0, 0.0),
            100.0,
            50.0,
            5,
        );
        assert!(ix.affects_channel(999));
    }
}
