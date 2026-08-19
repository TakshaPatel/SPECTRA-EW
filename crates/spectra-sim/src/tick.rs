use spectra_core::emitter::EmitterState;
use spectra_core::event::{Event, EventDetails, EventKind};
use spectra_core::id::{EntityId, EventId};
use spectra_core::metrics::Metrics;
use spectra_core::world::World;
use spectra_sensors::observation_processor::ObservationProcessor;

/// Process one simulation tick with sensor-layer observations.
/// Returns events generated during this tick.
pub fn process_tick(
    world: &mut World,
    processor: &mut ObservationProcessor,
    events: &mut Vec<Event>,
    metrics: &mut Metrics,
) {
    let tick = world.tick;

    // 1. Tick interference durations
    let expired: Vec<_> = world
        .active_interference
        .iter()
        .filter(|ix| ix.remaining_ticks == 1)
        .map(|ix| (ix.id, ix.kind, ix.intensity_db))
        .collect();
    world.tick_interference();
    for (id, kind, intensity) in expired {
        events.push(
            Event::new(
                EventId::from_raw(events.len() as u64 + 1),
                EventKind::InterferenceEnded,
                tick,
                id,
            )
            .with_details(EventDetails::Interference {
                kind,
                intensity_db: intensity,
            }),
        );
    }

    // 2. Update emitter states based on owner state
    let owner_states: std::collections::HashMap<EntityId, bool> = world
        .entities
        .iter()
        .map(|e| (e.id, e.is_active()))
        .collect();
    for emitter in &mut world.emitters {
        if let Some(&active) = owner_states.get(&emitter.owner_id) {
            if !active && emitter.state == EmitterState::Transmitting {
                emitter.state = EmitterState::Silent;
            }
        }
    }

    // 3. Compute ground truth signals
    let ground_truths = ObservationProcessor::compute_ground_truth(world);

    // 4. Process through sensor layer (adds noise, FP/FN, degradation)
    let observations = processor.process_with_world(world, &ground_truths, tick);

    // 5. Record observations and events
    for obs in &observations {
        let usable = obs.is_usable();
        metrics.record_observation(obs.confidence, usable);

        if usable && obs.estimated_source_id.is_some() {
            events.push(
                Event::new(
                    EventId::from_raw(events.len() as u64 + 1),
                    EventKind::SignalDetected,
                    tick,
                    obs.observer_id,
                )
                .with_details(EventDetails::Signal {
                    channel: obs.estimated_channel.unwrap_or(0),
                    frequency_mhz: obs.estimated_frequency_mhz.unwrap_or(0.0),
                }),
            );
        }
    }

    // 6. Tick all sensors (decay, calibration)
    processor.tick_all();

    // 7. Record observation event
    metrics.record_event(&Event::new(
        EventId::from_raw(0),
        EventKind::ObservationGenerated,
        tick,
        EntityId::ZERO,
    ));
}

#[cfg(test)]
mod tests {
    use super::*;
    use spectra_core::emitter::{Emitter, SignalCategory};
    use spectra_core::entity::{Entity, EntityKind};
    use spectra_core::id::IdGenerator;
    use spectra_core::receiver::Receiver;
    use spectra_core::world::{Faction, Position, WorldConfig};

    fn make_world_and_processor() -> (World, ObservationProcessor) {
        let mut ids = IdGenerator::new();
        let config = WorldConfig {
            max_ticks: 10,
            ..Default::default()
        };
        let mut world = World::new(config);

        let blue_id = ids.next_id();
        world.add_entity(Entity::new(
            blue_id,
            EntityKind::Aircraft,
            Faction::Blue,
            Position::new(0.0, 0.0),
        ));

        let red_id = ids.next_id();
        world.add_entity(Entity::new(
            red_id,
            EntityKind::Aircraft,
            Faction::Red,
            Position::new(100.0, 0.0),
        ));

        let em_id = ids.next_id();
        world.add_emitter(Emitter::new(
            em_id,
            red_id,
            1,
            2400.0,
            SignalCategory::Communication,
            Position::new(100.0, 0.0),
        ));

        let rx_id = ids.next_id();
        world.add_receiver(Receiver::new(
            rx_id,
            blue_id,
            -90.0,
            Position::new(0.0, 0.0),
        ));

        let mut proc = ObservationProcessor::new(42);
        proc.build_from_world(&world);

        (world, proc)
    }

    #[test]
    fn tick_with_sensors() {
        let (mut world, mut proc) = make_world_and_processor();
        let mut events = Vec::new();
        let mut metrics = Metrics::default();
        process_tick(&mut world, &mut proc, &mut events, &mut metrics);
        assert!(metrics.total_observations > 0 || world.emitters.is_empty());
    }

    #[test]
    fn tick_advances_with_sensors() {
        let (mut world, mut proc) = make_world_and_processor();
        assert_eq!(world.tick, 0);
        let mut events = Vec::new();
        let mut metrics = Metrics::default();
        process_tick(&mut world, &mut proc, &mut events, &mut metrics);
        world.advance_tick();
        assert_eq!(world.tick, 1);
    }

    #[test]
    fn tick_sensor_decay() {
        let (mut world, mut proc) = make_world_and_processor();
        let mut events = Vec::new();
        let mut metrics = Metrics::default();

        let initial_integrity = proc.array(EntityId::from_raw(1)).unwrap().sensors[0]
            .health
            .integrity;

        process_tick(&mut world, &mut proc, &mut events, &mut metrics);

        let after_integrity = proc.array(EntityId::from_raw(1)).unwrap().sensors[0]
            .health
            .integrity;

        assert!(after_integrity < initial_integrity);
    }

    #[test]
    fn tick_deterministic_with_sensors() {
        let (world_template, _) = make_world_and_processor();

        let mut proc1 = ObservationProcessor::new(42);
        proc1.build_from_world(&world_template);
        let mut w1 = world_template.clone();
        let mut e1 = Vec::new();
        let mut m1 = Metrics::default();
        process_tick(&mut w1, &mut proc1, &mut e1, &mut m1);

        let mut proc2 = ObservationProcessor::new(42);
        proc2.build_from_world(&world_template);
        let mut w2 = world_template.clone();
        let mut e2 = Vec::new();
        let mut m2 = Metrics::default();
        process_tick(&mut w2, &mut proc2, &mut e2, &mut m2);

        assert_eq!(m1.total_observations, m2.total_observations);
        assert_eq!(e1.len(), e2.len());
    }
}
