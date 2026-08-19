use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct EntityId(u64);

impl EntityId {
    pub const ZERO: Self = EntityId(0);

    pub fn from_raw(raw: u64) -> Self {
        EntityId(raw)
    }

    pub fn raw(self) -> u64 {
        self.0
    }
}

impl fmt::Display for EntityId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "EID-{}", self.0)
    }
}

/// Generates deterministic sequential EntityIds.
#[derive(Debug)]
pub struct IdGenerator {
    next: u64,
}

impl IdGenerator {
    pub fn new() -> Self {
        Self { next: 1 }
    }

    pub fn next_id(&mut self) -> EntityId {
        let id = EntityId(self.next);
        self.next += 1;
        id
    }
}

impl Default for IdGenerator {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SignalId(u64);

impl SignalId {
    pub fn from_raw(raw: u64) -> Self {
        SignalId(raw)
    }

    pub fn raw(self) -> u64 {
        self.0
    }
}

impl fmt::Display for SignalId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "SIG-{}", self.0)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct EventId(u64);

impl EventId {
    pub fn from_raw(raw: u64) -> Self {
        EventId(raw)
    }

    pub fn raw(self) -> u64 {
        self.0
    }
}

impl fmt::Display for EventId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "EVT-{}", self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn id_generator_sequential() {
        let mut gen = IdGenerator::new();
        let a = gen.next_id();
        let b = gen.next_id();
        let c = gen.next_id();
        assert_eq!(a, EntityId::from_raw(1));
        assert_eq!(b, EntityId::from_raw(2));
        assert_eq!(c, EntityId::from_raw(3));
    }

    #[test]
    fn entity_id_display() {
        let id = EntityId::from_raw(42);
        assert_eq!(format!("{}", id), "EID-42");
    }

    #[test]
    fn signal_id_display() {
        let id = SignalId::from_raw(7);
        assert_eq!(format!("{}", id), "SIG-7");
    }

    #[test]
    fn event_id_display() {
        let id = EventId::from_raw(99);
        assert_eq!(format!("{}", id), "EVT-99");
    }

    #[test]
    fn entity_id_raw_roundtrip() {
        let id = EntityId::from_raw(999);
        assert_eq!(id.raw(), 999);
    }
}
