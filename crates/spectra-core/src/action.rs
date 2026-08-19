use serde::{Deserialize, Serialize};

use crate::id::EntityId;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum EwAction {
    Observe,
    Monitor,
    ProtectChannel,
    ChangeChannel,
    DeployDecoy,
    SuppressSignal,
    Disengage,
}

impl std::fmt::Display for EwAction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EwAction::Observe => write!(f, "OBSERVE"),
            EwAction::Monitor => write!(f, "MONITOR"),
            EwAction::ProtectChannel => write!(f, "PROTECT_CHANNEL"),
            EwAction::ChangeChannel => write!(f, "CHANGE_CHANNEL"),
            EwAction::DeployDecoy => write!(f, "DEPLOY_DECOY"),
            EwAction::SuppressSignal => write!(f, "SUPPRESS_SIGNAL"),
            EwAction::Disengage => write!(f, "DISENGAGE"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionTarget {
    pub channel: Option<u32>,
    pub entity_id: Option<EntityId>,
    pub frequency_mhz: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformedAction {
    pub actor_id: EntityId,
    pub action: EwAction,
    pub target: ActionTarget,
    pub timestamp: u64,
    pub success: bool,
}

impl PerformedAction {
    pub fn new(actor_id: EntityId, action: EwAction, timestamp: u64) -> Self {
        Self {
            actor_id,
            action,
            target: ActionTarget {
                channel: None,
                entity_id: None,
                frequency_mhz: None,
            },
            timestamp,
            success: true,
        }
    }

    pub fn with_target_channel(mut self, channel: u32) -> Self {
        self.target.channel = Some(channel);
        self
    }

    pub fn with_target_entity(mut self, entity_id: EntityId) -> Self {
        self.target.entity_id = Some(entity_id);
        self
    }

    pub fn failed(mut self) -> Self {
        self.success = false;
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn action_display() {
        assert_eq!(EwAction::ProtectChannel.to_string(), "PROTECT_CHANNEL");
        assert_eq!(EwAction::Disengage.to_string(), "DISENGAGE");
    }

    #[test]
    fn performed_action_creation() {
        let actor = EntityId::from_raw(1);
        let action = PerformedAction::new(actor, EwAction::Observe, 0);
        assert!(action.success);
        assert_eq!(action.timestamp, 0);
    }

    #[test]
    fn performed_action_with_targets() {
        let actor = EntityId::from_raw(1);
        let target_entity = EntityId::from_raw(2);
        let action = PerformedAction::new(actor, EwAction::SuppressSignal, 5)
            .with_target_channel(3)
            .with_target_entity(target_entity);
        assert_eq!(action.target.channel, Some(3));
        assert_eq!(action.target.entity_id, Some(target_entity));
    }

    #[test]
    fn performed_action_failure() {
        let actor = EntityId::from_raw(1);
        let action = PerformedAction::new(actor, EwAction::ChangeChannel, 1).failed();
        assert!(!action.success);
    }
}
