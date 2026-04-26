//! # Differentiable Dielectric BSDF
//!
//! Dielectric material with analytical gradient computation.

use super::super::unified_bsdf::{BSDFContext, BSDFResponse, BSDFSample, EnergyValidation, BSDF};
use super::gradients::fresnel_schlick_gradient;
use super::traits::{
    DifferentiableBSDF, DifferentiableResponse, ParameterBounds, ParameterGradients,
};

// ============================================================================
// DIFFERENTIABLE DIELECTRIC
// ============================================================================

/// Dielectric BSDF with analytical gradient support.
///
/// Supports gradients w.r.t. IOR and roughness.
#[derive(Debug, Clone)]
pub struct DifferentiableDielectric {
    /// Refractive index.
    pub ior: f64,
    /// Surface roughness (0 = perfectly smooth).
    pub roughness: f64,
    /// Roughness squared for GGX.
    alpha: f64,
}

impl DifferentiableDielectric {
    /// Create new differentiable dielectric.
    pub fn new(ior: f64, roughness: f64) -> Self {
        let roughness = roughness.clamp(0.001, 1.0);
        Self {
            ior: ior.max(1.0),
            roughness,
            alpha: roughness * roughness,
        }
    }

    /// Create glass material.
    pub fn glass() -> Self {
        Self::new(1.5, 0.0)
    }

    /// Create frosted glass.
    pub fn frosted_glass(roughness: f64) -> Self {
        Self::new(1.5, roughness)
    }

    /// Create water.
    pub fn water() -> Self {
        Self::new(1.33, 0.001)
    }

    /// Create diamond.
    pub fn diamond() -> Self {
        Self::new(2.42, 0.001)
    }

    /// Set IOR (returns new instance for chaining).
    pub fn with_ior(mut self, ior: f64) -> Self {
        self.ior = ior.max(1.0);
        self
    }

    /// Set roughness (returns new instance for chaining).
    pub fn with_roughness(mut self, roughness: f64) -> Self {
        self.roughness = roughness.clamp(0.001, 1.0);
        self.alpha = self.roughness * self.roughness;
        self
    }

    /// Compute Fresnel reflectance at normal incidence.
    #[allow(dead_code)]
    fn f0(&self) -> f64 {
        let r = (self.ior - 1.0) / (self.ior + 1.0);
        r * r
    }

    /// Compute transmission with gradient.
    fn evaluate_transmission_with_gradient(&self, ctx: &BSDFContext) -> (f64, f64) {
        let cos_theta_i = ctx.cos_theta_i();

        // Fresnel at interface
        let (fresnel, df_dn) = fresnel_schlick_gradient(cos_theta_i, self.ior);

        // Transmission = 1 - F (energy conservation)
        let transmission = (1.0 - fresnel).max(0.0);
        let dt_dn = -df_dn;

        (transmission, dt_dn)
    }
}

impl BSDF for DifferentiableDielectric {
    fn evaluate(&self, ctx: &BSDFContext) -> BSDFResponse {
        let cos_theta_i = ctx.cos_theta_i();

        // Fresnel at this angle
        let (fresnel, _) = fresnel_schlick_gradient(cos_theta_i, self.ior);

        // Simple energy split
        let reflectance = fresnel;
        let transmittance = 1.0 - fresnel;

        BSDFResponse::new(reflectance, transmittance, 0.0)
    }

    fn sample(&self, ctx: &BSDFContext, u1: f64, _u2: f64) -> BSDFSample {
        let cos_theta_i = ctx.cos_theta_i();
        let (fresnel, _) = fresnel_schlick_gradient(cos_theta_i, self.ior);

        let is_reflection = u1 < fresnel;
        let pdf = if is_reflection {
            fresnel
        } else {
            1.0 - fresnel
        };

        // Perfect specular directions
        let wo = if is_reflection {
            // Reflect
            let d = ctx.wi * -1.0;
            d + ctx.normal * (2.0 * cos_theta_i)
        } else {
            // Refract (simplified)
            let eta = 1.0 / self.ior;
            let cos_t_sq = 1.0 - eta * eta * (1.0 - cos_theta_i * cos_theta_i);
            if cos_t_sq < 0.0 {
                // Total internal reflection
                let d = ctx.wi * -1.0;
                d + ctx.normal * (2.0 * cos_theta_i)
            } else {
                let cos_t = cos_t_sq.sqrt();
                ctx.wi * (-eta) + ctx.normal * (eta * cos_theta_i - cos_t)
            }
        };

        let value = if is_reflection {
            BSDFResponse::pure_reflection(fresnel)
        } else {
            BSDFResponse::pure_transmission(1.0 - fresnel)
        };

        BSDFSample {
            wo,
            value,
            pdf,
            is_delta: self.roughness < 0.01,
        }
    }

    fn pdf(&self, ctx: &BSDFContext) -> f64 {
        let cos_theta_i = ctx.cos_theta_i();
        let (fresnel, _) = fresnel_schlick_gradient(cos_theta_i, self.ior);
        fresnel.max(1.0 - fresnel)
    }

    fn validate_energy(&self, ctx: &BSDFContext) -> EnergyValidation {
        let response = self.evaluate(ctx);
        let error = (response.total_energy() - 1.0).abs();
        if error < 1e-6 {
            EnergyValidation::pass(error)
        } else {
            EnergyValidation::fail(error, "R + T + A != 1")
        }
    }

    fn name(&self) -> &str {
        "DifferentiableDielectric"
    }

    fn is_delta(&self) -> bool {
        self.roughness < 0.01
    }
}

impl DifferentiableBSDF for DifferentiableDielectric {
    fn eval_with_gradients(&self, ctx: &BSDFContext) -> DifferentiableResponse {
        let cos_theta_i = ctx.cos_theta_i();

        // Forward pass
        let (fresnel, df_dn) = fresnel_schlick_gradient(cos_theta_i, self.ior);

        let response = BSDFResponse::new(fresnel, 1.0 - fresnel, 0.0);

        // Gradients
        // ∂R/∂n = ∂F/∂n
        // ∂T/∂n = -∂F/∂n (energy conservation)
        // Note: This simple Fresnel model doesn't use roughness in evaluate(),
        // so d_roughness = 0. A full microfacet model would have non-zero roughness gradient.

        let mut gradients = ParameterGradients::zero();
        gradients.d_ior = df_dn;
        gradients.d_reflectance = df_dn;
        gradients.d_transmittance = -df_dn;
        // d_roughness = 0 since this simple model doesn't use roughness

        DifferentiableResponse::new(response, gradients)
    }

    fn parameter_bounds(&self) -> ParameterBounds {
        ParameterBounds::dielectric()
    }

    fn param_count(&self) -> usize {
        2 // ior, roughness
    }

    fn params_to_vec(&self) -> Vec<f64> {
        vec![self.ior, 0.0, self.roughness, 0.0, 0.0, 0.0]
    }

    fn from_param_vec(params: &[f64]) -> Self {
        Self::new(
            params.get(0).copied().unwrap_or(1.5),
            params.get(2).copied().unwrap_or(0.0),
        )
    }
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::super::super::unified_bsdf::Vector3;
    use super::*;

    fn create_ctx(cos_theta: f64) -> BSDFContext {
        let sin_theta = (1.0 - cos_theta * cos_theta).sqrt();
        BSDFContext {
            wi: Vector3::new(sin_theta, 0.0, cos_theta),
            wo: Vector3::new(-sin_theta, 0.0, cos_theta),
            normal: Vector3::new(0.0, 0.0, 1.0),
            tangent: Vector3::new(1.0, 0.0, 0.0),
            bitangent: Vector3::new(0.0, 1.0, 0.0),
            wavelength: 550.0,
            wavelengths: None,
        }
    }

    #[test]
    fn test_differentiable_dielectric_new() {
        let glass = DifferentiableDielectric::new(1.5, 0.1);
        assert!((glass.ior - 1.5).abs() < 1e-10);
        assert!((glass.roughness - 0.1).abs() < 1e-10);
    }

    #[test]
    fn test_differentiable_dielectric_presets() {
        let glass = DifferentiableDielectric::glass();
        assert!((glass.ior - 1.5).abs() < 1e-10);

        let water = DifferentiableDielectric::water();
        assert!((water.ior - 1.33).abs() < 1e-10);

        let diamond = DifferentiableDielectric::diamond();
        assert!((diamond.ior - 2.42).abs() < 1e-10);
    }

    #[test]
    fn test_evaluate_energy_conservation() {
        let glass = DifferentiableDielectric::new(1.5, 0.1);
        let ctx = create_ctx(0.8);

        let response = glass.evaluate(&ctx);
        let total = response.reflectance + response.transmittance + response.absorption;

        assert!((total - 1.0).abs() < 1e-6);
    }

    #[test]
    fn test_eval_with_gradients() {
        let glass = DifferentiableDielectric::new(1.5, 0.1);
        let ctx = create_ctx(0.8);

        let result = glass.eval_with_gradients(&ctx);

        assert!(result.response.reflectance >= 0.0);
        assert!(result.response.transmittance >= 0.0);
        assert!(result.energy_conserved);

        // Gradient should be non-zero for non-trivial IOR
        assert!(result.gradients.d_ior.abs() > 0.0);
    }

    #[test]
    fn test_gradient_vs_numerical() {
        let glass = DifferentiableDielectric::new(1.5, 0.1);
        let ctx = create_ctx(0.8);

        let verification = glass.verify_gradients(&ctx, 1e-5);

        assert!(verification.passed, "Max error: {}", verification.max_error);
    }

    #[test]
    fn test_gradient_at_normal_incidence() {
        let glass = DifferentiableDielectric::new(1.5, 0.0);
        let ctx = create_ctx(1.0);

        let result = glass.eval_with_gradients(&ctx);

        // At normal incidence, F₀ = ((n-1)/(n+1))²
        let expected_f0 = ((1.5_f64 - 1.0) / (1.5_f64 + 1.0)).powi(2);
        assert!((result.response.reflectance - expected_f0).abs() < 1e-4);
    }

    #[test]
    fn test_gradient_at_grazing_angle() {
        let glass = DifferentiableDielectric::new(1.5, 0.0);
        let ctx = create_ctx(0.01); // Nearly grazing

        let result = glass.eval_with_gradients(&ctx);

        // At grazing angle, reflectance approaches 1
        assert!(result.response.reflectance > 0.9);
    }

    #[test]
    fn test_params_to_vec_from_vec() {
        let glass = DifferentiableDielectric::new(1.52, 0.15);
        let params = glass.params_to_vec();

        let restored = DifferentiableDielectric::from_param_vec(&params);

        assert!((restored.ior - 1.52).abs() < 1e-10);
        assert!((restored.roughness - 0.15).abs() < 1e-10);
    }

    #[test]
    fn test_parameter_bounds() {
        let glass = DifferentiableDielectric::new(1.5, 0.1);
        let bounds = glass.parameter_bounds();

        assert_eq!(bounds.ior.0, 1.0);
        assert!(bounds.ior.1 <= 4.0);
        assert_eq!(bounds.roughness.0, 0.001);
        assert_eq!(bounds.roughness.1, 1.0);
    }

    #[test]
    fn test_higher_ior_higher_reflectance() {
        let ctx = create_ctx(0.8);

        let low_ior = DifferentiableDielectric::new(1.3, 0.0);
        let high_ior = DifferentiableDielectric::new(2.0, 0.0);

        let r_low = low_ior.evaluate(&ctx).reflectance;
        let r_high = high_ior.evaluate(&ctx).reflectance;

        assert!(r_high > r_low);
    }

    #[test]
    fn test_gradient_direction() {
        let ctx = create_ctx(0.8);
        let glass = DifferentiableDielectric::new(1.5, 0.0);

        let result = glass.eval_with_gradients(&ctx);

        // Increasing IOR should increase reflectance (positive gradient)
        assert!(result.gradients.d_ior > 0.0);
    }
}
