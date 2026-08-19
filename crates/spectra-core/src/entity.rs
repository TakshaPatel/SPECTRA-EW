use serde::{Deserialize, Serialize};

use crate::id::EntityId;
use crate::world::{Faction, Position};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum EntityKind {
    Platform,
    Aircraft,
    Ship,
    GroundStation,
    Satellite,
    Drone,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ActivityState {
    Active,
    Idle,
    Degraded,
    Disabled,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Capabilities {
    pub can_emit: bool,
    pub can_receive: bool,
    pub can_jam: bool,
    pub can_deceive: bool,
    pub max_power_db: f64,
}

impl Default for Capabilities {
    fn default() -> Self {
        Self {
            can_emit: false,
            can_receive: false,
            can_jam: false,
            can_deceive: false,
            max_power_db: 0.0,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Entity {
    pub id: EntityId,
    pub kind: EntityKind,
    pub faction: Faction,
    pub position: Position,
    pub capabilities: Capabilities,
    pub state: ActivityState,
    pub label: String,
}

impl Entity {
    pub fn new(id: EntityId, kind: EntityKind, faction: Faction, position: Position) -> Self {
        Self {
            id,
            kind,
            faction,
            position,
            capabilities: Capabilities::default(),
            state: ActivityState::Active,
            label: String::new(),
        }
    }

    pub fn with_label(mut self, label: impl Into<String>) -> Self {
        self.label = label.into();
        self
    }

    pub fn with_capabilities(mut self, caps: Capabilities) -> Self {
        self.capabilities = caps;
        self
    }

    pub fn is_active(&self) -> bool {
        self.state == ActivityState::Active
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn entity_creation() {
        let id = EntityId::from_raw(1);
        let entity = Entity::new(
            id,
            EntityKind::Aircraft,
            Faction::Blue,
            Position::new(10.0, 20.0),
        );
        assert_eq!(entity.id, EntityId::from_raw(1));
        assert_eq!(entity.kind, EntityKind::Aircraft);
        assert_eq!(entity.faction, Faction::Blue);
        assert!(entity.is_active());
    }

    #[test]
    fn entity_with_label() {
        let id = EntityId::from_raw(2);
        let entity = Entity::new(id, EntityKind::Ship, Faction::Red, Position::new(0.0, 0.0))
            .with_label("Destroyer-1");
        assert_eq!(entity.label, "Destroyer-1");
    }

    #[test]
    fn entity_state() {
        let id = EntityId::from_raw(3);
        let mut entity = Entity::new(
            id,
            EntityKind::Drone,
            Faction::Blue,
            Position::new(0.0, 0.0),
        );
        assert!(entity.is_active());
        entity.state = ActivityState::Degraded;
        assert!(!entity.is_active());
    }

    #[test]
    fn entity_capabilities_default() {
        let caps = Capabilities::default();
        assert!(!caps.can_emit);
        assert!(!caps.can_jam);
    }
}
