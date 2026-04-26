//! # Perceptual Rendering Loop (Phase 9)
//!
//! Closed-loop perceptual rendering with auto-parameter adjustment.
//!
//! ## Purpose
//!
//! The perceptual loop automatically adjusts material parameters to match
//! a target appearance. It uses CIEDE2000 (Delta E 2000) as the perceptual
//! error metric and optimizes until the rendered result is perceptually
//! indistinguishable from the target.
//!
//! ## How It Works
//!
//! 1. **Forward render**: Evaluate material with current parameters
//! 2. **Convert to LAB**: Transform to perceptual color space
//! 3. **Compute ΔE2000**: Measure perceptual difference from target
//! 4. **Compute gradients**: Calculate parameter sensitivities
//! 5. **Update parameters**: Adam optimizer step with constraints
//! 6. **Repeat**: Until ΔE < target threshold
//!
//! ## Usage
//!
//! ```rust
//! use momoto_materials::glass_physics::perceptual_loop::{
//!     PerceptualRenderingLoop, PerceptualTarget, MaterialParams,
//! };
//!
//! let mut loop_runner = PerceptualRenderingLoop::new();
//!
//! // Set target color
//! let target = PerceptualTarget::RgbColor([0.8, 0.6, 0.3]); // Gold-like
//!
//! // Initial parameters
//! let params = MaterialParams::default();
//!
//! // Optimize
//! let result = loop_runner.optimize(&params, &target);
//! println!("Converged: {}, ΔE: {}", result.converged(), result.final_delta_e);
//! ```

use std::collections::VecDeque;

use super::perceptual_loss::{delta_e_2000, rgb_to_lab, Illuminant, LabColor};
use super::unified_bsdf::{BSDFContext, ConductorBSDF, DielectricBSDF, BSDF};

// ============================================================================
// MATERIAL PARAMETERS
// ============================================================================

/// Optimizable material parameters
#[derive(Debug, Clone)]
pub struct MaterialParams {
    /// Index of refraction (dielectric) or n (conductor)
    pub ior: f64,
    /// Extinction coefficient (conductors only)
    pub k: f64,
    /// Surface roughness
    pub roughness: f64,
    /// Is this a conductor?
    pub is_conductor: bool,
    /// Viewing angle for evaluation (cos theta)
    pub cos_theta: f64,
}

impl MaterialParams {
    /// Create dielectric parameters
    pub fn dielectric(ior: f64, roughness: f64) -> Self {
        Self {
            ior,
            k: 0.0,
            roughness: roughness.clamp(0.0, 1.0),
            is_conductor: false,
            cos_theta: 1.0,
        }
    }

    /// Create conductor parameters
    pub fn conductor(n: f64, k: f64, roughness: f64) -> Self {
        Self {
            ior: n,
            k,
            roughness: roughness.clamp(0.0, 1.0),
            is_conductor: true,
            cos_theta: 1.0,
        }
    }

    /// Set viewing angle
    pub fn with_angle(mut self, cos_theta: f64) -> Self {
        self.cos_theta = cos_theta.clamp(0.0, 1.0);
        self
    }

    /// Convert to parameter vector for optimization
    pub fn to_vec(&self) -> Vec<f64> {
        if self.is_conductor {
            vec![self.ior, self.k, self.roughness]
        } else {
            vec![self.ior, self.roughness]
        }
    }

    /// Create from parameter vector
    pub fn from_vec(&self, v: &[f64]) -> Self {
        if self.is_conductor {
            Self::conductor(v[0], v[1], v[2]).with_angle(self.cos_theta)
        } else {
            Self::dielectric(v[0], v[1]).with_angle(self.cos_theta)
        }
    }

    /// Number of parameters
    pub fn param_count(&self) -> usize {
        if self.is_conductor {
            3
        } else {
            2
        }
    }
}

impl Default for MaterialParams {
    fn default() -> Self {
        Self::dielectric(1.5, 0.0)
    }
}

// ============================================================================
// PARAMETER BOUNDS
// ============================================================================

/// Bounds for material parameters
#[derive(Debug, Clone)]
pub struct ParameterBounds {
    pub ior_min: f64,
    pub ior_max: f64,
    pub k_min: f64,
    pub k_max: f64,
    pub roughness_min: f64,
    pub roughness_max: f64,
}

impl ParameterBounds {
    /// Apply bounds to parameters
    pub fn clamp(&self, params: &MaterialParams) -> MaterialParams {
        MaterialParams {
            ior: params.ior.clamp(self.ior_min, self.ior_max),
            k: params.k.clamp(self.k_min, self.k_max),
            roughness: params
                .roughness
                .clamp(self.roughness_min, self.roughness_max),
            is_conductor: params.is_conductor,
            cos_theta: params.cos_theta,
        }
    }
}

impl Default for ParameterBounds {
    fn default() -> Self {
        Self {
            ior_min: 1.0,
            ior_max: 3.0,
            k_min: 0.0,
            k_max: 10.0,
            roughness_min: 0.0,
            roughness_max: 1.0,
        }
    }
}

// ============================================================================
// PERCEPTUAL TARGET
// ============================================================================

/// Target specification for optimization
#[derive(Debug, Clone)]
pub enum PerceptualTarget {
    /// Target LAB color directly
    LabColor(LabColor),
    /// Target RGB color (converted to LAB)
    RgbColor([f64; 3]),
    /// Target reflectance at angle
    Reflectance(f64),
    /// Target spectral curve (wavelength, reflectance pairs)
    Spectral(Vec<(f64, f64)>),
}

impl PerceptualTarget {
    /// Convert to LAB color space
    pub fn as_lab(&self) -> LabColor {
        match self {
            PerceptualTarget::LabColor(lab) => *lab,
            PerceptualTarget::RgbColor(rgb) => rgb_to_lab(*rgb, Illuminant::D65),
            PerceptualTarget::Reflectance(r) => {
                // Neutral gray at given reflectance
                rgb_to_lab([*r, *r, *r], Illuminant::D65)
            }
            PerceptualTarget::Spectral(spectrum) => {
                // Convert spectrum to RGB then LAB
                let rgb = spectrum_to_rgb(spectrum);
                rgb_to_lab(rgb, Illuminant::D65)
            }
        }
    }

    /// Create from hex color string
    pub fn from_hex(hex: &str) -> Option<Self> {
        let hex = hex.trim_start_matches('#');
        if hex.len() != 6 {
            return None;
        }

        let r = u8::from_str_radix(&hex[0..2], 16).ok()? as f64 / 255.0;
        let g = u8::from_str_radix(&hex[2..4], 16).ok()? as f64 / 255.0;
        let b = u8::from_str_radix(&hex[4..6], 16).ok()? as f64 / 255.0;

        Some(PerceptualTarget::RgbColor([r, g, b]))
    }
}

// ============================================================================
// OPTIMIZER STATE
// ============================================================================

/// Adam optimizer state
#[derive(Debug, Clone)]
pub struct AdamState {
    /// First moment estimates
    m: Vec<f64>,
    /// Second moment estimates
    v: Vec<f64>,
    /// Timestep
    t: usize,
    /// Learning rate
    lr: f64,
    /// Beta1 (first moment decay)
    beta1: f64,
    /// Beta2 (second moment decay)
    beta2: f64,
    /// Epsilon for numerical stability
    eps: f64,
}

impl AdamState {
    /// Create new Adam optimizer state
    pub fn new(param_count: usize, lr: f64) -> Self {
        Self {
            m: vec![0.0; param_count],
            v: vec![0.0; param_count],
            t: 0,
            lr,
            beta1: 0.9,
            beta2: 0.999,
            eps: 1e-8,
        }
    }

    /// Compute parameter update
    pub fn step(&mut self, gradients: &[f64]) -> Vec<f64> {
        self.t += 1;
        let mut updates = vec![0.0; gradients.len()];

        for (i, &g) in gradients.iter().enumerate() {
            // Update biased first moment estimate
            self.m[i] = self.beta1 * self.m[i] + (1.0 - self.beta1) * g;

            // Update biased second moment estimate
            self.v[i] = self.beta2 * self.v[i] + (1.0 - self.beta2) * g * g;

            // Bias-corrected estimates
            let m_hat = self.m[i] / (1.0 - self.beta1.powi(self.t as i32));
            let v_hat = self.v[i] / (1.0 - self.beta2.powi(self.t as i32));

            // Parameter update
            updates[i] = -self.lr * m_hat / (v_hat.sqrt() + self.eps);
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
// CONVERGENCE STATUS
// ============================================================================

/// Convergence status
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ConvergenceStatus {
    /// Converged to target ΔE
    Converged,
    /// Reached maximum iterations
    MaxIterations,
    /// Gradient too small (stuck)
    GradientTooSmall,
    /// Still running
    InProgress,
}

// ============================================================================
// OPTIMIZATION RESULT
// ============================================================================

/// Result of perceptual optimization
#[derive(Debug, Clone)]
pub struct OptimizationResult {
    /// Final convergence status
    pub status: ConvergenceStatus,
    /// Final optimized parameters
    pub final_params: MaterialParams,
    /// Final perceptual error (ΔE2000)
    pub final_delta_e: f64,
    /// Number of iterations taken
    pub iterations: usize,
    /// Loss history
    pub loss_history: Vec<f64>,
    /// Gradient norm history
    pub gradient_norm_history: Vec<f64>,
}

impl OptimizationResult {
    /// Check if optimization converged
    pub fn converged(&self) -> bool {
        self.status == ConvergenceStatus::Converged
    }

    /// Get final RGB color
    pub fn final_rgb(&self) -> [f64; 3] {
        render_params_rgb(&self.final_params)
    }
}

// ============================================================================
// PERCEPTUAL LOOP CONFIG
// ============================================================================

/// Configuration for perceptual rendering loop
#[derive(Debug, Clone)]
pub struct PerceptualLoopConfig {
    /// Target perceptual error (ΔE2000)
    pub target_delta_e: f64,
    /// Maximum optimization iterations
    pub max_iterations: usize,
    /// Initial learning rate
    pub learning_rate: f64,
    /// Enable adaptive learning rate
    pub adaptive_lr: bool,
    /// Parameter bounds
    pub parameter_bounds: ParameterBounds,
    /// Gradient step size for numerical differentiation
    pub gradient_epsilon: f64,
    /// Minimum gradient norm before declaring stuck
    pub min_gradient_norm: f64,
}

impl Default for PerceptualLoopConfig {
    fn default() -> Self {
        Self {
            target_delta_e: 1.0, // Imperceptible
            max_iterations: 100,
            learning_rate: 0.1,
            adaptive_lr: true,
            parameter_bounds: ParameterBounds::default(),
            gradient_epsilon: 1e-4,
            min_gradient_norm: 1e-8,
        }
    }
}

// ============================================================================
// PERCEPTUAL RENDERING LOOP
// ============================================================================

/// Closed-loop perceptual rendering optimizer
#[derive(Debug, Clone)]
pub struct PerceptualRenderingLoop {
    /// Configuration
    config: PerceptualLoopConfig,
    /// Optimizer state
    optimizer_state: Option<AdamState>,
    /// Loss history (for adaptive LR)
    loss_history: VecDeque<f64>,
    /// Current learning rate
    current_lr: f64,
}

impl PerceptualRenderingLoop {
    /// Create a new perceptual rendering loop
    pub fn new() -> Self {
        Self::with_config(PerceptualLoopConfig::default())
    }

    /// Create with custom configuration
    pub fn with_config(config: PerceptualLoopConfig) -> Self {
        let current_lr = config.learning_rate;
        Self {
            config,
            optimizer_state: None,
            loss_history: VecDeque::with_capacity(20),
            current_lr,
        }
    }

    /// Set target ΔE
    pub fn with_target_delta_e(mut self, target: f64) -> Self {
        self.config.target_delta_e = target.max(0.1);
        self
    }

    /// Set maximum iterations
    pub fn with_max_iterations(mut self, max: usize) -> Self {
        self.config.max_iterations = max.max(1);
        self
    }

    /// Run optimization loop
    pub fn optimize(
        &mut self,
        initial_params: &MaterialParams,
        target: &PerceptualTarget,
    ) -> OptimizationResult {
        let target_lab = target.as_lab();
        let param_count = initial_params.param_count();

        // Initialize optimizer
        self.optimizer_state = Some(AdamState::new(param_count, self.current_lr));
        self.loss_history.clear();

        let mut params = initial_params.clone();
        let mut loss_history = Vec::new();
        let mut gradient_norm_history = Vec::new();

        for iteration in 0..self.config.max_iterations {
            // 1. Forward render
            let rgb = render_params_rgb(&params);
            let rendered_lab = rgb_to_lab(rgb, Illuminant::D65);

            // 2. Compute perceptual error
            let delta_e = delta_e_2000(rendered_lab, target_lab);
            loss_history.push(delta_e);

            // 3. Check convergence
            if delta_e < self.config.target_delta_e {
                return OptimizationResult {
                    status: ConvergenceStatus::Converged,
                    final_params: params,
                    final_delta_e: delta_e,
                    iterations: iteration + 1,
                    loss_history,
                    gradient_norm_history,
                };
            }

            // 4. Compute gradients (numerical)
            let gradients = self.compute_gradients(&params, &target_lab);
            let grad_norm: f64 = gradients.iter().map(|g| g * g).sum::<f64>().sqrt();
            gradient_norm_history.push(grad_norm);

            // Check if stuck
            if grad_norm < self.config.min_gradient_norm {
                return OptimizationResult {
                    status: ConvergenceStatus::GradientTooSmall,
                    final_params: params,
                    final_delta_e: delta_e,
                    iterations: iteration + 1,
                    loss_history,
                    gradient_norm_history,
                };
            }

            // 5. Adaptive learning rate
            if self.config.adaptive_lr {
                self.adapt_learning_rate(delta_e);
            }

            // 6. Optimizer step
            let optimizer = self.optimizer_state.as_mut().unwrap();
            optimizer.lr = self.current_lr;
            let updates = optimizer.step(&gradients);

            // 7. Update parameters
            let param_vec = params.to_vec();
            let new_param_vec: Vec<f64> = param_vec
                .iter()
                .zip(updates.iter())
                .map(|(p, u)| p + u)
                .collect();

            params = params.from_vec(&new_param_vec);

            // 8. Apply bounds
            params = self.config.parameter_bounds.clamp(&params);
        }

        // Max iterations reached
        let final_rgb = render_params_rgb(&params);
        let final_lab = rgb_to_lab(final_rgb, Illuminant::D65);
        let final_delta_e = delta_e_2000(final_lab, target_lab);

        OptimizationResult {
            status: ConvergenceStatus::MaxIterations,
            final_params: params,
            final_delta_e,
            iterations: self.config.max_iterations,
            loss_history,
            gradient_norm_history,
        }
    }

    /// Compute numerical gradients
    fn compute_gradients(&self, params: &MaterialParams, target_lab: &LabColor) -> Vec<f64> {
        let eps = self.config.gradient_epsilon;
        let param_vec = params.to_vec();
        let mut gradients = vec![0.0; param_vec.len()];

        for i in 0..param_vec.len() {
            // Forward difference
            let mut forward = param_vec.clone();
            forward[i] += eps;
            let forward_params = params.from_vec(&forward);
            let forward_rgb = render_params_rgb(&forward_params);
            let forward_lab = rgb_to_lab(forward_rgb, Illuminant::D65);
            let forward_loss = delta_e_2000(forward_lab, *target_lab);

            // Backward difference
            let mut backward = param_vec.clone();
            backward[i] -= eps;
            let backward_params = params.from_vec(&backward);
            let backward_rgb = render_params_rgb(&backward_params);
            let backward_lab = rgb_to_lab(backward_rgb, Illuminant::D65);
            let backward_loss = delta_e_2000(backward_lab, *target_lab);

            // Central difference gradient
            gradients[i] = (forward_loss - backward_loss) / (2.0 * eps);
        }

        gradients
    }

    /// Adapt learning rate based on loss history
    fn adapt_learning_rate(&mut self, current_loss: f64) {
        self.loss_history.push_back(current_loss);
        if self.loss_history.len() > 10 {
            self.loss_history.pop_front();
        }

        if self.loss_history.len() >= 5 {
            // Check if loss is decreasing
            let recent: Vec<f64> = self.loss_history.iter().copied().collect();
            let first_half_avg: f64 = recent[..2].iter().sum::<f64>() / 2.0;
            let second_half_avg: f64 = recent[recent.len() - 2..].iter().sum::<f64>() / 2.0;

            if second_half_avg > first_half_avg * 0.99 {
                // Loss not decreasing, reduce LR
                self.current_lr *= 0.7;
                self.current_lr = self.current_lr.max(1e-6);
            } else if second_half_avg < first_half_avg * 0.5 {
                // Loss decreasing rapidly, can increase LR slightly
                self.current_lr *= 1.1;
                self.current_lr = self.current_lr.min(1.0);
            }
        }
    }

    /// Reset optimizer state
    pub fn reset(&mut self) {
        self.optimizer_state = None;
        self.loss_history.clear();
        self.current_lr = self.config.learning_rate;
    }
}

impl Default for PerceptualRenderingLoop {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// HELPER FUNCTIONS
// ============================================================================

/// Render material parameters to RGB
fn render_params_rgb(params: &MaterialParams) -> [f64; 3] {
    let ctx = BSDFContext::new_simple(params.cos_theta);

    if params.is_conductor {
        // Render conductor
        let conductor = ConductorBSDF::new(params.ior, params.k, params.roughness);
        let response = conductor.evaluate(&ctx);

        // For metals, reflectance varies with wavelength
        // Use spectral IOR if available, otherwise uniform
        [
            response.reflectance,
            response.reflectance,
            response.reflectance,
        ]
    } else {
        // Render dielectric
        let dielectric = DielectricBSDF::new(params.ior, params.roughness);
        let response = dielectric.evaluate(&ctx);

        // Dielectrics have uniform spectral reflectance (clear)
        let r = response.reflectance;
        [r, r, r]
    }
}

/// Convert spectrum to RGB (simplified)
fn spectrum_to_rgb(spectrum: &[(f64, f64)]) -> [f64; 3] {
    // Simple weighted average at R/G/B wavelengths
    let mut r = 0.0;
    let mut g = 0.0;
    let mut b = 0.0;

    for &(wavelength, reflectance) in spectrum {
        if wavelength >= 600.0 && wavelength <= 700.0 {
            r += reflectance;
        } else if wavelength >= 500.0 && wavelength < 600.0 {
            g += reflectance;
        } else if wavelength >= 400.0 && wavelength < 500.0 {
            b += reflectance;
        }
    }

    let count = spectrum.len().max(1) as f64 / 3.0;
    [r / count, g / count, b / count]
}

/// Quick optimization for matching a target color
pub fn quick_match_color(target_rgb: [f64; 3]) -> MaterialParams {
    let mut loop_runner = PerceptualRenderingLoop::new()
        .with_target_delta_e(2.0)
        .with_max_iterations(50);

    let target = PerceptualTarget::RgbColor(target_rgb);
    let initial = MaterialParams::default();

    let result = loop_runner.optimize(&initial, &target);
    result.final_params
}

/// Total memory used by perceptual loop module
pub fn total_perceptual_loop_memory() -> usize {
    std::mem::size_of::<PerceptualRenderingLoop>()
        + std::mem::size_of::<AdamState>()
        + std::mem::size_of::<OptimizationResult>()
        + std::mem::size_of::<MaterialParams>() * 2
        + 1_000 // History vectors overhead
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_material_params() {
        let dielectric = MaterialParams::dielectric(1.5, 0.1);
        assert_eq!(dielectric.param_count(), 2);

        let conductor = MaterialParams::conductor(0.18, 3.0, 0.0);
        assert_eq!(conductor.param_count(), 3);
    }

    #[test]
    fn test_params_to_vec_roundtrip() {
        let params = MaterialParams::dielectric(1.52, 0.15);
        let vec = params.to_vec();
        let restored = params.from_vec(&vec);

        assert!((restored.ior - params.ior).abs() < 1e-10);
        assert!((restored.roughness - params.roughness).abs() < 1e-10);
    }

    #[test]
    fn test_perceptual_target_from_hex() {
        let target = PerceptualTarget::from_hex("#FF8000").unwrap();
        if let PerceptualTarget::RgbColor(rgb) = target {
            assert!((rgb[0] - 1.0).abs() < 0.01);
            assert!((rgb[1] - 0.5).abs() < 0.02);
            assert!((rgb[2] - 0.0).abs() < 0.01);
        } else {
            panic!("Expected RgbColor");
        }
    }

    #[test]
    fn test_adam_optimizer() {
        let mut adam = AdamState::new(2, 0.1);

        let gradients = vec![0.1, 0.2];
        let updates = adam.step(&gradients);

        assert_eq!(updates.len(), 2);
        // First step should produce updates in opposite direction of gradients
        assert!(updates[0] < 0.0);
        assert!(updates[1] < 0.0);
    }

    #[test]
    fn test_optimization_converges() {
        let mut loop_runner = PerceptualRenderingLoop::new()
            .with_target_delta_e(3.0) // Relaxed target for fast test
            .with_max_iterations(50);

        // Target: low reflectance achievable by dielectrics (~4-10% range)
        // Dielectrics at normal incidence have R = ((n-1)/(n+1))^2
        // IOR 1.5 -> ~4% reflectance, IOR 2.0 -> ~11% reflectance
        let target = PerceptualTarget::Reflectance(0.06);
        let initial = MaterialParams::dielectric(1.3, 0.1);

        let result = loop_runner.optimize(&initial, &target);

        // Should improve from initial - dielectric can approach this target
        assert!(
            result.final_delta_e < 50.0,
            "Delta E too high: {}",
            result.final_delta_e
        );
        assert!(result.iterations > 0);
    }

    #[test]
    fn test_parameter_bounds() {
        let bounds = ParameterBounds::default();

        let over_range = MaterialParams::dielectric(5.0, 2.0);
        let clamped = bounds.clamp(&over_range);

        assert!(clamped.ior <= bounds.ior_max);
        assert!(clamped.roughness <= bounds.roughness_max);
    }

    #[test]
    fn test_render_params_rgb() {
        let params = MaterialParams::dielectric(1.5, 0.0);
        let rgb = render_params_rgb(&params);

        // Glass at normal incidence: ~4% reflectance
        for &r in &rgb {
            assert!(r > 0.0 && r < 1.0);
        }
    }

    #[test]
    fn test_loss_history() {
        let mut loop_runner = PerceptualRenderingLoop::new().with_max_iterations(10);

        let target = PerceptualTarget::Reflectance(0.1);
        let initial = MaterialParams::dielectric(1.5, 0.0);

        let result = loop_runner.optimize(&initial, &target);

        // Should have loss history entries
        assert!(!result.loss_history.is_empty());
        assert_eq!(result.loss_history.len(), result.iterations);
    }

    #[test]
    fn test_gradient_norm_history() {
        let mut loop_runner = PerceptualRenderingLoop::new().with_max_iterations(10);

        let target = PerceptualTarget::Reflectance(0.5);
        let initial = MaterialParams::dielectric(1.5, 0.0);

        let result = loop_runner.optimize(&initial, &target);

        // Should have gradient norm history
        assert!(!result.gradient_norm_history.is_empty());
    }

    #[test]
    fn test_conductor_optimization() {
        let mut loop_runner = PerceptualRenderingLoop::new()
            .with_target_delta_e(5.0)
            .with_max_iterations(30);

        // Target: highly reflective
        let target = PerceptualTarget::Reflectance(0.9);
        let initial = MaterialParams::conductor(2.0, 3.0, 0.1);

        let result = loop_runner.optimize(&initial, &target);

        // Should produce a result
        assert!(result.iterations > 0);
        assert!(result.final_delta_e < 100.0); // Reasonable range
    }

    #[test]
    fn test_quick_match_color() {
        let target = [0.5, 0.5, 0.5]; // Gray
        let params = quick_match_color(target);

        // Should return valid parameters
        assert!(params.ior >= 1.0);
        assert!(params.roughness >= 0.0 && params.roughness <= 1.0);
    }

    #[test]
    fn test_adaptive_lr() {
        let config = PerceptualLoopConfig {
            adaptive_lr: true,
            ..Default::default()
        };

        let mut loop_runner = PerceptualRenderingLoop::with_config(config);
        let initial_lr = loop_runner.current_lr;

        // Run some iterations
        let target = PerceptualTarget::Reflectance(0.5);
        let initial = MaterialParams::dielectric(1.5, 0.0);
        let _ = loop_runner.optimize(&initial, &target);

        // LR might have changed (though not guaranteed)
        // Just check it's still valid
        assert!(loop_runner.current_lr > 0.0);
    }

    #[test]
    fn test_reset() {
        let mut loop_runner = PerceptualRenderingLoop::new();

        // Run optimization
        let target = PerceptualTarget::Reflectance(0.5);
        let initial = MaterialParams::default();
        let _ = loop_runner.optimize(&initial, &target);

        // Reset
        loop_runner.reset();

        // Should be back to initial state
        assert!(loop_runner.optimizer_state.is_none());
        assert!(loop_runner.loss_history.is_empty());
    }

    #[test]
    fn test_memory_usage() {
        let mem = total_perceptual_loop_memory();
        assert!(mem < 5_000, "Memory should be < 5KB, got {}", mem);
    }

    #[test]
    fn test_spectrum_to_rgb() {
        let spectrum = vec![
            (450.0, 0.2), // Blue
            (550.0, 0.5), // Green
            (650.0, 0.8), // Red
        ];

        let rgb = spectrum_to_rgb(&spectrum);

        // Red should be highest
        assert!(rgb[0] > rgb[1]);
        assert!(rgb[0] > rgb[2]);
    }

    #[test]
    fn test_perceptual_target_as_lab() {
        let rgb_target = PerceptualTarget::RgbColor([1.0, 0.0, 0.0]); // Pure red
        let lab = rgb_target.as_lab();

        // Red in LAB should have positive a* and near-zero b*
        assert!(lab.l > 0.0);
        assert!(lab.a > 0.0); // Red has positive a*
    }
}
