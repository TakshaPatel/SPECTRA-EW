use spectra_core::action::EwAction;
use spectra_core::observation::Observation;

#[derive(Debug, Clone)]
pub struct Decision {
    pub action: EwAction,
    pub confidence: f64,
    pub explanation: String,
    pub alternatives: Vec<EwAction>,
}

pub trait DecisionPolicy {
    fn evaluate(&self, observations: &[Observation]) -> Decision;
}

#[cfg(test)]
mod tests {
    use super::*;
    use spectra_core::observation::Observation;

    struct DummyPolicy;

    impl DecisionPolicy for DummyPolicy {
        fn evaluate(&self, _observations: &[Observation]) -> Decision {
            Decision {
                action: EwAction::Observe,
                confidence: 0.5,
                explanation: "dummy".to_string(),
                alternatives: vec![],
            }
        }
    }

    #[test]
    fn policy_returns_decision() {
        let policy = DummyPolicy;
        let decision = policy.evaluate(&[]);
        assert_eq!(decision.action, EwAction::Observe);
        assert!((decision.confidence - 0.5).abs() < f64::EPSILON);
    }
}
