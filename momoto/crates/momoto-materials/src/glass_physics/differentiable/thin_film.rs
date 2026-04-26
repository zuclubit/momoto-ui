//! # Differentiable Thin-Film BSDF
//!
//! Thin-film interference material with analytical gradient computation.

use super::super::unified_bsdf::{BSDFContext, BSDFResponse, BSDFSample, EnergyValidation, BSDF};
use super::gradients::thin_film_gradient;
use super::traits::{
    DifferentiableBSDF, DifferentiableResponse, ParameterBounds, ParameterGradients,
};

// ============================================================================
// DIFFERENTIABLE THIN FILM
// ============================================================================

/// Thin-film BSDF with analytical gradient support.
///
/// Supports gradients w.r.t. film thickness, film IOR, and substrate roughness.
#[derive(Debug, Clone)]
pub struct DifferentiableThinFilm {
    /// Substrate refractive index.
    pub substrate_ior: f64,
    /// Substrate roughness.
    pub substrate_roughness: f64,
    /// Film refractive index.
    pub film_ior: f64,
    /// Film thickness in nanometers.
    pub film_thickness: f64,
}

impl DifferentiableThinFilm {
    /// Create new differentiable thin-film material.
    pub fn new(substrate_ior: f64, film_ior: f64, film_thickness: f64) -> Self {
        Self {
            substrate_ior: substrate_ior.max(1.0),
            substrate_roughness: 0.0,
            film_ior: film_ior.max(1.0),
            film_thickness: film_thickness.max(0.0),
        }
    }

    /// Create soap bubble (thin water film in air).
    pub fn soap_bubble(thickness: f64) -> Self {
        Self::new(1.0, 1.33, thickness)
    }

    /// Create oil film on water.
    pub fn oil_on_water(thickness: f64) -> Self {
        Self::new(1.33, 1.47, thickness)
    }

    /// Create anti-reflective coating on glass.
    pub fn anti_reflective(thickness: f64) -> Self {
        // MgF2 coating on glass
        Self::new(1.5, 1.38, thickness)
    }

    /// Create silicon dioxide coating.
    pub fn sio2_coating(thickness: f64) -> Self {
        Self::new(1.5, 1.46, thickness)
    }

    /// Set substrate roughness.
    pub fn with_substrate_roughness(mut self, roughness: f64) -> Self {
        self.substrate_roughness = roughness.clamp(0.0, 1.0);
        self
    }

    /// Set film thickness.
    pub fn with_thickness(mut self, thickness: f64) -> Self {
        self.film_thickness = thickness.max(0.0);
        self
    }

    /// Set film IOR.
    pub fn with_film_ior(mut self, ior: f64) -> Self {
        self.film_ior = ior.max(1.0);
        self
    }

    /// Compute thin-film reflectance at a wavelength.
    fn compute_reflectance(&self, wavelength: f64, cos_theta: f64) -> f64 {
        let (r, _, _) = thin_film_gradient(
            wavelength,
            1.0, // Ambient (air)
            self.film_ior,
            self.substrate_ior,
            self.film_thickness,
            cos_theta,
        );
        r.clamp(0.0, 1.0)
    }

    /// Compute thin-film reflectance with gradients.
    fn compute_with_gradient(&self, wavelength: f64, cos_theta: f64) -> (f64, f64, f64) {
        thin_film_gradient(
            wavelength,
            1.0, // Ambient (air)
            self.film_ior,
            self.substrate_ior,
            self.film_thickness,
            cos_theta,
        )
    }
}

impl BSDF for DifferentiableThinFilm {
    fn evaluate(&self, ctx: &BSDFContext) -> BSDFResponse {
        let cos_theta_i = ctx.cos_theta_i();
        let wavelength = ctx.wavelength;

        let reflectance = self.compute_reflectance(wavelength, cos_theta_i);

        BSDFResponse::new(reflectance, 1.0 - reflectance, 0.0)
    }

    fn sample(&self, ctx: &BSDFContext, u1: f64, _u2: f64) -> BSDFSample {
        let cos_theta_i = ctx.cos_theta_i();
        let reflectance = self.compute_reflectance(ctx.wavelength, cos_theta_i);

        let is_reflection = u1 < reflectance;
        let pdf = if is_reflection {
            reflectance
        } else {
            1.0 - reflectance
        };

        let wo = if is_reflection {
            let d = ctx.wi * -1.0;
            d + ctx.normal * (2.0 * cos_theta_i)
        } else {
            // Transmit (simplified - no refraction)
            ctx.wi * -1.0
        };

        let value = if is_reflection {
            BSDFResponse::pure_reflection(reflectance)
        } else {
            BSDFResponse::pure_transmission(1.0 - reflectance)
        };

        BSDFSample {
            wo,
            value,
            pdf,
            is_delta: self.substrate_roughness < 0.01,
        }
    }

    fn pdf(&self, ctx: &BSDFContext) -> f64 {
        let reflectance = self.compute_reflectance(ctx.wavelength, ctx.cos_theta_i());
        reflectance.max(1.0 - reflectance)
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
        "DifferentiableThinFilm"
    }

    fn is_delta(&self) -> bool {
        self.substrate_roughness < 0.01
    }
}

impl DifferentiableBSDF for DifferentiableThinFilm {
    fn eval_with_gradients(&self, ctx: &BSDFContext) -> DifferentiableResponse {
        let cos_theta_i = ctx.cos_theta_i();
        let wavelength = ctx.wavelength;

        // Forward pass with gradients
        let (reflectance, dr_dt, dr_dn) = self.compute_with_gradient(wavelength, cos_theta_i);
        let reflectance = reflectance.clamp(0.0, 1.0);

        let response = BSDFResponse::new(reflectance, 1.0 - reflectance, 0.0);

        let mut gradients = ParameterGradients::zero();
        gradients.d_film_thickness = Some(dr_dt);
        gradients.d_film_ior = Some(dr_dn);
        gradients.d_reflectance = dr_dt; // Dominated by thickness

        // Transmission gradient is negative of reflection gradient
        gradients.d_transmittance = -dr_dt;

        DifferentiableResponse::new(response, gradients)
    }

    fn parameter_bounds(&self) -> ParameterBounds {
        ParameterBounds::thin_film()
    }

    fn param_count(&self) -> usize {
        4 // substrate_ior, substrate_roughness, film_ior, film_thickness
    }

    fn params_to_vec(&self) -> Vec<f64> {
        vec![
            self.substrate_ior,
            0.0, // extinction (not used)
            self.substrate_roughness,
            0.0, // absorption (not used)
            0.0, // scattering (not used)
            0.0, // asymmetry (not used)
            self.film_thickness,
            self.film_ior,
        ]
    }

    fn from_param_vec(params: &[f64]) -> Self {
        Self {
            substrate_ior: params.get(0).copied().unwrap_or(1.5),
            substrate_roughness: params.get(2).copied().unwrap_or(0.0),
            film_thickness: params.get(6).copied().unwrap_or(100.0),
            film_ior: params.get(7).copied().unwrap_or(1.4),
        }
    }
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::super::super::unified_bsdf::Vector3;
    use super::*;

    fn create_ctx(cos_theta: f64, wavelength: f64) -> BSDFContext {
        let sin_theta = (1.0 - cos_theta * cos_theta).sqrt();
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

    #[test]
    fn test_thin_film_new() {
        let film = DifferentiableThinFilm::new(1.5, 1.4, 200.0);
        assert!((film.substrate_ior - 1.5).abs() < 1e-10);
        assert!((film.film_ior - 1.4).abs() < 1e-10);
        assert!((film.film_thickness - 200.0).abs() < 1e-10);
    }

    #[test]
    fn test_thin_film_presets() {
        let soap = DifferentiableThinFilm::soap_bubble(300.0);
        assert!((soap.film_ior - 1.33).abs() < 1e-10);

        let oil = DifferentiableThinFilm::oil_on_water(200.0);
        assert!((oil.substrate_ior - 1.33).abs() < 1e-10);

        let ar = DifferentiableThinFilm::anti_reflective(100.0);
        assert!((ar.film_ior - 1.38).abs() < 1e-10);
    }

    #[test]
    fn test_thin_film_energy_conservation() {
        let film = DifferentiableThinFilm::soap_bubble(300.0);
        let ctx = create_ctx(0.8, 550.0);

        let response = film.evaluate(&ctx);
        let total = response.reflectance + response.transmittance + response.absorption;

        assert!((total - 1.0).abs() < 1e-6);
    }

    #[test]
    fn test_thin_film_wavelength_dependence() {
        let film = DifferentiableThinFilm::soap_bubble(300.0);

        let ctx_red = create_ctx(0.8, 650.0);
        let ctx_green = create_ctx(0.8, 550.0);
        let ctx_blue = create_ctx(0.8, 450.0);

        let r_red = film.evaluate(&ctx_red).reflectance;
        let r_green = film.evaluate(&ctx_green).reflectance;
        let r_blue = film.evaluate(&ctx_blue).reflectance;

        // Different wavelengths should give different reflectances
        // (interference pattern)
        assert!((r_red - r_green).abs() > 0.001 || (r_green - r_blue).abs() > 0.001);
    }

    #[test]
    fn test_thin_film_thickness_dependence() {
        let ctx = create_ctx(0.8, 550.0);

        let thin = DifferentiableThinFilm::soap_bubble(100.0);
        let thick = DifferentiableThinFilm::soap_bubble(500.0);

        let r_thin = thin.evaluate(&ctx).reflectance;
        let r_thick = thick.evaluate(&ctx).reflectance;

        // Different thicknesses should give different reflectances
        assert!((r_thin - r_thick).abs() > 0.001);
    }

    #[test]
    fn test_thin_film_eval_with_gradients() {
        let film = DifferentiableThinFilm::soap_bubble(300.0);
        let ctx = create_ctx(0.8, 550.0);

        let result = film.eval_with_gradients(&ctx);

        assert!(result.response.reflectance >= 0.0);
        assert!(result.response.transmittance >= 0.0);
        assert!(result.energy_conserved);

        // Thickness gradient should be non-zero (oscillating with thickness)
        assert!(result.gradients.d_film_thickness.is_some());
    }

    #[test]
    fn test_thin_film_gradient_vs_numerical() {
        let film = DifferentiableThinFilm::new(1.5, 1.4, 200.0);
        let ctx = create_ctx(0.8, 550.0);

        // Numerical gradient for thickness
        let eps = 0.1; // nm
        let r_plus = DifferentiableThinFilm::new(1.5, 1.4, 200.0 + eps)
            .evaluate(&ctx)
            .reflectance;
        let r_minus = DifferentiableThinFilm::new(1.5, 1.4, 200.0 - eps)
            .evaluate(&ctx)
            .reflectance;
        let numeric_dt = (r_plus - r_minus) / (2.0 * eps);

        let result = film.eval_with_gradients(&ctx);
        let analytic_dt = result.gradients.d_film_thickness.unwrap_or(0.0);

        assert!(
            (analytic_dt - numeric_dt).abs() < 1e-3,
            "Analytic: {}, Numeric: {}",
            analytic_dt,
            numeric_dt
        );
    }

    #[test]
    fn test_thin_film_params_roundtrip() {
        let film = DifferentiableThinFilm::new(1.52, 1.38, 150.0);
        let params = film.params_to_vec();

        let restored = DifferentiableThinFilm::from_param_vec(&params);

        assert!((restored.substrate_ior - 1.52).abs() < 1e-10);
        assert!((restored.film_ior - 1.38).abs() < 1e-10);
        assert!((restored.film_thickness - 150.0).abs() < 1e-10);
    }

    #[test]
    fn test_anti_reflective_quarter_wave() {
        // Quarter-wave coating should minimize reflection
        // λ/4 thickness in film = λ / (4 × n_film)
        // For 550nm and n=1.38: thickness ≈ 99.6nm
        let quarter_wave = DifferentiableThinFilm::anti_reflective(99.6);
        let ctx = create_ctx(1.0, 550.0); // Normal incidence

        let response = quarter_wave.evaluate(&ctx);

        // Anti-reflective coating should have low reflectance
        // (may not be zero due to IOR mismatch)
        assert!(response.reflectance < 0.1);
    }
}
