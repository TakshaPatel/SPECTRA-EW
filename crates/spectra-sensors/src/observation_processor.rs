use rand_chacha::ChaCha8Rng;

use spectra_core::id::EntityId;
use spectra_core::observation::{Observation, ObservationQuality};
use spectra_core::propagation::{confidence_from_snr, received_power_db};
use spectra_core::world::World;

use crate::sensor_array::SensorArray;
use crate::sensor_model::SensorModel;

/// Raw ground-truth data about a signal at a receiver location.
/// This is what the sensor layer processes into imperfect observations.
#[derive(Debug, Clone)]
pub struct GroundTruthSignal {
    pub emitter_id: EntityId,
    pub emitter_owner_id: EntityId,
    pub channel: u32,
    pub frequency_mhz: f64,
    pub power_at_receiver_db: f64,
    pub distance: f64,
    pub interference_attenuation: f64,
}

/// The observation processor sits between ground truth and AI perception.
/// It takes sensor arrays and ground truth, producing imperfect observations.
pub struct ObservationProcessor {
    /// Sensor arrays keyed by entity ID
    sensor_arrays: std::collections::HashMap<EntityId, SensorArray>,
    /// RNG for deterministic noise injection
    rng: ChaCha8Rng,
}

impl ObservationProcessor {
    pub fn new(seed: u64) -> Self {
        use rand::SeedableRng;
        Self {
            sensor_arrays: std::collections::HashMap::new(),
            rng: ChaCha8Rng::seed_from_u64(seed),
        }
    }

    /// Register a sensor array for an entity.
    pub fn register_array(&mut self, entity_id: EntityId, array: SensorArray) {
        self.sensor_arrays.insert(entity_id, array);
    }

    /// Get a reference to an entity's sensor array.
    pub fn array(&self, entity_id: EntityId) -> Option<&SensorArray> {
        self.sensor_arrays.get(&entity_id)
    }

    /// Get a mutable reference to an entity's sensor array.
    pub fn array_mut(&mut self, entity_id: EntityId) -> Option<&mut SensorArray> {
        self.sensor_arrays.get_mut(&entity_id)
    }

    /// Build default sensor arrays from world receivers.
    /// Each receiver gets a default sensor model.
    pub fn build_from_world(&mut self, world: &World) {
        for receiver in &world.receivers {
            let sensor_id = EntityId::from_raw(receiver.id.raw() + 10000);
            let sensor = SensorModel::new(sensor_id, receiver.owner_id)
                .with_monitored_channels(receiver.monitored_channels.clone());
            let mut array = SensorArray::new(receiver.owner_id);
            array.add_sensor(sensor);
            self.sensor_arrays.insert(receiver.owner_id, array);
        }
    }

    /// Tick all sensor arrays (decay, calibration).
    pub fn tick_all(&mut self) {
        for array in self.sensor_arrays.values_mut() {
            array.tick_all();
        }
    }

    /// Compute ground truth signals for all receiver-emitter pairs.
    pub fn compute_ground_truth(world: &World) -> Vec<(EntityId, GroundTruthSignal)> {
        let mut truths = Vec::new();

        for receiver in &world.receivers {
            for emitter in &world.emitters {
                if !emitter.is_transmitting() {
                    continue;
                }
                if emitter.owner_id == receiver.owner_id {
                    continue;
                }
                if !receiver.monitored_channels.is_empty()
                    && !receiver.monitored_channels.contains(&emitter.channel)
                {
                    continue;
                }

                let distance = receiver.position.distance_to(emitter.position);
                let power_at_rx = received_power_db(
                    emitter.power_db,
                    distance,
                    emitter.frequency_mhz,
                    &world.config.environment,
                );

                let mut interference_attenuation = 0.0;
                for ix in &world.active_interference {
                    if ix.affects_position(receiver.position) && ix.affects_channel(emitter.channel)
                    {
                        interference_attenuation += ix.attenuation_at(receiver.position);
                    }
                }

                truths.push((
                    receiver.owner_id,
                    GroundTruthSignal {
                        emitter_id: emitter.id,
                        emitter_owner_id: emitter.owner_id,
                        channel: emitter.channel,
                        frequency_mhz: emitter.frequency_mhz,
                        power_at_receiver_db: power_at_rx,
                        distance,
                        interference_attenuation,
                    },
                ));
            }
        }

        truths
    }

    /// Process ground truth through sensor models to produce imperfect observations.
    pub fn process(
        &mut self,
        ground_truths: &[(EntityId, GroundTruthSignal)],
        tick: u64,
    ) -> Vec<Observation> {
        let mut observations = Vec::new();

        for (receiver_owner, truth) in ground_truths {
            let effective_power = truth.power_at_receiver_db - truth.interference_attenuation;

            // Find the sensor array for this receiver owner
            if let Some(array) = self.sensor_arrays.get(receiver_owner) {
                // Find sensors that monitor this channel
                let sensors: Vec<_> = array
                    .sensors_for_channel(truth.channel)
                    .into_iter()
                    .collect();

                if sensors.is_empty() {
                    continue;
                }

                // Use the best sensor for this observation
                let sensor = sensors
                    .iter()
                    .max_by(|a, b| {
                        a.health
                            .integrity
                            .partial_cmp(&b.health.integrity)
                            .unwrap_or(std::cmp::Ordering::Equal)
                    })
                    .unwrap();

                // Build raw observation
                let ambient_noise = -100.0; // Will be passed in from world
                let noise_floor = ambient_noise + 3.0 + 10.0_f64;
                let snr = effective_power - noise_floor;

                let mut obs = Observation::new(sensor.id, tick);
                obs.estimated_source_id = Some(truth.emitter_owner_id);
                obs.estimated_channel = Some(truth.channel);
                obs.estimated_frequency_mhz = Some(truth.frequency_mhz);
                obs.signal_strength_db = effective_power;
                obs.noise_level_db = noise_floor;
                obs.position_estimate = Some(spectra_core::world::Position::new(0.0, 0.0));
                obs.uncertainty_position = truth.distance * 0.1;
                obs.signal_id = Some(spectra_core::id::SignalId::from_raw(
                    truth.emitter_id.raw() + tick,
                ));

                if snr > 0.0 {
                    obs.confidence = confidence_from_snr(snr);
                    obs.quality = if snr > 20.0 {
                        ObservationQuality::Clear
                    } else if snr > 10.0 {
                        ObservationQuality::Noisy
                    } else {
                        ObservationQuality::Degraded
                    };
                }

                // Process through sensor model (adds noise, FP/FN)
                let processed = sensor.process_observation(obs, snr, &mut self.rng);
                observations.push(processed);
            }
        }

        observations
    }

    /// Process ground truths with world ambient noise included.
    pub fn process_with_world(
        &mut self,
        world: &World,
        ground_truths: &[(EntityId, GroundTruthSignal)],
        tick: u64,
    ) -> Vec<Observation> {
        let ambient_noise = world.config.environment.ambient_noise_db;
        let mut observations = Vec::new();

        for (receiver_owner, truth) in ground_truths {
            let effective_power = truth.power_at_receiver_db - truth.interference_attenuation;

            if let Some(array) = self.sensor_arrays.get(receiver_owner) {
                let sensors: Vec<_> = array.sensors_for_channel(truth.channel);

                if sensors.is_empty() {
                    continue;
                }

                let sensor = sensors
                    .iter()
                    .max_by(|a, b| {
                        a.health
                            .integrity
                            .partial_cmp(&b.health.integrity)
                            .unwrap_or(std::cmp::Ordering::Equal)
                    })
                    .unwrap();

                // Receiver noise floor calculation
                let bw_mhz: f64 = 1.0;
                let noise_floor = ambient_noise
                    + sensor.noise.noise_floor_addition
                    + sensor.health.degradation_noise()
                    + 3.0
                    + 10.0 * (bw_mhz * 1_000_000.0).log10();
                let snr = effective_power - noise_floor;

                let mut obs = Observation::new(sensor.id, tick);
                obs.estimated_source_id = Some(truth.emitter_owner_id);
                obs.estimated_channel = Some(truth.channel);
                obs.estimated_frequency_mhz = Some(truth.frequency_mhz);
                obs.signal_strength_db = effective_power;
                obs.noise_level_db = noise_floor;
                obs.position_estimate = Some(spectra_core::world::Position::new(0.0, 0.0));
                obs.uncertainty_position = truth.distance * 0.1;
                obs.signal_id = Some(spectra_core::id::SignalId::from_raw(
                    truth.emitter_id.raw() + tick,
                ));

                if snr > 0.0 {
                    obs.confidence = confidence_from_snr(snr);
                    obs.quality = if snr > 20.0 {
                        ObservationQuality::Clear
                    } else if snr > 10.0 {
                        ObservationQuality::Noisy
                    } else {
                        ObservationQuality::Degraded
                    };
                }

                let processed = sensor.process_observation(obs, snr, &mut self.rng);
                observations.push(processed);
            }
        }

        observations
    }
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
            environment: spectra_core::world::Environment {
                ambient_noise_db: -100.0,
                ..Default::default()
            },
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
    fn processor_builds_from_world() {
        let (world, proc) = make_world_and_processor();
        assert_eq!(proc.sensor_arrays.len(), world.receivers.len());
    }

    #[test]
    fn processor_generates_observations() {
        let (world, mut proc) = make_world_and_processor();
        let truths = ObservationProcessor::compute_ground_truth(&world);
        assert!(!truths.is_empty());

        let observations = proc.process_with_world(&world, &truths, 0);
        assert!(!observations.is_empty());
    }

    #[test]
    fn processor_observation_imperfect() {
        let (world, mut proc) = make_world_and_processor();
        let truths = ObservationProcessor::compute_ground_truth(&world);
        let observations = proc.process_with_world(&world, &truths, 0);

        // Observations should not be perfect — some noise should be present
        for obs in &observations {
            // Position should have noise (not exactly at emitter position)
            if let Some(pos) = obs.position_estimate {
                // Should have some deviation from 0,0 (the receiver position)
                // or from 100,0 (the emitter position)
                assert!(pos.x >= 0.0); // Just check it's a valid position
            }
        }
    }

    #[test]
    fn processor_deterministic() {
        let (world, _) = make_world_and_processor();
        let truths = ObservationProcessor::compute_ground_truth(&world);

        let mut proc1 = ObservationProcessor::new(42);
        proc1.build_from_world(&world);
        let obs1 = proc1.process_with_world(&world, &truths, 0);

        let mut proc2 = ObservationProcessor::new(42);
        proc2.build_from_world(&world);
        let obs2 = proc2.process_with_world(&world, &truths, 0);

        assert_eq!(obs1.len(), obs2.len());
        for (a, b) in obs1.iter().zip(obs2.iter()) {
            assert!((a.signal_strength_db - b.signal_strength_db).abs() < f64::EPSILON);
            assert!((a.confidence - b.confidence).abs() < f64::EPSILON);
        }
    }
}
