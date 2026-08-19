use spectra_core::action::EwAction;
use spectra_core::id::EntityId;

/// Represents an error in AI decision-making.
/// Simulates imperfect reasoning under fog of war.
#[derive(Debug, Clone)]
pub enum DecisionError {
    /// AI missed a signal that was present (false negative).
    MissedSignal { observer_id: EntityId, tick: u64 },
    /// AI classified a signal incorrectly.
    MisclassifiedSignal {
        observer_id: EntityId,
        tick: u64,
        actual_action: EwAction,
        chosen_action: EwAction,
    },
    /// AI responded with a delay (ticks late).
    DelayedResponse {
        observer_id: EntityId,
        intended_tick: u64,
        actual_tick: u64,
        action: EwAction,
    },
    /// AI made a decision based on stale information.
    StaleInformation {
        observer_id: EntityId,
        tick: u64,
        staleness_ticks: u64,
    },
    /// AI saw a ghost — a signal that wasn't actually there.
    FalseDetection { observer_id: EntityId, tick: u64 },
}

impl DecisionError {
    pub fn observer_id(&self) -> EntityId {
        match self {
            DecisionError::MissedSignal { observer_id, .. } => *observer_id,
            DecisionError::MisclassifiedSignal { observer_id, .. } => *observer_id,
            DecisionError::DelayedResponse { observer_id, .. } => *observer_id,
            DecisionError::StaleInformation { observer_id, .. } => *observer_id,
            DecisionError::FalseDetection { observer_id, .. } => *observer_id,
        }
    }

    pub fn tick(&self) -> u64 {
        match self {
            DecisionError::MissedSignal { tick, .. } => *tick,
            DecisionError::MisclassifiedSignal { tick, .. } => *tick,
            DecisionError::DelayedResponse { actual_tick, .. } => *actual_tick,
            DecisionError::StaleInformation { tick, .. } => *tick,
            DecisionError::FalseDetection { tick, .. } => *tick,
        }
    }
}

impl std::fmt::Display for DecisionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DecisionError::MissedSignal { observer_id, tick } => {
                write!(f, "[{tick}] {observer_id} missed a signal")
            }
            DecisionError::MisclassifiedSignal {
                observer_id,
                tick,
                actual_action,
                chosen_action,
            } => {
                write!(
                    f,
                    "[{tick}] {observer_id} misclassified: chose {chosen_action}, should have been {actual_action}"
                )
            }
            DecisionError::DelayedResponse {
                observer_id,
                intended_tick,
                actual_tick,
                action,
            } => {
                write!(
                    f,
                    "[{actual_tick}] {observer_id} delayed response to {action} (intended at {intended_tick})"
                )
            }
            DecisionError::StaleInformation {
                observer_id,
                tick,
                staleness_ticks,
            } => {
                write!(
                    f,
                    "[{tick}] {observer_id} made decision based on {staleness_ticks}-tick-old information"
                )
            }
            DecisionError::FalseDetection { observer_id, tick } => {
                write!(f, "[{tick}] {observer_id} detected a false signal")
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn error_observer_ids() {
        let e1 = DecisionError::MissedSignal {
            observer_id: EntityId::from_raw(1),
            tick: 5,
        };
        assert_eq!(e1.observer_id(), EntityId::from_raw(1));
        assert_eq!(e1.tick(), 5);

        let e2 = DecisionError::DelayedResponse {
            observer_id: EntityId::from_raw(2),
            intended_tick: 3,
            actual_tick: 7,
            action: EwAction::SuppressSignal,
        };
        assert_eq!(e2.observer_id(), EntityId::from_raw(2));
        assert_eq!(e2.tick(), 7);
    }

    #[test]
    fn error_display() {
        let e = DecisionError::FalseDetection {
            observer_id: EntityId::from_raw(1),
            tick: 10,
        };
        let msg = format!("{e}");
        assert!(msg.contains("EID-1"));
        assert!(msg.contains("10"));
    }
}
