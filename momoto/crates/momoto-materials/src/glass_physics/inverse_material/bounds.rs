//! # Parameter Bounds Enforcement
//!
//! Ensure material parameters stay within physically valid ranges during optimization.
//!
//! ## Overview
//!
//! Material parameters have physical constraints:
//! - IOR must be ≥ 1.0 (vacuum is the minimum)
//! - Roughness must be in [0, 1]
//! - Extinction coefficient must be ≥ 0
//! - Film thickness must be ≥ 0
//!
//! This module provides methods to project parameters back into valid regions
//! while preserving gradient information where possible.

// ============================================================================
// PROJECTION METHODS
// ============================================================================

/// Methods for projecting parameters back to valid bounds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProjectionMethod {
    /// Simple clamp to bounds (discontinuous gradient at boundaries).
    Clamp,
    /// Soft projection using sigmoid (preserves gradient flow).
    Sigmoid,
    /// Reflection at boundaries (maintains momentum direction).
    Reflect,
    /// Logarithmic barrier (infinite cost at boundary).
    Barrier,
}

impl Default for ProjectionMethod {
    fn default() -> Self {
        Self::Clamp
    }
}

// ============================================================================
// BOUNDS CONFIGURATION
// ============================================================================

/// Configuration for parameter bounds enforcement.
#[derive(Debug, Clone)]
pub struct BoundsConfig {
    /// Projection method to use.
    pub method: ProjectionMethod,
    /// Barrier strength (for Barrier method).
    pub barrier_strength: f64,
    /// Sigmoid sharpness (for Sigmoid method).
    pub sigmoid_sharpness: f64,
    /// Whether to apply bounds during optimization.
    pub enabled: bool,
}

impl Default for BoundsConfig {
    fn default() -> Self {
        Self {
            method: ProjectionMethod::Clamp,
            barrier_strength: 1e-3,
            sigmoid_sharpness: 10.0,
            enabled: true,
        }
    }
}

impl BoundsConfig {
    /// Create config with sigmoid projection (gradient-preserving).
    pub fn sigmoid() -> Self {
        Self {
            method: ProjectionMethod::Sigmoid,
            ..Default::default()
        }
    }

    /// Create config with barrier method (for interior-point optimization).
    pub fn barrier(strength: f64) -> Self {
        Self {
            method: ProjectionMethod::Barrier,
            barrier_strength: strength,
            ..Default::default()
        }
    }

    /// Create config with reflection method (momentum-preserving).
    pub fn reflect() -> Self {
        Self {
            method: ProjectionMethod::Reflect,
            ..Default::default()
        }
    }
}

// ============================================================================
// PARAMETER BOUND
// ============================================================================

/// Bounds for a single parameter.
#[derive(Debug, Clone, Copy)]
pub struct ParameterBound {
    /// Minimum value (inclusive).
    pub min: f64,
    /// Maximum value (inclusive).
    pub max: f64,
    /// Parameter name for debugging.
    pub name: &'static str,
}

impl ParameterBound {
    /// Create new parameter bound.
    pub const fn new(min: f64, max: f64, name: &'static str) -> Self {
        Self { min, max, name }
    }

    /// Check if value is within bounds.
    pub fn contains(&self, value: f64) -> bool {
        value >= self.min && value <= self.max
    }

    /// Clamp value to bounds.
    pub fn clamp(&self, value: f64) -> f64 {
        value.clamp(self.min, self.max)
    }

    /// Sigmoid projection to bounds.
    pub fn sigmoid_project(&self, value: f64, sharpness: f64) -> f64 {
        // Map value to [0, 1] via sigmoid, then scale to bounds
        let centered = (value - (self.min + self.max) / 2.0) * sharpness / (self.max - self.min);
        let sigmoid = 1.0 / (1.0 + (-centered).exp());
        self.min + sigmoid * (self.max - self.min)
    }

    /// Sigmoid projection gradient (for backprop).
    pub fn sigmoid_gradient(&self, value: f64, sharpness: f64) -> f64 {
        let centered = (value - (self.min + self.max) / 2.0) * sharpness / (self.max - self.min);
        let sigmoid = 1.0 / (1.0 + (-centered).exp());
        let sigmoid_grad = sigmoid * (1.0 - sigmoid);
        sigmoid_grad * sharpness
    }

    /// Reflect value at boundaries.
    pub fn reflect(&self, value: f64) -> f64 {
        let range = self.max - self.min;
        if range <= 0.0 {
            return self.min;
        }

        // Normalize to [0, range]
        let mut normalized = value - self.min;

        // Reflect as needed
        let periods = (normalized / range).floor();
        normalized -= periods * range;

        // If in odd period, reflect
        if periods as i64 % 2 != 0 {
            normalized = range - normalized;
        }

        // Handle negative values
        if normalized < 0.0 {
            normalized = -normalized;
            if normalized > range {
                normalized = range - (normalized - range);
            }
        }

        self.min + normalized.clamp(0.0, range)
    }

    /// Log-barrier penalty (approaches infinity at boundary).
    pub fn barrier_penalty(&self, value: f64, strength: f64) -> f64 {
        let eps = 1e-10;
        let dist_min = (value - self.min).max(eps);
        let dist_max = (self.max - value).max(eps);
        -strength * (dist_min.ln() + dist_max.ln())
    }

    /// Log-barrier gradient.
    pub fn barrier_gradient(&self, value: f64, strength: f64) -> f64 {
        let eps = 1e-10;
        let dist_min = (value - self.min).max(eps);
        let dist_max = (self.max - value).max(eps);
        // Gradient points away from boundaries: positive near min, negative near max
        strength * (1.0 / dist_min - 1.0 / dist_max)
    }

    /// Get range of bounds.
    pub fn range(&self) -> f64 {
        self.max - self.min
    }

    /// Get center of bounds.
    pub fn center(&self) -> f64 {
        (self.min + self.max) / 2.0
    }
}

// ============================================================================
// PREDEFINED BOUNDS
// ============================================================================

/// Standard bounds for IOR parameter.
pub const IOR_BOUNDS: ParameterBound = ParameterBound::new(1.0, 4.0, "ior");

/// Standard bounds for extinction coefficient.
pub const EXTINCTION_BOUNDS: ParameterBound = ParameterBound::new(0.0, 10.0, "extinction");

/// Standard bounds for roughness parameter.
pub const ROUGHNESS_BOUNDS: ParameterBound = ParameterBound::new(0.001, 1.0, "roughness");

/// Standard bounds for absorption coefficient.
pub const ABSORPTION_BOUNDS: ParameterBound = ParameterBound::new(0.0, 10.0, "absorption");

/// Standard bounds for scattering coefficient.
pub const SCATTERING_BOUNDS: ParameterBound = ParameterBound::new(0.0, 10.0, "scattering");

/// Standard bounds for asymmetry parameter g.
pub const ASYMMETRY_BOUNDS: ParameterBound = ParameterBound::new(-0.99, 0.99, "asymmetry_g");

/// Standard bounds for thin-film thickness (nm).
pub const FILM_THICKNESS_BOUNDS: ParameterBound =
    ParameterBound::new(0.0, 2000.0, "film_thickness");

/// Standard bounds for thin-film IOR.
pub const FILM_IOR_BOUNDS: ParameterBound = ParameterBound::new(1.0, 3.0, "film_ior");

/// Standard bounds for evolution rate.
pub const EVOLUTION_RATE_BOUNDS: ParameterBound = ParameterBound::new(0.0, 10.0, "evolution_rate");

/// Standard bounds for evolution tau.
pub const EVOLUTION_TAU_BOUNDS: ParameterBound = ParameterBound::new(0.01, 1000.0, "evolution_tau");

// ============================================================================
// BOUNDS ENFORCER
// ============================================================================

/// Enforces parameter bounds during optimization.
#[derive(Debug, Clone)]
pub struct BoundsEnforcer {
    /// Bounds for each parameter.
    pub bounds: Vec<ParameterBound>,
    /// Configuration.
    pub config: BoundsConfig,
}

impl BoundsEnforcer {
    /// Create new bounds enforcer with given bounds.
    pub fn new(bounds: Vec<ParameterBound>) -> Self {
        Self {
            bounds,
            config: BoundsConfig::default(),
        }
    }

    /// Create enforcer with configuration.
    pub fn with_config(bounds: Vec<ParameterBound>, config: BoundsConfig) -> Self {
        Self { bounds, config }
    }

    /// Create enforcer for standard material parameters.
    pub fn standard_material() -> Self {
        Self::new(vec![
            IOR_BOUNDS,
            EXTINCTION_BOUNDS,
            ROUGHNESS_BOUNDS,
            ABSORPTION_BOUNDS,
            SCATTERING_BOUNDS,
            ASYMMETRY_BOUNDS,
            FILM_THICKNESS_BOUNDS,
            FILM_IOR_BOUNDS,
        ])
    }

    /// Create enforcer for dielectric parameters.
    pub fn dielectric() -> Self {
        Self::new(vec![IOR_BOUNDS, ROUGHNESS_BOUNDS])
    }

    /// Create enforcer for conductor parameters.
    pub fn conductor() -> Self {
        Self::new(vec![IOR_BOUNDS, EXTINCTION_BOUNDS, ROUGHNESS_BOUNDS])
    }

    /// Create enforcer for thin-film parameters.
    pub fn thin_film() -> Self {
        Self::new(vec![
            IOR_BOUNDS,
            ROUGHNESS_BOUNDS,
            FILM_THICKNESS_BOUNDS,
            FILM_IOR_BOUNDS,
        ])
    }

    /// Project parameters to valid region.
    pub fn project(&self, params: &mut [f64]) {
        if !self.config.enabled {
            return;
        }

        for (i, param) in params.iter_mut().enumerate() {
            if let Some(bound) = self.bounds.get(i) {
                *param = match self.config.method {
                    ProjectionMethod::Clamp => bound.clamp(*param),
                    ProjectionMethod::Sigmoid => {
                        bound.sigmoid_project(*param, self.config.sigmoid_sharpness)
                    }
                    ProjectionMethod::Reflect => bound.reflect(*param),
                    ProjectionMethod::Barrier => *param, // Barrier doesn't project, only penalizes
                };
            }
        }
    }

    /// Compute total barrier penalty for interior-point methods.
    pub fn barrier_penalty(&self, params: &[f64]) -> f64 {
        if !self.config.enabled || self.config.method != ProjectionMethod::Barrier {
            return 0.0;
        }

        let mut penalty = 0.0;
        for (i, &param) in params.iter().enumerate() {
            if let Some(bound) = self.bounds.get(i) {
                penalty += bound.barrier_penalty(param, self.config.barrier_strength);
            }
        }
        penalty
    }

    /// Compute barrier gradient contribution.
    pub fn barrier_gradient(&self, params: &[f64]) -> Vec<f64> {
        let mut gradient = vec![0.0; params.len()];

        if !self.config.enabled || self.config.method != ProjectionMethod::Barrier {
            return gradient;
        }

        for (i, &param) in params.iter().enumerate() {
            if let Some(bound) = self.bounds.get(i) {
                gradient[i] = bound.barrier_gradient(param, self.config.barrier_strength);
            }
        }

        gradient
    }

    /// Check if all parameters are within bounds.
    pub fn is_valid(&self, params: &[f64]) -> bool {
        for (i, &param) in params.iter().enumerate() {
            if let Some(bound) = self.bounds.get(i) {
                if !bound.contains(param) {
                    return false;
                }
            }
        }
        true
    }

    /// Get violation distances (positive = outside bounds).
    pub fn violations(&self, params: &[f64]) -> Vec<f64> {
        let mut violations = vec![0.0; params.len()];

        for (i, &param) in params.iter().enumerate() {
            if let Some(bound) = self.bounds.get(i) {
                if param < bound.min {
                    violations[i] = bound.min - param;
                } else if param > bound.max {
                    violations[i] = param - bound.max;
                }
            }
        }

        violations
    }

    /// Get maximum violation.
    pub fn max_violation(&self, params: &[f64]) -> f64 {
        self.violations(params).iter().cloned().fold(0.0, f64::max)
    }

    /// Project gradient to respect bounds (for active set methods).
    pub fn project_gradient(&self, params: &[f64], gradient: &mut [f64]) {
        if !self.config.enabled {
            return;
        }

        let eps = 1e-8;
        for (i, (&param, grad)) in params.iter().zip(gradient.iter_mut()).enumerate() {
            if let Some(bound) = self.bounds.get(i) {
                // Zero gradient if at boundary and pointing outward
                if param <= bound.min + eps && *grad < 0.0 {
                    *grad = 0.0;
                } else if param >= bound.max - eps && *grad > 0.0 {
                    *grad = 0.0;
                }
            }
        }
    }

    /// Apply sigmoid transformation to gradient for smooth projection.
    pub fn sigmoid_transform_gradient(&self, params: &[f64], gradient: &mut [f64]) {
        if !self.config.enabled || self.config.method != ProjectionMethod::Sigmoid {
            return;
        }

        for (i, (&param, grad)) in params.iter().zip(gradient.iter_mut()).enumerate() {
            if let Some(bound) = self.bounds.get(i) {
                let sigmoid_grad = bound.sigmoid_gradient(param, self.config.sigmoid_sharpness);
                *grad *= sigmoid_grad;
            }
        }
    }

    /// Get number of parameters.
    pub fn param_count(&self) -> usize {
        self.bounds.len()
    }

    /// Get bounds for parameter at index.
    pub fn get_bounds(&self, index: usize) -> Option<&ParameterBound> {
        self.bounds.get(index)
    }
}

impl Default for BoundsEnforcer {
    fn default() -> Self {
        Self::standard_material()
    }
}

// ============================================================================
// BOX CONSTRAINT OPTIMIZER WRAPPER
// ============================================================================

/// Wrapper that adds box constraints to any optimizer.
#[derive(Debug)]
pub struct BoxConstrainedOptimizer<T> {
    /// Inner optimizer.
    pub inner: T,
    /// Bounds enforcer.
    pub bounds: BoundsEnforcer,
    /// Current parameter values.
    pub params: Vec<f64>,
}

impl<T> BoxConstrainedOptimizer<T> {
    /// Create new box-constrained optimizer.
    pub fn new(inner: T, bounds: BoundsEnforcer, initial_params: Vec<f64>) -> Self {
        let mut params = initial_params;
        bounds.project(&mut params);
        Self {
            inner,
            bounds,
            params,
        }
    }

    /// Get current parameters.
    pub fn params(&self) -> &[f64] {
        &self.params
    }

    /// Update parameters with projection.
    pub fn update(&mut self, new_params: &[f64]) {
        self.params.clear();
        self.params.extend_from_slice(new_params);
        self.bounds.project(&mut self.params);
    }

    /// Check if converged (all within bounds and gradient small).
    pub fn is_feasible(&self) -> bool {
        self.bounds.is_valid(&self.params)
    }
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parameter_bound_clamp() {
        let bound = ParameterBound::new(1.0, 4.0, "ior");

        assert!((bound.clamp(0.5) - 1.0).abs() < 1e-10);
        assert!((bound.clamp(2.5) - 2.5).abs() < 1e-10);
        assert!((bound.clamp(5.0) - 4.0).abs() < 1e-10);
    }

    #[test]
    fn test_parameter_bound_contains() {
        let bound = ParameterBound::new(0.0, 1.0, "roughness");

        assert!(!bound.contains(-0.1));
        assert!(bound.contains(0.0));
        assert!(bound.contains(0.5));
        assert!(bound.contains(1.0));
        assert!(!bound.contains(1.1));
    }

    #[test]
    fn test_parameter_bound_sigmoid() {
        let bound = ParameterBound::new(0.0, 1.0, "roughness");
        let sharpness = 10.0;

        // Center should map to center
        let center_proj = bound.sigmoid_project(0.5, sharpness);
        assert!((center_proj - 0.5).abs() < 0.1);

        // Values should stay in bounds
        for x in [-10.0, -1.0, 0.0, 0.5, 1.0, 2.0, 10.0] {
            let proj = bound.sigmoid_project(x, sharpness);
            assert!(proj >= 0.0 && proj <= 1.0);
        }
    }

    #[test]
    fn test_parameter_bound_reflect() {
        let bound = ParameterBound::new(0.0, 1.0, "test");

        // Inside bounds - no change
        assert!((bound.reflect(0.5) - 0.5).abs() < 1e-10);

        // Just outside - reflected back
        let reflected = bound.reflect(1.2);
        assert!(reflected >= 0.0 && reflected <= 1.0);

        // Far outside - still valid
        let far_reflected = bound.reflect(3.5);
        assert!(far_reflected >= 0.0 && far_reflected <= 1.0);
    }

    #[test]
    fn test_parameter_bound_barrier() {
        let bound = ParameterBound::new(0.0, 1.0, "test");
        let strength = 0.01;

        // Interior point - finite penalty
        let penalty_interior = bound.barrier_penalty(0.5, strength);
        assert!(penalty_interior.is_finite());

        // Near boundary - higher penalty
        let penalty_near = bound.barrier_penalty(0.01, strength);
        assert!(penalty_near > penalty_interior);

        // Gradient points inward at boundary
        let grad_low = bound.barrier_gradient(0.1, strength);
        let grad_high = bound.barrier_gradient(0.9, strength);
        assert!(grad_low > 0.0); // Push away from lower bound
        assert!(grad_high < 0.0); // Push away from upper bound
    }

    #[test]
    fn test_bounds_enforcer_project() {
        let enforcer = BoundsEnforcer::dielectric();
        let mut params = vec![0.5, 0.0, 0.5]; // IOR too low

        enforcer.project(&mut params);

        assert!(params[0] >= 1.0); // IOR clamped
    }

    #[test]
    fn test_bounds_enforcer_is_valid() {
        let enforcer = BoundsEnforcer::new(vec![
            ParameterBound::new(1.0, 4.0, "ior"),
            ParameterBound::new(0.0, 1.0, "roughness"),
        ]);

        assert!(enforcer.is_valid(&[1.5, 0.5]));
        assert!(!enforcer.is_valid(&[0.5, 0.5])); // IOR too low
        assert!(!enforcer.is_valid(&[1.5, 1.5])); // Roughness too high
    }

    #[test]
    fn test_bounds_enforcer_violations() {
        let enforcer = BoundsEnforcer::new(vec![
            ParameterBound::new(1.0, 4.0, "ior"),
            ParameterBound::new(0.0, 1.0, "roughness"),
        ]);

        let violations = enforcer.violations(&[0.5, 1.5]);

        assert!((violations[0] - 0.5).abs() < 1e-10); // 1.0 - 0.5
        assert!((violations[1] - 0.5).abs() < 1e-10); // 1.5 - 1.0
    }

    #[test]
    fn test_bounds_enforcer_project_gradient() {
        let enforcer = BoundsEnforcer::new(vec![ParameterBound::new(0.0, 1.0, "test")]);

        // At lower bound, negative gradient should be zeroed
        let params = vec![0.0];
        let mut gradient = vec![-1.0];
        enforcer.project_gradient(&params, &mut gradient);
        assert!((gradient[0] - 0.0).abs() < 1e-10);

        // At upper bound, positive gradient should be zeroed
        let params = vec![1.0];
        let mut gradient = vec![1.0];
        enforcer.project_gradient(&params, &mut gradient);
        assert!((gradient[0] - 0.0).abs() < 1e-10);

        // Interior point, gradient unchanged
        let params = vec![0.5];
        let mut gradient = vec![1.0];
        enforcer.project_gradient(&params, &mut gradient);
        assert!((gradient[0] - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_bounds_enforcer_sigmoid_config() {
        let enforcer = BoundsEnforcer::with_config(
            vec![ParameterBound::new(0.0, 1.0, "test")],
            BoundsConfig::sigmoid(),
        );

        let mut params = vec![2.0]; // Outside bounds
        enforcer.project(&mut params);

        // Should be projected inside bounds via sigmoid
        assert!(params[0] >= 0.0 && params[0] <= 1.0);
    }

    #[test]
    fn test_bounds_enforcer_barrier_penalty() {
        let enforcer = BoundsEnforcer::with_config(
            vec![ParameterBound::new(0.0, 1.0, "test")],
            BoundsConfig::barrier(0.01),
        );

        let penalty_center = enforcer.barrier_penalty(&[0.5]);
        let penalty_edge = enforcer.barrier_penalty(&[0.1]);

        assert!(penalty_edge > penalty_center);
    }

    #[test]
    fn test_predefined_bounds() {
        assert!((IOR_BOUNDS.min - 1.0).abs() < 1e-10);
        assert!((IOR_BOUNDS.max - 4.0).abs() < 1e-10);

        assert!((ROUGHNESS_BOUNDS.min - 0.001).abs() < 1e-10);
        assert!((ROUGHNESS_BOUNDS.max - 1.0).abs() < 1e-10);

        assert!((FILM_THICKNESS_BOUNDS.max - 2000.0).abs() < 1e-10);
    }

    #[test]
    fn test_standard_material_enforcer() {
        let enforcer = BoundsEnforcer::standard_material();
        assert_eq!(enforcer.param_count(), 8);
    }

    #[test]
    fn test_max_violation() {
        let enforcer = BoundsEnforcer::new(vec![
            ParameterBound::new(0.0, 1.0, "a"),
            ParameterBound::new(0.0, 1.0, "b"),
        ]);

        let max_viol = enforcer.max_violation(&[-0.5, 1.3]);
        assert!((max_viol - 0.5).abs() < 1e-10);
    }
}
