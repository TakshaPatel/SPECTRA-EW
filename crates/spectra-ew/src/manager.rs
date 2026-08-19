use spectra_core::world::World;

use crate::effect::EwEffect;

/// Manages all active EW effects and applies them each tick.
///
/// Effects are applied BEFORE ground truth is computed, so they
/// modify the world state that sensors observe.
pub struct EwManager {
    effects: Vec<Box<dyn EwEffect>>,
}

impl EwManager {
    pub fn new() -> Self {
        Self {
            effects: Vec::new(),
        }
    }

    /// Add an EW effect to be managed.
    pub fn add_effect(&mut self, effect: Box<dyn EwEffect>) {
        self.effects.push(effect);
    }

    /// Apply all active effects to the world at the given tick.
    pub fn apply_effects(&self, world: &mut World, tick: u64) {
        for effect in &self.effects {
            if effect.is_active(tick) {
                effect.apply(world, tick);
            }
        }
    }

    /// Remove effects that are no longer active.
    pub fn cleanup(&mut self, tick: u64) {
        self.effects
            .retain(|e| e.is_active(tick) || e.is_active(tick + 1));
    }

    /// Number of managed effects.
    pub fn active_count(&self, tick: u64) -> usize {
        self.effects.iter().filter(|e| e.is_active(tick)).count()
    }

    /// Total number of managed effects (active + expired).
    pub fn total_count(&self) -> usize {
        self.effects.len()
    }

    /// Remove all effects.
    pub fn clear(&mut self) {
        self.effects.clear();
    }

    /// Get all effect names for debugging.
    pub fn effect_names(&self) -> Vec<&str> {
        self.effects.iter().map(|e| e.name()).collect()
    }
}

impl Default for EwManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use spectra_core::id::EntityId;
    use spectra_core::interference::{Interference, InterferenceKind};
    use spectra_core::world::{Position, WorldConfig};

    struct TimedEffect {
        source: EntityId,
        start: u64,
        end: u64,
    }

    impl EwEffect for TimedEffect {
        fn apply(&self, world: &mut World, _tick: u64) {
            world.add_interference(Interference::new(
                EntityId::from_raw(999),
                self.source,
                InterferenceKind::NoiseJamming,
                Position::new(0.0, 0.0),
                50.0,
                30.0,
                5,
            ));
        }
        fn source(&self) -> EntityId {
            self.source
        }
        fn is_active(&self, tick: u64) -> bool {
            tick >= self.start && tick < self.end
        }
        fn name(&self) -> &str {
            "TimedEffect"
        }
    }

    #[test]
    fn manager_apply_active_effects() {
        let mut manager = EwManager::new();
        manager.add_effect(Box::new(TimedEffect {
            source: EntityId::from_raw(1),
            start: 0,
            end: 5,
        }));

        let config = WorldConfig::default();
        let mut world = World::new(config);
        manager.apply_effects(&mut world, 2);
        assert_eq!(world.active_interference.len(), 1);
    }

    #[test]
    fn manager_skips_inactive_effects() {
        let mut manager = EwManager::new();
        manager.add_effect(Box::new(TimedEffect {
            source: EntityId::from_raw(1),
            start: 5,
            end: 10,
        }));

        let config = WorldConfig::default();
        let mut world = World::new(config);
        manager.apply_effects(&mut world, 0);
        assert_eq!(world.active_interference.len(), 0);
    }

    #[test]
    fn manager_active_count() {
        let mut manager = EwManager::new();
        manager.add_effect(Box::new(TimedEffect {
            source: EntityId::from_raw(1),
            start: 0,
            end: 5,
        }));
        manager.add_effect(Box::new(TimedEffect {
            source: EntityId::from_raw(2),
            start: 3,
            end: 10,
        }));

        assert_eq!(manager.active_count(0), 1);
        assert_eq!(manager.active_count(3), 2);
        assert_eq!(manager.active_count(5), 1);
        assert_eq!(manager.active_count(10), 0);
    }

    #[test]
    fn manager_cleanup() {
        let mut manager = EwManager::new();
        manager.add_effect(Box::new(TimedEffect {
            source: EntityId::from_raw(1),
            start: 0,
            end: 3,
        }));
        manager.add_effect(Box::new(TimedEffect {
            source: EntityId::from_raw(2),
            start: 0,
            end: 10,
        }));
        assert_eq!(manager.total_count(), 2);
        manager.cleanup(5);
        assert_eq!(manager.total_count(), 1);
    }

    #[test]
    fn manager_clear() {
        let mut manager = EwManager::new();
        manager.add_effect(Box::new(TimedEffect {
            source: EntityId::from_raw(1),
            start: 0,
            end: 5,
        }));
        manager.add_effect(Box::new(TimedEffect {
            source: EntityId::from_raw(2),
            start: 0,
            end: 5,
        }));
        manager.clear();
        assert_eq!(manager.total_count(), 0);
    }

    #[test]
    fn manager_effect_names() {
        let mut manager = EwManager::new();
        manager.add_effect(Box::new(TimedEffect {
            source: EntityId::from_raw(1),
            start: 0,
            end: 5,
        }));
        assert_eq!(manager.effect_names(), vec!["TimedEffect"]);
    }
}
