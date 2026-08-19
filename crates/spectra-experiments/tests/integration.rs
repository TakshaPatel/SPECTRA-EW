use spectra_experiments::runner::{ExperimentConfig, ExperimentRunner};

#[test]
fn combat_scenario_produces_observations() {
    let yaml = r#"
name: Combat Test
description: Full combat scenario
world:
  map_width: 400.0
  map_height: 400.0
  max_ticks: 5
  environment:
    propagation_loss_exponent: 2.0
    ambient_noise_db: -100.0
    weather_attenuation: 0.0
    terrain_masking: false
entities:
  - kind: Aircraft
    faction: Blue
    position: { x: 100.0, y: 200.0 }
    label: Blue-1
    emits:
      - channel: 1
        frequency_mhz: 100.0
        category: Communication
        power_db: 60.0
    receives: true
  - kind: Aircraft
    faction: Blue
    position: { x: 140.0, y: 200.0 }
    label: Blue-2
    receives: true
  - kind: Aircraft
    faction: Red
    position: { x: 200.0, y: 200.0 }
    label: Red-1
    emits:
      - channel: 3
        frequency_mhz: 100.0
        category: Radar
        power_db: 60.0
      - channel: 5
        frequency_mhz: 100.0
        category: Communication
        power_db: 50.0
    receives: true
  - kind: Aircraft
    faction: Red
    position: { x: 220.0, y: 210.0 }
    label: Red-2
    emits:
      - channel: 7
        frequency_mhz: 100.0
        category: Radar
        power_db: 70.0
    receives: true
"#;
    let config = ExperimentConfig {
        name: "test".to_string(),
        seed: 42,
        ..Default::default()
    };
    let mut runner = ExperimentRunner::from_scenario(yaml, config).unwrap();

    let mut total_usable = 0u64;
    for _ in 0..5 {
        runner.tick();
        let usable = runner
            .last_observations
            .iter()
            .filter(|o| o.is_usable())
            .count();
        total_usable += usable as u64;
        eprintln!(
            "tick {}: {} observations, {} usable, metrics.obs={}",
            runner.world.tick - 1,
            runner.last_observations.len(),
            usable,
            runner.metrics.total_observations
        );
        for obs in &runner.last_observations {
            if obs.is_usable() {
                eprintln!(
                    "  observer={:?} source={:?} conf={:.3} snr_est={:.1} quality={:?}",
                    obs.observer_id,
                    obs.estimated_source_id,
                    obs.confidence,
                    obs.signal_strength_db - obs.noise_level_db,
                    obs.quality
                );
            }
        }
    }
    assert!(
        total_usable > 0,
        "Expected at least some usable observations, got {}",
        total_usable
    );
}
