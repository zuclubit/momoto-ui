//! # Real-Time Auto-Calibration Module (Phase 7)
//!
//! Frame-budgeted auto-calibration with CIEDE2000 perceptual feedback loop.
//! Enables real-time material parameter optimization for perceptual matching.
//!
//! ## Features
//!
//! - Frame-budgeted optimization (configurable iterations per frame)
//! - CIEDE2000 perceptual loss function
//! - Adam optimizer with momentum
//! - Convergence detection and rollback
//! - Material database comparison
//!
//! ## Usage
//!
//! ```rust,ignore
//! let mut loop = CalibrationFeedbackLoop::new(RealtimeCalibrationConfig::default());
//! while !loop.is_converged() {
//!     loop.step(&mut params, &reference, forward_fn);
//! }
//! ```

use std::collections::VecDeque;
use std::time::{Duration, Instant};

use super::differentiable_render::MaterialParams;
use super::material_datasets::MaterialDatabase;
use super::perceptual_loss::{delta_e_2000, rgb_to_lab, DeltaEFormula};

// ============================================================================
// CONFIGURATION
// ============================================================================

/// Real-time calibration configuration
#[derive(Debug, Clone)]
pub struct RealtimeCalibrationConfig {
    /// Delta E formula to use
    pub formula: DeltaEFormula,
    /// Maximum optimizer iterations per frame
    pub max_iterations_per_frame: usize,
    /// Learning rate for Adam optimizer
    pub learning_rate: f64,
    /// Target Delta E tolerance (convergence threshold)
    pub tolerance: f64,
    /// Use momentum (Adam vs SGD)
    pub use_momentum: bool,
    /// Beta1 for Adam momentum
    pub beta1: f64,
    /// Beta2 for Adam velocity
    pub beta2: f64,
    /// Epsilon for numerical stability
    pub epsilon: f64,
    /// Maximum total iterations before declaring stall
    pub max_total_iterations: usize,
    /// History window size for convergence detection
    pub history_window: usize,
    /// Minimum improvement rate to avoid stall
    pub min_improvement_rate: f64,
}

impl RealtimeCalibrationConfig {
    /// Create default configuration
    pub fn new() -> Self {
        Self {
            formula: DeltaEFormula::CIEDE2000,
            max_iterations_per_frame: 10,
            learning_rate: 0.001,
            tolerance: 1.0, // Delta E < 1.0 is imperceptible
            use_momentum: true,
            beta1: 0.9,
            beta2: 0.999,
            epsilon: 1e-8,
            max_total_iterations: 1000,
            history_window: 20,
            min_improvement_rate: 0.001,
        }
    }

    /// Fast configuration (fewer iterations, higher learning rate)
    pub fn fast() -> Self {
        Self {
            max_iterations_per_frame: 5,
            learning_rate: 0.01,
            tolerance: 2.0,
            ..Self::new()
        }
    }

    /// Precise configuration (more iterations, lower learning rate)
    pub fn precise() -> Self {
        Self {
            max_iterations_per_frame: 20,
            learning_rate: 0.0001,
            tolerance: 0.5,
            ..Self::new()
        }
    }

    /// Set tolerance
    pub fn with_tolerance(mut self, tolerance: f64) -> Self {
        self.tolerance = tolerance;
        self
    }

    /// Set learning rate
    pub fn with_learning_rate(mut self, lr: f64) -> Self {
        self.learning_rate = lr;
        self
    }
}

impl Default for RealtimeCalibrationConfig {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// CONVERGENCE STATUS
// ============================================================================

/// Convergence status of calibration
#[derive(Debug, Clone)]
pub enum ConvergenceStatus {
    /// Not started yet
    NotStarted,
    /// In progress with current metrics
    InProgress {
        iterations: usize,
        current_loss: f64,
        improvement_rate: f64,
    },
    /// Successfully converged
    Converged { iterations: usize, final_loss: f64 },
    /// Stalled (no improvement)
    Stalled { iterations: usize, best_loss: f64 },
    /// Hit iteration limit
    MaxIterations { iterations: usize, final_loss: f64 },
}

impl ConvergenceStatus {
    /// Check if calibration is done (converged, stalled, or max iterations)
    pub fn is_done(&self) -> bool {
        matches!(
            self,
            ConvergenceStatus::Converged { .. }
                | ConvergenceStatus::Stalled { .. }
                | ConvergenceStatus::MaxIterations { .. }
        )
    }

    /// Check if successfully converged
    pub fn is_converged(&self) -> bool {
        matches!(self, ConvergenceStatus::Converged { .. })
    }

    /// Get current loss if available
    pub fn loss(&self) -> Option<f64> {
        match self {
            ConvergenceStatus::NotStarted => None,
            ConvergenceStatus::InProgress { current_loss, .. } => Some(*current_loss),
            ConvergenceStatus::Converged { final_loss, .. } => Some(*final_loss),
            ConvergenceStatus::Stalled { best_loss, .. } => Some(*best_loss),
            ConvergenceStatus::MaxIterations { final_loss, .. } => Some(*final_loss),
        }
    }
}

// ============================================================================
// CALIBRATION RESULT
// ============================================================================

/// Result of calibration process
#[derive(Debug, Clone)]
pub struct CalibrationResult {
    /// Final convergence status
    pub status: ConvergenceStatus,
    /// Final Delta E value
    pub final_delta_e: f64,
    /// Number of iterations performed
    pub iterations: usize,
    /// Total time taken
    pub elapsed_ms: f64,
    /// Loss history
    pub loss_history: Vec<f64>,
    /// Final parameters (if available)
    pub final_params: Option<MaterialParams>,
}

impl CalibrationResult {
    /// Check if calibration succeeded
    pub fn success(&self) -> bool {
        self.status.is_converged()
    }

    /// Generate summary string
    pub fn summary(&self) -> String {
        let status_str = match &self.status {
            ConvergenceStatus::NotStarted => "Not Started",
            ConvergenceStatus::InProgress { .. } => "In Progress",
            ConvergenceStatus::Converged { .. } => "Converged",
            ConvergenceStatus::Stalled { .. } => "Stalled",
            ConvergenceStatus::MaxIterations { .. } => "Max Iterations",
        };

        format!(
            "Calibration {}: ΔE={:.2}, {} iterations, {:.1}ms",
            status_str, self.final_delta_e, self.iterations, self.elapsed_ms
        )
    }
}

// ============================================================================
// ADAM OPTIMIZER STATE
// ============================================================================

/// Adam optimizer state for a parameter vector
#[derive(Debug, Clone)]
struct AdamState {
    /// First moment (momentum)
    m: Vec<f64>,
    /// Second moment (velocity)
    v: Vec<f64>,
    /// Timestep
    t: usize,
}

impl AdamState {
    fn new(size: usize) -> Self {
        Self {
            m: vec![0.0; size],
            v: vec![0.0; size],
            t: 0,
        }
    }

    fn step(
        &mut self,
        params: &mut [f64],
        gradients: &[f64],
        lr: f64,
        beta1: f64,
        beta2: f64,
        epsilon: f64,
    ) {
        self.t += 1;

        for i in 0..params.len() {
            // Update biased first moment
            self.m[i] = beta1 * self.m[i] + (1.0 - beta1) * gradients[i];
            // Update biased second moment
            self.v[i] = beta2 * self.v[i] + (1.0 - beta2) * gradients[i] * gradients[i];

            // Compute bias-corrected moments
            let m_hat = self.m[i] / (1.0 - beta1.powi(self.t as i32));
            let v_hat = self.v[i] / (1.0 - beta2.powi(self.t as i32));

            // Update parameters
            params[i] -= lr * m_hat / (v_hat.sqrt() + epsilon);
        }
    }
}

// ============================================================================
// CALIBRATION FEEDBACK LOOP
// ============================================================================

/// Real-time calibration feedback loop
#[derive(Debug, Clone)]
pub struct CalibrationFeedbackLoop {
    config: RealtimeCalibrationConfig,
    adam_state: Option<AdamState>,
    current_loss: f64,
    best_loss: f64,
    best_params: Option<Vec<f64>>,
    iterations: usize,
    loss_history: VecDeque<f64>,
    status: ConvergenceStatus,
}

impl CalibrationFeedbackLoop {
    /// Create new feedback loop
    pub fn new(config: RealtimeCalibrationConfig) -> Self {
        Self {
            config,
            adam_state: None,
            current_loss: f64::MAX,
            best_loss: f64::MAX,
            best_params: None,
            iterations: 0,
            loss_history: VecDeque::with_capacity(100),
            status: ConvergenceStatus::NotStarted,
        }
    }

    /// Reset calibration state
    pub fn reset(&mut self) {
        self.adam_state = None;
        self.current_loss = f64::MAX;
        self.best_loss = f64::MAX;
        self.best_params = None;
        self.iterations = 0;
        self.loss_history.clear();
        self.status = ConvergenceStatus::NotStarted;
    }

    /// Check if converged
    pub fn is_converged(&self) -> bool {
        self.status.is_converged()
    }

    /// Check if done
    pub fn is_done(&self) -> bool {
        self.status.is_done()
    }

    /// Get current status
    pub fn status(&self) -> &ConvergenceStatus {
        &self.status
    }

    /// Get current loss
    pub fn current_loss(&self) -> f64 {
        self.current_loss
    }

    /// Get best loss achieved
    pub fn best_loss(&self) -> f64 {
        self.best_loss
    }

    /// Perform one calibration step
    pub fn step<F>(
        &mut self,
        params: &mut MaterialParams,
        reference_rgb: &[[f64; 3]],
        forward_fn: F,
    ) -> f64
    where
        F: Fn(&MaterialParams) -> Vec<[f64; 3]>,
    {
        // Initialize Adam state if needed
        let param_vec = params.to_vec();
        if self.adam_state.is_none() {
            self.adam_state = Some(AdamState::new(param_vec.len()));
        }

        let mut current_params = param_vec.clone();

        // Run iterations for this frame
        for _ in 0..self.config.max_iterations_per_frame {
            self.iterations += 1;

            // Forward pass
            let rendered = forward_fn(params);

            // Compute loss (mean Delta E)
            let loss = compute_mean_delta_e(&rendered, reference_rgb, self.config.formula);
            self.current_loss = loss;
            self.loss_history.push_back(loss);

            // Keep history bounded
            while self.loss_history.len() > self.config.history_window {
                self.loss_history.pop_front();
            }

            // Track best
            if loss < self.best_loss {
                self.best_loss = loss;
                self.best_params = Some(current_params.clone());
            }

            // Check convergence
            if loss < self.config.tolerance {
                self.status = ConvergenceStatus::Converged {
                    iterations: self.iterations,
                    final_loss: loss,
                };
                return loss;
            }

            // Check max iterations
            if self.iterations >= self.config.max_total_iterations {
                self.status = ConvergenceStatus::MaxIterations {
                    iterations: self.iterations,
                    final_loss: loss,
                };
                return loss;
            }

            // Check for stall
            if self.loss_history.len() >= self.config.history_window {
                let improvement = self.compute_improvement_rate();
                if improvement < self.config.min_improvement_rate {
                    self.status = ConvergenceStatus::Stalled {
                        iterations: self.iterations,
                        best_loss: self.best_loss,
                    };
                    return loss;
                }
            }

            // Compute gradients (numerical)
            let gradients =
                compute_gradient(params, reference_rgb, &forward_fn, self.config.formula);

            // Adam update
            if let Some(ref mut adam) = self.adam_state {
                adam.step(
                    &mut current_params,
                    &gradients,
                    self.config.learning_rate,
                    self.config.beta1,
                    self.config.beta2,
                    self.config.epsilon,
                );
            }

            // Update params from vector
            *params = MaterialParams::from_vec(&current_params);
            params.clamp_valid();
        }

        // Update status
        let improvement = if self.loss_history.len() >= 2 {
            self.compute_improvement_rate()
        } else {
            1.0
        };

        self.status = ConvergenceStatus::InProgress {
            iterations: self.iterations,
            current_loss: self.current_loss,
            improvement_rate: improvement,
        };

        self.current_loss
    }

    /// Compute improvement rate from history
    fn compute_improvement_rate(&self) -> f64 {
        if self.loss_history.len() < 2 {
            return 1.0;
        }

        let first = *self.loss_history.front().unwrap();
        let last = *self.loss_history.back().unwrap();

        if first > 0.0 {
            (first - last) / first
        } else {
            0.0
        }
    }

    /// Restore best parameters
    pub fn restore_best(&self, params: &mut MaterialParams) {
        if let Some(ref best) = self.best_params {
            *params = MaterialParams::from_vec(best);
        }
    }

    /// Get loss history
    pub fn loss_history(&self) -> &VecDeque<f64> {
        &self.loss_history
    }
}

// ============================================================================
// HELPER FUNCTIONS
// ============================================================================

/// Compute mean Delta E between rendered and reference
fn compute_mean_delta_e(
    rendered: &[[f64; 3]],
    reference: &[[f64; 3]],
    formula: DeltaEFormula,
) -> f64 {
    if rendered.len() != reference.len() || rendered.is_empty() {
        return f64::MAX;
    }

    let mut total = 0.0;
    for i in 0..rendered.len() {
        let lab1 = rgb_to_lab(rendered[i], super::perceptual_loss::Illuminant::D65);
        let lab2 = rgb_to_lab(reference[i], super::perceptual_loss::Illuminant::D65);

        total += match formula {
            DeltaEFormula::CIE76 => super::perceptual_loss::delta_e_76(lab1, lab2),
            DeltaEFormula::CIE94 => super::perceptual_loss::delta_e_94(lab1, lab2),
            DeltaEFormula::CIEDE2000 => delta_e_2000(lab1, lab2),
        };
    }

    total / rendered.len() as f64
}

/// Compute numerical gradient
fn compute_gradient<F>(
    params: &MaterialParams,
    reference: &[[f64; 3]],
    forward_fn: &F,
    formula: DeltaEFormula,
) -> Vec<f64>
where
    F: Fn(&MaterialParams) -> Vec<[f64; 3]>,
{
    let h = 1e-5; // Finite difference step
    let param_vec = params.to_vec();
    let mut gradients = vec![0.0; param_vec.len()];

    for i in 0..param_vec.len() {
        // Forward step
        let mut forward_params = param_vec.clone();
        forward_params[i] += h;
        let forward_mp = MaterialParams::from_vec(&forward_params);
        let forward_render = forward_fn(&forward_mp);
        let forward_loss = compute_mean_delta_e(&forward_render, reference, formula);

        // Backward step
        let mut backward_params = param_vec.clone();
        backward_params[i] -= h;
        let backward_mp = MaterialParams::from_vec(&backward_params);
        let backward_render = forward_fn(&backward_mp);
        let backward_loss = compute_mean_delta_e(&backward_render, reference, formula);

        // Central difference
        gradients[i] = (forward_loss - backward_loss) / (2.0 * h);
    }

    gradients
}

// ============================================================================
// HIGH-LEVEL API
// ============================================================================

/// Compare material to database and return similarity scores
pub fn compare_to_dataset(
    material_rgb: &[f64; 3],
    database: &MaterialDatabase,
) -> Vec<(String, f64)> {
    let material_lab = rgb_to_lab(*material_rgb, super::perceptual_loss::Illuminant::D65);
    let mut results = Vec::new();

    for name in database.names() {
        if let Some(measurement) = database.get(name) {
            // Get representative color from material (center of spectrum)
            if let Some(&r) = measurement.reflectance.get(15) {
                let ref_rgb = [r, r, r]; // Simplified - should use full spectral
                let ref_lab = rgb_to_lab(ref_rgb, super::perceptual_loss::Illuminant::D65);
                let delta_e = delta_e_2000(material_lab, ref_lab);
                results.push((name.to_string(), delta_e));
            }
        }
    }

    // Sort by similarity (lowest Delta E first)
    results.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));
    results
}

/// Run calibration with time budget
pub fn realtime_calibrate<F>(
    params: &mut MaterialParams,
    reference_rgb: &[[f64; 3]],
    forward_fn: F,
    budget_ms: f64,
    config: RealtimeCalibrationConfig,
) -> CalibrationResult
where
    F: Fn(&MaterialParams) -> Vec<[f64; 3]>,
{
    let start = Instant::now();
    let budget = Duration::from_secs_f64(budget_ms / 1000.0);

    let mut feedback = CalibrationFeedbackLoop::new(config);
    let mut loss_history = Vec::new();

    while !feedback.is_done() && start.elapsed() < budget {
        let loss = feedback.step(params, reference_rgb, &forward_fn);
        loss_history.push(loss);
    }

    // Restore best parameters if we stalled
    if matches!(feedback.status(), ConvergenceStatus::Stalled { .. }) {
        feedback.restore_best(params);
    }

    CalibrationResult {
        status: feedback.status().clone(),
        final_delta_e: feedback.current_loss(),
        iterations: feedback.iterations,
        elapsed_ms: start.elapsed().as_secs_f64() * 1000.0,
        loss_history,
        final_params: Some(params.clone()),
    }
}

/// Quick perceptual match score (single RGB comparison)
pub fn perceptual_match_score(rendered: &[f64; 3], reference: &[f64; 3]) -> f64 {
    let lab1 = rgb_to_lab(*rendered, super::perceptual_loss::Illuminant::D65);
    let lab2 = rgb_to_lab(*reference, super::perceptual_loss::Illuminant::D65);
    delta_e_2000(lab1, lab2)
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn dummy_forward(_params: &MaterialParams) -> Vec<[f64; 3]> {
        vec![[0.5, 0.5, 0.5]; 10]
    }

    #[test]
    fn test_config_defaults() {
        let config = RealtimeCalibrationConfig::default();
        assert_eq!(config.max_iterations_per_frame, 10);
        assert!((config.learning_rate - 0.001).abs() < 1e-6);
        assert!((config.tolerance - 1.0).abs() < 1e-6);
    }

    #[test]
    fn test_convergence_status() {
        let status = ConvergenceStatus::NotStarted;
        assert!(!status.is_done());
        assert!(!status.is_converged());

        let status = ConvergenceStatus::Converged {
            iterations: 10,
            final_loss: 0.5,
        };
        assert!(status.is_done());
        assert!(status.is_converged());

        let status = ConvergenceStatus::Stalled {
            iterations: 100,
            best_loss: 2.0,
        };
        assert!(status.is_done());
        assert!(!status.is_converged());
    }

    #[test]
    fn test_feedback_loop_creation() {
        let config = RealtimeCalibrationConfig::default();
        let feedback = CalibrationFeedbackLoop::new(config);

        assert!(!feedback.is_done());
        assert!(!feedback.is_converged());
        assert_eq!(feedback.current_loss(), f64::MAX);
    }

    #[test]
    fn test_feedback_loop_step() {
        let config = RealtimeCalibrationConfig::new().with_tolerance(100.0); // High tolerance for test

        let mut feedback = CalibrationFeedbackLoop::new(config);
        let mut params = MaterialParams::default();
        let reference: Vec<[f64; 3]> = vec![[0.5, 0.5, 0.5]; 10];

        let loss = feedback.step(&mut params, &reference, dummy_forward);

        assert!(loss < f64::MAX);
        assert!(feedback.current_loss() < f64::MAX);
    }

    #[test]
    fn test_perceptual_match_score() {
        let same = [0.5, 0.5, 0.5];
        let score = perceptual_match_score(&same, &same);
        assert!(score < 0.01); // Same color should have ~0 Delta E

        let different = [0.8, 0.2, 0.1];
        let score = perceptual_match_score(&same, &different);
        assert!(score > 10.0); // Different colors should have high Delta E
    }

    #[test]
    fn test_adam_state() {
        let mut adam = AdamState::new(3);
        let mut params = vec![1.0, 2.0, 3.0];
        let gradients = vec![0.1, 0.2, 0.3];

        adam.step(&mut params, &gradients, 0.01, 0.9, 0.999, 1e-8);

        // Parameters should have changed
        assert!((params[0] - 1.0).abs() > 1e-6);
        assert!((params[1] - 2.0).abs() > 1e-6);
        assert!((params[2] - 3.0).abs() > 1e-6);

        // Timestep should have incremented
        assert_eq!(adam.t, 1);
    }

    #[test]
    fn test_compute_mean_delta_e() {
        let same: Vec<[f64; 3]> = vec![[0.5, 0.5, 0.5]; 5];
        let delta_e = compute_mean_delta_e(&same, &same, DeltaEFormula::CIEDE2000);
        assert!(delta_e < 0.01);

        let rendered: Vec<[f64; 3]> = vec![[0.5, 0.5, 0.5]; 5];
        let reference: Vec<[f64; 3]> = vec![[0.6, 0.6, 0.6]; 5];
        let delta_e = compute_mean_delta_e(&rendered, &reference, DeltaEFormula::CIEDE2000);
        assert!(delta_e > 1.0);
    }

    #[test]
    fn test_calibration_result_summary() {
        let result = CalibrationResult {
            status: ConvergenceStatus::Converged {
                iterations: 50,
                final_loss: 0.8,
            },
            final_delta_e: 0.8,
            iterations: 50,
            elapsed_ms: 100.0,
            loss_history: vec![5.0, 3.0, 1.0, 0.8],
            final_params: None,
        };

        let summary = result.summary();
        assert!(summary.contains("Converged"));
        assert!(summary.contains("0.8"));
        assert!(summary.contains("50"));
    }
}
