// ============================================================================
// PHASE 10: TRAINING DATASET
// ============================================================================
//
// Dataset generation for neural correction training.
// Generates training pairs from:
// - Synthetic data via reference renderer (ground truth) vs approximate renderer
// - MERL measured BRDF database (when available)
//
// Each training sample contains:
// - Input: material/geometric parameters (CorrectionInput)
// - Physical: approximate physics prediction (BSDFResponse)
// - Target: ground truth (BSDFResponse)
// ============================================================================

use super::neural_correction::CorrectionInput;
use super::unified_bsdf::BSDFResponse;

// ============================================================================
// TRAINING SAMPLE
// ============================================================================

/// A single training sample for neural correction.
#[derive(Debug, Clone)]
pub struct TrainingSample {
    /// Input features for the neural network
    pub input: CorrectionInput,
    /// Physical model prediction (approximate)
    pub physical_response: BSDFResponse,
    /// Target response (ground truth from reference renderer or measurement)
    pub target_response: BSDFResponse,
    /// Optional material identifier
    pub material_id: Option<String>,
}

impl TrainingSample {
    /// Create a new training sample
    pub fn new(
        input: CorrectionInput,
        physical_response: BSDFResponse,
        target_response: BSDFResponse,
    ) -> Self {
        Self {
            input,
            physical_response,
            target_response,
            material_id: None,
        }
    }

    /// Create with material identifier
    pub fn with_material_id(mut self, id: &str) -> Self {
        self.material_id = Some(id.to_string());
        self
    }

    /// Compute the error between physical and target
    pub fn error(&self) -> f64 {
        let dr = self.physical_response.reflectance - self.target_response.reflectance;
        let dt = self.physical_response.transmittance - self.target_response.transmittance;
        (dr * dr + dt * dt).sqrt()
    }

    /// Get the ideal correction (what the neural network should output)
    pub fn ideal_correction(&self) -> (f64, f64) {
        (
            self.target_response.reflectance - self.physical_response.reflectance,
            self.target_response.transmittance - self.physical_response.transmittance,
        )
    }
}

// ============================================================================
// DATASET SOURCE
// ============================================================================

/// Source specification for training data
#[derive(Debug, Clone)]
pub enum DatasetSource {
    /// Synthetic data from reference renderer
    Synthetic {
        /// Number of materials to generate
        num_materials: usize,
        /// Number of angle samples per material
        angle_samples: usize,
        /// Number of wavelength samples
        wavelength_samples: usize,
    },
    /// MERL measured materials (placeholder - requires MERL dataset integration)
    Merl {
        /// Material names to use
        materials: Vec<String>,
    },
    /// Combined synthetic and measured data
    Combined {
        /// Weight for synthetic data (0-1)
        synthetic_weight: f64,
        /// Synthetic configuration
        num_synthetic_materials: usize,
        /// MERL materials to include
        merl_materials: Vec<String>,
    },
}

impl Default for DatasetSource {
    fn default() -> Self {
        DatasetSource::Synthetic {
            num_materials: 20,
            angle_samples: 10,
            wavelength_samples: 5,
        }
    }
}

// ============================================================================
// AUGMENTATION CONFIG
// ============================================================================

/// Data augmentation strategies
#[derive(Debug, Clone)]
pub struct AugmentationConfig {
    /// Random wavelength jitter (±nm)
    pub wavelength_jitter: f64,
    /// Random angle noise (±radians)
    pub angle_noise: f64,
    /// Random parameter perturbation (±fraction)
    pub parameter_noise: f64,
    /// Enable spectral shift
    pub spectral_shift: bool,
}

impl Default for AugmentationConfig {
    fn default() -> Self {
        Self {
            wavelength_jitter: 5.0,
            angle_noise: 0.02,
            parameter_noise: 0.05,
            spectral_shift: false,
        }
    }
}

// ============================================================================
// DATASET METADATA
// ============================================================================

/// Metadata about the training dataset
#[derive(Debug, Clone)]
pub struct DatasetMetadata {
    /// Source of the data
    pub source: DatasetSource,
    /// Total number of samples
    pub num_samples: usize,
    /// Wavelength range [min, max] in nm
    pub wavelength_range: (f64, f64),
    /// Number of unique materials
    pub num_materials: usize,
    /// Mean error between physical and target
    pub mean_error: f64,
    /// Max error between physical and target
    pub max_error: f64,
}

// ============================================================================
// TRAINING DATASET
// ============================================================================

/// Complete training dataset for neural correction
#[derive(Debug, Clone)]
pub struct TrainingDataset {
    /// Training samples
    pub samples: Vec<TrainingSample>,
    /// Dataset metadata
    pub metadata: DatasetMetadata,
}

impl TrainingDataset {
    /// Create an empty dataset
    pub fn new() -> Self {
        Self {
            samples: Vec::new(),
            metadata: DatasetMetadata {
                source: DatasetSource::default(),
                num_samples: 0,
                wavelength_range: (400.0, 700.0),
                num_materials: 0,
                mean_error: 0.0,
                max_error: 0.0,
            },
        }
    }

    /// Generate synthetic training data.
    ///
    /// Creates pairs of (approximate_physics, reference_ground_truth) by:
    /// 1. Generating random material parameters
    /// 2. Computing reference response (high precision)
    /// 3. Computing approximate response (fast physics)
    /// 4. Creating training samples where the network learns the difference
    pub fn generate_synthetic(
        num_materials: usize,
        angle_samples: usize,
        wavelength_samples: usize,
        seed: u64,
    ) -> Self {
        let mut rng = SimpleRng::new(seed);
        let mut samples = Vec::new();
        let mut max_error = 0.0f64;
        let mut total_error = 0.0f64;

        // Wavelengths to sample
        let wavelengths: Vec<f64> = (0..wavelength_samples)
            .map(|i| 400.0 + (i as f64) * 300.0 / (wavelength_samples as f64))
            .collect();

        // Generate random materials
        for mat_idx in 0..num_materials {
            // Random material parameters
            let ior = 1.0 + rng.uniform(0.0, 3.0);
            let roughness = rng.uniform(0.0, 1.0);
            let k = rng.uniform(0.0, 5.0); // Extinction (for conductors)

            // Determine if dielectric or conductor
            let is_conductor = rng.uniform(0.0, 1.0) > 0.7;

            for angle_idx in 0..angle_samples {
                // Sample cos_theta uniformly in [0, 1] for better coverage
                let cos_theta = (angle_idx as f64 + rng.uniform(0.0, 1.0)) / angle_samples as f64;
                let cos_theta = cos_theta.clamp(0.01, 1.0);

                for &wavelength in &wavelengths {
                    // Create input
                    let input = CorrectionInput::new(
                        wavelength,
                        cos_theta,
                        cos_theta, // Same for reflection
                        roughness,
                        ior,
                        if is_conductor { k } else { 0.0 },
                        0.0, // thickness
                        0.0, // absorption
                        0.0, // scattering
                        0.0, // g
                    );

                    // Compute "reference" response (simulated ground truth)
                    // In production, this would come from ReferenceRenderer
                    let target = compute_reference_response(
                        ior,
                        if is_conductor { k } else { 0.0 },
                        roughness,
                        cos_theta,
                        wavelength,
                    );

                    // Compute "approximate" response (simulated fast physics)
                    let physical = compute_approximate_response(
                        ior,
                        if is_conductor { k } else { 0.0 },
                        roughness,
                        cos_theta,
                        wavelength,
                    );

                    let sample = TrainingSample::new(input, physical, target)
                        .with_material_id(&format!("synthetic_{}", mat_idx));

                    let error = sample.error();
                    max_error = max_error.max(error);
                    total_error += error;

                    samples.push(sample);
                }
            }
        }

        let num_samples = samples.len();
        let mean_error = if num_samples > 0 {
            total_error / num_samples as f64
        } else {
            0.0
        };

        Self {
            samples,
            metadata: DatasetMetadata {
                source: DatasetSource::Synthetic {
                    num_materials,
                    angle_samples,
                    wavelength_samples,
                },
                num_samples,
                wavelength_range: (400.0, 700.0),
                num_materials,
                mean_error,
                max_error,
            },
        }
    }

    /// Generate a small test dataset for unit tests
    pub fn generate_test_dataset(seed: u64) -> Self {
        Self::generate_synthetic(5, 5, 3, seed)
    }

    /// Add a sample to the dataset
    pub fn add_sample(&mut self, sample: TrainingSample) {
        let error = sample.error();
        self.metadata.max_error = self.metadata.max_error.max(error);

        // Update running mean
        let n = self.samples.len() as f64;
        self.metadata.mean_error = (self.metadata.mean_error * n + error) / (n + 1.0);

        self.samples.push(sample);
        self.metadata.num_samples = self.samples.len();
    }

    /// Number of samples
    pub fn len(&self) -> usize {
        self.samples.len()
    }

    /// Check if empty
    pub fn is_empty(&self) -> bool {
        self.samples.is_empty()
    }

    /// Get a batch of samples
    pub fn get_batch(&self, start: usize, size: usize) -> &[TrainingSample] {
        let end = (start + size).min(self.samples.len());
        &self.samples[start..end]
    }

    /// Shuffle the dataset using a seed
    pub fn shuffle(&mut self, seed: u64) {
        let mut rng = SimpleRng::new(seed);
        let n = self.samples.len();

        // Fisher-Yates shuffle
        for i in (1..n).rev() {
            let j = rng.uniform_int(0, i as u64) as usize;
            self.samples.swap(i, j);
        }
    }

    /// Split into training and validation sets
    pub fn split(&self, train_fraction: f64) -> (Self, Self) {
        let split_idx = (self.samples.len() as f64 * train_fraction) as usize;

        let train_samples = self.samples[..split_idx].to_vec();
        let val_samples = self.samples[split_idx..].to_vec();

        let train = Self {
            samples: train_samples.clone(),
            metadata: DatasetMetadata {
                source: self.metadata.source.clone(),
                num_samples: train_samples.len(),
                wavelength_range: self.metadata.wavelength_range,
                num_materials: self.metadata.num_materials,
                mean_error: compute_mean_error(&train_samples),
                max_error: compute_max_error(&train_samples),
            },
        };

        let val = Self {
            samples: val_samples.clone(),
            metadata: DatasetMetadata {
                source: self.metadata.source.clone(),
                num_samples: val_samples.len(),
                wavelength_range: self.metadata.wavelength_range,
                num_materials: self.metadata.num_materials,
                mean_error: compute_mean_error(&val_samples),
                max_error: compute_max_error(&val_samples),
            },
        };

        (train, val)
    }

    /// Apply augmentation to create additional samples
    pub fn augment(&mut self, config: &AugmentationConfig, seed: u64) {
        let mut rng = SimpleRng::new(seed);
        let original_len = self.samples.len();

        for i in 0..original_len {
            let sample = &self.samples[i];

            // Create augmented sample with noise
            let wavelength_noise = rng.uniform(-config.wavelength_jitter, config.wavelength_jitter);
            let angle_noise = rng.uniform(-config.angle_noise, config.angle_noise);

            let augmented_input = CorrectionInput::new(
                (sample.input.wavelength_normalized * 300.0 + 400.0 + wavelength_noise)
                    .clamp(400.0, 700.0),
                (sample.input.cos_theta_i + angle_noise).clamp(0.01, 1.0),
                (sample.input.cos_theta_o + angle_noise).clamp(0.01, 1.0),
                sample.input.roughness,
                sample.input.ior_normalized * 3.0 + 1.0,
                sample.input.k_normalized * 10.0,
                0.0,
                0.0,
                0.0,
                0.0,
            );

            // Target stays the same (slight perturbation of physical)
            let param_noise = 1.0 + rng.uniform(-config.parameter_noise, config.parameter_noise);
            let augmented_physical = BSDFResponse::new(
                (sample.physical_response.reflectance * param_noise).clamp(0.0, 1.0),
                sample.physical_response.transmittance,
                sample.physical_response.absorption,
            );

            let augmented_sample = TrainingSample::new(
                augmented_input,
                augmented_physical,
                sample.target_response.clone(),
            );

            self.samples.push(augmented_sample);
        }

        self.metadata.num_samples = self.samples.len();
    }
}

impl Default for TrainingDataset {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// HELPER FUNCTIONS
// ============================================================================

/// Compute reference response (simulated high-quality physics).
/// In production, this would call ReferenceRenderer.
fn compute_reference_response(
    ior: f64,
    k: f64,
    roughness: f64,
    cos_theta: f64,
    _wavelength: f64,
) -> BSDFResponse {
    // Full Fresnel with roughness microfacet
    let (r, t) = if k > 0.01 {
        // Conductor: use complex Fresnel
        let n2_k2 = ior * ior + k * k;
        let cos2 = cos_theta * cos_theta;
        let sin2 = 1.0 - cos2;

        let a = (n2_k2 - sin2).sqrt();
        let r_s = ((a - cos_theta).powi(2)) / ((a + cos_theta).powi(2));

        // Simplified conductor reflectance
        let r = r_s * (1.0 - roughness * 0.3); // Roughness reduces specular
        (r.clamp(0.0, 1.0), 0.0)
    } else {
        // Dielectric: Fresnel equations
        let sin_theta = (1.0 - cos_theta * cos_theta).sqrt();
        let sin_t = sin_theta / ior;

        if sin_t >= 1.0 {
            // Total internal reflection
            (1.0, 0.0)
        } else {
            let cos_t = (1.0 - sin_t * sin_t).sqrt();

            // Fresnel reflectance (average of s and p polarization)
            let r_s = ((cos_theta - ior * cos_t) / (cos_theta + ior * cos_t)).powi(2);
            let r_p = ((ior * cos_theta - cos_t) / (ior * cos_theta + cos_t)).powi(2);
            let r = 0.5 * (r_s + r_p);

            // Apply roughness effect
            let r = r * (1.0 - roughness * 0.2);
            let t = (1.0 - r) * (1.0 - roughness * 0.1);

            (r.clamp(0.0, 1.0), t.clamp(0.0, 1.0 - r))
        }
    };

    BSDFResponse::new(r, t, 1.0 - r - t)
}

/// Compute approximate response (simulated fast physics).
/// Uses Schlick approximation and simplified roughness.
fn compute_approximate_response(
    ior: f64,
    k: f64,
    roughness: f64,
    cos_theta: f64,
    _wavelength: f64,
) -> BSDFResponse {
    // Schlick approximation (less accurate than full Fresnel)
    let f0 = if k > 0.01 {
        // Conductor F0
        let n2_k2 = ior * ior + k * k;
        ((n2_k2 - 2.0 * ior + 1.0) / (n2_k2 + 2.0 * ior + 1.0)).min(1.0)
    } else {
        // Dielectric F0
        ((ior - 1.0) / (ior + 1.0)).powi(2)
    };

    let one_minus_cos = 1.0 - cos_theta;
    let r = f0 + (1.0 - f0) * one_minus_cos.powi(5);

    // Simplified roughness effect (introduces error vs reference)
    let r = r * (1.0 - roughness * 0.15); // Different coefficient than reference
    let t = if k > 0.01 {
        0.0
    } else {
        (1.0 - r) * (1.0 - roughness * 0.05)
    };

    BSDFResponse::new(r.clamp(0.0, 1.0), t.clamp(0.0, 1.0), 1.0 - r - t)
}

/// Compute mean error for a set of samples
fn compute_mean_error(samples: &[TrainingSample]) -> f64 {
    if samples.is_empty() {
        return 0.0;
    }
    samples.iter().map(|s| s.error()).sum::<f64>() / samples.len() as f64
}

/// Compute max error for a set of samples
fn compute_max_error(samples: &[TrainingSample]) -> f64 {
    samples.iter().map(|s| s.error()).fold(0.0, f64::max)
}

// ============================================================================
// SIMPLE RNG (for deterministic shuffling)
// ============================================================================

struct SimpleRng {
    state: u64,
}

impl SimpleRng {
    fn new(seed: u64) -> Self {
        Self { state: seed.max(1) }
    }

    fn next(&mut self) -> u64 {
        self.state ^= self.state << 13;
        self.state ^= self.state >> 7;
        self.state ^= self.state << 17;
        self.state
    }

    fn uniform(&mut self, min: f64, max: f64) -> f64 {
        let t = (self.next() as f64) / (u64::MAX as f64);
        min + t * (max - min)
    }

    fn uniform_int(&mut self, min: u64, max: u64) -> u64 {
        min + (self.next() % (max - min + 1))
    }
}

// ============================================================================
// MEMORY UTILITIES
// ============================================================================

/// Estimate memory usage of a dataset
pub fn estimate_dataset_memory(num_samples: usize) -> usize {
    let sample_size = std::mem::size_of::<TrainingSample>() + 32; // Estimated String allocation for material_id

    sample_size * num_samples
        + std::mem::size_of::<DatasetMetadata>()
        + std::mem::size_of::<Vec<TrainingSample>>()
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_training_sample() {
        let input = CorrectionInput::default();
        let physical = BSDFResponse::new(0.5, 0.3, 0.2);
        let target = BSDFResponse::new(0.55, 0.28, 0.17);

        let sample = TrainingSample::new(input, physical, target);

        assert!(sample.error() > 0.0);

        let (dr, dt) = sample.ideal_correction();
        assert!((dr - 0.05).abs() < 1e-10);
        assert!((dt - (-0.02)).abs() < 1e-10);
    }

    #[test]
    fn test_generate_synthetic() {
        let dataset = TrainingDataset::generate_synthetic(5, 5, 3, 42);

        assert_eq!(dataset.len(), 5 * 5 * 3); // materials * angles * wavelengths
        assert!(dataset.metadata.mean_error > 0.0);
    }

    #[test]
    fn test_shuffle_deterministic() {
        let mut dataset1 = TrainingDataset::generate_synthetic(3, 3, 2, 42);
        let mut dataset2 = TrainingDataset::generate_synthetic(3, 3, 2, 42);

        dataset1.shuffle(123);
        dataset2.shuffle(123);

        // Same seed should produce same shuffle
        for (s1, s2) in dataset1.samples.iter().zip(dataset2.samples.iter()) {
            assert_eq!(
                s1.input.wavelength_normalized,
                s2.input.wavelength_normalized
            );
        }
    }

    #[test]
    fn test_split() {
        let dataset = TrainingDataset::generate_synthetic(10, 5, 2, 42);
        let (train, val) = dataset.split(0.8);

        assert_eq!(train.len() + val.len(), dataset.len());
        assert!(train.len() > val.len());
    }

    #[test]
    fn test_augmentation() {
        let mut dataset = TrainingDataset::generate_synthetic(5, 3, 2, 42);
        let original_len = dataset.len();

        dataset.augment(&AugmentationConfig::default(), 123);

        // Augmentation should double the dataset
        assert_eq!(dataset.len(), original_len * 2);
    }

    #[test]
    fn test_get_batch() {
        let dataset = TrainingDataset::generate_synthetic(5, 5, 3, 42);

        let batch = dataset.get_batch(0, 10);
        assert_eq!(batch.len(), 10);

        let batch = dataset.get_batch(dataset.len() - 5, 10);
        assert_eq!(batch.len(), 5); // Capped at remaining
    }

    #[test]
    fn test_reference_vs_approximate() {
        // The reference and approximate should differ (that's the point)
        let reference = compute_reference_response(1.5, 0.0, 0.1, 0.866, 550.0);
        let approximate = compute_approximate_response(1.5, 0.0, 0.1, 0.866, 550.0);

        // Should be similar but not identical
        let diff = (reference.reflectance - approximate.reflectance).abs();
        assert!(diff > 0.0, "Reference and approximate should differ");
        assert!(diff < 0.1, "Difference should be small");
    }

    #[test]
    fn test_memory_estimate() {
        let memory = estimate_dataset_memory(1000);
        assert!(
            memory < 500_000,
            "Memory {} too large for 1000 samples",
            memory
        );
    }
}
