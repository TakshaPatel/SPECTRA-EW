use serde::{Deserialize, Serialize};

use crate::world::WorldConfig;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScenarioFile {
    pub name: String,
    pub description: String,
    pub world: WorldConfig,
    pub entities: Vec<ScenarioEntity>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScenarioEntity {
    pub kind: crate::entity::EntityKind,
    pub faction: crate::world::Faction,
    pub position: crate::world::Position,
    pub label: String,
    #[serde(default)]
    pub emits: Vec<ScenarioEmitter>,
    #[serde(default = "default_true")]
    pub receives: bool,
}

fn default_true() -> bool {
    true
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScenarioEmitter {
    pub channel: u32,
    pub frequency_mhz: f64,
    pub category: crate::emitter::SignalCategory,
    #[serde(default = "default_power")]
    pub power_db: f64,
    #[serde(default = "default_bandwidth")]
    pub bandwidth_mhz: f64,
}

fn default_power() -> f64 {
    30.0
}

fn default_bandwidth() -> f64 {
    1.0
}

impl ScenarioFile {
    pub fn load(yaml: &str) -> Result<Self, serde_yaml::Error> {
        serde_yaml::from_str(yaml)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scenario_yaml_roundtrip() {
        let yaml = r#"
name: test
description: test scenario
world:
  map_width: 500.0
  map_height: 500.0
  max_ticks: 100
  environment:
    propagation_loss_exponent: 2.5
    ambient_noise_db: -95.0
    weather_attenuation: 0.0
    terrain_masking: false
entities:
  - kind: Aircraft
    faction: Blue
    position:
      x: 100.0
      y: 200.0
    label: Scout-1
    emits:
      - channel: 1
        frequency_mhz: 2400.0
        category: Communication
        bandwidth_mhz: 2.0
    receives: true
"#;
        let scenario = ScenarioFile::load(yaml).unwrap();
        assert_eq!(scenario.name, "test");
        assert_eq!(scenario.entities.len(), 1);
        assert_eq!(scenario.entities[0].label, "Scout-1");
        assert_eq!(scenario.entities[0].emits.len(), 1);
        assert_eq!(scenario.entities[0].emits[0].bandwidth_mhz, 2.0);
    }

    #[test]
    fn scenario_yaml_defaults() {
        let yaml = r#"
name: defaults
description: test defaults
world:
  map_width: 100.0
  map_height: 100.0
  max_ticks: 10
  environment:
    propagation_loss_exponent: 2.0
    ambient_noise_db: -100.0
    weather_attenuation: 0.0
    terrain_masking: false
entities:
  - kind: Drone
    faction: Red
    position:
      x: 0.0
      y: 0.0
    label: Drone-1
    emits:
      - channel: 2
        frequency_mhz: 5800.0
        category: Radar
"#;
        let scenario = ScenarioFile::load(yaml).unwrap();
        assert_eq!(scenario.entities[0].emits[0].power_db, 30.0);
        assert_eq!(scenario.entities[0].emits[0].bandwidth_mhz, 1.0);
        assert!(scenario.entities[0].receives);
    }
}
