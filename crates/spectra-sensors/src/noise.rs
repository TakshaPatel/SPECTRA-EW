use rand::Rng;

/// Gaussian noise generator using Box-Muller transform.
#[derive(Debug, Clone)]
pub struct NoiseGenerator {
    pub base_level_db: f64,
    pub std_dev: f64,
}

impl NoiseGenerator {
    pub fn new(base_level_db: f64, std_dev: f64) -> Self {
        Self {
            base_level_db,
            std_dev,
        }
    }

    /// Generate ambient noise level (uniform perturbation around base).
    pub fn ambient(&self, rng: &mut impl Rng) -> f64 {
        let noise: f64 = rng.gen_range(-1.0..1.0) * self.std_dev;
        self.base_level_db + noise
    }

    /// Generate Gaussian noise using Box-Muller transform.
    pub fn gaussian(&self, rng: &mut impl Rng) -> f64 {
        let u1: f64 = rng.gen_range(0.0001..1.0);
        let u2: f64 = rng.gen_range(0.0..1.0);
        let z = (-2.0 * u1.ln()).sqrt() * (2.0 * std::f64::consts::PI * u2).cos();
        self.base_level_db + z * self.std_dev
    }
}

impl Default for NoiseGenerator {
    fn default() -> Self {
        Self {
            base_level_db: -100.0,
            std_dev: 2.0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::SeedableRng;
    use rand_chacha::ChaCha8Rng;

    #[test]
    fn noise_generation() {
        let gen = NoiseGenerator::new(-100.0, 2.0);
        let mut rng = ChaCha8Rng::seed_from_u64(42);
        let level = gen.ambient(&mut rng);
        assert!((level - (-100.0)).abs() < 5.0);
    }

    #[test]
    fn noise_gaussian() {
        let gen = NoiseGenerator::new(0.0, 1.0);
        let mut rng = ChaCha8Rng::seed_from_u64(42);
        let samples: Vec<f64> = (0..1000).map(|_| gen.gaussian(&mut rng)).collect();
        let mean = samples.iter().sum::<f64>() / samples.len() as f64;
        let variance =
            samples.iter().map(|x| (x - mean).powi(2)).sum::<f64>() / samples.len() as f64;
        // Mean should be near 0, variance near 1
        assert!(mean.abs() < 0.5);
        assert!((variance - 1.0).abs() < 0.5);
    }

    #[test]
    fn noise_deterministic() {
        let gen = NoiseGenerator::new(-100.0, 2.0);
        let mut rng1 = ChaCha8Rng::seed_from_u64(99);
        let mut rng2 = ChaCha8Rng::seed_from_u64(99);
        assert!((gen.ambient(&mut rng1) - gen.ambient(&mut rng2)).abs() < f64::EPSILON);
    }
}
