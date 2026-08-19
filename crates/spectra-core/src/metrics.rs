use serde::{Deserialize, Serialize};

use crate::event::Event;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Metrics {
    pub total_observations: u64,
    pub usable_observations: u64,
    pub total_actions: u64,
    pub successful_actions: u64,
    pub interference_events: u64,
    pub signals_detected: u64,
    pub signals_lost: u64,
    pub avg_confidence: f64,
    confidence_sum: f64,
    confidence_count: u64,
}

impl Metrics {
    pub fn record_observation(&mut self, confidence: f64, usable: bool) {
        self.total_observations += 1;
        if usable {
            self.usable_observations += 1;
        }
        self.confidence_sum += confidence;
        self.confidence_count += 1;
        self.avg_confidence = self.confidence_sum / self.confidence_count as f64;
    }

    pub fn record_action(&mut self, success: bool) {
        self.total_actions += 1;
        if success {
            self.successful_actions += 1;
        }
    }

    pub fn record_event(&mut self, event: &Event) {
        use crate::event::EventKind;
        match event.kind {
            EventKind::SignalDetected => self.signals_detected += 1,
            EventKind::SignalLost => self.signals_lost += 1,
            EventKind::InterferenceStarted | EventKind::InterferenceEnded => {
                self.interference_events += 1;
            }
            _ => {}
        }
    }

    pub fn observation_rate(&self) -> f64 {
        if self.total_observations == 0 {
            return 0.0;
        }
        self.usable_observations as f64 / self.total_observations as f64
    }

    pub fn action_success_rate(&self) -> f64 {
        if self.total_actions == 0 {
            return 0.0;
        }
        self.successful_actions as f64 / self.total_actions as f64
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::event::{Event, EventKind};
    use crate::id::{EntityId, EventId};

    #[test]
    fn metrics_observation_tracking() {
        let mut m = Metrics::default();
        m.record_observation(0.8, true);
        m.record_observation(0.2, false);
        assert_eq!(m.total_observations, 2);
        assert_eq!(m.usable_observations, 1);
        assert!((m.observation_rate() - 0.5).abs() < f64::EPSILON);
    }

    #[test]
    fn metrics_action_tracking() {
        let mut m = Metrics::default();
        m.record_action(true);
        m.record_action(true);
        m.record_action(false);
        assert_eq!(m.total_actions, 3);
        assert_eq!(m.successful_actions, 2);
        assert!((m.action_success_rate() - 2.0 / 3.0).abs() < f64::EPSILON);
    }

    #[test]
    fn metrics_event_tracking() {
        let mut m = Metrics::default();
        let actor = EntityId::from_raw(1);
        let e1 = Event::new(EventId::from_raw(1), EventKind::SignalDetected, 0, actor);
        let e2 = Event::new(EventId::from_raw(2), EventKind::SignalDetected, 0, actor);
        let e3 = Event::new(EventId::from_raw(3), EventKind::SignalLost, 1, actor);
        m.record_event(&e1);
        m.record_event(&e2);
        m.record_event(&e3);
        assert_eq!(m.signals_detected, 2);
        assert_eq!(m.signals_lost, 1);
    }

    #[test]
    fn metrics_confidence_average() {
        let mut m = Metrics::default();
        m.record_observation(0.6, true);
        m.record_observation(0.8, true);
        assert!((m.avg_confidence - 0.7).abs() < f64::EPSILON);
    }

    #[test]
    fn metrics_empty() {
        let m = Metrics::default();
        assert_eq!(m.observation_rate(), 0.0);
        assert_eq!(m.action_success_rate(), 0.0);
    }
}
