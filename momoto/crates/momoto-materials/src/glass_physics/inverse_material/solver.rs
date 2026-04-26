//! # Inverse Material Solver
//!
//! Recover material parameters from observed reflectance data using
//! gradient-based optimization.
//!
//! ## Overview
//!
//! The inverse problem: given observed reflectance R_obs, find material
//! parameters θ such that R(θ) ≈ R_obs.
//!
//! This is solved by minimizing a loss function:
//! ```text
//! L(θ) = Σ_i w_i × d(R(θ, ctx_i), R_obs_i)²
//! ```
//!
//! Where d() is a distance metric (MSE, perceptual, etc.).

use super::super::differentiable::traits::DifferentiableBSDF;
use super::super::unified_bsdf::{BSDFContext, Vector3};
use super::bounds::BoundsEnforcer;
use super::optimizer::{
    AdamConfig, AdamOptimizer, DifferentiableOptimizer, LBFGSConfig, LBFGSOptimizer,
};

// ============================================================================
// LOSS FUNCTIONS
// ============================================================================

/// Loss function for inverse material optimization.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LossFunction {
    /// Mean squared error (L2).
    MSE,
    /// Mean absolute error (L1).
    MAE,
    /// Huber loss (L1 near 0, L2 far from 0).
    Huber,
    /// Log-cosh loss (smooth approximation of MAE).
    LogCosh,
    /// Perceptual loss (weighted by visual importance).
    Perceptual,
}

impl Default for LossFunction {
    fn default() -> Self {
        Self::MSE
    }
}

impl LossFunction {
    /// Compute loss and gradient for a single residual.
    pub fn loss_and_gradient(&self, residual: f64, delta: f64) -> (f64, f64) {
        match self {
            Self::MSE => {
                let loss = 0.5 * residual * residual;
                let grad = residual;
                (loss, grad)
            }
            Self::MAE => {
                let loss = residual.abs();
                let grad = residual.signum();
                (loss, grad)
            }
            Self::Huber => {
                if residual.abs() <= delta {
                    let loss = 0.5 * residual * residual;
                    let grad = residual;
                    (loss, grad)
                } else {
                    let loss = delta * (residual.abs() - 0.5 * delta);
                    let grad = delta * residual.signum();
                    (loss, grad)
                }
            }
            Self::LogCosh => {
                let cosh_r = residual.cosh();
                let loss = cosh_r.ln();
                let grad = residual.tanh();
                (loss, grad)
            }
            Self::Perceptual => {
                // Weight reflectance errors more heavily in perceptually important ranges
                // Human vision is more sensitive to changes around 4% reflectance
                let sensitivity = 1.0 + 10.0 * (-10.0 * (residual - 0.04).powi(2)).exp();
                let loss = 0.5 * sensitivity * residual * residual;
                let grad = sensitivity * residual;
                (loss, grad)
            }
        }
    }
}

// ============================================================================
// REFERENCE DATA
// ============================================================================

/// Reference observation for inverse problem.
#[derive(Debug, Clone)]
pub struct ReferenceObservation {
    /// Context (viewing angle, wavelength, etc.).
    pub context: BSDFContext,
    /// Observed reflectance.
    pub reflectance: f64,
    /// Observed transmittance (optional).
    pub transmittance: Option<f64>,
    /// Weight for this observation.
    pub weight: f64,
}

impl ReferenceObservation {
    /// Create from reflectance only.
    pub fn from_reflectance(context: BSDFContext, reflectance: f64) -> Self {
        Self {
            context,
            reflectance,
            transmittance: None,
            weight: 1.0,
        }
    }

    /// Create from R and T.
    pub fn from_response(context: BSDFContext, reflectance: f64, transmittance: f64) -> Self {
        Self {
            context,
            reflectance,
            transmittance: Some(transmittance),
            weight: 1.0,
        }
    }

    /// Set weight.
    pub fn with_weight(mut self, weight: f64) -> Self {
        self.weight = weight;
        self
    }
}

/// Collection of reference data for inverse problem.
#[derive(Debug, Clone)]
pub struct ReferenceData {
    /// Observations.
    pub observations: Vec<ReferenceObservation>,
    /// Total weight.
    pub total_weight: f64,
}

impl ReferenceData {
    /// Create empty reference data.
    pub fn new() -> Self {
        Self {
            observations: Vec::new(),
            total_weight: 0.0,
        }
    }

    /// Create from reflectance values at different angles.
    pub fn from_reflectance(reflectances: &[(f64, f64)]) -> Self {
        let mut data = Self::new();
        for &(cos_theta, r) in reflectances {
            let ctx = create_context(cos_theta, 550.0);
            data.add(ReferenceObservation::from_reflectance(ctx, r));
        }
        data
    }

    /// Create from reflectance array at equal angle spacing.
    pub fn from_reflectance_array(reflectances: &[f64], wavelength: f64) -> Self {
        let mut data = Self::new();
        let n = reflectances.len();
        for (i, &r) in reflectances.iter().enumerate() {
            let cos_theta = if n == 1 {
                1.0
            } else {
                1.0 - (i as f64) / ((n - 1) as f64)
            };
            let ctx = create_context(cos_theta, wavelength);
            data.add(ReferenceObservation::from_reflectance(ctx, r));
        }
        data
    }

    /// Add observation.
    pub fn add(&mut self, obs: ReferenceObservation) {
        self.total_weight += obs.weight;
        self.observations.push(obs);
    }

    /// Get number of observations.
    pub fn len(&self) -> usize {
        self.observations.len()
    }

    /// Check if empty.
    pub fn is_empty(&self) -> bool {
        self.observations.is_empty()
    }
}

impl Default for ReferenceData {
    fn default() -> Self {
        Self::new()
    }
}

/// Helper to create BSDFContext from cos_theta and wavelength.
fn create_context(cos_theta: f64, wavelength: f64) -> BSDFContext {
    let sin_theta = (1.0 - cos_theta * cos_theta).sqrt().max(0.0);
    BSDFContext {
        wi: Vector3::new(sin_theta, 0.0, cos_theta),
        wo: Vector3::new(-sin_theta, 0.0, cos_theta),
        normal: Vector3::new(0.0, 0.0, 1.0),
        tangent: Vector3::new(1.0, 0.0, 0.0),
        bitangent: Vector3::new(0.0, 1.0, 0.0),
        wavelength,
        wavelengths: None,
    }
}

// ============================================================================
// SOLVER CONFIGURATION
// ============================================================================

/// Configuration for inverse material solver.
#[derive(Debug, Clone)]
pub struct InverseSolverConfig {
    /// Maximum iterations.
    pub max_iterations: usize,
    /// Convergence tolerance for loss.
    pub loss_tolerance: f64,
    /// Convergence tolerance for gradient norm.
    pub gradient_tolerance: f64,
    /// Convergence tolerance for parameter change.
    pub param_tolerance: f64,
    /// Loss function to use.
    pub loss_function: LossFunction,
    /// Huber delta (for Huber loss).
    pub huber_delta: f64,
    /// Whether to use L-BFGS (vs Adam).
    pub use_lbfgs: bool,
    /// Regularization strength.
    pub regularization: f64,
    /// Early stopping patience.
    pub patience: usize,
    /// Verbose output.
    pub verbose: bool,
}

impl Default for InverseSolverConfig {
    fn default() -> Self {
        Self {
            max_iterations: 200,
            loss_tolerance: 1e-8,
            gradient_tolerance: 1e-6,
            param_tolerance: 1e-8,
            loss_function: LossFunction::MSE,
            huber_delta: 0.01,
            use_lbfgs: false,
            regularization: 0.0,
            patience: 20,
            verbose: false,
        }
    }
}

impl InverseSolverConfig {
    /// Use Adam optimizer.
    pub fn with_adam() -> Self {
        Self {
            use_lbfgs: false,
            ..Default::default()
        }
    }

    /// Use L-BFGS optimizer.
    pub fn with_lbfgs() -> Self {
        Self {
            use_lbfgs: true,
            ..Default::default()
        }
    }

    /// Set max iterations.
    pub fn max_iterations(mut self, n: usize) -> Self {
        self.max_iterations = n;
        self
    }

    /// Set loss function.
    pub fn loss_function(mut self, loss: LossFunction) -> Self {
        self.loss_function = loss;
        self
    }

    /// Enable verbose output.
    pub fn verbose(mut self) -> Self {
        self.verbose = true;
        self
    }
}

// ============================================================================
// INVERSE RESULT
// ============================================================================

/// Result of inverse material solving.
#[derive(Debug, Clone)]
pub struct InverseResult {
    /// Recovered parameters.
    pub params: Vec<f64>,
    /// Final loss value.
    pub final_loss: f64,
    /// Number of iterations.
    pub iterations: usize,
    /// Whether converged.
    pub converged: bool,
    /// Convergence reason.
    pub convergence_reason: ConvergenceReason,
    /// Loss history.
    pub loss_history: Vec<f64>,
    /// Final gradient norm.
    pub gradient_norm: f64,
}

impl InverseResult {
    /// Create successful result.
    pub fn success(
        params: Vec<f64>,
        final_loss: f64,
        iterations: usize,
        reason: ConvergenceReason,
        loss_history: Vec<f64>,
        gradient_norm: f64,
    ) -> Self {
        Self {
            params,
            final_loss,
            iterations,
            converged: true,
            convergence_reason: reason,
            loss_history,
            gradient_norm,
        }
    }

    /// Create failed result.
    pub fn failure(
        params: Vec<f64>,
        final_loss: f64,
        iterations: usize,
        reason: ConvergenceReason,
        loss_history: Vec<f64>,
        gradient_norm: f64,
    ) -> Self {
        Self {
            params,
            final_loss,
            iterations,
            converged: false,
            convergence_reason: reason,
            loss_history,
            gradient_norm,
        }
    }
}

/// Reason for convergence or termination.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConvergenceReason {
    /// Loss below tolerance.
    LossConverged,
    /// Gradient norm below tolerance.
    GradientConverged,
    /// Parameter change below tolerance.
    ParamConverged,
    /// Maximum iterations reached.
    MaxIterations,
    /// Loss increased for too many iterations (early stopping).
    EarlyStopping,
    /// Numerical issues (NaN, inf).
    NumericalIssue,
}

// ============================================================================
// INVERSE MATERIAL SOLVER
// ============================================================================

/// Gradient-based solver for inverse material problems.
#[derive(Debug)]
pub struct InverseMaterialSolver {
    /// Configuration.
    pub config: InverseSolverConfig,
    /// Bounds enforcer.
    pub bounds: BoundsEnforcer,
    /// Adam optimizer (if using Adam).
    adam: Option<AdamOptimizer>,
    /// L-BFGS optimizer (if using L-BFGS).
    lbfgs: Option<LBFGSOptimizer>,
}

impl InverseMaterialSolver {
    /// Create solver with default configuration.
    pub fn new() -> Self {
        Self {
            config: InverseSolverConfig::default(),
            bounds: BoundsEnforcer::standard_material(),
            adam: None,
            lbfgs: None,
        }
    }

    /// Create solver with Adam optimizer.
    pub fn with_adam() -> Self {
        Self {
            config: InverseSolverConfig::with_adam(),
            bounds: BoundsEnforcer::standard_material(),
            adam: None,
            lbfgs: None,
        }
    }

    /// Create solver with L-BFGS optimizer.
    pub fn with_lbfgs() -> Self {
        Self {
            config: InverseSolverConfig::with_lbfgs(),
            bounds: BoundsEnforcer::standard_material(),
            adam: None,
            lbfgs: None,
        }
    }

    /// Set configuration.
    pub fn config(mut self, config: InverseSolverConfig) -> Self {
        self.config = config;
        self
    }

    /// Set bounds enforcer.
    pub fn bounds(mut self, bounds: BoundsEnforcer) -> Self {
        self.bounds = bounds;
        self
    }

    /// Solve inverse problem.
    ///
    /// Given reference data and initial material guess, recover parameters
    /// that best match the observations.
    pub fn solve<M: DifferentiableBSDF + Clone>(
        &mut self,
        reference: &ReferenceData,
        initial: &M,
    ) -> InverseResult {
        if reference.is_empty() {
            return InverseResult::failure(
                initial.params_to_vec(),
                f64::INFINITY,
                0,
                ConvergenceReason::NumericalIssue,
                vec![],
                0.0,
            );
        }

        // Initialize parameters
        let mut params = initial.params_to_vec();
        self.bounds.project(&mut params);

        // Initialize optimizer
        let param_count = params.len();
        if self.config.use_lbfgs {
            self.lbfgs = Some(LBFGSOptimizer::new(LBFGSConfig::default()));
        } else {
            self.adam = Some(AdamOptimizer::new(AdamConfig::default(), param_count));
        }

        // Optimization loop
        let mut loss_history = Vec::with_capacity(self.config.max_iterations);
        let mut best_loss = f64::INFINITY;
        let mut best_params = params.clone();
        let mut stagnant_count = 0;
        let mut gradient_norm = 0.0;

        for iteration in 0..self.config.max_iterations {
            // Compute loss and gradient
            let material = M::from_param_vec(&params);
            let (loss, gradient) = self.compute_loss_and_gradient(&material, reference);

            // Check for numerical issues
            if !loss.is_finite() || gradient.iter().any(|&g| !g.is_finite()) {
                return InverseResult::failure(
                    best_params,
                    best_loss,
                    iteration,
                    ConvergenceReason::NumericalIssue,
                    loss_history,
                    gradient_norm,
                );
            }

            loss_history.push(loss);
            gradient_norm = gradient.iter().map(|&g| g * g).sum::<f64>().sqrt();

            // Check convergence
            if loss < self.config.loss_tolerance {
                return InverseResult::success(
                    params,
                    loss,
                    iteration + 1,
                    ConvergenceReason::LossConverged,
                    loss_history,
                    gradient_norm,
                );
            }

            if gradient_norm < self.config.gradient_tolerance {
                return InverseResult::success(
                    params,
                    loss,
                    iteration + 1,
                    ConvergenceReason::GradientConverged,
                    loss_history,
                    gradient_norm,
                );
            }

            // Track best solution and early stopping
            if loss < best_loss {
                best_loss = loss;
                best_params = params.clone();
                stagnant_count = 0;
            } else {
                stagnant_count += 1;
                if stagnant_count >= self.config.patience {
                    return InverseResult::success(
                        best_params,
                        best_loss,
                        iteration + 1,
                        ConvergenceReason::EarlyStopping,
                        loss_history,
                        gradient_norm,
                    );
                }
            }

            // Optimizer step
            let update = if self.config.use_lbfgs {
                self.lbfgs.as_mut().unwrap().step(&gradient)
            } else {
                self.adam.as_mut().unwrap().step(&gradient)
            };

            // Apply update
            let old_params = params.clone();
            for (p, u) in params.iter_mut().zip(update.iter()) {
                *p -= u; // Update is already signed appropriately
            }

            // Project to bounds
            self.bounds.project(&mut params);

            // Check parameter convergence
            let param_change: f64 = params
                .iter()
                .zip(old_params.iter())
                .map(|(&new, &old)| (new - old).powi(2))
                .sum::<f64>()
                .sqrt();

            if param_change < self.config.param_tolerance {
                return InverseResult::success(
                    params,
                    loss,
                    iteration + 1,
                    ConvergenceReason::ParamConverged,
                    loss_history,
                    gradient_norm,
                );
            }
        }

        // Max iterations reached
        InverseResult::failure(
            best_params,
            best_loss,
            self.config.max_iterations,
            ConvergenceReason::MaxIterations,
            loss_history,
            gradient_norm,
        )
    }

    /// Compute loss and gradient for current parameters.
    fn compute_loss_and_gradient<M: DifferentiableBSDF>(
        &self,
        material: &M,
        reference: &ReferenceData,
    ) -> (f64, Vec<f64>) {
        let param_count = material.param_count();
        let mut total_loss = 0.0;
        let mut total_gradient = vec![0.0; param_count];

        for obs in &reference.observations {
            // Evaluate material with gradients
            let result = material.eval_with_gradients(&obs.context);

            // Reflectance loss
            let r_residual = result.response.reflectance - obs.reflectance;
            let (r_loss, r_grad_loss) = self
                .config
                .loss_function
                .loss_and_gradient(r_residual, self.config.huber_delta);

            total_loss += obs.weight * r_loss;

            // Chain rule: ∂L/∂θ = ∂L/∂R × ∂R/∂θ
            let param_grads = result.gradients.to_vec();
            for (i, &pg) in param_grads.iter().enumerate().take(param_count) {
                // ∂R/∂θ_i (reflectance gradient w.r.t. parameter i)
                total_gradient[i] += obs.weight * r_grad_loss * pg * result.gradients.d_reflectance;
            }

            // Transmittance loss (if observed)
            if let Some(t_obs) = obs.transmittance {
                let t_residual = result.response.transmittance - t_obs;
                let (t_loss, t_grad_loss) = self
                    .config
                    .loss_function
                    .loss_and_gradient(t_residual, self.config.huber_delta);

                total_loss += obs.weight * t_loss;

                for (i, &pg) in param_grads.iter().enumerate().take(param_count) {
                    total_gradient[i] +=
                        obs.weight * t_grad_loss * pg * result.gradients.d_transmittance;
                }
            }
        }

        // Normalize by total weight
        if reference.total_weight > 0.0 {
            total_loss /= reference.total_weight;
            for g in &mut total_gradient {
                *g /= reference.total_weight;
            }
        }

        // Add regularization
        if self.config.regularization > 0.0 {
            let params = material.params_to_vec();
            for (i, &p) in params.iter().enumerate().take(param_count) {
                total_loss += 0.5 * self.config.regularization * p * p;
                total_gradient[i] += self.config.regularization * p;
            }
        }

        // Add barrier penalty (if using barrier method)
        let barrier_penalty = self.bounds.barrier_penalty(&material.params_to_vec());
        total_loss += barrier_penalty;

        let barrier_grad = self.bounds.barrier_gradient(&material.params_to_vec());
        for (g, bg) in total_gradient.iter_mut().zip(barrier_grad.iter()) {
            *g += bg;
        }

        (total_loss, total_gradient)
    }

    /// Reset optimizer state.
    pub fn reset(&mut self) {
        if let Some(ref mut adam) = self.adam {
            adam.reset();
        }
        if let Some(ref mut lbfgs) = self.lbfgs {
            lbfgs.reset();
        }
    }
}

impl Default for InverseMaterialSolver {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// CONVENIENCE FUNCTIONS
// ============================================================================

/// Quick inverse solve for recovering IOR from reflectance at normal incidence.
pub fn recover_ior_from_normal_reflectance(reflectance: f64) -> f64 {
    // Fresnel at normal incidence: R = ((n-1)/(n+1))²
    // Solve for n: n = (1 + sqrt(R)) / (1 - sqrt(R))
    let r_sqrt = reflectance.sqrt().clamp(0.0, 0.999);
    (1.0 + r_sqrt) / (1.0 - r_sqrt)
}

/// Quick inverse solve for roughness from glossiness measurement.
pub fn recover_roughness_from_glossiness(glossiness: f64) -> f64 {
    // Simple model: glossiness ≈ 1 - roughness²
    (1.0 - glossiness.clamp(0.0, 1.0)).sqrt()
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::super::super::differentiable::dielectric::DifferentiableDielectric;
    use super::*;

    #[test]
    fn test_loss_function_mse() {
        let loss_fn = LossFunction::MSE;

        let (loss, grad) = loss_fn.loss_and_gradient(0.1, 0.01);
        assert!((loss - 0.005).abs() < 1e-10); // 0.5 × 0.1²
        assert!((grad - 0.1).abs() < 1e-10);
    }

    #[test]
    fn test_loss_function_mae() {
        let loss_fn = LossFunction::MAE;

        let (loss, grad) = loss_fn.loss_and_gradient(0.1, 0.01);
        assert!((loss - 0.1).abs() < 1e-10);
        assert!((grad - 1.0).abs() < 1e-10);

        let (loss_neg, grad_neg) = loss_fn.loss_and_gradient(-0.1, 0.01);
        assert!((loss_neg - 0.1).abs() < 1e-10);
        assert!((grad_neg - (-1.0)).abs() < 1e-10);
    }

    #[test]
    fn test_loss_function_huber() {
        let loss_fn = LossFunction::Huber;
        let delta = 0.05;

        // Small residual (L2 region)
        let (loss_small, grad_small) = loss_fn.loss_and_gradient(0.01, delta);
        assert!((loss_small - 0.5 * 0.01 * 0.01).abs() < 1e-10);
        assert!((grad_small - 0.01).abs() < 1e-10);

        // Large residual (L1 region)
        let (loss_large, _grad_large) = loss_fn.loss_and_gradient(0.1, delta);
        assert!(loss_large < 0.5 * 0.1 * 0.1); // L1 grows slower than L2
    }

    #[test]
    fn test_reference_data_from_reflectance() {
        let data = ReferenceData::from_reflectance(&[
            (1.0, 0.04), // Normal incidence
            (0.5, 0.06), // 60°
        ]);

        assert_eq!(data.len(), 2);
        assert!((data.observations[0].reflectance - 0.04).abs() < 1e-10);
    }

    #[test]
    fn test_inverse_solver_basic() {
        // Create reference data from known material
        let target = DifferentiableDielectric::new(1.5, 0.1);
        let ctx = create_context(1.0, 550.0);
        let response = target.eval_with_gradients(&ctx);

        let reference = ReferenceData::from_reflectance(&[(1.0, response.response.reflectance)]);

        // Start from different initial guess
        let initial = DifferentiableDielectric::new(1.3, 0.1);

        // Solve
        let mut solver = InverseMaterialSolver::with_adam();
        solver.config.max_iterations = 100;

        let result = solver.solve(&reference, &initial);

        // Should converge to something reasonable
        assert!(result.final_loss < 0.1);
        assert!(result.iterations > 0);
    }

    #[test]
    fn test_recover_ior_from_normal_reflectance() {
        // Glass: n=1.5, R ≈ 0.04
        let r = 0.04;
        let n = recover_ior_from_normal_reflectance(r);
        assert!((n - 1.5).abs() < 0.1);

        // Water: n=1.33, R ≈ 0.02
        let r_water = 0.02;
        let n_water = recover_ior_from_normal_reflectance(r_water);
        assert!((n_water - 1.33).abs() < 0.2);
    }

    #[test]
    fn test_recover_roughness_from_glossiness() {
        let roughness = recover_roughness_from_glossiness(0.96);
        assert!(roughness < 0.3); // High gloss → low roughness

        let roughness_matte = recover_roughness_from_glossiness(0.1);
        assert!(roughness_matte > 0.8); // Low gloss → high roughness
    }

    #[test]
    fn test_solver_with_lbfgs() {
        let target = DifferentiableDielectric::new(1.5, 0.1);
        let ctx = create_context(1.0, 550.0);
        let response = target.eval_with_gradients(&ctx);

        let reference = ReferenceData::from_reflectance(&[(1.0, response.response.reflectance)]);

        let initial = DifferentiableDielectric::new(1.3, 0.1);

        let mut solver = InverseMaterialSolver::with_lbfgs();
        solver.config.max_iterations = 50;

        let result = solver.solve(&reference, &initial);

        assert!(result.iterations > 0);
    }

    #[test]
    fn test_inverse_solver_multiple_angles() {
        let target = DifferentiableDielectric::new(1.5, 0.1);

        // Generate reference at multiple angles
        let angles = [1.0, 0.9, 0.8, 0.7, 0.6];
        let mut data = ReferenceData::new();
        for &cos_theta in &angles {
            let ctx = create_context(cos_theta, 550.0);
            let r = target.eval_with_gradients(&ctx).response.reflectance;
            data.add(ReferenceObservation::from_reflectance(ctx, r));
        }

        let initial = DifferentiableDielectric::new(1.3, 0.2);

        let mut solver = InverseMaterialSolver::with_adam();
        solver.config.max_iterations = 200;

        let result = solver.solve(&data, &initial);

        // With multiple angles, should converge better
        assert!(result.final_loss < 0.01);
    }

    #[test]
    fn test_convergence_reasons() {
        let target = DifferentiableDielectric::new(1.5, 0.1);
        let ctx = create_context(1.0, 550.0);
        let r = target.eval_with_gradients(&ctx).response.reflectance;

        let reference = ReferenceData::from_reflectance(&[(1.0, r)]);

        // Start very close to target - should converge quickly
        let initial = DifferentiableDielectric::new(1.5, 0.1);

        let mut solver = InverseMaterialSolver::with_adam();
        solver.config.loss_tolerance = 0.01;

        let result = solver.solve(&reference, &initial);

        // Should converge due to loss being small
        assert!(result.converged);
    }

    #[test]
    fn test_empty_reference_data() {
        let reference = ReferenceData::new();
        let initial = DifferentiableDielectric::glass();

        let mut solver = InverseMaterialSolver::new();
        let result = solver.solve(&reference, &initial);

        // Should fail gracefully
        assert!(!result.converged);
        assert_eq!(result.convergence_reason, ConvergenceReason::NumericalIssue);
    }

    #[test]
    fn test_solver_reset() {
        let mut solver = InverseMaterialSolver::with_adam();

        // Force initialization
        let reference = ReferenceData::from_reflectance(&[(1.0, 0.04)]);
        let initial = DifferentiableDielectric::glass();
        let _ = solver.solve(&reference, &initial);

        // Reset
        solver.reset();

        // Should be able to solve again
        let result = solver.solve(&reference, &initial);
        assert!(result.iterations > 0);
    }
}
