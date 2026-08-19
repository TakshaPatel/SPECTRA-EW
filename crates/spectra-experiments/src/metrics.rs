use spectra_core::metrics::Metrics;

/// Experiment-level metrics that go beyond per-tick metrics.
/// Tracks AI decision quality, EW effectiveness, and scenario outcomes.
#[derive(Debug, Clone, Default, Serialize)]
pub struct ExperimentMetrics {
    /// Total ticks simulated.
    pub total_ticks: u64,
    /// Number of AI decisions made.
    pub ai_decisions: u64,
    /// Number of AI errors (missed, misclassified, delayed, false).
    pub ai_errors: u64,
    /// Number of EW effects deployed.
    pub ew_effects_deployed: u64,
    /// Number of EW effects that expired naturally.
    pub ew_effects_expired: u64,
    /// Average AI confidence across all decisions.
    pub avg_ai_confidence: f64,
    ai_confidence_sum: f64,
    /// Per-tick metrics snapshots.
    pub tick_metrics: Vec<TickSnapshot>,
    /// Decision latency (ticks between detection and action).
    pub avg_decision_latency: f64,
    decision_latency_sum: u64,
    decision_latency_count: u64,
    /// Signals correctly identified.
    pub correct_classifications: u64,
    /// Signals misclassified.
    pub misclassifications: u64,
    /// Whether the scenario achieved "spectrum control" (Red signals suppressed).
    pub spectrum_control_achieved: bool,
}

use serde::Serialize;

/// Snapshot of metrics at a single tick.
#[derive(Debug, Clone, Default, Serialize)]
pub struct TickSnapshot {
    pub tick: u64,
    pub observations: u64,
    pub usable: u64,
    pub signals_detected: u64,
    pub interference_count: u64,
    pub avg_confidence: f64,
}

impl ExperimentMetrics {
    pub fn new() -> Self {
        Self::default()
    }

    /// Record a tick snapshot.
    pub fn record_tick(&mut self, tick: u64, metrics: &Metrics, interference_count: usize) {
        self.total_ticks = tick + 1;
        self.tick_metrics.push(TickSnapshot {
            tick,
            observations: metrics.total_observations,
            usable: metrics.usable_observations,
            signals_detected: metrics.signals_detected,
            interference_count: interference_count as u64,
            avg_confidence: metrics.avg_confidence,
        });
    }

    /// Record an AI decision.
    pub fn record_ai_decision(&mut self, confidence: f64) {
        self.ai_decisions += 1;
        self.ai_confidence_sum += confidence;
        self.avg_ai_confidence = self.ai_confidence_sum / self.ai_decisions as f64;
    }

    /// Record an AI error.
    pub fn record_ai_error(&mut self) {
        self.ai_errors += 1;
    }

    /// Record a decision latency.
    pub fn record_decision_latency(&mut self, latency: u64) {
        self.decision_latency_sum += latency;
        self.decision_latency_count += 1;
        self.avg_decision_latency =
            self.decision_latency_sum as f64 / self.decision_latency_count as f64;
    }

    /// Record a correct classification.
    pub fn record_correct_classification(&mut self) {
        self.correct_classifications += 1;
    }

    /// Record a misclassification.
    pub fn record_misclassification(&mut self) {
        self.misclassifications += 1;
    }

    /// Record an EW effect deployment.
    pub fn record_ew_effect_deployed(&mut self) {
        self.ew_effects_deployed += 1;
    }

    /// Record an EW effect expiration.
    pub fn record_ew_effect_expired(&mut self) {
        self.ew_effects_expired += 1;
    }

    /// Mark spectrum control as achieved.
    pub fn set_spectrum_control(&mut self) {
        self.spectrum_control_achieved = true;
    }

    /// Classification accuracy.
    pub fn classification_accuracy(&self) -> f64 {
        let total = self.correct_classifications + self.misclassifications;
        if total == 0 {
            return 0.0;
        }
        self.correct_classifications as f64 / total as f64
    }

    /// AI error rate.
    pub fn ai_error_rate(&self) -> f64 {
        if self.ai_decisions == 0 {
            return 0.0;
        }
        self.ai_errors as f64 / self.ai_decisions as f64
    }

    /// EW utilization rate (deployed / total decisions).
    pub fn ew_utilization_rate(&self) -> f64 {
        if self.ai_decisions == 0 {
            return 0.0;
        }
        self.ew_effects_deployed as f64 / self.ai_decisions as f64
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn metrics_tick_recording() {
        let mut m = ExperimentMetrics::new();
        let core = Metrics::default();
        m.record_tick(0, &core, 0);
        assert_eq!(m.total_ticks, 1);
        assert_eq!(m.tick_metrics.len(), 1);
    }

    #[test]
    fn metrics_ai_decision_tracking() {
        let mut m = ExperimentMetrics::new();
        m.record_ai_decision(0.8);
        m.record_ai_decision(0.6);
        assert_eq!(m.ai_decisions, 2);
        assert!((m.avg_ai_confidence - 0.7).abs() < f64::EPSILON);
    }

    #[test]
    fn metrics_ai_error_rate() {
        let mut m = ExperimentMetrics::new();
        m.record_ai_decision(0.8);
        m.record_ai_error();
        m.record_ai_decision(0.6);
        assert!((m.ai_error_rate() - 0.5).abs() < f64::EPSILON);
    }

    #[test]
    fn metrics_classification_accuracy() {
        let mut m = ExperimentMetrics::new();
        m.record_correct_classification();
        m.record_correct_classification();
        m.record_misclassification();
        assert!((m.classification_accuracy() - 2.0 / 3.0).abs() < f64::EPSILON);
    }

    #[test]
    fn metrics_decision_latency() {
        let mut m = ExperimentMetrics::new();
        m.record_decision_latency(2);
        m.record_decision_latency(4);
        assert!((m.avg_decision_latency - 3.0).abs() < f64::EPSILON);
    }

    #[test]
    fn metrics_ew_utilization() {
        let mut m = ExperimentMetrics::new();
        m.record_ai_decision(0.8);
        m.record_ai_decision(0.6);
        m.record_ew_effect_deployed();
        assert!((m.ew_utilization_rate() - 0.5).abs() < f64::EPSILON);
    }

    #[test]
    fn metrics_empty() {
        let m = ExperimentMetrics::new();
        assert_eq!(m.ai_error_rate(), 0.0);
        assert_eq!(m.classification_accuracy(), 0.0);
        assert_eq!(m.ew_utilization_rate(), 0.0);
    }
}
