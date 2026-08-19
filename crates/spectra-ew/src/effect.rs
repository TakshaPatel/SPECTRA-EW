use spectra_core::id::EntityId;
use spectra_core::world::World;

/// Trait for all electronic warfare effects.
///
/// Effects operate ONLY on simulation state (`&mut World`).
/// No real RF, no real hardware, no real transmissions.
pub trait EwEffect {
    /// Apply this effect to the world state.
    fn apply(&self, world: &mut World, tick: u64);

    /// The entity that deployed this effect.
    fn source(&self) -> EntityId;

    /// Whether this effect is still active (has remaining duration).
    fn is_active(&self, tick: u64) -> bool;

    /// Human-readable name for debugging.
    fn name(&self) -> &str;
}

#[cfg(test)]
mod tests {
    use super::*;
    use spectra_core::id::EntityId;
    use spectra_core::interference::{Interference, InterferenceKind};
    use spectra_core::world::{Position, WorldConfig};

    struct TestEffect {
        source: EntityId,
    }

    impl EwEffect for TestEffect {
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
        fn is_active(&self, _tick: u64) -> bool {
            true
        }
        fn name(&self) -> &str {
            "TestEffect"
        }
    }

    #[test]
    fn effect_apply() {
        let config = WorldConfig::default();
        let mut world = World::new(config);
        let effect = TestEffect {
            source: EntityId::from_raw(1),
        };
        effect.apply(&mut world, 0);
        assert_eq!(world.active_interference.len(), 1);
    }

    #[test]
    fn effect_source() {
        let effect = TestEffect {
            source: EntityId::from_raw(5),
        };
        assert_eq!(effect.source(), EntityId::from_raw(5));
    }
}
