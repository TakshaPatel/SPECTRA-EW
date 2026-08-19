use serde::{Deserialize, Serialize};

use crate::action::EwAction;
use crate::id::{EntityId, EventId};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum EventKind {
    SignalDetected,
    SignalLost,
    InterferenceStarted,
    InterferenceEnded,
    ActionPerformed,
    ActionFailed,
    ChannelChanged,
    DecoyDeployed,
    ObservationGenerated,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Event {
    pub id: EventId,
    pub kind: EventKind,
    pub tick: u64,
    pub actor_id: EntityId,
    pub details: EventDetails,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EventDetails {
    Signal {
        channel: u32,
        frequency_mhz: f64,
    },
    Interference {
        kind: crate::interference::InterferenceKind,
        intensity_db: f64,
    },
    Action {
        action: EwAction,
        success: bool,
    },
    ChannelChange {
        old_channel: u32,
        new_channel: u32,
    },
    Observation {
        confidence: f64,
    },
    None,
}

impl Event {
    pub fn new(id: EventId, kind: EventKind, tick: u64, actor_id: EntityId) -> Self {
        Self {
            id,
            kind,
            tick,
            actor_id,
            details: EventDetails::None,
        }
    }

    pub fn with_details(mut self, details: EventDetails) -> Self {
        self.details = details;
        self
    }
}
