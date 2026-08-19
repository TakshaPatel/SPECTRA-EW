use serde::{Deserialize, Serialize};

use spectra_core::id::EntityId;

use crate::sensor_model::SensorModel;

/// A collection of sensors belonging to a single entity.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SensorArray {
    pub owner_id: EntityId,
    pub sensors: Vec<SensorModel>,
}

impl SensorArray {
    pub fn new(owner_id: EntityId) -> Self {
        Self {
            owner_id,
            sensors: Vec::new(),
        }
    }

    pub fn add_sensor(&mut self, sensor: SensorModel) {
        self.sensors.push(sensor);
    }

    pub fn sensor_count(&self) -> usize {
        self.sensors.len()
    }

    pub fn operational_count(&self) -> usize {
        self.sensors
            .iter()
            .filter(|s| s.health.is_operational())
            .count()
    }

    pub fn tick_all(&mut self) {
        for sensor in &mut self.sensors {
            sensor.tick();
        }
    }

    /// Get all sensors that monitor a given channel.
    pub fn sensors_for_channel(&self, channel: u32) -> Vec<&SensorModel> {
        self.sensors
            .iter()
            .filter(|s| s.monitors_channel(channel))
            .collect()
    }

    /// Get the best (highest integrity) sensor that monitors a channel.
    pub fn best_sensor_for_channel(&self, channel: u32) -> Option<&SensorModel> {
        self.sensors_for_channel(channel)
            .into_iter()
            .max_by(|a, b| {
                a.health
                    .integrity
                    .partial_cmp(&b.health.integrity)
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
    }

    /// Average detection probability across all operational sensors.
    pub fn average_detection_probability(&self, snr_db: f64) -> f64 {
        let operational: Vec<_> = self
            .sensors
            .iter()
            .filter(|s| s.health.is_operational())
            .collect();
        if operational.is_empty() {
            return 0.0;
        }
        let sum: f64 = operational
            .iter()
            .map(|s| s.effective_detection_probability(snr_db))
            .sum();
        sum / operational.len() as f64
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::degradation::SensorHealth;
    use crate::sensor_model::SensorModel;

    #[test]
    fn array_creation() {
        let mut arr = SensorArray::new(EntityId::from_raw(1));
        assert_eq!(arr.sensor_count(), 0);
        arr.add_sensor(SensorModel::new(
            EntityId::from_raw(10),
            EntityId::from_raw(1),
        ));
        assert_eq!(arr.sensor_count(), 1);
    }

    #[test]
    fn array_operational_count() {
        let mut arr = SensorArray::new(EntityId::from_raw(1));
        arr.add_sensor(SensorModel::new(
            EntityId::from_raw(10),
            EntityId::from_raw(1),
        ));
        arr.add_sensor(
            SensorModel::new(EntityId::from_raw(11), EntityId::from_raw(1))
                .with_health(SensorHealth::failed()),
        );
        assert_eq!(arr.sensor_count(), 2);
        assert_eq!(arr.operational_count(), 1);
    }

    #[test]
    fn array_tick() {
        let mut arr = SensorArray::new(EntityId::from_raw(1));
        arr.add_sensor(SensorModel::new(
            EntityId::from_raw(10),
            EntityId::from_raw(1),
        ));
        let initial = arr.sensors[0].health.integrity;
        arr.tick_all();
        assert!(arr.sensors[0].health.integrity < initial);
    }

    #[test]
    fn array_channel_filter() {
        let mut arr = SensorArray::new(EntityId::from_raw(1));
        arr.add_sensor(
            SensorModel::new(EntityId::from_raw(10), EntityId::from_raw(1))
                .with_monitored_channels(vec![1]),
        );
        arr.add_sensor(
            SensorModel::new(EntityId::from_raw(11), EntityId::from_raw(1))
                .with_monitored_channels(vec![2]),
        );
        assert_eq!(arr.sensors_for_channel(1).len(), 1);
        assert_eq!(arr.sensors_for_channel(2).len(), 1);
        assert_eq!(arr.sensors_for_channel(3).len(), 0);
    }

    #[test]
    fn array_best_sensor() {
        let mut arr = SensorArray::new(EntityId::from_raw(1));
        arr.add_sensor(
            SensorModel::new(EntityId::from_raw(10), EntityId::from_raw(1))
                .with_health(SensorHealth::degraded(0.3, 0.0)),
        );
        arr.add_sensor(SensorModel::new(
            EntityId::from_raw(11),
            EntityId::from_raw(1),
        ));
        let best = arr.best_sensor_for_channel(1).unwrap();
        assert_eq!(best.id, EntityId::from_raw(11));
    }

    #[test]
    fn array_average_detection() {
        let mut arr = SensorArray::new(EntityId::from_raw(1));
        arr.add_sensor(SensorModel::new(
            EntityId::from_raw(10),
            EntityId::from_raw(1),
        ));
        arr.add_sensor(SensorModel::new(
            EntityId::from_raw(11),
            EntityId::from_raw(1),
        ));
        let avg = arr.average_detection_probability(30.0);
        assert!(avg > 0.0 && avg <= 1.0);
    }
}
