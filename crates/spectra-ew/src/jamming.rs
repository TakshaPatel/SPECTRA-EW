use spectra_core::id::EntityId;
use spectra_core::interference::{Interference, InterferenceKind};
use spectra_core::world::{Position, World};

use crate::effect::EwEffect;

/// Configuration for DeceptionJamming effect.
#[derive(Debug, Clone)]
pub struct DeceptionConfig {
    pub source_id: EntityId,
    pub false_emitter_id: EntityId,
    pub owner_id: EntityId,
    pub channel: u32,
    pub frequency_mhz: f64,
    pub power_db: f64,
    pub position: Position,
    pub duration_ticks: u64,
    pub start_tick: u64,
}

/// Configuration for CrossChannelInterference effect.
#[derive(Debug, Clone)]
pub struct CrossChannelConfig {
    pub source_id: EntityId,
    pub center: Position,
    pub radius: f64,
    pub intensity_db: f64,
    pub duration_ticks: u64,
    pub start_tick: u64,
    pub source_channel: u32,
    pub affected_channels: Vec<u32>,
}

/// Noise jamming: adds broadband interference around a position.
/// Raises the effective noise floor for all receivers within radius.
#[derive(Debug, Clone)]
pub struct NoiseJamming {
    pub source_id: EntityId,
    pub center: Position,
    pub radius: f64,
    pub intensity_db: f64,
    pub duration_ticks: u64,
    pub start_tick: u64,
    pub affected_channels: Vec<u32>,
}

impl NoiseJamming {
    pub fn new(
        source_id: EntityId,
        center: Position,
        radius: f64,
        intensity_db: f64,
        duration_ticks: u64,
        start_tick: u64,
    ) -> Self {
        Self {
            source_id,
            center,
            radius,
            intensity_db,
            duration_ticks,
            start_tick,
            affected_channels: Vec::new(),
        }
    }

    pub fn with_channels(mut self, channels: Vec<u32>) -> Self {
        self.affected_channels = channels;
        self
    }
}

impl EwEffect for NoiseJamming {
    fn apply(&self, world: &mut World, _tick: u64) {
        let id = EntityId::from_raw(world.active_interference.len() as u64 + 1000);
        let mut ix = Interference::new(
            id,
            self.source_id,
            InterferenceKind::NoiseJamming,
            self.center,
            self.radius,
            self.intensity_db,
            self.duration_ticks,
        );
        if !self.affected_channels.is_empty() {
            ix = ix.with_affected_channels(self.affected_channels.clone());
        }
        world.add_interference(ix);
    }

    fn source(&self) -> EntityId {
        self.source_id
    }

    fn is_active(&self, tick: u64) -> bool {
        tick >= self.start_tick && tick < self.start_tick + self.duration_ticks
    }

    fn name(&self) -> &str {
        "NoiseJamming"
    }
}

/// Deception jamming: creates false emitter signatures to confuse sensors.
/// False emitters appear as real signals in the world state.
#[derive(Debug, Clone)]
pub struct DeceptionJamming {
    pub config: DeceptionConfig,
    deployed: std::cell::Cell<bool>,
}

impl DeceptionJamming {
    pub fn new(config: DeceptionConfig) -> Self {
        Self {
            config,
            deployed: std::cell::Cell::new(false),
        }
    }
}

impl EwEffect for DeceptionJamming {
    fn apply(&self, world: &mut World, _tick: u64) {
        let already_has = world
            .emitters
            .iter()
            .any(|e| e.id == self.config.false_emitter_id);

        if !already_has {
            let emitter = spectra_core::emitter::Emitter::new(
                self.config.false_emitter_id,
                self.config.owner_id,
                self.config.channel,
                self.config.frequency_mhz,
                spectra_core::emitter::SignalCategory::Unknown,
                self.config.position,
            )
            .with_power_db(self.config.power_db);
            world.add_emitter(emitter);
            self.deployed.set(true);
        }
    }

    fn source(&self) -> EntityId {
        self.config.source_id
    }

    fn is_active(&self, tick: u64) -> bool {
        tick >= self.config.start_tick && tick < self.config.start_tick + self.config.duration_ticks
    }

    fn name(&self) -> &str {
        "DeceptionJamming"
    }
}

/// Channel flooding: floods specific channels with noise interference.
/// More targeted than broadband noise jamming.
#[derive(Debug, Clone)]
pub struct ChannelFlooding {
    pub source_id: EntityId,
    pub center: Position,
    pub radius: f64,
    pub intensity_db: f64,
    pub duration_ticks: u64,
    pub start_tick: u64,
    pub target_channels: Vec<u32>,
}

impl ChannelFlooding {
    pub fn new(
        source_id: EntityId,
        center: Position,
        radius: f64,
        intensity_db: f64,
        duration_ticks: u64,
        start_tick: u64,
        target_channels: Vec<u32>,
    ) -> Self {
        Self {
            source_id,
            center,
            radius,
            intensity_db,
            duration_ticks,
            start_tick,
            target_channels,
        }
    }
}

impl EwEffect for ChannelFlooding {
    fn apply(&self, world: &mut World, _tick: u64) {
        let id = EntityId::from_raw(world.active_interference.len() as u64 + 2000);
        let ix = Interference::new(
            id,
            self.source_id,
            InterferenceKind::ChannelFlooding,
            self.center,
            self.radius,
            self.intensity_db,
            self.duration_ticks,
        )
        .with_affected_channels(self.target_channels.clone());
        world.add_interference(ix);
    }

    fn source(&self) -> EntityId {
        self.source_id
    }

    fn is_active(&self, tick: u64) -> bool {
        tick >= self.start_tick && tick < self.start_tick + self.duration_ticks
    }

    fn name(&self) -> &str {
        "ChannelFlooding"
    }
}

/// Cross-channel interference: creates interference between adjacent channels.
/// Simulates intermodulation products and harmonic interference.
#[derive(Debug, Clone)]
pub struct CrossChannelInterference {
    pub config: CrossChannelConfig,
}

impl CrossChannelInterference {
    pub fn new(config: CrossChannelConfig) -> Self {
        Self { config }
    }
}

impl EwEffect for CrossChannelInterference {
    fn apply(&self, world: &mut World, _tick: u64) {
        let id = EntityId::from_raw(world.active_interference.len() as u64 + 3000);
        let ix = Interference::new(
            id,
            self.config.source_id,
            InterferenceKind::CrossChannelInterference,
            self.config.center,
            self.config.radius,
            self.config.intensity_db,
            self.config.duration_ticks,
        )
        .with_affected_channels(self.config.affected_channels.clone());
        world.add_interference(ix);
    }

    fn source(&self) -> EntityId {
        self.config.source_id
    }

    fn is_active(&self, tick: u64) -> bool {
        tick >= self.config.start_tick && tick < self.config.start_tick + self.config.duration_ticks
    }

    fn name(&self) -> &str {
        "CrossChannelInterference"
    }
}

/// Sensor confusion: adds noise directly to the environment,
/// raising the ambient noise floor and degrading all sensor observations.
#[derive(Debug, Clone)]
pub struct SensorConfusion {
    pub source_id: EntityId,
    pub center: Position,
    pub radius: f64,
    pub noise_floor_increase_db: f64,
    pub duration_ticks: u64,
    pub start_tick: u64,
}

impl SensorConfusion {
    pub fn new(
        source_id: EntityId,
        center: Position,
        radius: f64,
        noise_floor_increase_db: f64,
        duration_ticks: u64,
        start_tick: u64,
    ) -> Self {
        Self {
            source_id,
            center,
            radius,
            noise_floor_increase_db,
            duration_ticks,
            start_tick,
        }
    }
}

impl EwEffect for SensorConfusion {
    fn apply(&self, world: &mut World, _tick: u64) {
        world.config.environment.ambient_noise_db += self.noise_floor_increase_db;
    }

    fn source(&self) -> EntityId {
        self.source_id
    }

    fn is_active(&self, tick: u64) -> bool {
        tick >= self.start_tick && tick < self.start_tick + self.duration_ticks
    }

    fn name(&self) -> &str {
        "SensorConfusion"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use spectra_core::entity::{Entity, EntityKind};
    use spectra_core::id::IdGenerator;
    use spectra_core::world::Faction;

    fn make_world() -> World {
        let config = spectra_core::world::WorldConfig::default();
        let mut world = World::new(config);
        let mut ids = IdGenerator::new();
        world.add_entity(Entity::new(
            ids.next_id(),
            EntityKind::Aircraft,
            Faction::Blue,
            Position::new(0.0, 0.0),
        ));
        world.add_entity(Entity::new(
            ids.next_id(),
            EntityKind::Aircraft,
            Faction::Red,
            Position::new(100.0, 0.0),
        ));
        world
    }

    #[test]
    fn noise_jamming_adds_interference() {
        let mut world = make_world();
        let effect = NoiseJamming::new(
            EntityId::from_raw(1),
            Position::new(50.0, 50.0),
            100.0,
            40.0,
            5,
            0,
        );
        effect.apply(&mut world, 0);
        assert_eq!(world.active_interference.len(), 1);
        assert_eq!(
            world.active_interference[0].kind,
            InterferenceKind::NoiseJamming
        );
    }

    #[test]
    fn noise_jamming_with_channels() {
        let mut world = make_world();
        let effect = NoiseJamming::new(
            EntityId::from_raw(1),
            Position::new(50.0, 50.0),
            100.0,
            40.0,
            5,
            0,
        )
        .with_channels(vec![1, 2]);
        effect.apply(&mut world, 0);
        assert_eq!(world.active_interference[0].affected_channels, vec![1, 2]);
    }

    #[test]
    fn noise_jamming_active_period() {
        let effect = NoiseJamming::new(
            EntityId::from_raw(1),
            Position::new(0.0, 0.0),
            50.0,
            30.0,
            10,
            5,
        );
        assert!(!effect.is_active(0));
        assert!(!effect.is_active(4));
        assert!(effect.is_active(5));
        assert!(effect.is_active(14));
        assert!(!effect.is_active(15));
    }

    #[test]
    fn deception_jamming_adds_false_emitter() {
        let mut world = make_world();
        let effect = DeceptionJamming::new(DeceptionConfig {
            source_id: EntityId::from_raw(1),
            false_emitter_id: EntityId::from_raw(100),
            owner_id: EntityId::from_raw(2),
            channel: 1,
            frequency_mhz: 2400.0,
            power_db: 30.0,
            position: Position::new(75.0, 0.0),
            duration_ticks: 10,
            start_tick: 0,
        });
        effect.apply(&mut world, 0);
        assert_eq!(world.emitters.len(), 1);
        assert_eq!(world.emitters[0].id, EntityId::from_raw(100));
        assert_eq!(
            world.emitters[0].category,
            spectra_core::emitter::SignalCategory::Unknown
        );
    }

    #[test]
    fn deception_jamming_does_not_duplicate() {
        let mut world = make_world();
        let effect = DeceptionJamming::new(DeceptionConfig {
            source_id: EntityId::from_raw(1),
            false_emitter_id: EntityId::from_raw(100),
            owner_id: EntityId::from_raw(2),
            channel: 1,
            frequency_mhz: 2400.0,
            power_db: 30.0,
            position: Position::new(75.0, 0.0),
            duration_ticks: 10,
            start_tick: 0,
        });
        effect.apply(&mut world, 0);
        effect.apply(&mut world, 1);
        assert_eq!(world.emitters.len(), 1);
    }

    #[test]
    fn channel_flooding_adds_interference() {
        let mut world = make_world();
        let effect = ChannelFlooding::new(
            EntityId::from_raw(1),
            Position::new(50.0, 50.0),
            80.0,
            50.0,
            5,
            0,
            vec![1, 3],
        );
        effect.apply(&mut world, 0);
        assert_eq!(world.active_interference.len(), 1);
        assert_eq!(
            world.active_interference[0].kind,
            InterferenceKind::ChannelFlooding
        );
        assert_eq!(world.active_interference[0].affected_channels, vec![1, 3]);
    }

    #[test]
    fn cross_channel_interference_adds() {
        let mut world = make_world();
        let effect = CrossChannelInterference::new(CrossChannelConfig {
            source_id: EntityId::from_raw(1),
            center: Position::new(50.0, 50.0),
            radius: 80.0,
            intensity_db: 20.0,
            duration_ticks: 5,
            start_tick: 0,
            source_channel: 1,
            affected_channels: vec![2, 3],
        });
        effect.apply(&mut world, 0);
        assert_eq!(world.active_interference.len(), 1);
        assert_eq!(
            world.active_interference[0].kind,
            InterferenceKind::CrossChannelInterference
        );
    }

    #[test]
    fn sensor_confusion_raises_noise_floor() {
        let mut world = make_world();
        let initial_noise = world.config.environment.ambient_noise_db;
        let effect = SensorConfusion::new(
            EntityId::from_raw(1),
            Position::new(0.0, 0.0),
            200.0,
            10.0,
            5,
            0,
        );
        effect.apply(&mut world, 0);
        assert!(
            (world.config.environment.ambient_noise_db - (initial_noise + 10.0)).abs()
                < f64::EPSILON
        );
    }

    #[test]
    fn effect_names() {
        let noise = NoiseJamming::new(
            EntityId::from_raw(1),
            Position::new(0.0, 0.0),
            50.0,
            30.0,
            5,
            0,
        );
        assert_eq!(noise.name(), "NoiseJamming");

        let deception = DeceptionJamming::new(DeceptionConfig {
            source_id: EntityId::from_raw(1),
            false_emitter_id: EntityId::from_raw(100),
            owner_id: EntityId::from_raw(2),
            channel: 1,
            frequency_mhz: 2400.0,
            power_db: 30.0,
            position: Position::new(0.0, 0.0),
            duration_ticks: 5,
            start_tick: 0,
        });
        assert_eq!(deception.name(), "DeceptionJamming");

        let flooding = ChannelFlooding::new(
            EntityId::from_raw(1),
            Position::new(0.0, 0.0),
            50.0,
            30.0,
            5,
            0,
            vec![1],
        );
        assert_eq!(flooding.name(), "ChannelFlooding");

        let cross = CrossChannelInterference::new(CrossChannelConfig {
            source_id: EntityId::from_raw(1),
            center: Position::new(0.0, 0.0),
            radius: 50.0,
            intensity_db: 20.0,
            duration_ticks: 5,
            start_tick: 0,
            source_channel: 1,
            affected_channels: vec![2],
        });
        assert_eq!(cross.name(), "CrossChannelInterference");

        let confusion = SensorConfusion::new(
            EntityId::from_raw(1),
            Position::new(0.0, 0.0),
            100.0,
            5.0,
            5,
            0,
        );
        assert_eq!(confusion.name(), "SensorConfusion");
    }
}
