use rand::SeedableRng;
use rand_chacha::ChaCha8Rng;
use spectra_core::event::Event;
use spectra_core::metrics::Metrics;
use spectra_core::world::World;
use spectra_sensors::observation_processor::ObservationProcessor;

use crate::scenario::ScenarioLoader;
use crate::tick;

pub struct SimulationEngine {
    pub world: World,
    pub processor: ObservationProcessor,
    pub events: Vec<Event>,
    pub metrics: Metrics,
    _rng: ChaCha8Rng,
    seed: u64,
}

impl SimulationEngine {
    pub fn from_scenario(yaml: &str, seed: u64) -> Result<Self, String> {
        let world = ScenarioLoader::load_from_yaml(yaml)
            .map_err(|e| format!("Failed to load scenario: {}", e))?;
        Ok(Self::new(world, seed))
    }

    pub fn new(world: World, seed: u64) -> Self {
        let _rng = ChaCha8Rng::seed_from_u64(seed);
        let mut processor = ObservationProcessor::new(seed);
        processor.build_from_world(&world);

        Self {
            world,
            processor,
            events: Vec::new(),
            metrics: Metrics::default(),
            _rng,
            seed,
        }
    }

    pub fn seed(&self) -> u64 {
        self.seed
    }

    pub fn run_to_completion(&mut self) {
        while !self.world.is_complete() {
            self.step();
        }
    }

    pub fn step(&mut self) {
        tick::process_tick(
            &mut self.world,
            &mut self.processor,
            &mut self.events,
            &mut self.metrics,
        );
        self.world.advance_tick();
    }

    pub fn step_n(&mut self, n: u64) {
        for _ in 0..n {
            if self.world.is_complete() {
                break;
            }
            self.step();
        }
    }

    pub fn run_with_rng(world: World, seed: u64, steps: u64) -> (World, Vec<Event>, Metrics) {
        let mut engine = Self::new(world, seed);
        engine.step_n(steps);
        (engine.world, engine.events, engine.metrics)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use spectra_core::entity::{Entity, EntityKind};
    use spectra_core::id::IdGenerator;
    use spectra_core::world::{Faction, Position, WorldConfig};

    fn make_simple_world() -> World {
        let mut ids = IdGenerator::new();
        let config = WorldConfig {
            max_ticks: 20,
            ..Default::default()
        };
        let mut world = World::new(config);

        let blue_id = ids.next_id();
        world.add_entity(Entity::new(
            blue_id,
            EntityKind::Aircraft,
            Faction::Blue,
            Position::new(0.0, 0.0),
        ));

        let red_id = ids.next_id();
        world.add_entity(Entity::new(
            red_id,
            EntityKind::Aircraft,
            Faction::Red,
            Position::new(200.0, 0.0),
        ));

        let em_id = ids.next_id();
        world.add_emitter(spectra_core::emitter::Emitter::new(
            em_id,
            red_id,
            1,
            2400.0,
            spectra_core::emitter::SignalCategory::Communication,
            Position::new(200.0, 0.0),
        ));

        let rx_id = ids.next_id();
        world.add_receiver(spectra_core::receiver::Receiver::new(
            rx_id,
            blue_id,
            -90.0,
            Position::new(0.0, 0.0),
        ));

        world
    }

    #[test]
    fn engine_run_to_completion() {
        let world = make_simple_world();
        let mut engine = SimulationEngine::new(world, 42);
        engine.run_to_completion();
        assert!(engine.world.is_complete());
    }

    #[test]
    fn engine_deterministic_replay() {
        let world = make_simple_world();
        let (w1, e1, m1) = SimulationEngine::run_with_rng(world.clone(), 12345, 10);
        let (w2, e2, m2) = SimulationEngine::run_with_rng(world.clone(), 12345, 10);

        assert_eq!(w1.tick, w2.tick);
        assert_eq!(w1.entities.len(), w2.entities.len());
        assert_eq!(e1.len(), e2.len());
        assert_eq!(m1.total_observations, m2.total_observations);
    }

    #[test]
    fn engine_different_seeds_differ() {
        let yaml = r#"
name: test
description: test
world:
  map_width: 300.0
  map_height: 300.0
  max_ticks: 10
  environment:
    propagation_loss_exponent: 2.0
    ambient_noise_db: -100.0
    weather_attenuation: 0.0
    terrain_masking: false
entities:
  - kind: Aircraft
    faction: Blue
    position:
      x: 0.0
      y: 0.0
    label: Blue-1
    receives: true
  - kind: Aircraft
    faction: Red
    position:
      x: 50.0
      y: 0.0
    label: Red-1
    emits:
      - channel: 1
        frequency_mhz: 2400.0
        category: Communication
        power_db: 40.0
    receives: true
"#;
        let w1 = ScenarioLoader::load_from_yaml(yaml).unwrap();
        let w2 = ScenarioLoader::load_from_yaml(yaml).unwrap();
        let (_, e1, _) = SimulationEngine::run_with_rng(w1, 1, 10);
        let (_, e2, _) = SimulationEngine::run_with_rng(w2, 2, 10);
        assert_eq!(e1.len(), e2.len());
    }

    #[test]
    fn engine_step_by_step() {
        let world = make_simple_world();
        let mut engine = SimulationEngine::new(world, 42);
        assert_eq!(engine.world.tick, 0);
        engine.step();
        assert_eq!(engine.world.tick, 1);
        engine.step_n(5);
        assert_eq!(engine.world.tick, 6);
    }

    #[test]
    fn engine_from_scenario() {
        let yaml = r#"
name: engine-test
description: test
world:
  map_width: 300.0
  map_height: 300.0
  max_ticks: 5
  environment:
    propagation_loss_exponent: 2.0
    ambient_noise_db: -100.0
    weather_attenuation: 0.0
    terrain_masking: false
entities:
  - kind: Aircraft
    faction: Blue
    position:
      x: 0.0
      y: 0.0
    label: Blue-1
    receives: true
  - kind: Aircraft
    faction: Red
    position:
      x: 300.0
      y: 0.0
    label: Red-1
    emits:
      - channel: 1
        frequency_mhz: 2400.0
        category: Communication
        power_db: 40.0
    receives: true
"#;
        let mut engine = SimulationEngine::from_scenario(yaml, 99).unwrap();
        engine.run_to_completion();
        assert!(engine.world.is_complete());
        assert!(engine.metrics.total_observations > 0);
    }
}
