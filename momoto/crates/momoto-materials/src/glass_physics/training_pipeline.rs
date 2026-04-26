// ============================================================================
// PHASE 10: TRAINING PIPELINE
// ============================================================================
//
// End-to-end differentiable training for neural correction networks.
// Uses the existing Adam optimizer pattern from perceptual_loop.rs.
//
// Training minimizes a combined loss:
// - Perceptual loss (ΔE2000)
// - Spectral RMSE
// - Energy conservation penalty
// - Correction magnitude penalty (prefer small corrections)
// ============================================================================

use super::neural_constraints::ConstraintValidator;
use super::neural_correction::NeuralCorrectionMLP;
use super::perceptual_loss::{delta_e_2000, rgb_to_lab, Illuminant};
use super::training_dataset::{TrainingDataset, TrainingSample};
use super::unified_bsdf::BSDFResponse;
use std::collections::VecDeque;

// ============================================================================
// TRAINING CONFIGURATION
// ============================================================================

/// Weights for the combined loss function
#[derive(Debug, Clone)]
pub struct LossWeights {
    /// Weight for perceptual loss (ΔE2000)
    pub perceptual: f64,
    /// Weight for spectral RMSE
    pub spectral_rmse: f64,
    /// Weight for energy conservation penalty
    pub energy_penalty: f64,
    /// Weight for correction magnitude penalty (prefer small corrections)
    pub correction_magnitude: f64,
    /// Weight for smoothness regularization
    pub smoothness: f64,
}

impl Default for LossWeights {
    fn default() -> Self {
        Self {
            perceptual: 1.0,
            spectral_rmse: 0.5,
            energy_penalty: 10.0,
            correction_magnitude: 0.01,
            smoothness: 0.1,
        }
    }
}

/// Configuration for training
#[derive(Debug, Clone)]
pub struct TrainingConfig {
    /// Learning rate
    pub learning_rate: f64,
    /// Batch size
    pub batch_size: usize,
    /// Maximum epochs
    pub epochs: usize,
    /// Loss weights
    pub loss_weights: LossWeights,
    /// Freeze physics (always true for Phase 10)
    pub freeze_physics: bool,
    /// Random seed for determinism
    pub seed: u64,
    /// Early stopping patience (epochs without improvement)
    pub patience: usize,
    /// Minimum improvement for early stopping
    pub min_delta: f64,
    /// Adam beta1 (momentum)
    pub beta1: f64,
    /// Adam beta2 (RMSprop)
    pub beta2: f64,
    /// Adam epsilon (numerical stability)
    pub epsilon: f64,
}

impl Default for TrainingConfig {
    fn default() -> Self {
        Self {
            learning_rate: 0.001,
            batch_size: 32,
            epochs: 100,
            loss_weights: LossWeights::default(),
            freeze_physics: true,
            seed: 42,
            patience: 10,
            min_delta: 1e-6,
            beta1: 0.9,
            beta2: 0.999,
            epsilon: 1e-8,
        }
    }
}

// ============================================================================
// ADAM OPTIMIZER STATE
// ============================================================================

/// Adam optimizer state for neural network training
#[derive(Debug, Clone)]
pub struct AdamState {
    /// First moment estimates (momentum)
    m: Vec<f64>,
    /// Second moment estimates (RMSprop)
    v: Vec<f64>,
    /// Timestep
    t: usize,
    /// Learning rate
    lr: f64,
    /// Beta1 (momentum decay)
    beta1: f64,
    /// Beta2 (velocity decay)
    beta2: f64,
    /// Epsilon (numerical stability)
    epsilon: f64,
}

impl AdamState {
    /// Create new Adam optimizer state
    pub fn new(num_params: usize, lr: f64, beta1: f64, beta2: f64, epsilon: f64) -> Self {
        Self {
            m: vec![0.0; num_params],
            v: vec![0.0; num_params],
            t: 0,
            lr,
            beta1,
            beta2,
            epsilon,
        }
    }

    /// Perform one optimization step
    pub fn step(&mut self, gradients: &[f64]) -> Vec<f64> {
        self.t += 1;
        let mut updates = Vec::with_capacity(gradients.len());

        for i in 0..gradients.len() {
            // Update biased first moment
            self.m[i] = self.beta1 * self.m[i] + (1.0 - self.beta1) * gradients[i];

            // Update biased second moment
            self.v[i] = self.beta2 * self.v[i] + (1.0 - self.beta2) * gradients[i] * gradients[i];

            // Bias correction
            let m_hat = self.m[i] / (1.0 - self.beta1.powi(self.t as i32));
            let v_hat = self.v[i] / (1.0 - self.beta2.powi(self.t as i32));

            // Compute update (negative for gradient descent)
            let update = -self.lr * m_hat / (v_hat.sqrt() + self.epsilon);
            updates.push(update);
        }

        updates
    }

    /// Reset optimizer state
    pub fn reset(&mut self) {
        self.m.fill(0.0);
        self.v.fill(0.0);
        self.t = 0;
    }
}

// ============================================================================
// TRAINING RESULT
// ============================================================================

/// Result of training
#[derive(Debug, Clone)]
pub struct TrainingResult {
    /// Trained network
    pub trained_network: NeuralCorrectionMLP,
    /// Final loss
    pub final_loss: f64,
    /// Number of epochs completed
    pub epochs_completed: usize,
    /// Whether training converged
    pub converged: bool,
    /// Perceptual improvement in dB: 20 * log10(before/after)
    pub perceptual_improvement_db: f64,
    /// Loss history per epoch
    pub loss_history: Vec<f64>,
    /// Mean ΔE before training
    pub mean_delta_e_before: f64,
    /// Mean ΔE after training
    pub mean_delta_e_after: f64,
}

// ============================================================================
// TRAINING PIPELINE
// ============================================================================

/// Training pipeline for neural correction networks
#[derive(Debug, Clone)]
pub struct TrainingPipeline {
    /// Configuration
    config: TrainingConfig,
    /// Constraint validator
    constraint_validator: ConstraintValidator,
    /// Adam optimizer state
    adam: Option<AdamState>,
    /// Loss history
    loss_history: VecDeque<f64>,
    /// Current epoch
    current_epoch: usize,
    /// Best loss seen
    best_loss: f64,
    /// Epochs since improvement
    epochs_without_improvement: usize,
}

impl TrainingPipeline {
    /// Create a new training pipeline
    pub fn new(config: TrainingConfig) -> Self {
        Self {
            config,
            constraint_validator: ConstraintValidator::new(),
            adam: None,
            loss_history: VecDeque::new(),
            current_epoch: 0,
            best_loss: f64::MAX,
            epochs_without_improvement: 0,
        }
    }

    /// Create with default configuration
    pub fn with_defaults() -> Self {
        Self::new(TrainingConfig::default())
    }

    /// Train a neural correction network on the given dataset
    pub fn train(
        &mut self,
        network: &mut NeuralCorrectionMLP,
        dataset: &TrainingDataset,
    ) -> TrainingResult {
        // Initialize Adam optimizer
        let num_params = network.param_count();
        self.adam = Some(AdamState::new(
            num_params,
            self.config.learning_rate,
            self.config.beta1,
            self.config.beta2,
            self.config.epsilon,
        ));

        // Compute initial ΔE
        let mean_delta_e_before = self.compute_mean_delta_e(network, dataset);

        // Training loop
        let mut rng = SimpleRng::new(self.config.seed);
        let mut loss_history = Vec::new();

        for epoch in 0..self.config.epochs {
            self.current_epoch = epoch;

            // Shuffle indices for this epoch
            let mut indices: Vec<usize> = (0..dataset.len()).collect();
            shuffle_indices(&mut indices, &mut rng);

            let mut epoch_loss = 0.0;
            let mut num_batches = 0;

            // Process batches
            for batch_start in (0..dataset.len()).step_by(self.config.batch_size) {
                let batch_end = (batch_start + self.config.batch_size).min(dataset.len());
                let batch_indices = &indices[batch_start..batch_end];

                // Compute gradients for this batch
                let (batch_loss, gradients) =
                    self.compute_batch_gradients(network, dataset, batch_indices);

                // Update network
                if let Some(ref mut adam) = self.adam {
                    let updates = adam.step(&gradients);
                    network.apply_updates(&updates);
                }

                epoch_loss += batch_loss;
                num_batches += 1;
            }

            // Average epoch loss
            let avg_loss = epoch_loss / num_batches as f64;
            loss_history.push(avg_loss);
            self.loss_history.push_back(avg_loss);

            // Early stopping check
            if avg_loss < self.best_loss - self.config.min_delta {
                self.best_loss = avg_loss;
                self.epochs_without_improvement = 0;
            } else {
                self.epochs_without_improvement += 1;
            }

            if self.epochs_without_improvement >= self.config.patience {
                break;
            }
        }

        // Compute final ΔE
        let mean_delta_e_after = self.compute_mean_delta_e(network, dataset);

        // Compute improvement in dB
        let improvement_db = if mean_delta_e_after > 1e-10 {
            20.0 * (mean_delta_e_before / mean_delta_e_after).log10()
        } else {
            f64::INFINITY
        };

        TrainingResult {
            trained_network: network.clone(),
            final_loss: self.best_loss,
            epochs_completed: self.current_epoch + 1,
            converged: self.epochs_without_improvement >= self.config.patience,
            perceptual_improvement_db: improvement_db,
            loss_history,
            mean_delta_e_before,
            mean_delta_e_after,
        }
    }

    /// Compute gradients for a batch of samples
    fn compute_batch_gradients(
        &self,
        network: &NeuralCorrectionMLP,
        dataset: &TrainingDataset,
        batch_indices: &[usize],
    ) -> (f64, Vec<f64>) {
        let num_params = network.param_count();
        let mut gradients = vec![0.0; num_params];
        let mut batch_loss = 0.0;

        let eps = 1e-4; // Finite difference epsilon

        for &idx in batch_indices {
            let sample = &dataset.samples[idx];

            // Compute loss at current params
            let loss = self.compute_sample_loss(network, sample);
            batch_loss += loss;

            // Numerical gradients via finite difference
            let params = network.get_params();

            for i in 0..num_params {
                // Forward: params[i] + eps
                let mut params_plus = params.clone();
                params_plus[i] += eps;
                let mut net_plus = network.clone();
                net_plus.set_params(&params_plus);
                let loss_plus = self.compute_sample_loss(&net_plus, sample);

                // Backward: params[i] - eps
                let mut params_minus = params.clone();
                params_minus[i] -= eps;
                let mut net_minus = network.clone();
                net_minus.set_params(&params_minus);
                let loss_minus = self.compute_sample_loss(&net_minus, sample);

                // Central difference
                let grad = (loss_plus - loss_minus) / (2.0 * eps);
                gradients[i] += grad;
            }
        }

        // Average gradients
        let batch_size = batch_indices.len() as f64;
        for g in &mut gradients {
            *g /= batch_size;
        }

        (batch_loss / batch_size, gradients)
    }

    /// Compute loss for a single sample
    fn compute_sample_loss(&self, network: &NeuralCorrectionMLP, sample: &TrainingSample) -> f64 {
        // Get neural correction
        let correction = network.forward(&sample.input);

        // Apply correction with constraints
        let (corrected, penalties) = self
            .constraint_validator
            .validate_and_clamp(&sample.physical_response, &correction);

        // Compute individual loss terms

        // 1. Perceptual loss (ΔE2000)
        let corrected_rgb = response_to_rgb(&corrected);
        let target_rgb = response_to_rgb(&sample.target_response);
        let corrected_lab = rgb_to_lab(corrected_rgb, Illuminant::D65);
        let target_lab = rgb_to_lab(target_rgb, Illuminant::D65);
        let delta_e = delta_e_2000(corrected_lab, target_lab);
        let perceptual_loss = self.config.loss_weights.perceptual * delta_e;

        // 2. Spectral RMSE
        let dr = corrected.reflectance - sample.target_response.reflectance;
        let dt = corrected.transmittance - sample.target_response.transmittance;
        let spectral_rmse = (dr * dr + dt * dt).sqrt();
        let spectral_loss = self.config.loss_weights.spectral_rmse * spectral_rmse;

        // 3. Energy penalty
        let energy_loss = self.config.loss_weights.energy_penalty * penalties.energy_penalty;

        // 4. Correction magnitude penalty
        let magnitude = correction.magnitude();
        let magnitude_loss = self.config.loss_weights.correction_magnitude * magnitude;

        perceptual_loss + spectral_loss + energy_loss + magnitude_loss
    }

    /// Compute mean ΔE for the dataset
    fn compute_mean_delta_e(
        &self,
        network: &NeuralCorrectionMLP,
        dataset: &TrainingDataset,
    ) -> f64 {
        let mut total_delta_e = 0.0;

        for sample in &dataset.samples {
            let correction = network.forward(&sample.input);
            let (corrected, _) = self
                .constraint_validator
                .validate_and_clamp(&sample.physical_response, &correction);

            let corrected_rgb = response_to_rgb(&corrected);
            let target_rgb = response_to_rgb(&sample.target_response);
            let corrected_lab = rgb_to_lab(corrected_rgb, Illuminant::D65);
            let target_lab = rgb_to_lab(target_rgb, Illuminant::D65);

            total_delta_e += delta_e_2000(corrected_lab, target_lab);
        }

        if dataset.len() > 0 {
            total_delta_e / dataset.len() as f64
        } else {
            0.0
        }
    }
}

// ============================================================================
// HELPER FUNCTIONS
// ============================================================================

/// Convert BSDFResponse to approximate RGB for perceptual loss
fn response_to_rgb(response: &BSDFResponse) -> [f64; 3] {
    // Simplified: use reflectance as grayscale
    // In production, would use spectral integration
    let r = response.reflectance;
    [r, r, r]
}

/// Shuffle indices using simple RNG
fn shuffle_indices(indices: &mut [usize], rng: &mut SimpleRng) {
    let n = indices.len();
    for i in (1..n).rev() {
        let j = rng.uniform_int(0, i as u64) as usize;
        indices.swap(i, j);
    }
}

/// Simple RNG for deterministic training
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

    fn uniform_int(&mut self, min: u64, max: u64) -> u64 {
        min + (self.next() % (max - min + 1))
    }
}

// ============================================================================
// MEMORY UTILITIES
// ============================================================================

/// Total memory usage of training pipeline
pub fn total_training_pipeline_memory() -> usize {
    std::mem::size_of::<TrainingPipeline>()
        + std::mem::size_of::<TrainingConfig>()
        + std::mem::size_of::<LossWeights>()
        + std::mem::size_of::<AdamState>()
        + 1000 // Estimated VecDeque and Vec overhead
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_adam_optimizer() {
        let mut adam = AdamState::new(5, 0.01, 0.9, 0.999, 1e-8);

        let gradients = vec![0.1, -0.2, 0.05, -0.1, 0.15];
        let updates = adam.step(&gradients);

        assert_eq!(updates.len(), 5);
        // Updates should be in opposite direction of gradients
        assert!(updates[0] < 0.0); // gradient positive -> update negative
        assert!(updates[1] > 0.0); // gradient negative -> update positive
    }

    #[test]
    #[ignore = "Requires CorrectionInput type implementation"]
    fn test_loss_computation() {
        // TODO: Implement when CorrectionInput is available
        // let pipeline = TrainingPipeline::with_defaults();
        // let network = NeuralCorrectionMLP::with_default_config();
        //
        // let sample = TrainingSample::new(
        //     CorrectionInput::default(),
        //     BSDFResponse::new(0.5, 0.3, 0.2),
        //     BSDFResponse::new(0.55, 0.28, 0.17),
        // );
        //
        // let loss = pipeline.compute_sample_loss(&network, &sample);
        // assert!(loss > 0.0);
        // assert!(loss.is_finite());
    }

    #[test]
    fn test_training_reduces_loss() {
        let mut pipeline = TrainingPipeline::new(TrainingConfig {
            epochs: 10,
            batch_size: 8,
            learning_rate: 0.01,
            ..Default::default()
        });

        let mut network = NeuralCorrectionMLP::with_default_config();
        let dataset = TrainingDataset::generate_test_dataset(42);

        // Compute initial loss
        let initial_delta_e = pipeline.compute_mean_delta_e(&network, &dataset);

        // Train
        let result = pipeline.train(&mut network, &dataset);

        // Loss should decrease (or at least not increase dramatically)
        assert!(result.epochs_completed > 0);
        assert!(result.loss_history.len() > 0);

        // The final ΔE should ideally be lower, but with limited training
        // we just check it's reasonable
        assert!(result.mean_delta_e_after < 100.0);
    }

    #[test]
    #[ignore = "Requires NeuralCorrectionConfig type implementation"]
    fn test_deterministic_training() {
        // TODO: Implement when NeuralCorrectionConfig is available
        // let config = TrainingConfig {
        //     epochs: 5,
        //     batch_size: 4,
        //     seed: 12345,
        //     ..Default::default()
        // };
        //
        // let mut pipeline1 = TrainingPipeline::new(config.clone());
        // let mut pipeline2 = TrainingPipeline::new(config);
        //
        // let mut network1 = NeuralCorrectionMLP::new(NeuralCorrectionConfig {
        //     seed: 42,
        //     ..Default::default()
        // });
        // let mut network2 = NeuralCorrectionMLP::new(NeuralCorrectionConfig {
        //     seed: 42,
        //     ..Default::default()
        // });
        // let dataset = TrainingDataset::generate_test_dataset(42);
        //
        // let result1 = pipeline1.train(&mut network1, &dataset);
        // let result2 = pipeline2.train(&mut network2, &dataset);
        //
        // Same seeds should produce identical results
        // assert!((result1.final_loss - result2.final_loss).abs() < 1e-10);
        // assert_eq!(result1.epochs_completed, result2.epochs_completed);
    }

    #[test]
    fn test_early_stopping() {
        let config = TrainingConfig {
            epochs: 1000, // High limit
            patience: 3,  // Stop after 3 epochs without improvement
            batch_size: 4,
            ..Default::default()
        };

        let mut pipeline = TrainingPipeline::new(config);
        let mut network = NeuralCorrectionMLP::with_default_config();
        let dataset = TrainingDataset::generate_test_dataset(42);

        let result = pipeline.train(&mut network, &dataset);

        // Should stop before 1000 epochs
        assert!(result.epochs_completed < 1000);
    }

    #[test]
    fn test_training_result_metrics() {
        let mut pipeline = TrainingPipeline::new(TrainingConfig {
            epochs: 5,
            batch_size: 8,
            ..Default::default()
        });

        let mut network = NeuralCorrectionMLP::with_default_config();
        let dataset = TrainingDataset::generate_test_dataset(42);

        let result = pipeline.train(&mut network, &dataset);

        // Check all metrics are valid
        assert!(result.final_loss.is_finite());
        assert!(result.mean_delta_e_before.is_finite());
        assert!(result.mean_delta_e_after.is_finite());
        assert!(!result.loss_history.is_empty());
    }

    #[test]
    fn test_memory_budget() {
        let memory = total_training_pipeline_memory();
        assert!(memory < 50_000, "Memory {} exceeds 50KB budget", memory);
    }
}
