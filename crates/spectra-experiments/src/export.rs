use crate::runner::AggregateMetrics;
use crate::runner::ExperimentResult;

/// Export experiment results to JSON string.
pub fn to_json(result: &ExperimentResult) -> Result<String, serde_json::Error> {
    serde_json::to_string_pretty(result)
}

/// Export aggregate metrics to JSON string.
pub fn aggregate_to_json(agg: &AggregateMetrics) -> Result<String, serde_json::Error> {
    serde_json::to_string_pretty(agg)
}

/// Export experiment results to a simple text summary.
pub fn to_text_summary(result: &ExperimentResult) -> String {
    format!(
        "Experiment: {}\n\
         Seed: {}\n\
         Final tick: {}\n\
         Total observations: {}\n\
         Usable observations: {}\n\
         Signals detected: {}\n\
         AI decisions: {}\n\
         AI errors: {} ({:.1}%)\n\
         AI avg confidence: {:.3}\n\
         Classification accuracy: {:.1}%\n\
         EW effects deployed: {}\n\
         EW utilization: {:.1}%\n\
         Spectrum control: {}\n\
         Events: {}",
        result.config.name,
        result.config.seed,
        result.final_tick,
        result.core_metrics.total_observations,
        result.core_metrics.usable_observations,
        result.core_metrics.signals_detected,
        result.metrics.ai_decisions,
        result.metrics.ai_errors,
        result.metrics.ai_error_rate() * 100.0,
        result.metrics.avg_ai_confidence,
        result.metrics.classification_accuracy() * 100.0,
        result.metrics.ew_effects_deployed,
        result.metrics.ew_utilization_rate() * 100.0,
        result.metrics.spectrum_control_achieved,
        result.events_count,
    )
}

/// Export aggregate results to a text summary.
pub fn aggregate_to_text(agg: &AggregateMetrics) -> String {
    format!(
        "Aggregate Results ({} runs)\n\
         Avg ticks: {:.1}\n\
         Total observations: {}\n\
         Total signals detected: {}\n\
         Avg AI confidence: {:.3}\n\
         Avg AI error rate: {:.1}%\n\
         Avg classification accuracy: {:.1}%\n\
         Avg EW utilization: {:.1}%\n\
         Spectrum control rate: {:.1}%",
        agg.run_count,
        agg.avg_ticks,
        agg.total_observations,
        agg.total_signals_detected,
        agg.avg_ai_confidence,
        agg.avg_ai_error_rate * 100.0,
        agg.avg_classification_accuracy * 100.0,
        agg.avg_ew_utilization * 100.0,
        agg.spectrum_control_rate * 100.0,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::metrics::ExperimentMetrics;
    use spectra_core::metrics::Metrics;

    #[test]
    fn text_summary_generation() {
        let mut em = ExperimentMetrics::new();
        em.record_ai_decision(0.8);
        em.record_correct_classification();

        let result = ExperimentResult {
            config: crate::runner::ExperimentConfig {
                name: "test".to_string(),
                seed: 42,
                ..Default::default()
            },
            metrics: em,
            core_metrics: Metrics::default(),
            final_tick: 10,
            events_count: 5,
        };

        let text = to_text_summary(&result);
        assert!(text.contains("test"));
        assert!(text.contains("10"));
    }

    #[test]
    fn aggregate_text_generation() {
        let agg = AggregateMetrics {
            run_count: 3,
            avg_ticks: 10.0,
            ..Default::default()
        };
        let text = aggregate_to_text(&agg);
        assert!(text.contains("3 runs"));
    }
}
