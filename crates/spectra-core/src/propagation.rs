use crate::world::Environment;

/// Calculate received signal power in dBm using abstract free-space-like propagation.
///
/// power_at_receiver = emitter_power - path_loss - environmental_effects
///
/// path_loss = 10 * n * log10(distance) + 20 * log10(frequency)
/// where n is the propagation loss exponent (2.0 = free space).
pub fn received_power_db(
    emitter_power_db: f64,
    distance: f64,
    frequency_mhz: f64,
    environment: &Environment,
) -> f64 {
    if distance <= 1.0 {
        return emitter_power_db;
    }

    let n = environment.propagation_loss_exponent;
    let path_loss = 10.0 * n * distance.log10() + 20.0 * frequency_mhz.log10();
    let weather = environment.weather_attenuation;

    emitter_power_db - path_loss - weather
}

/// Calculate confidence [0.0, 1.0] from SNR in dB.
/// Uses a sigmoid-like curve centered at 10 dB SNR.
pub fn confidence_from_snr(snr_db: f64) -> f64 {
    let x = (snr_db - 10.0) / 10.0;
    let sigmoid = 1.0 / (1.0 + (-x).exp());
    sigmoid.clamp(0.0, 1.0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::Environment;

    #[test]
    fn received_power_close_range() {
        let env = Environment::default();
        let power = received_power_db(30.0, 1.0, 2400.0, &env);
        assert!((power - 30.0).abs() < f64::EPSILON);
    }

    #[test]
    fn received_power_medium_range() {
        let env = Environment::default();
        let power = received_power_db(50.0, 100.0, 2400.0, &env);
        assert!(power > -60.0 && power < 20.0);
    }

    #[test]
    fn received_power_far_range() {
        let env = Environment::default();
        let power = received_power_db(30.0, 1000.0, 2400.0, &env);
        assert!(power < -50.0);
    }

    #[test]
    fn received_power_with_weather() {
        let env_no_weather = Environment::default();
        let env_weather = Environment {
            weather_attenuation: 10.0,
            ..Default::default()
        };
        let p1 = received_power_db(30.0, 100.0, 2400.0, &env_no_weather);
        let p2 = received_power_db(30.0, 100.0, 2400.0, &env_weather);
        assert!((p1 - p2 - 10.0).abs() < f64::EPSILON);
    }

    #[test]
    fn confidence_snr_high() {
        let c = confidence_from_snr(40.0);
        assert!(c > 0.9);
    }

    #[test]
    fn confidence_snr_low() {
        let c = confidence_from_snr(-20.0);
        assert!(c < 0.1);
    }

    #[test]
    fn confidence_snr_mid() {
        let c = confidence_from_snr(10.0);
        assert!((c - 0.5).abs() < 0.01);
    }
}
