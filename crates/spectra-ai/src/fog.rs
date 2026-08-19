use spectra_core::id::EntityId;
use spectra_core::observation::Observation;

/// Tracks observation gaps and staleness for fog of war simulation.
///
/// The AI never has perfect information. Observations arrive late,
/// have gaps, and become stale over time.
#[derive(Debug, Clone)]
pub struct FogOfWar {
    /// Maximum ticks an observation can be considered fresh.
    pub fresh_threshold: u64,
    /// Maximum ticks before observation is considered stale.
    pub stale_threshold: u64,
    /// Record of last observation tick per observer.
    last_observation_tick: std::collections::HashMap<EntityId, u64>,
    /// Observations currently tracked (observer_id -> observations).
    tracked_observations: std::collections::HashMap<EntityId, Vec<Observation>>,
}

impl FogOfWar {
    pub fn new(fresh_threshold: u64, stale_threshold: u64) -> Self {
        Self {
            fresh_threshold,
            stale_threshold,
            last_observation_tick: std::collections::HashMap::new(),
            tracked_observations: std::collections::HashMap::new(),
        }
    }

    /// Record a new observation, updating staleness tracking.
    pub fn record_observation(&mut self, obs: &Observation) {
        let tick = obs.timestamp;
        let observer = obs.observer_id;
        self.last_observation_tick.insert(observer, tick);
        self.tracked_observations
            .entry(observer)
            .or_default()
            .push(obs.clone());
    }

    /// Record multiple observations.
    pub fn record_observations(&mut self, observations: &[Observation]) {
        for obs in observations {
            self.record_observation(obs);
        }
    }

    /// How many ticks since the last observation from this observer.
    pub fn ticks_since_last_observation(&self, observer_id: EntityId, current_tick: u64) -> u64 {
        self.last_observation_tick
            .get(&observer_id)
            .map(|&last| current_tick.saturating_sub(last))
            .unwrap_or(u64::MAX)
    }

    /// Whether the observation from this observer is still fresh.
    pub fn is_fresh(&self, observer_id: EntityId, current_tick: u64) -> bool {
        self.ticks_since_last_observation(observer_id, current_tick) <= self.fresh_threshold
    }

    /// Whether the observation is stale (exists but old).
    pub fn is_stale(&self, observer_id: EntityId, current_tick: u64) -> bool {
        let gap = self.ticks_since_last_observation(observer_id, current_tick);
        gap > self.fresh_threshold && gap <= self.stale_threshold
    }

    /// Whether we have no observations at all from this observer.
    pub fn is_unknown(&self, observer_id: EntityId) -> bool {
        !self.last_observation_tick.contains_key(&observer_id)
    }

    /// Get tracked observations for an observer.
    pub fn observations_for(&self, observer_id: EntityId) -> &[Observation] {
        self.tracked_observations
            .get(&observer_id)
            .map(|v| v.as_slice())
            .unwrap_or(&[])
    }

    /// Get the most recent observation for an observer.
    pub fn latest_observation(&self, observer_id: EntityId) -> Option<&Observation> {
        self.tracked_observations
            .get(&observer_id)
            .and_then(|obs| obs.last())
    }

    /// Filter observations to only those within the fresh window.
    pub fn fresh_observations(&self, current_tick: u64) -> Vec<&Observation> {
        self.tracked_observations
            .values()
            .flat_map(|obs| obs.iter())
            .filter(|obs| {
                let age = current_tick.saturating_sub(obs.timestamp);
                age <= self.fresh_threshold
            })
            .collect()
    }

    /// Get total number of tracked observations.
    pub fn total_observations(&self) -> usize {
        self.tracked_observations.values().map(|v| v.len()).sum()
    }

    /// Clear all tracking data.
    pub fn clear(&mut self) {
        self.last_observation_tick.clear();
        self.tracked_observations.clear();
    }
}

impl Default for FogOfWar {
    fn default() -> Self {
        Self::new(2, 5)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use spectra_core::observation::ObservationQuality;

    fn make_obs(observer: u64, tick: u64, confidence: f64) -> Observation {
        let mut obs = Observation::new(EntityId::from_raw(observer), tick);
        obs.confidence = confidence;
        obs.quality = ObservationQuality::Clear;
        obs
    }

    #[test]
    fn fog_tracks_last_observation() {
        let mut fog = FogOfWar::new(2, 5);
        fog.record_observation(&make_obs(1, 0, 0.8));
        fog.record_observation(&make_obs(1, 3, 0.9));
        assert_eq!(
            fog.ticks_since_last_observation(EntityId::from_raw(1), 5),
            2
        );
    }

    #[test]
    fn fog_freshness() {
        let mut fog = FogOfWar::new(2, 5);
        fog.record_observation(&make_obs(1, 0, 0.8));
        assert!(fog.is_fresh(EntityId::from_raw(1), 0));
        assert!(fog.is_fresh(EntityId::from_raw(1), 2));
        assert!(!fog.is_fresh(EntityId::from_raw(1), 3));
    }

    #[test]
    fn fog_staleness() {
        let mut fog = FogOfWar::new(2, 5);
        fog.record_observation(&make_obs(1, 0, 0.8));
        assert!(!fog.is_stale(EntityId::from_raw(1), 0));
        assert!(fog.is_stale(EntityId::from_raw(1), 3));
        assert!(!fog.is_stale(EntityId::from_raw(1), 6));
    }

    #[test]
    fn fog_unknown_observer() {
        let mut fog = FogOfWar::new(2, 5);
        assert!(fog.is_unknown(EntityId::from_raw(99)));
        fog.record_observation(&make_obs(1, 0, 0.8));
        assert!(!fog.is_unknown(EntityId::from_raw(1)));
    }

    #[test]
    fn fog_latest_observation() {
        let mut fog = FogOfWar::new(2, 5);
        fog.record_observation(&make_obs(1, 0, 0.5));
        fog.record_observation(&make_obs(1, 3, 0.9));
        let latest = fog.latest_observation(EntityId::from_raw(1)).unwrap();
        assert_eq!(latest.timestamp, 3);
        assert!((latest.confidence - 0.9).abs() < f64::EPSILON);
    }

    #[test]
    fn fog_fresh_observations() {
        let mut fog = FogOfWar::new(2, 5);
        fog.record_observation(&make_obs(1, 0, 0.8));
        fog.record_observation(&make_obs(2, 5, 0.9));
        let fresh = fog.fresh_observations(5);
        assert_eq!(fresh.len(), 1);
        assert_eq!(fresh[0].observer_id, EntityId::from_raw(2));
    }

    #[test]
    fn fog_total_observations() {
        let mut fog = FogOfWar::new(2, 5);
        fog.record_observation(&make_obs(1, 0, 0.8));
        fog.record_observation(&make_obs(1, 1, 0.7));
        fog.record_observation(&make_obs(2, 0, 0.9));
        assert_eq!(fog.total_observations(), 3);
    }

    #[test]
    fn fog_clear() {
        let mut fog = FogOfWar::new(2, 5);
        fog.record_observation(&make_obs(1, 0, 0.8));
        fog.clear();
        assert_eq!(fog.total_observations(), 0);
        assert!(fog.is_unknown(EntityId::from_raw(1)));
    }

    #[test]
    fn fog_default() {
        let fog = FogOfWar::default();
        assert_eq!(fog.fresh_threshold, 2);
        assert_eq!(fog.stale_threshold, 5);
    }
}
