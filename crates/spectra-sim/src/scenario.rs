use spectra_core::config::ScenarioFile;
use spectra_core::emitter::{Emitter, EmitterState};
use spectra_core::entity::Entity;
use spectra_core::id::IdGenerator;
use spectra_core::receiver::Receiver;
use spectra_core::world::World;

pub struct ScenarioLoader;

impl ScenarioLoader {
    pub fn load_from_yaml(yaml: &str) -> Result<World, ScenarioLoadError> {
        let scenario = ScenarioFile::load(yaml).map_err(ScenarioLoadError::Parse)?;
        Ok(Self::build_world(scenario))
    }

    pub fn load_from_file(path: &std::path::Path) -> Result<World, ScenarioLoadError> {
        let content =
            std::fs::read_to_string(path).map_err(|e| ScenarioLoadError::Io(e.to_string()))?;
        Self::load_from_yaml(&content)
    }

    fn build_world(scenario: ScenarioFile) -> World {
        let mut world = World::new(scenario.world);
        let mut ids = IdGenerator::new();

        for se in &scenario.entities {
            let entity_id = ids.next_id();
            let entity =
                Entity::new(entity_id, se.kind, se.faction, se.position).with_label(&se.label);
            world.add_entity(entity);

            for emit in &se.emits {
                let emitter_id = ids.next_id();
                let mut emitter = Emitter::new(
                    emitter_id,
                    entity_id,
                    emit.channel,
                    emit.frequency_mhz,
                    emit.category,
                    se.position,
                )
                .with_power_db(emit.power_db)
                .with_bandwidth(emit.bandwidth_mhz);

                if !se.receives {
                    emitter.state = EmitterState::Silent;
                }

                world.add_emitter(emitter);
            }

            if se.receives {
                let receiver_id = ids.next_id();
                let receiver = Receiver::new(receiver_id, entity_id, -90.0, se.position);
                world.add_receiver(receiver);
            }
        }

        world
    }
}

#[derive(Debug)]
pub enum ScenarioLoadError {
    Parse(serde_yaml::Error),
    Io(String),
}

impl std::fmt::Display for ScenarioLoadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ScenarioLoadError::Parse(e) => write!(f, "YAML parse error: {}", e),
            ScenarioLoadError::Io(e) => write!(f, "IO error: {}", e),
        }
    }
}

impl std::error::Error for ScenarioLoadError {}

#[cfg(test)]
mod tests {
    use super::*;
    use spectra_core::entity::EntityKind;
    use spectra_core::world::Faction;

    #[test]
    fn load_basic_scenario() {
        let yaml = r#"
name: test
description: test
world:
  map_width: 500.0
  map_height: 500.0
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
      x: 100.0
      y: 200.0
    label: Scout
    emits:
      - channel: 1
        frequency_mhz: 2400.0
        category: Communication
    receives: true
  - kind: Ship
    faction: Red
    position:
      x: 400.0
      y: 200.0
    label: Destroyer
    emits:
      - channel: 3
        frequency_mhz: 9000.0
        category: Radar
        power_db: 50.0
    receives: true
"#;
        let world = ScenarioLoader::load_from_yaml(yaml).unwrap();
        assert_eq!(world.entities.len(), 2);
        assert_eq!(world.emitters.len(), 2);
        assert_eq!(world.receivers.len(), 2);
        assert_eq!(world.config.max_ticks, 10);

        let blue = world.entities_by_faction(Faction::Blue);
        assert_eq!(blue.len(), 1);
        assert_eq!(blue[0].label, "Scout");
        assert_eq!(blue[0].kind, EntityKind::Aircraft);

        let red = world.entities_by_faction(Faction::Red);
        assert_eq!(red.len(), 1);
        assert_eq!(red[0].label, "Destroyer");
    }

    #[test]
    fn load_scenario_deterministic_ids() {
        let yaml = r#"
name: test
description: test
world:
  map_width: 100.0
  map_height: 100.0
  max_ticks: 5
  environment:
    propagation_loss_exponent: 2.0
    ambient_noise_db: -100.0
    weather_attenuation: 0.0
    terrain_masking: false
entities:
  - kind: Drone
    faction: Blue
    position:
      x: 0.0
      y: 0.0
    label: Drone-A
    emits:
      - channel: 1
        frequency_mhz: 2400.0
        category: Communication
    receives: true
"#;
        let w1 = ScenarioLoader::load_from_yaml(yaml).unwrap();
        let w2 = ScenarioLoader::load_from_yaml(yaml).unwrap();
        assert_eq!(w1.entities[0].id, w2.entities[0].id);
        assert_eq!(w1.emitters[0].id, w2.emitters[0].id);
    }
}
