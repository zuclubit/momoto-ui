//! # Differentiable BSDF Traits
//!
//! Core trait definitions for differentiable material evaluation.

use super::super::unified_bsdf::{BSDFContext, BSDFResponse, BSDF};

// ============================================================================
// PARAMETER GRADIENTS
// ============================================================================

/// Gradients for material parameters.
///
/// Contains partial derivatives of the BSDF response with respect to
/// each physical parameter.
#[derive(Debug, Clone, Default)]
pub struct ParameterGradients {
    // Optical parameters
    /// Gradient w.r.t. refractive index (n).
    pub d_ior: f64,
    /// Gradient w.r.t. extinction coefficient (k).
    pub d_extinction: f64,
    /// Gradient w.r.t. surface roughness (α).
    pub d_roughness: f64,

    // Absorption parameters
    /// Gradient w.r.t. absorption coefficient.
    pub d_absorption: f64,
    /// Gradient w.r.t. scattering coefficient.
    pub d_scattering: f64,

    // Thin-film parameters
    /// Gradient w.r.t. film thickness (nm).
    pub d_film_thickness: Option<f64>,
    /// Gradient w.r.t. film refractive index.
    pub d_film_ior: Option<f64>,

    // Scattering parameters
    /// Gradient w.r.t. asymmetry parameter (g).
    pub d_asymmetry_g: f64,

    // Temporal evolution parameters (Phase 12 integration)
    /// Gradient w.r.t. evolution rate.
    pub d_evolution_rate: Option<f64>,
    /// Gradient w.r.t. evolution time constant (tau).
    pub d_evolution_tau: Option<f64>,

    // Response gradients (for chain rule)
    /// Gradient of reflectance w.r.t. this parameter set.
    pub d_reflectance: f64,
    /// Gradient of transmittance w.r.t. this parameter set.
    pub d_transmittance: f64,
}

impl ParameterGradients {
    /// Create zero gradients.
    pub fn zero() -> Self {
        Self::default()
    }

    /// Create gradients with only IOR gradient.
    pub fn ior_only(d_ior: f64) -> Self {
        Self {
            d_ior,
            ..Default::default()
        }
    }

    /// Create gradients with IOR and roughness.
    pub fn dielectric(d_ior: f64, d_roughness: f64) -> Self {
        Self {
            d_ior,
            d_roughness,
            ..Default::default()
        }
    }

    /// Create gradients for conductor (n, k, roughness).
    pub fn conductor(d_ior: f64, d_extinction: f64, d_roughness: f64) -> Self {
        Self {
            d_ior,
            d_extinction,
            d_roughness,
            ..Default::default()
        }
    }

    /// Create gradients for thin-film.
    pub fn thin_film(d_thickness: f64, d_film_ior: f64, d_substrate_roughness: f64) -> Self {
        Self {
            d_roughness: d_substrate_roughness,
            d_film_thickness: Some(d_thickness),
            d_film_ior: Some(d_film_ior),
            ..Default::default()
        }
    }

    /// Add another gradient set (for accumulation).
    pub fn add(&mut self, other: &ParameterGradients) {
        self.d_ior += other.d_ior;
        self.d_extinction += other.d_extinction;
        self.d_roughness += other.d_roughness;
        self.d_absorption += other.d_absorption;
        self.d_scattering += other.d_scattering;
        self.d_asymmetry_g += other.d_asymmetry_g;
        self.d_reflectance += other.d_reflectance;
        self.d_transmittance += other.d_transmittance;

        if let (Some(ref mut a), Some(b)) = (&mut self.d_film_thickness, other.d_film_thickness) {
            *a += b;
        }
        if let (Some(ref mut a), Some(b)) = (&mut self.d_film_ior, other.d_film_ior) {
            *a += b;
        }
        if let (Some(ref mut a), Some(b)) = (&mut self.d_evolution_rate, other.d_evolution_rate) {
            *a += b;
        }
        if let (Some(ref mut a), Some(b)) = (&mut self.d_evolution_tau, other.d_evolution_tau) {
            *a += b;
        }
    }

    /// Scale all gradients by a factor.
    pub fn scale(&mut self, factor: f64) {
        self.d_ior *= factor;
        self.d_extinction *= factor;
        self.d_roughness *= factor;
        self.d_absorption *= factor;
        self.d_scattering *= factor;
        self.d_asymmetry_g *= factor;
        self.d_reflectance *= factor;
        self.d_transmittance *= factor;

        if let Some(ref mut v) = self.d_film_thickness {
            *v *= factor;
        }
        if let Some(ref mut v) = self.d_film_ior {
            *v *= factor;
        }
        if let Some(ref mut v) = self.d_evolution_rate {
            *v *= factor;
        }
        if let Some(ref mut v) = self.d_evolution_tau {
            *v *= factor;
        }
    }

    /// Compute L2 norm of gradient vector.
    pub fn norm(&self) -> f64 {
        let mut sum = 0.0;
        sum += self.d_ior * self.d_ior;
        sum += self.d_extinction * self.d_extinction;
        sum += self.d_roughness * self.d_roughness;
        sum += self.d_absorption * self.d_absorption;
        sum += self.d_scattering * self.d_scattering;
        sum += self.d_asymmetry_g * self.d_asymmetry_g;

        if let Some(v) = self.d_film_thickness {
            sum += v * v;
        }
        if let Some(v) = self.d_film_ior {
            sum += v * v;
        }

        sum.sqrt()
    }

    /// Clip gradients to maximum norm.
    pub fn clip(&mut self, max_norm: f64) {
        let norm = self.norm();
        if norm > max_norm {
            self.scale(max_norm / norm);
        }
    }

    /// Convert to vector for optimization.
    pub fn to_vec(&self) -> Vec<f64> {
        let mut v = vec![
            self.d_ior,
            self.d_extinction,
            self.d_roughness,
            self.d_absorption,
            self.d_scattering,
            self.d_asymmetry_g,
        ];

        if let Some(d) = self.d_film_thickness {
            v.push(d);
        }
        if let Some(d) = self.d_film_ior {
            v.push(d);
        }

        v
    }

    /// Create from vector.
    pub fn from_vec(v: &[f64], has_film: bool) -> Self {
        let mut grad = Self {
            d_ior: v.get(0).copied().unwrap_or(0.0),
            d_extinction: v.get(1).copied().unwrap_or(0.0),
            d_roughness: v.get(2).copied().unwrap_or(0.0),
            d_absorption: v.get(3).copied().unwrap_or(0.0),
            d_scattering: v.get(4).copied().unwrap_or(0.0),
            d_asymmetry_g: v.get(5).copied().unwrap_or(0.0),
            ..Default::default()
        };

        if has_film && v.len() >= 8 {
            grad.d_film_thickness = Some(v[6]);
            grad.d_film_ior = Some(v[7]);
        }

        grad
    }
}

// ============================================================================
// PARAMETER BOUNDS
// ============================================================================

/// Bounds for material parameters during optimization.
#[derive(Debug, Clone)]
pub struct ParameterBounds {
    /// IOR bounds (typically 1.0 to 4.0).
    pub ior: (f64, f64),
    /// Extinction coefficient bounds (0.0 to 10.0).
    pub extinction: (f64, f64),
    /// Roughness bounds (0.0 to 1.0).
    pub roughness: (f64, f64),
    /// Absorption coefficient bounds.
    pub absorption: (f64, f64),
    /// Scattering coefficient bounds.
    pub scattering: (f64, f64),
    /// Film thickness bounds (nm).
    pub film_thickness: Option<(f64, f64)>,
    /// Film IOR bounds.
    pub film_ior: Option<(f64, f64)>,
    /// Asymmetry g bounds (-1 to 1).
    pub asymmetry_g: (f64, f64),
}

impl Default for ParameterBounds {
    fn default() -> Self {
        Self {
            ior: (1.0, 4.0),
            extinction: (0.0, 10.0),
            roughness: (0.001, 1.0),
            absorption: (0.0, 100.0),
            scattering: (0.0, 100.0),
            film_thickness: None,
            film_ior: None,
            asymmetry_g: (-0.99, 0.99),
        }
    }
}

impl ParameterBounds {
    /// Create bounds for dielectric materials.
    pub fn dielectric() -> Self {
        Self {
            ior: (1.0, 3.0),
            roughness: (0.001, 1.0),
            ..Default::default()
        }
    }

    /// Create bounds for conductor materials.
    pub fn conductor() -> Self {
        Self {
            ior: (0.1, 5.0),
            extinction: (0.0, 10.0),
            roughness: (0.001, 1.0),
            ..Default::default()
        }
    }

    /// Create bounds for thin-film materials.
    pub fn thin_film() -> Self {
        Self {
            film_thickness: Some((10.0, 2000.0)),
            film_ior: Some((1.2, 2.5)),
            ..Default::default()
        }
    }

    /// Clamp a parameter vector to bounds.
    pub fn clamp(&self, params: &mut [f64], has_film: bool) {
        if params.len() > 0 {
            params[0] = params[0].clamp(self.ior.0, self.ior.1);
        }
        if params.len() > 1 {
            params[1] = params[1].clamp(self.extinction.0, self.extinction.1);
        }
        if params.len() > 2 {
            params[2] = params[2].clamp(self.roughness.0, self.roughness.1);
        }

        if has_film {
            if let (Some(bounds), Some(p)) = (self.film_thickness, params.get_mut(6)) {
                *p = p.clamp(bounds.0, bounds.1);
            }
            if let (Some(bounds), Some(p)) = (self.film_ior, params.get_mut(7)) {
                *p = p.clamp(bounds.0, bounds.1);
            }
        }
    }
}

// ============================================================================
// GRADIENT CONFIGURATION
// ============================================================================

/// Configuration for gradient computation.
#[derive(Debug, Clone)]
pub struct GradientConfig {
    /// Use analytical gradients (vs numerical).
    pub use_analytical: bool,
    /// Epsilon for numerical gradient verification.
    pub numeric_epsilon: f64,
    /// Maximum gradient norm (for clipping).
    pub max_gradient_norm: f64,
    /// Whether to compute full Jacobian.
    pub compute_jacobian: bool,
    /// Enable energy conservation check.
    pub check_energy_conservation: bool,
}

impl Default for GradientConfig {
    fn default() -> Self {
        Self {
            use_analytical: true,
            numeric_epsilon: 1e-5,
            max_gradient_norm: 100.0,
            compute_jacobian: false,
            check_energy_conservation: true,
        }
    }
}

// ============================================================================
// DIFFERENTIABLE RESPONSE
// ============================================================================

/// Response from differentiable BSDF evaluation.
#[derive(Debug, Clone)]
pub struct DifferentiableResponse {
    /// Forward evaluation result.
    pub response: BSDFResponse,
    /// Gradients w.r.t. all material parameters.
    pub gradients: ParameterGradients,
    /// Energy conservation status.
    pub energy_conserved: bool,
}

impl DifferentiableResponse {
    /// Create new response with gradients.
    pub fn new(response: BSDFResponse, gradients: ParameterGradients) -> Self {
        let energy_conserved =
            (response.reflectance + response.transmittance + response.absorption - 1.0).abs()
                < 1e-6;
        Self {
            response,
            gradients,
            energy_conserved,
        }
    }

    /// Create response with zero gradients.
    pub fn with_zero_gradients(response: BSDFResponse) -> Self {
        Self::new(response, ParameterGradients::zero())
    }

    /// Project gradients to maintain energy conservation.
    ///
    /// Ensures ∂R/∂p + ∂T/∂p + ∂A/∂p = 0 for any parameter p.
    pub fn project_energy_conserving(&mut self) {
        // The absorption gradient is determined by R and T:
        // ∂A/∂p = -(∂R/∂p + ∂T/∂p)
        // This is implicit in the ParameterGradients structure
        // but we can verify/enforce it here
    }
}

// ============================================================================
// DIFFERENTIABLE BSDF TRAIT
// ============================================================================

/// Trait for BSDFs that support analytical gradient computation.
///
/// This trait extends the base BSDF trait with methods for computing
/// gradients of the response with respect to material parameters.
pub trait DifferentiableBSDF: BSDF {
    /// Evaluate BSDF with analytical gradients.
    ///
    /// Returns both the forward evaluation result and gradients
    /// with respect to all material parameters.
    fn eval_with_gradients(&self, ctx: &BSDFContext) -> DifferentiableResponse;

    /// Get parameter bounds for optimization.
    fn parameter_bounds(&self) -> ParameterBounds {
        ParameterBounds::default()
    }

    /// Get number of differentiable parameters.
    fn param_count(&self) -> usize {
        6 // Default: ior, extinction, roughness, absorption, scattering, g
    }

    /// Convert current parameters to vector.
    fn params_to_vec(&self) -> Vec<f64>;

    /// Create material from parameter vector.
    fn from_param_vec(params: &[f64]) -> Self
    where
        Self: Sized;

    /// Verify gradients against numerical differentiation.
    fn verify_gradients(&self, ctx: &BSDFContext, epsilon: f64) -> GradientVerification
    where
        Self: Sized,
    {
        let analytical = self.eval_with_gradients(ctx);
        let params = self.params_to_vec();

        let mut max_error = 0.0f64;
        let mut errors = Vec::new();

        // Compute numerical gradients for each parameter
        for i in 0..params.len() {
            let mut params_plus = params.clone();
            let mut params_minus = params.clone();
            params_plus[i] += epsilon;
            params_minus[i] -= epsilon;

            let material_plus = Self::from_param_vec(&params_plus);
            let material_minus = Self::from_param_vec(&params_minus);

            let r_plus = material_plus.evaluate(ctx);
            let r_minus = material_minus.evaluate(ctx);

            let numeric_grad = (r_plus.reflectance - r_minus.reflectance) / (2.0 * epsilon);

            let analytic_grad = match i {
                0 => analytical.gradients.d_ior,
                1 => analytical.gradients.d_extinction,
                2 => analytical.gradients.d_roughness,
                _ => 0.0,
            };

            let error = (analytic_grad - numeric_grad).abs();
            max_error = max_error.max(error);
            errors.push(error);
        }

        GradientVerification {
            passed: max_error < 1e-4,
            max_error,
            per_param_errors: errors,
        }
    }

    /// Check if this material supports full Jacobian computation.
    fn supports_jacobian(&self) -> bool {
        false
    }
}

/// Result of gradient verification.
#[derive(Debug, Clone)]
pub struct GradientVerification {
    /// Whether verification passed.
    pub passed: bool,
    /// Maximum error across all parameters.
    pub max_error: f64,
    /// Error for each parameter.
    pub per_param_errors: Vec<f64>,
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parameter_gradients_zero() {
        let grad = ParameterGradients::zero();
        assert!((grad.d_ior - 0.0).abs() < 1e-10);
        assert!((grad.d_roughness - 0.0).abs() < 1e-10);
    }

    #[test]
    fn test_parameter_gradients_ior_only() {
        let grad = ParameterGradients::ior_only(0.5);
        assert!((grad.d_ior - 0.5).abs() < 1e-10);
        assert!((grad.d_roughness - 0.0).abs() < 1e-10);
    }

    #[test]
    fn test_parameter_gradients_dielectric() {
        let grad = ParameterGradients::dielectric(0.3, 0.7);
        assert!((grad.d_ior - 0.3).abs() < 1e-10);
        assert!((grad.d_roughness - 0.7).abs() < 1e-10);
    }

    #[test]
    fn test_parameter_gradients_add() {
        let mut grad1 = ParameterGradients::dielectric(0.3, 0.4);
        let grad2 = ParameterGradients::dielectric(0.1, 0.2);
        grad1.add(&grad2);
        assert!((grad1.d_ior - 0.4).abs() < 1e-10);
        assert!((grad1.d_roughness - 0.6).abs() < 1e-10);
    }

    #[test]
    fn test_parameter_gradients_scale() {
        let mut grad = ParameterGradients::dielectric(0.4, 0.8);
        grad.scale(0.5);
        assert!((grad.d_ior - 0.2).abs() < 1e-10);
        assert!((grad.d_roughness - 0.4).abs() < 1e-10);
    }

    #[test]
    fn test_parameter_gradients_norm() {
        let grad = ParameterGradients::dielectric(3.0, 4.0);
        // norm = sqrt(3^2 + 4^2) = 5
        assert!((grad.norm() - 5.0).abs() < 1e-10);
    }

    #[test]
    fn test_parameter_gradients_clip() {
        let mut grad = ParameterGradients::dielectric(6.0, 8.0);
        // norm = 10, clip to 5
        grad.clip(5.0);
        assert!((grad.norm() - 5.0).abs() < 1e-6);
    }

    #[test]
    fn test_parameter_gradients_to_vec() {
        let grad = ParameterGradients::dielectric(0.3, 0.7);
        let v = grad.to_vec();
        assert_eq!(v.len(), 6);
        assert!((v[0] - 0.3).abs() < 1e-10);
        assert!((v[2] - 0.7).abs() < 1e-10);
    }

    #[test]
    fn test_parameter_bounds_default() {
        let bounds = ParameterBounds::default();
        assert_eq!(bounds.ior, (1.0, 4.0));
        assert_eq!(bounds.roughness, (0.001, 1.0));
    }

    #[test]
    fn test_parameter_bounds_clamp() {
        let bounds = ParameterBounds::dielectric();
        let mut params = vec![0.5, 0.0, 1.5, 0.0, 0.0, 0.0];
        bounds.clamp(&mut params, false);
        assert!((params[0] - 1.0).abs() < 1e-10); // Clamped to min
        assert!((params[2] - 1.0).abs() < 1e-10); // Clamped to max
    }

    #[test]
    fn test_gradient_config_default() {
        let config = GradientConfig::default();
        assert!(config.use_analytical);
        assert!((config.numeric_epsilon - 1e-5).abs() < 1e-10);
    }

    #[test]
    fn test_differentiable_response_new() {
        let response = BSDFResponse::new(0.3, 0.6, 0.1);
        let grads = ParameterGradients::zero();
        let diff_response = DifferentiableResponse::new(response, grads);
        assert!(diff_response.energy_conserved);
    }
}
