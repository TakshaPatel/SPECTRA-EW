use spectra_ai::policy::{Decision, DecisionPolicy};
use spectra_ai::tactical::{AiConfig, TacticalAI};
use spectra_core::action::EwAction;
use spectra_core::id::EntityId;
use spectra_core::metrics::Metrics;
use spectra_core::world::World;
use spectra_ew::effect::EwEffect;
use spectra_ew::jamming::{
    ChannelFlooding, DeceptionConfig, DeceptionJamming, NoiseJamming, SensorConfusion,
};
use spectra_ew::manager::EwManager;
use spectra_sensors::observation_processor::ObservationProcessor;

use crate::metrics::ExperimentMetrics;

/// Configuration for a single experiment run.
#[derive(Debug, Clone, serde::Serialize)]
pub struct ExperimentConfig {
    /// Name of this experiment.
    pub name: String,
    /// Seed for deterministic replay.
    pub seed: u64,
    /// AI configuration.
    pub ai_config: AiConfig,
    /// Whether to apply AI decisions as EW effects.
    pub enable_ew_effects: bool,
}

impl Default for ExperimentConfig {
    fn default() -> Self {
        Self {
            name: "default".to_string(),
            seed: 42,
            ai_config: AiConfig::default(),
            enable_ew_effects: true,
        }
    }
}

/// Result of a single experiment run.
#[derive(Debug, Clone, serde::Serialize)]
pub struct ExperimentResult {
    pub config: ExperimentConfig,
    pub metrics: ExperimentMetrics,
    pub core_metrics: Metrics,
    pub final_tick: u64,
    pub events_count: usize,
}

/// Runs a single simulation with AI + EW integration.
pub struct ExperimentRunner {
    pub world: World,
    pub processor: ObservationProcessor,
    pub ai: TacticalAI,
    pub ew_manager: EwManager,
    pub events: Vec<spectra_core::event::Event>,
    pub metrics: Metrics,
    pub experiment_metrics: ExperimentMetrics,
    pub last_observations: Vec<spectra_core::observation::Observation>,
    config: ExperimentConfig,
}

impl ExperimentRunner {
    /// Create a runner from a world and config.
    pub fn new(world: World, config: ExperimentConfig) -> Self {
        let seed = config.seed;
        let observer_id = find_blue_receiver(&world);
        let mut processor = ObservationProcessor::new(seed);
        processor.build_from_world(&world);

        let ai = TacticalAI::new(observer_id, seed).with_config(config.ai_config.clone());

        Self {
            world,
            processor,
            ai,
            ew_manager: EwManager::new(),
            events: Vec::new(),
            metrics: Metrics::default(),
            experiment_metrics: ExperimentMetrics::new(),
            last_observations: Vec::new(),
            config,
        }
    }

    /// Create from YAML scenario.
    pub fn from_scenario(yaml: &str, config: ExperimentConfig) -> Result<Self, String> {
        let world = spectra_sim::scenario::ScenarioLoader::load_from_yaml(yaml)
            .map_err(|e| format!("Failed to load scenario: {}", e))?;
        Ok(Self::new(world, config))
    }

    /// Run one tick of the simulation.
    pub fn tick(&mut self) {
        let tick = self.world.tick;

        // 1. Apply EW effects (modifies world before ground truth)
        if self.config.enable_ew_effects {
            self.ew_manager.apply_effects(&mut self.world, tick);
        }

        // 2. Tick interference durations
        let expired: Vec<_> = self
            .world
            .active_interference
            .iter()
            .filter(|ix| ix.remaining_ticks == 1)
            .map(|ix| (ix.id, ix.kind, ix.intensity_db))
            .collect();
        self.world.tick_interference();
        for (id, kind, intensity) in expired {
            self.events.push(
                spectra_core::event::Event::new(
                    spectra_core::id::EventId::from_raw(self.events.len() as u64 + 1),
                    spectra_core::event::EventKind::InterferenceEnded,
                    tick,
                    id,
                )
                .with_details(spectra_core::event::EventDetails::Interference {
                    kind,
                    intensity_db: intensity,
                }),
            );
        }

        // 3. Update emitter states
        let owner_states: std::collections::HashMap<EntityId, bool> = self
            .world
            .entities
            .iter()
            .map(|e| (e.id, e.is_active()))
            .collect();
        for emitter in &mut self.world.emitters {
            if let Some(&active) = owner_states.get(&emitter.owner_id) {
                if !active && emitter.state == spectra_core::emitter::EmitterState::Transmitting {
                    emitter.state = spectra_core::emitter::EmitterState::Silent;
                }
            }
        }

        // 4. Compute ground truth
        let ground_truths = ObservationProcessor::compute_ground_truth(&self.world);

        // 5. Process through sensor layer
        let observations = self
            .processor
            .process_with_world(&self.world, &ground_truths, tick);

        // 6. Record observations and events
        for obs in &observations {
            let usable = obs.is_usable();
            self.metrics.record_observation(obs.confidence, usable);

            if usable && obs.estimated_source_id.is_some() {
                self.events.push(
                    spectra_core::event::Event::new(
                        spectra_core::id::EventId::from_raw(self.events.len() as u64 + 1),
                        spectra_core::event::EventKind::SignalDetected,
                        tick,
                        obs.observer_id,
                    )
                    .with_details(spectra_core::event::EventDetails::Signal {
                        channel: obs.estimated_channel.unwrap_or(0),
                        frequency_mhz: obs.estimated_frequency_mhz.unwrap_or(0.0),
                    }),
                );
            }
        }

        // 7. Store observations for AI
        self.last_observations = observations;

        // 8. Tick sensors
        self.processor.tick_all();

        // 9. Feed observations to AI
        self.ai.process_observations(&self.last_observations, tick);

        // 10. AI makes decision
        let decision = self.ai.evaluate(&self.last_observations);

        // 11. Record AI metrics
        self.experiment_metrics
            .record_ai_decision(decision.confidence);

        // 12. Check for AI errors at this tick
        let error_count = self.ai.errors().iter().filter(|e| e.tick() == tick).count();
        for _ in 0..error_count {
            self.experiment_metrics.record_ai_error();
        }

        // 13. Map decision to EW effect (if enabled)
        if self.config.enable_ew_effects {
            if let Some(effect) = decision_to_effect(&decision, &self.world, tick) {
                self.ew_manager.add_effect(effect);
                self.experiment_metrics.record_ew_effect_deployed();
            }
        }

        // 14. Record experiment metrics
        let interference_count = self.world.active_interference.len();
        self.experiment_metrics
            .record_tick(tick, &self.metrics, interference_count);

        // 15. Cleanup expired effects
        self.ew_manager.cleanup(tick + 1);

        // 16. Record observation event
        self.metrics.record_event(&spectra_core::event::Event::new(
            spectra_core::id::EventId::from_raw(0),
            spectra_core::event::EventKind::ObservationGenerated,
            tick,
            EntityId::ZERO,
        ));

        // 17. Advance world tick
        self.world.advance_tick();
    }

    /// Run to completion.
    pub fn run_to_completion(&mut self) {
        while !self.world.is_complete() {
            self.tick();
        }
    }

    /// Run N ticks.
    pub fn run_n(&mut self, n: u64) {
        for _ in 0..n {
            if self.world.is_complete() {
                break;
            }
            self.tick();
        }
    }

    /// Consume and return the result.
    pub fn into_result(self) -> ExperimentResult {
        ExperimentResult {
            config: self.config,
            metrics: self.experiment_metrics,
            core_metrics: self.metrics,
            final_tick: self.world.tick,
            events_count: self.events.len(),
        }
    }
}

/// Find the first Blue faction entity with a receiver.
fn find_blue_receiver(world: &World) -> EntityId {
    world
        .entities
        .iter()
        .find(|e| e.faction == spectra_core::world::Faction::Blue)
        .map(|e| e.id)
        .unwrap_or(EntityId::ZERO)
}

/// Map an AI decision to an EW effect (if applicable).
fn decision_to_effect(decision: &Decision, world: &World, tick: u64) -> Option<Box<dyn EwEffect>> {
    let source = world
        .entities
        .iter()
        .find(|e| e.faction == spectra_core::world::Faction::Blue)
        .map(|e| e.id)?;

    let red_pos = world
        .entities
        .iter()
        .find(|e| e.faction == spectra_core::world::Faction::Red)
        .map(|e| e.position)?;

    match decision.action {
        EwAction::SuppressSignal => Some(Box::new(NoiseJamming::new(
            source, red_pos, 200.0, 40.0, 5, tick,
        ))),
        EwAction::DeployDecoy => {
            let false_id = EntityId::from_raw(world.emitters.len() as u64 + 5000);
            Some(Box::new(DeceptionJamming::new(DeceptionConfig {
                source_id: source,
                false_emitter_id: false_id,
                owner_id: source,
                channel: 1,
                frequency_mhz: 2400.0,
                power_db: 30.0,
                position: red_pos,
                duration_ticks: 10,
                start_tick: tick,
            })))
        }
        EwAction::ProtectChannel => Some(Box::new(ChannelFlooding::new(
            source,
            red_pos,
            150.0,
            30.0,
            3,
            tick,
            vec![1, 2, 3],
        ))),
        EwAction::ChangeChannel => Some(Box::new(SensorConfusion::new(
            source, red_pos, 300.0, 5.0, 3, tick,
        ))),
        EwAction::Monitor => Some(Box::new(NoiseJamming::new(
            source, red_pos, 150.0, 25.0, 3, tick,
        ))),
        _ => None, // Observe, Disengage don't deploy EW
    }
}

/// Run a batch of experiments with the same scenario but different seeds.
pub fn run_batch(
    yaml: &str,
    seeds: &[u64],
    config_template: ExperimentConfig,
) -> Vec<ExperimentResult> {
    seeds
        .iter()
        .map(|&seed| {
            let mut config = config_template.clone();
            config.seed = seed;
            let mut runner =
                ExperimentRunner::from_scenario(yaml, config).expect("Failed to create runner");
            runner.run_to_completion();
            runner.into_result()
        })
        .collect()
}

/// Aggregate metrics from multiple experiment results.
pub fn aggregate_results(results: &[ExperimentResult]) -> AggregateMetrics {
    let n = results.len() as f64;
    if n == 0.0 {
        return AggregateMetrics::default();
    }

    AggregateMetrics {
        run_count: results.len(),
        avg_ticks: results.iter().map(|r| r.final_tick as f64).sum::<f64>() / n,
        avg_ai_confidence: results
            .iter()
            .map(|r| r.metrics.avg_ai_confidence)
            .sum::<f64>()
            / n,
        avg_ai_error_rate: results
            .iter()
            .map(|r| r.metrics.ai_error_rate())
            .sum::<f64>()
            / n,
        avg_classification_accuracy: results
            .iter()
            .map(|r| r.metrics.classification_accuracy())
            .sum::<f64>()
            / n,
        avg_ew_utilization: results
            .iter()
            .map(|r| r.metrics.ew_utilization_rate())
            .sum::<f64>()
            / n,
        total_observations: results
            .iter()
            .map(|r| r.core_metrics.total_observations)
            .sum(),
        total_signals_detected: results
            .iter()
            .map(|r| r.core_metrics.signals_detected)
            .sum(),
        spectrum_control_rate: results
            .iter()
            .filter(|r| r.metrics.spectrum_control_achieved)
            .count() as f64
            / n,
    }
}

/// Aggregated metrics across multiple experiment runs.
#[derive(Debug, Clone, Default, serde::Serialize)]
pub struct AggregateMetrics {
    pub run_count: usize,
    pub avg_ticks: f64,
    pub avg_ai_confidence: f64,
    pub avg_ai_error_rate: f64,
    pub avg_classification_accuracy: f64,
    pub avg_ew_utilization: f64,
    pub total_observations: u64,
    pub total_signals_detected: u64,
    pub spectrum_control_rate: f64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use spectra_core::emitter::{Emitter, SignalCategory};
    use spectra_core::entity::{Entity, EntityKind};
    use spectra_core::id::IdGenerator;
    use spectra_core::receiver::Receiver;
    use spectra_core::world::{Faction, Position, WorldConfig};

    fn make_world() -> World {
        let mut ids = IdGenerator::new();
        let config = WorldConfig {
            max_ticks: 10,
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
            Position::new(100.0, 0.0),
        ));

        let em_id = ids.next_id();
        world.add_emitter(Emitter::new(
            em_id,
            red_id,
            1,
            2400.0,
            SignalCategory::Communication,
            Position::new(100.0, 0.0),
        ));

        let rx_id = ids.next_id();
        world.add_receiver(Receiver::new(
            rx_id,
            blue_id,
            -90.0,
            Position::new(0.0, 0.0),
        ));

        world
    }

    #[test]
    fn runner_creation() {
        let world = make_world();
        let config = ExperimentConfig::default();
        let runner = ExperimentRunner::new(world, config);
        assert_eq!(runner.world.tick, 0);
    }

    #[test]
    fn runner_single_tick() {
        let world = make_world();
        let config = ExperimentConfig::default();
        let mut runner = ExperimentRunner::new(world, config);
        runner.tick();
        assert_eq!(runner.world.tick, 1);
        assert!(runner.experiment_metrics.total_ticks >= 1);
    }

    #[test]
    fn runner_run_to_completion() {
        let world = make_world();
        let config = ExperimentConfig::default();
        let mut runner = ExperimentRunner::new(world, config);
        runner.run_to_completion();
        assert!(runner.world.is_complete());
    }

    #[test]
    fn runner_experiment_metrics() {
        let world = make_world();
        let config = ExperimentConfig::default();
        let mut runner = ExperimentRunner::new(world, config);
        runner.run_to_completion();
        let result = runner.into_result();
        assert!(result.metrics.ai_decisions > 0);
        assert!(result.metrics.total_ticks > 0);
    }

    #[test]
    fn runner_deterministic() {
        let config = ExperimentConfig {
            seed: 42,
            ..Default::default()
        };
        let w1 = make_world();
        let w2 = make_world();
        let mut r1 = ExperimentRunner::new(w1, config.clone());
        let mut r2 = ExperimentRunner::new(w2, config);
        r1.run_to_completion();
        r2.run_to_completion();
        assert_eq!(r1.world.tick, r2.world.tick);
        assert_eq!(
            r1.experiment_metrics.ai_decisions,
            r2.experiment_metrics.ai_decisions
        );
    }

    #[test]
    fn decision_to_effect_suppress() {
        let world = make_world();
        let decision = Decision {
            action: EwAction::SuppressSignal,
            confidence: 0.9,
            explanation: "test".to_string(),
            alternatives: vec![],
        };
        let effect = decision_to_effect(&decision, &world, 0);
        assert!(effect.is_some());
    }

    #[test]
    fn decision_to_effect_observe() {
        let world = make_world();
        let decision = Decision {
            action: EwAction::Observe,
            confidence: 0.3,
            explanation: "test".to_string(),
            alternatives: vec![],
        };
        let effect = decision_to_effect(&decision, &world, 0);
        assert!(effect.is_none());
    }

    #[test]
    fn batch_run() {
        let yaml = r#"
name: batch-test
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
      x: 100.0
      y: 0.0
    label: Red-1
    emits:
      - channel: 1
        frequency_mhz: 2400.0
        category: Communication
        power_db: 40.0
    receives: true
"#;
        let results = run_batch(yaml, &[1, 2, 3], ExperimentConfig::default());
        assert_eq!(results.len(), 3);
        let agg = aggregate_results(&results);
        assert_eq!(agg.run_count, 3);
    }
}
