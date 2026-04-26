//! Research API for ML Integration
//!
//! Phase 8: Interface for external ML frameworks and multi-objective optimization.
//!
//! Provides:
//! - ForwardFunction trait for differentiable material models
//! - MaterialForwardFunction implementation for spectral evaluation
//! - ParameterBounds for constrained optimization
//! - Multi-objective optimization structures
//! - Jacobian computation for gradient-based methods

use std::collections::HashMap;

// ============================================================================
// Forward Function Trait
// ============================================================================

/// Trait for forward functions usable by ML frameworks
///
/// Enables integration with external optimization libraries (e.g., scipy, optuna)
/// by providing a simple `&[f64] -> Vec<f64>` interface.
pub trait ForwardFunction: Send + Sync {
    /// Evaluate forward function
    ///
    /// # Arguments
    /// * `params` - Input parameters (flattened)
    ///
    /// # Returns
    /// Output values (e.g., spectral reflectance)
    fn forward(&self, params: &[f64]) -> Vec<f64>;

    /// Input dimension (number of parameters)
    fn input_dim(&self) -> usize;

    /// Output dimension (number of outputs)
    fn output_dim(&self) -> usize;

    /// Compute Jacobian matrix (optional)
    ///
    /// Returns `Some(jacobian)` where jacobian[i][j] = d(output[i])/d(param[j])
    fn jacobian(&self, params: &[f64]) -> Option<Vec<Vec<f64>>> {
        // Default: numerical differentiation
        let eps = 1e-7;
        let n_in = self.input_dim();
        let n_out = self.output_dim();

        if params.len() != n_in {
            return None;
        }

        let f0 = self.forward(params);
        if f0.len() != n_out {
            return None;
        }

        let mut jac = vec![vec![0.0; n_in]; n_out];

        for j in 0..n_in {
            let mut params_plus = params.to_vec();
            params_plus[j] += eps;
            let f_plus = self.forward(&params_plus);

            for i in 0..n_out {
                jac[i][j] = (f_plus[i] - f0[i]) / eps;
            }
        }

        Some(jac)
    }

    /// Parameter names (for debugging/visualization)
    fn param_names(&self) -> Vec<&str> {
        (0..self.input_dim()).map(|_| "param").collect()
    }

    /// Output names (for debugging/visualization)
    fn output_names(&self) -> Vec<&str> {
        (0..self.output_dim()).map(|_| "output").collect()
    }
}

// ============================================================================
// Parameter Bounds
// ============================================================================

/// Parameter bounds for constrained optimization
#[derive(Debug, Clone)]
pub struct ParameterBounds {
    /// Lower bounds
    pub lower: Vec<f64>,
    /// Upper bounds
    pub upper: Vec<f64>,
    /// Parameter names (optional)
    pub names: Vec<String>,
}

impl ParameterBounds {
    /// Create new bounds
    pub fn new(lower: Vec<f64>, upper: Vec<f64>) -> Self {
        assert_eq!(lower.len(), upper.len(), "Bounds dimensions must match");
        let names = (0..lower.len()).map(|i| format!("param_{}", i)).collect();
        Self {
            lower,
            upper,
            names,
        }
    }

    /// Create bounds with names
    pub fn with_names(lower: Vec<f64>, upper: Vec<f64>, names: Vec<String>) -> Self {
        assert_eq!(lower.len(), upper.len(), "Bounds dimensions must match");
        assert_eq!(lower.len(), names.len(), "Names length must match bounds");
        Self {
            lower,
            upper,
            names,
        }
    }

    /// Standard PBR material bounds
    ///
    /// Parameters: [metallic, roughness, ior, r, g, b]
    pub fn standard_pbr() -> Self {
        Self::with_names(
            vec![0.0, 0.0, 1.0, 0.0, 0.0, 0.0],
            vec![1.0, 1.0, 3.0, 1.0, 1.0, 1.0],
            vec![
                "metallic".to_string(),
                "roughness".to_string(),
                "ior".to_string(),
                "base_r".to_string(),
                "base_g".to_string(),
                "base_b".to_string(),
            ],
        )
    }

    /// Glass material bounds
    ///
    /// Parameters: [ior, roughness, transmission, absorption_coeff, r, g, b]
    pub fn glass() -> Self {
        Self::with_names(
            vec![1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0],
            vec![2.5, 1.0, 1.0, 10.0, 1.0, 1.0, 1.0],
            vec![
                "ior".to_string(),
                "roughness".to_string(),
                "transmission".to_string(),
                "absorption".to_string(),
                "color_r".to_string(),
                "color_g".to_string(),
                "color_b".to_string(),
            ],
        )
    }

    /// Thin-film material bounds
    ///
    /// Parameters: [base_ior, film_thickness, film_ior, roughness]
    pub fn thin_film() -> Self {
        Self::with_names(
            vec![1.0, 0.0, 1.0, 0.0],
            vec![3.0, 1000.0, 3.0, 1.0],
            vec![
                "base_ior".to_string(),
                "film_thickness".to_string(),
                "film_ior".to_string(),
                "roughness".to_string(),
            ],
        )
    }

    /// Dimension
    pub fn dim(&self) -> usize {
        self.lower.len()
    }

    /// Clamp parameters to bounds
    pub fn clamp(&self, params: &[f64]) -> Vec<f64> {
        params
            .iter()
            .enumerate()
            .map(|(i, &p)| {
                if i < self.lower.len() {
                    p.clamp(self.lower[i], self.upper[i])
                } else {
                    p
                }
            })
            .collect()
    }

    /// Check if parameters are within bounds
    pub fn is_valid(&self, params: &[f64]) -> bool {
        if params.len() != self.lower.len() {
            return false;
        }
        params
            .iter()
            .enumerate()
            .all(|(i, &p)| p >= self.lower[i] && p <= self.upper[i])
    }

    /// Generate random parameters within bounds
    pub fn random(&self, rng: &mut impl FnMut() -> f64) -> Vec<f64> {
        self.lower
            .iter()
            .zip(self.upper.iter())
            .map(|(&lo, &hi)| lo + rng() * (hi - lo))
            .collect()
    }

    /// Generate center point of bounds
    pub fn center(&self) -> Vec<f64> {
        self.lower
            .iter()
            .zip(self.upper.iter())
            .map(|(&lo, &hi)| (lo + hi) / 2.0)
            .collect()
    }

    /// Normalize parameters to [0, 1] range
    pub fn normalize(&self, params: &[f64]) -> Vec<f64> {
        params
            .iter()
            .enumerate()
            .map(|(i, &p)| {
                if i < self.lower.len() {
                    let range = self.upper[i] - self.lower[i];
                    if range > 1e-10 {
                        (p - self.lower[i]) / range
                    } else {
                        0.5
                    }
                } else {
                    p
                }
            })
            .collect()
    }

    /// Denormalize parameters from [0, 1] range
    pub fn denormalize(&self, normalized: &[f64]) -> Vec<f64> {
        normalized
            .iter()
            .enumerate()
            .map(|(i, &n)| {
                if i < self.lower.len() {
                    self.lower[i] + n * (self.upper[i] - self.lower[i])
                } else {
                    n
                }
            })
            .collect()
    }
}

// ============================================================================
// Parameter Mapping
// ============================================================================

/// Mapping between flat parameter array and material parameters
#[derive(Debug, Clone)]
pub struct ParameterMapping {
    /// Parameter indices and names
    pub params: Vec<(usize, String)>,
    /// Total dimension
    pub total_dim: usize,
}

impl ParameterMapping {
    /// Create mapping for standard PBR
    pub fn standard_pbr() -> Self {
        Self {
            params: vec![
                (0, "metallic".to_string()),
                (1, "roughness".to_string()),
                (2, "ior".to_string()),
                (3, "base_r".to_string()),
                (4, "base_g".to_string()),
                (5, "base_b".to_string()),
            ],
            total_dim: 6,
        }
    }

    /// Create mapping for glass
    pub fn glass() -> Self {
        Self {
            params: vec![
                (0, "ior".to_string()),
                (1, "roughness".to_string()),
                (2, "transmission".to_string()),
                (3, "absorption".to_string()),
                (4, "color_r".to_string()),
                (5, "color_g".to_string()),
                (6, "color_b".to_string()),
            ],
            total_dim: 7,
        }
    }

    /// Get parameter index by name
    pub fn index_of(&self, name: &str) -> Option<usize> {
        self.params.iter().find(|(_, n)| n == name).map(|(i, _)| *i)
    }

    /// Get parameter name by index
    pub fn name_of(&self, index: usize) -> Option<&str> {
        self.params
            .iter()
            .find(|(i, _)| *i == index)
            .map(|(_, n)| n.as_str())
    }
}

// ============================================================================
// Material Forward Function
// ============================================================================

/// Forward function for material spectral evaluation
#[derive(Clone)]
pub struct MaterialForwardFunction {
    /// Wavelengths for evaluation (nm)
    pub wavelengths: Vec<f64>,
    /// Evaluation angle theta
    pub evaluation_angle: f64,
    /// Parameter mapping
    pub param_mapping: ParameterMapping,
    /// Parameter bounds
    pub bounds: ParameterBounds,
}

impl Default for MaterialForwardFunction {
    fn default() -> Self {
        Self::standard_pbr()
    }
}

impl MaterialForwardFunction {
    /// Create for standard PBR materials
    pub fn standard_pbr() -> Self {
        Self {
            wavelengths: Self::visible_wavelengths(31),
            evaluation_angle: 0.0,
            param_mapping: ParameterMapping::standard_pbr(),
            bounds: ParameterBounds::standard_pbr(),
        }
    }

    /// Create for glass materials
    pub fn glass() -> Self {
        Self {
            wavelengths: Self::visible_wavelengths(31),
            evaluation_angle: 0.0,
            param_mapping: ParameterMapping::glass(),
            bounds: ParameterBounds::glass(),
        }
    }

    /// Create with custom wavelengths
    pub fn with_wavelengths(mut self, wavelengths: Vec<f64>) -> Self {
        self.wavelengths = wavelengths;
        self
    }

    /// Create with custom angle
    pub fn with_angle(mut self, angle: f64) -> Self {
        self.evaluation_angle = angle;
        self
    }

    /// Generate visible wavelengths
    pub fn visible_wavelengths(n: usize) -> Vec<f64> {
        (0..n)
            .map(|i| 380.0 + (i as f64 / (n - 1).max(1) as f64) * 400.0)
            .collect()
    }

    /// Evaluate material at wavelength (simplified PBR model)
    fn evaluate_pbr(&self, params: &[f64], wavelength: f64) -> f64 {
        if params.len() < 6 {
            return 0.5;
        }

        let metallic = params[0].clamp(0.0, 1.0);
        let roughness = params[1].clamp(0.0, 1.0);
        let ior = params[2].clamp(1.0, 3.0);
        let base_r = params[3].clamp(0.0, 1.0);
        let base_g = params[4].clamp(0.0, 1.0);
        let base_b = params[5].clamp(0.0, 1.0);

        // Wavelength-dependent base color (simplified)
        let base = if wavelength < 500.0 {
            base_b * 0.5 + base_g * 0.3 + base_r * 0.2
        } else if wavelength < 600.0 {
            base_g * 0.6 + base_r * 0.3 + base_b * 0.1
        } else {
            base_r * 0.7 + base_g * 0.2 + base_b * 0.1
        };

        // Fresnel at normal incidence
        let f0 = ((ior - 1.0) / (ior + 1.0)).powi(2);

        // Mix metallic and dielectric
        let reflectance = metallic * base + (1.0 - metallic) * f0;

        // Roughness affects overall reflectance slightly
        reflectance * (1.0 - roughness * 0.1)
    }

    /// Evaluate glass at wavelength (simplified)
    fn evaluate_glass(&self, params: &[f64], wavelength: f64) -> f64 {
        if params.len() < 7 {
            return 0.5;
        }

        let ior = params[0].clamp(1.0, 2.5);
        let _roughness = params[1].clamp(0.0, 1.0);
        let transmission = params[2].clamp(0.0, 1.0);
        let absorption = params[3].clamp(0.0, 10.0);
        let color_r = params[4].clamp(0.0, 1.0);
        let color_g = params[5].clamp(0.0, 1.0);
        let color_b = params[6].clamp(0.0, 1.0);

        // Wavelength-dependent absorption
        let color_factor = if wavelength < 500.0 {
            color_b
        } else if wavelength < 600.0 {
            color_g
        } else {
            color_r
        };

        // Fresnel reflectance
        let f0 = ((ior - 1.0) / (ior + 1.0)).powi(2);

        // Beer-Lambert absorption
        let transmitted = (-absorption * (1.0 - color_factor)).exp();

        // Mix reflection and transmission
        f0 * (1.0 - transmission) + transmission * transmitted * color_factor
    }
}

impl ForwardFunction for MaterialForwardFunction {
    fn forward(&self, params: &[f64]) -> Vec<f64> {
        let clamped = self.bounds.clamp(params);

        self.wavelengths
            .iter()
            .map(|&wl| {
                if self.param_mapping.total_dim == 7 {
                    self.evaluate_glass(&clamped, wl)
                } else {
                    self.evaluate_pbr(&clamped, wl)
                }
            })
            .collect()
    }

    fn input_dim(&self) -> usize {
        self.param_mapping.total_dim
    }

    fn output_dim(&self) -> usize {
        self.wavelengths.len()
    }

    fn param_names(&self) -> Vec<&str> {
        self.param_mapping
            .params
            .iter()
            .map(|(_, n)| n.as_str())
            .collect()
    }

    fn output_names(&self) -> Vec<&str> {
        vec!["reflectance"; self.wavelengths.len()]
    }
}

// ============================================================================
// Multi-Objective Optimization
// ============================================================================

/// Objective function type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ObjectiveType {
    /// Minimize this objective
    Minimize,
    /// Maximize this objective
    Maximize,
}

/// Single objective function
#[derive(Clone)]
pub struct ObjectiveFunction {
    /// Objective name
    pub name: String,
    /// Objective type
    pub objective_type: ObjectiveType,
    /// Weight for scalarization
    pub weight: f64,
    /// Evaluation function (returns objective value)
    evaluator: fn(&[f64], &[f64]) -> f64, // (params, targets) -> objective
}

impl ObjectiveFunction {
    /// Create new objective
    pub fn new(
        name: &str,
        objective_type: ObjectiveType,
        weight: f64,
        evaluator: fn(&[f64], &[f64]) -> f64,
    ) -> Self {
        Self {
            name: name.to_string(),
            objective_type,
            weight,
            evaluator,
        }
    }

    /// Evaluate objective
    pub fn evaluate(&self, params: &[f64], targets: &[f64]) -> f64 {
        (self.evaluator)(params, targets)
    }

    /// RMSE objective (minimize)
    pub fn rmse() -> Self {
        Self::new("rmse", ObjectiveType::Minimize, 1.0, |rendered, target| {
            if rendered.len() != target.len() || rendered.is_empty() {
                return f64::INFINITY;
            }
            let sum_sq: f64 = rendered
                .iter()
                .zip(target.iter())
                .map(|(r, t)| (r - t).powi(2))
                .sum();
            (sum_sq / rendered.len() as f64).sqrt()
        })
    }

    /// Peak error objective (minimize)
    pub fn peak_error() -> Self {
        Self::new(
            "peak_error",
            ObjectiveType::Minimize,
            0.5,
            |rendered, target| {
                rendered
                    .iter()
                    .zip(target.iter())
                    .map(|(r, t)| (r - t).abs())
                    .fold(0.0, f64::max)
            },
        )
    }

    /// Smoothness objective (minimize roughness variation)
    pub fn smoothness() -> Self {
        Self::new(
            "smoothness",
            ObjectiveType::Minimize,
            0.1,
            |rendered, _target| {
                if rendered.len() < 2 {
                    return 0.0;
                }
                let mut sum_diff = 0.0;
                for i in 1..rendered.len() {
                    sum_diff += (rendered[i] - rendered[i - 1]).abs();
                }
                sum_diff / (rendered.len() - 1) as f64
            },
        )
    }
}

/// Constraint type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConstraintType {
    /// Value must be <= bound
    LessOrEqual,
    /// Value must be >= bound
    GreaterOrEqual,
    /// Value must be == bound (with tolerance)
    Equal,
}

/// Optimization constraint
#[derive(Clone)]
pub struct Constraint {
    /// Constraint name
    pub name: String,
    /// Constraint type
    pub constraint_type: ConstraintType,
    /// Bound value
    pub bound: f64,
    /// Tolerance for equality constraints
    pub tolerance: f64,
    /// Constraint function (returns constraint value)
    evaluator: fn(&[f64]) -> f64,
}

impl Constraint {
    /// Create new constraint
    pub fn new(
        name: &str,
        constraint_type: ConstraintType,
        bound: f64,
        evaluator: fn(&[f64]) -> f64,
    ) -> Self {
        Self {
            name: name.to_string(),
            constraint_type,
            bound,
            tolerance: 1e-6,
            evaluator,
        }
    }

    /// Evaluate constraint
    pub fn evaluate(&self, params: &[f64]) -> f64 {
        (self.evaluator)(params)
    }

    /// Check if constraint is satisfied
    pub fn is_satisfied(&self, params: &[f64]) -> bool {
        let value = self.evaluate(params);
        match self.constraint_type {
            ConstraintType::LessOrEqual => value <= self.bound + self.tolerance,
            ConstraintType::GreaterOrEqual => value >= self.bound - self.tolerance,
            ConstraintType::Equal => (value - self.bound).abs() <= self.tolerance,
        }
    }

    /// Constraint violation (0 if satisfied, >0 if violated)
    pub fn violation(&self, params: &[f64]) -> f64 {
        let value = self.evaluate(params);
        match self.constraint_type {
            ConstraintType::LessOrEqual => (value - self.bound).max(0.0),
            ConstraintType::GreaterOrEqual => (self.bound - value).max(0.0),
            ConstraintType::Equal => (value - self.bound).abs(),
        }
    }

    /// Energy conservation constraint
    pub fn energy_conservation(bound: f64) -> Self {
        Self::new(
            "energy_conservation",
            ConstraintType::LessOrEqual,
            bound,
            |params| {
                // Sum of spectral values should not exceed 1
                let sum: f64 = params.iter().sum();
                sum / params.len() as f64
            },
        )
    }
}

/// Multi-objective optimization target
#[derive(Clone)]
pub struct MultiObjectiveTarget {
    /// Objective functions
    pub objectives: Vec<ObjectiveFunction>,
    /// Constraints
    pub constraints: Vec<Constraint>,
    /// Target values (for fitting)
    pub targets: Vec<f64>,
}

impl MultiObjectiveTarget {
    /// Create new target
    pub fn new() -> Self {
        Self {
            objectives: Vec::new(),
            constraints: Vec::new(),
            targets: Vec::new(),
        }
    }

    /// Add objective
    pub fn with_objective(mut self, objective: ObjectiveFunction) -> Self {
        self.objectives.push(objective);
        self
    }

    /// Add constraint
    pub fn with_constraint(mut self, constraint: Constraint) -> Self {
        self.constraints.push(constraint);
        self
    }

    /// Set target values
    pub fn with_targets(mut self, targets: Vec<f64>) -> Self {
        self.targets = targets;
        self
    }

    /// Evaluate all objectives
    pub fn evaluate_objectives(&self, rendered: &[f64]) -> Vec<f64> {
        self.objectives
            .iter()
            .map(|obj| obj.evaluate(rendered, &self.targets))
            .collect()
    }

    /// Compute scalarized objective (weighted sum)
    pub fn scalarize(&self, rendered: &[f64]) -> f64 {
        let mut total = 0.0;
        for obj in &self.objectives {
            let value = obj.evaluate(rendered, &self.targets);
            let contribution = match obj.objective_type {
                ObjectiveType::Minimize => value * obj.weight,
                ObjectiveType::Maximize => -value * obj.weight,
            };
            total += contribution;
        }
        total
    }

    /// Check all constraints
    pub fn constraints_satisfied(&self, params: &[f64]) -> bool {
        self.constraints.iter().all(|c| c.is_satisfied(params))
    }

    /// Total constraint violation
    pub fn total_violation(&self, params: &[f64]) -> f64 {
        self.constraints.iter().map(|c| c.violation(params)).sum()
    }

    /// Create standard spectral fitting target
    pub fn spectral_fitting(targets: Vec<f64>) -> Self {
        Self::new()
            .with_objective(ObjectiveFunction::rmse())
            .with_objective(ObjectiveFunction::peak_error())
            .with_targets(targets)
    }
}

impl Default for MultiObjectiveTarget {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Optimization Result
// ============================================================================

/// Result of optimization run
#[derive(Debug, Clone)]
pub struct OptimizationResult {
    /// Best parameters found
    pub params: Vec<f64>,
    /// Objective values at best params
    pub objectives: Vec<f64>,
    /// Scalarized objective value
    pub scalarized: f64,
    /// Constraint violation
    pub violation: f64,
    /// Number of function evaluations
    pub evaluations: usize,
    /// Convergence status
    pub converged: bool,
    /// Optimization metadata
    pub metadata: HashMap<String, f64>,
}

impl OptimizationResult {
    /// Create new result
    pub fn new(params: Vec<f64>) -> Self {
        Self {
            params,
            objectives: Vec::new(),
            scalarized: f64::INFINITY,
            violation: 0.0,
            evaluations: 0,
            converged: false,
            metadata: HashMap::new(),
        }
    }
}

// ============================================================================
// Simple Grid Search
// ============================================================================

/// Simple grid search optimizer for small parameter spaces
pub struct GridSearchOptimizer {
    /// Grid resolution per dimension
    pub resolution: usize,
    /// Maximum evaluations
    pub max_evaluations: usize,
}

impl Default for GridSearchOptimizer {
    fn default() -> Self {
        Self {
            resolution: 10,
            max_evaluations: 10000,
        }
    }
}

impl GridSearchOptimizer {
    /// Run grid search optimization
    pub fn optimize<F: ForwardFunction>(
        &self,
        forward: &F,
        bounds: &ParameterBounds,
        target: &MultiObjectiveTarget,
    ) -> OptimizationResult {
        let n_dim = bounds.dim();
        let mut best_params = bounds.center();
        let mut best_score = f64::INFINITY;
        let mut evaluations = 0;

        // For high dimensions, use random sampling instead
        let use_random = n_dim > 4 || self.resolution.pow(n_dim as u32) > self.max_evaluations;

        if use_random {
            // Random search
            let mut rng_state: u64 = 12345;
            let mut rng = || {
                rng_state = rng_state.wrapping_mul(6364136223846793005).wrapping_add(1);
                (rng_state as f64) / (u64::MAX as f64)
            };

            for _ in 0..self.max_evaluations {
                let params = bounds.random(&mut rng);
                let rendered = forward.forward(&params);
                let score = target.scalarize(&rendered);
                evaluations += 1;

                if score < best_score && target.constraints_satisfied(&params) {
                    best_score = score;
                    best_params = params;
                }
            }
        } else {
            // Full grid search
            let mut indices = vec![0usize; n_dim];

            loop {
                // Generate parameters at current grid point
                let params: Vec<f64> = indices
                    .iter()
                    .enumerate()
                    .map(|(i, &idx)| {
                        bounds.lower[i]
                            + (idx as f64 / (self.resolution - 1).max(1) as f64)
                                * (bounds.upper[i] - bounds.lower[i])
                    })
                    .collect();

                let rendered = forward.forward(&params);
                let score = target.scalarize(&rendered);
                evaluations += 1;

                if score < best_score && target.constraints_satisfied(&params) {
                    best_score = score;
                    best_params = params.clone();
                }

                // Advance to next grid point
                let mut carry = true;
                for i in 0..n_dim {
                    if carry {
                        indices[i] += 1;
                        if indices[i] >= self.resolution {
                            indices[i] = 0;
                        } else {
                            carry = false;
                        }
                    }
                }

                if carry || evaluations >= self.max_evaluations {
                    break;
                }
            }
        }

        let rendered = forward.forward(&best_params);
        OptimizationResult {
            params: best_params,
            objectives: target.evaluate_objectives(&rendered),
            scalarized: best_score,
            violation: target.total_violation(&rendered),
            evaluations,
            converged: best_score < f64::INFINITY,
            metadata: HashMap::new(),
        }
    }
}

// ============================================================================
// Memory Estimation
// ============================================================================

/// Estimate memory usage for research API structures
pub fn estimate_research_api_memory() -> usize {
    let mut total = 0;

    // MaterialForwardFunction
    total += std::mem::size_of::<MaterialForwardFunction>();
    total += 31 * 8; // wavelengths

    // ParameterBounds
    total += std::mem::size_of::<ParameterBounds>();
    total += 7 * 8 * 2; // bounds arrays

    // MultiObjectiveTarget
    total += std::mem::size_of::<MultiObjectiveTarget>();
    total += 3 * std::mem::size_of::<ObjectiveFunction>();
    total += 31 * 8; // targets

    // GridSearchOptimizer
    total += std::mem::size_of::<GridSearchOptimizer>();

    // Typical usage overhead
    total += 5 * 1024;

    total
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parameter_bounds() {
        let bounds = ParameterBounds::standard_pbr();
        assert_eq!(bounds.dim(), 6);

        let center = bounds.center();
        assert!(bounds.is_valid(&center));

        let clamped = bounds.clamp(&[2.0, 2.0, 5.0, 2.0, 2.0, 2.0]);
        assert!(bounds.is_valid(&clamped));
    }

    #[test]
    fn test_parameter_normalization() {
        let bounds = ParameterBounds::new(vec![0.0, 10.0], vec![1.0, 20.0]);

        let params = vec![0.5, 15.0];
        let normalized = bounds.normalize(&params);
        assert!((normalized[0] - 0.5).abs() < 1e-10);
        assert!((normalized[1] - 0.5).abs() < 1e-10);

        let denorm = bounds.denormalize(&normalized);
        assert!((denorm[0] - 0.5).abs() < 1e-10);
        assert!((denorm[1] - 15.0).abs() < 1e-10);
    }

    #[test]
    fn test_material_forward_function() {
        let forward = MaterialForwardFunction::standard_pbr();
        assert_eq!(forward.input_dim(), 6);
        assert_eq!(forward.output_dim(), 31);

        let params = vec![0.0, 0.5, 1.5, 0.8, 0.2, 0.2]; // Dielectric red
        let output = forward.forward(&params);
        assert_eq!(output.len(), 31);
        assert!(output.iter().all(|&v| v >= 0.0 && v <= 1.0));
    }

    #[test]
    fn test_jacobian_numerical() {
        let forward = MaterialForwardFunction::standard_pbr();
        let params = vec![0.0, 0.5, 1.5, 0.5, 0.5, 0.5];

        let jac = forward.jacobian(&params);
        assert!(jac.is_some());

        let jac = jac.unwrap();
        assert_eq!(jac.len(), 31); // output_dim
        assert_eq!(jac[0].len(), 6); // input_dim
    }

    #[test]
    fn test_objective_function_rmse() {
        let obj = ObjectiveFunction::rmse();

        let rendered = vec![0.1, 0.2, 0.3];
        let target = vec![0.1, 0.2, 0.3];
        let rmse = obj.evaluate(&rendered, &target);
        assert!(rmse < 1e-10);

        let target2 = vec![0.2, 0.3, 0.4];
        let rmse2 = obj.evaluate(&rendered, &target2);
        assert!(rmse2 > 0.0);
    }

    #[test]
    fn test_constraint() {
        let constraint = Constraint::new("max_value", ConstraintType::LessOrEqual, 0.5, |params| {
            params.iter().cloned().fold(0.0, f64::max)
        });

        assert!(constraint.is_satisfied(&[0.1, 0.2, 0.3]));
        assert!(!constraint.is_satisfied(&[0.1, 0.6, 0.3]));

        let violation = constraint.violation(&[0.1, 0.7, 0.3]);
        assert!((violation - 0.2).abs() < 1e-10);
    }

    #[test]
    fn test_multi_objective_target() {
        let target = MultiObjectiveTarget::spectral_fitting(vec![0.5; 10]);

        assert_eq!(target.objectives.len(), 2);
        assert!(target.constraints.is_empty());

        let rendered = vec![0.5; 10];
        let objectives = target.evaluate_objectives(&rendered);
        assert!(objectives[0] < 1e-10); // RMSE should be ~0
    }

    #[test]
    fn test_grid_search_simple() {
        let forward = MaterialForwardFunction::standard_pbr();
        let bounds = ParameterBounds::standard_pbr();
        let target = MultiObjectiveTarget::spectral_fitting(vec![0.5; 31]);

        let optimizer = GridSearchOptimizer {
            resolution: 3,
            max_evaluations: 100,
        };

        let result = optimizer.optimize(&forward, &bounds, &target);
        assert!(result.evaluations > 0);
        assert_eq!(result.params.len(), 6);
    }

    #[test]
    fn test_parameter_mapping() {
        let mapping = ParameterMapping::standard_pbr();
        assert_eq!(mapping.total_dim, 6);
        assert_eq!(mapping.index_of("metallic"), Some(0));
        assert_eq!(mapping.index_of("roughness"), Some(1));
        assert_eq!(mapping.name_of(2), Some("ior"));
    }

    #[test]
    fn test_glass_forward_function() {
        let forward = MaterialForwardFunction::glass();
        assert_eq!(forward.input_dim(), 7);
        assert_eq!(forward.output_dim(), 31);

        let params = vec![1.52, 0.0, 0.9, 0.1, 0.9, 0.9, 0.9]; // Clear glass
        let output = forward.forward(&params);
        assert_eq!(output.len(), 31);
    }

    #[test]
    fn test_memory_estimate() {
        let estimate = estimate_research_api_memory();
        assert!(estimate > 0);
        assert!(estimate < 15 * 1024); // Should be under 15KB
    }
}
