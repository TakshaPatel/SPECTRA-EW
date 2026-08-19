use serde::{Deserialize, Serialize};

use crate::emitter::Emitter;
use crate::entity::Entity;
use crate::id::EntityId;
use crate::interference::Interference;
use crate::receiver::Receiver;

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Position {
    pub x: f64,
    pub y: f64,
}

impl Position {
    pub fn new(x: f64, y: f64) -> Self {
        Self { x, y }
    }

    pub fn distance_to(self, other: Position) -> f64 {
        let dx = self.x - other.x;
        let dy = self.y - other.y;
        (dx * dx + dy * dy).sqrt()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Environment {
    pub propagation_loss_exponent: f64,
    pub ambient_noise_db: f64,
    pub weather_attenuation: f64,
    pub terrain_masking: bool,
}

impl Default for Environment {
    fn default() -> Self {
        Self {
            propagation_loss_exponent: 2.0,
            ambient_noise_db: -100.0,
            weather_attenuation: 0.0,
            terrain_masking: false,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Faction {
    Blue,
    Red,
    Neutral,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorldConfig {
    pub map_width: f64,
    pub map_height: f64,
    pub max_ticks: u64,
    pub environment: Environment,
}

impl Default for WorldConfig {
    fn default() -> Self {
        Self {
            map_width: 1000.0,
            map_height: 1000.0,
            max_ticks: 1000,
            environment: Environment::default(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct World {
    pub config: WorldConfig,
    pub tick: u64,
    pub entities: Vec<Entity>,
    pub emitters: Vec<Emitter>,
    pub receivers: Vec<Receiver>,
    pub active_interference: Vec<Interference>,
}

impl World {
    pub fn new(config: WorldConfig) -> Self {
        Self {
            config,
            tick: 0,
            entities: Vec::new(),
            emitters: Vec::new(),
            receivers: Vec::new(),
            active_interference: Vec::new(),
        }
    }

    pub fn advance_tick(&mut self) {
        self.tick += 1;
    }

    pub fn add_entity(&mut self, entity: Entity) -> EntityId {
        let id = entity.id;
        self.entities.push(entity);
        id
    }

    pub fn add_emitter(&mut self, emitter: Emitter) -> EntityId {
        let id = emitter.id;
        self.emitters.push(emitter);
        id
    }

    pub fn add_receiver(&mut self, receiver: Receiver) -> EntityId {
        let id = receiver.id;
        self.receivers.push(receiver);
        id
    }

    pub fn add_interference(&mut self, interference: Interference) -> EntityId {
        let id = interference.id;
        self.active_interference.push(interference);
        id
    }

    pub fn entity_by_id(&self, id: EntityId) -> Option<&Entity> {
        self.entities.iter().find(|e| e.id == id)
    }

    pub fn entities_by_faction(&self, faction: Faction) -> Vec<&Entity> {
        self.entities
            .iter()
            .filter(|e| e.faction == faction)
            .collect()
    }

    pub fn emitters_for_entity(&self, owner_id: EntityId) -> Vec<&Emitter> {
        self.emitters
            .iter()
            .filter(|e| e.owner_id == owner_id)
            .collect()
    }

    pub fn receivers_for_entity(&self, owner_id: EntityId) -> Vec<&Receiver> {
        self.receivers
            .iter()
            .filter(|r| r.owner_id == owner_id)
            .collect()
    }

    pub fn tick_interference(&mut self) {
        for ix in &mut self.active_interference {
            ix.tick();
        }
        self.active_interference.retain(|ix| ix.is_active());
    }

    pub fn is_complete(&self) -> bool {
        self.tick >= self.config.max_ticks
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::entity::{Entity, EntityKind};

    #[test]
    fn world_creation() {
        let config = WorldConfig::default();
        let world = World::new(config);
        assert_eq!(world.tick, 0);
        assert!(world.entities.is_empty());
    }

    #[test]
    fn world_tick_advance() {
        let config = WorldConfig::default();
        let mut world = World::new(config);
        world.advance_tick();
        assert_eq!(world.tick, 1);
    }

    #[test]
    fn world_add_entity() {
        let config = WorldConfig::default();
        let mut world = World::new(config);
        let entity = Entity::new(
            EntityId::from_raw(1),
            EntityKind::Platform,
            Faction::Blue,
            Position::new(0.0, 0.0),
        );
        let id = world.add_entity(entity);
        assert_eq!(world.entities.len(), 1);
        assert_eq!(world.entity_by_id(id).unwrap().kind, EntityKind::Platform);
    }

    #[test]
    fn world_add_emitter() {
        let config = WorldConfig::default();
        let mut world = World::new(config);
        let emitter = Emitter::new(
            EntityId::from_raw(10),
            EntityId::from_raw(1),
            1,
            2400.0,
            crate::emitter::SignalCategory::Communication,
            Position::new(0.0, 0.0),
        );
        let id = world.add_emitter(emitter);
        assert_eq!(world.emitters.len(), 1);
        assert_eq!(world.emitters[0].id, id);
    }

    #[test]
    fn world_emitters_for_entity() {
        let config = WorldConfig::default();
        let mut world = World::new(config);
        let owner = EntityId::from_raw(1);
        let other = EntityId::from_raw(2);
        world.add_emitter(Emitter::new(
            EntityId::from_raw(10),
            owner,
            1,
            2400.0,
            crate::emitter::SignalCategory::Communication,
            Position::new(0.0, 0.0),
        ));
        world.add_emitter(Emitter::new(
            EntityId::from_raw(11),
            other,
            2,
            5800.0,
            crate::emitter::SignalCategory::Radar,
            Position::new(0.0, 0.0),
        ));
        assert_eq!(world.emitters_for_entity(owner).len(), 1);
        assert_eq!(world.emitters_for_entity(other).len(), 1);
    }

    #[test]
    fn world_tick_interference() {
        use crate::interference::{Interference, InterferenceKind};

        let config = WorldConfig::default();
        let mut world = World::new(config);
        world.add_interference(Interference::new(
            EntityId::from_raw(30),
            EntityId::from_raw(1),
            InterferenceKind::NoiseJamming,
            Position::new(0.0, 0.0),
            100.0,
            40.0,
            2,
        ));
        assert_eq!(world.active_interference.len(), 1);
        world.tick_interference();
        assert_eq!(world.active_interference.len(), 1);
        world.tick_interference();
        assert_eq!(world.active_interference.len(), 0);
    }

    #[test]
    fn world_completion() {
        let config = WorldConfig {
            max_ticks: 5,
            ..Default::default()
        };
        let mut world = World::new(config);
        for _ in 0..5 {
            assert!(!world.is_complete());
            world.advance_tick();
        }
        assert!(world.is_complete());
    }

    #[test]
    fn position_distance() {
        let a = Position::new(0.0, 0.0);
        let b = Position::new(3.0, 4.0);
        assert!((a.distance_to(b) - 5.0).abs() < f64::EPSILON);
    }
}
