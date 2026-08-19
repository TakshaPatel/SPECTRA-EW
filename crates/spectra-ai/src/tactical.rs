use rand::Rng;
use rand::SeedableRng;
use rand_chacha::ChaCha8Rng;
use spectra_core::action::EwAction;
use spectra_core::id::EntityId;
use spectra_core::observation::{Observation, ObservationQuality};

use crate::decision::DecisionError;
use crate::fog::FogOfWar;
use crate::policy::{Decision, DecisionPolicy};

/// Configuration for the tactical AI.
#[derive(Debug, Clone, serde::Serialize)]
pub struct AiConfig {
    /// Probability of missing a signal per tick.
    pub miss_rate: f64,
    /// Probability of misclassifying a signal per tick.
    pub misclassify_rate: f64,
    /// Probability of a false detection per tick.
    pub false_detection_rate: f64,
    /// Ticks of delay before acting on a detection.
    pub response_delay: u64,
    /// Confidence threshold below which the AI defaults to Observe.
    pub observe_threshold: f64,
    /// Confidence threshold for deploying countermeasures.
    pub countermeasure_threshold: f64,
}

impl Default for AiConfig {
    fn default() -> Self {
        Self {
            miss_rate: 0.05,
            misclassify_rate: 0.03,
            false_detection_rate: 0.02,
            response_delay: 1,
            observe_threshold: 0.15,
            countermeasure_threshold: 0.35,
        }
    }
}

/// Tactical AI that makes EW decisions based on sensor observations.
///
/// Never sees raw World state — only sensor-processed observations.
/// Simulates decision errors and fog of war.
pub struct TacticalAI {
    pub config: AiConfig,
    pub fog: FogOfWar,
    rng: ChaCha8Rng,
    pending_actions: std::collections::VecDeque<(u64, EwAction)>,
    errors: Vec<DecisionError>,
    observer_id: EntityId,
}

impl TacticalAI {
    pub fn new(observer_id: EntityId, seed: u64) -> Self {
        Self {
            config: AiConfig::default(),
            fog: FogOfWar::default(),
            rng: ChaCha8Rng::seed_from_u64(seed),
            pending_actions: std::collections::VecDeque::new(),
            errors: Vec::new(),
            observer_id,
        }
    }

    pub fn with_config(mut self, config: AiConfig) -> Self {
        self.config = config;
        self
    }

    pub fn with_fog(mut self, fog: FogOfWar) -> Self {
        self.fog = fog;
        self
    }

    /// Process new observations, potentially generating decision errors.
    pub fn process_observations(&mut self, observations: &[Observation], tick: u64) {
        self.fog.record_observations(observations);

        for obs in observations {
            if obs.observer_id != self.observer_id {
                continue;
            }
            if !obs.is_usable() {
                continue;
            }

            // Simulate miss: might skip a real detection
            if self.rng.gen_range(0.0..1.0) < self.config.miss_rate {
                self.errors.push(DecisionError::MissedSignal {
                    observer_id: self.observer_id,
                    tick,
                });
                continue;
            }

            // Simulate false detection: might hallucinate a signal
            if self.rng.gen_range(0.0..1.0) < self.config.false_detection_rate {
                self.errors.push(DecisionError::FalseDetection {
                    observer_id: self.observer_id,
                    tick,
                });
            }
        }
    }

    /// Get the best response for the current observations.
    pub fn best_response(&self, observations: &[Observation]) -> EwAction {
        // Filter to only usable observations from our observer
        let relevant: Vec<&Observation> = observations
            .iter()
            .filter(|o| o.observer_id == self.observer_id && o.is_usable())
            .collect();

        if relevant.is_empty() {
            return EwAction::Observe;
        }

        // Find the strongest signal
        let strongest = relevant
            .iter()
            .max_by(|a, b| a.confidence.partial_cmp(&b.confidence).unwrap())
            .unwrap();

        // Rule-based decision
        if strongest.confidence < self.config.observe_threshold {
            EwAction::Observe
        } else if strongest.confidence < self.config.countermeasure_threshold {
            // Medium confidence: monitor or protect
            if strongest.quality == ObservationQuality::Noisy
                || strongest.quality == ObservationQuality::Degraded
            {
                EwAction::ProtectChannel
            } else {
                EwAction::Monitor
            }
        } else {
            // High confidence: take action
            match strongest.quality {
                ObservationQuality::Clear => EwAction::SuppressSignal,
                ObservationQuality::Noisy => EwAction::ProtectChannel,
                ObservationQuality::Degraded => EwAction::ChangeChannel,
                ObservationQuality::Unreliable => EwAction::Observe,
            }
        }
    }

    /// Get all accumulated errors.
    pub fn errors(&self) -> &[DecisionError] {
        &self.errors
    }

    /// Clear error history.
    pub fn clear_errors(&mut self) {
        self.errors.clear();
    }

    /// Get pending delayed actions that are ready to execute.
    pub fn ready_actions(&mut self, tick: u64) -> Vec<EwAction> {
        let mut ready = Vec::new();
        while let Some(&(action_tick, action)) = self.pending_actions.front() {
            if action_tick <= tick {
                self.pending_actions.pop_front();
                ready.push(action);
            } else {
                break;
            }
        }
        ready
    }

    /// Queue an action with a response delay.
    pub fn queue_action(&mut self, action: EwAction, current_tick: u64) {
        let execute_tick = current_tick + self.config.response_delay;
        self.pending_actions.push_back((execute_tick, action));
    }

    /// The entity this AI represents.
    pub fn observer_id(&self) -> EntityId {
        self.observer_id
    }
}

impl DecisionPolicy for TacticalAI {
    fn evaluate(&self, observations: &[Observation]) -> Decision {
        let action = self.best_response(observations);

        let relevant: Vec<&Observation> = observations
            .iter()
            .filter(|o| o.observer_id == self.observer_id && o.is_usable())
            .collect();

        let avg_confidence = if relevant.is_empty() {
            0.0
        } else {
            relevant.iter().map(|o| o.confidence).sum::<f64>() / relevant.len() as f64
        };

        let mut alternatives = Vec::new();
        if action != EwAction::Observe {
            alternatives.push(EwAction::Observe);
        }
        if action != EwAction::Monitor {
            alternatives.push(EwAction::Monitor);
        }

        Decision {
            action,
            confidence: avg_confidence,
            explanation: format!(
                "Evaluated {} observations, chose {}",
                relevant.len(),
                action
            ),
            alternatives,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_obs(
        observer: u64,
        tick: u64,
        confidence: f64,
        quality: ObservationQuality,
    ) -> Observation {
        let mut obs = Observation::new(EntityId::from_raw(observer), tick);
        obs.confidence = confidence;
        obs.quality = quality;
        obs
    }

    #[test]
    fn ai_observe_when_no_observations() {
        let ai = TacticalAI::new(EntityId::from_raw(1), 42);
        let decision = ai.evaluate(&[]);
        assert_eq!(decision.action, EwAction::Observe);
    }

    #[test]
    fn ai_monitor_medium_confidence() {
        let ai = TacticalAI::new(EntityId::from_raw(1), 42);
        let obs = make_obs(1, 0, 0.31, ObservationQuality::Clear);
        let decision = ai.evaluate(&[obs]);
        assert_eq!(decision.action, EwAction::Monitor);
    }

    #[test]
    fn ai_suppress_high_confidence_clear() {
        let ai = TacticalAI::new(EntityId::from_raw(1), 42);
        let obs = make_obs(1, 0, 0.8, ObservationQuality::Clear);
        let decision = ai.evaluate(&[obs]);
        assert_eq!(decision.action, EwAction::SuppressSignal);
    }

    #[test]
    fn ai_protect_noisy_signal() {
        let ai = TacticalAI::new(EntityId::from_raw(1), 42);
        let obs = make_obs(1, 0, 0.7, ObservationQuality::Noisy);
        let decision = ai.evaluate(&[obs]);
        assert_eq!(decision.action, EwAction::ProtectChannel);
    }

    #[test]
    fn ai_change_channel_degraded() {
        let ai = TacticalAI::new(EntityId::from_raw(1), 42);
        let obs = make_obs(1, 0, 0.8, ObservationQuality::Degraded);
        let decision = ai.evaluate(&[obs]);
        assert_eq!(decision.action, EwAction::ChangeChannel);
    }

    #[test]
    fn ai_miss_rate() {
        let mut ai = TacticalAI::new(EntityId::from_raw(1), 42).with_config(AiConfig {
            miss_rate: 1.0, // Always miss
            ..Default::default()
        });
        let obs = make_obs(1, 0, 0.8, ObservationQuality::Clear);
        ai.process_observations(&[obs], 0);
        assert_eq!(ai.errors().len(), 1);
        assert!(matches!(ai.errors()[0], DecisionError::MissedSignal { .. }));
    }

    #[test]
    fn ai_false_detection_rate() {
        let mut ai = TacticalAI::new(EntityId::from_raw(1), 42).with_config(AiConfig {
            false_detection_rate: 1.0, // Always false detect
            ..Default::default()
        });
        let obs = make_obs(1, 0, 0.8, ObservationQuality::Clear);
        ai.process_observations(&[obs], 0);
        assert!(ai
            .errors()
            .iter()
            .any(|e| matches!(e, DecisionError::FalseDetection { .. })));
    }

    #[test]
    fn ai_response_delay() {
        let mut ai = TacticalAI::new(EntityId::from_raw(1), 42).with_config(AiConfig {
            response_delay: 3,
            ..Default::default()
        });
        ai.queue_action(EwAction::SuppressSignal, 5);
        assert!(ai.ready_actions(5).is_empty());
        assert!(ai.ready_actions(7).is_empty());
        let ready = ai.ready_actions(8);
        assert_eq!(ready.len(), 1);
        assert_eq!(ready[0], EwAction::SuppressSignal);
    }

    #[test]
    fn ai_fog_integration() {
        let mut ai = TacticalAI::new(EntityId::from_raw(1), 42);
        let obs = make_obs(1, 0, 0.8, ObservationQuality::Clear);
        ai.process_observations(&[obs], 0);
        assert!(ai.fog.is_fresh(EntityId::from_raw(1), 0));
    }

    #[test]
    fn ai_skips_other_observers() {
        let mut ai = TacticalAI::new(EntityId::from_raw(1), 42);
        let obs = make_obs(2, 0, 0.8, ObservationQuality::Clear); // observer 2, not 1
        ai.process_observations(&[obs], 0);
        assert!(ai.errors().is_empty());
    }

    #[test]
    fn ai_clear_errors() {
        let mut ai = TacticalAI::new(EntityId::from_raw(1), 42).with_config(AiConfig {
            miss_rate: 1.0,
            ..Default::default()
        });
        let obs = make_obs(1, 0, 0.8, ObservationQuality::Clear);
        ai.process_observations(&[obs], 0);
        assert!(!ai.errors().is_empty());
        ai.clear_errors();
        assert!(ai.errors().is_empty());
    }
}
