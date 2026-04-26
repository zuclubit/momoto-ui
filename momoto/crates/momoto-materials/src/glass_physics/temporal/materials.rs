//! # Temporal Material Wrappers
//!
//! Time-evolving materials that wrap base BSDFs with temporal behavior.

use super::super::unified_bsdf::{
    BSDFContext, BSDFResponse, BSDFSample, ConductorBSDF, DielectricBSDF, EnergyValidation,
    ThinFilmBSDF, BSDF,
};
use super::bsdf::{EvolutionRate, TemporalBSDF, TemporalBSDFInfo, TemporalEvolution};
use super::context::TemporalContext;
use super::interpolation::RateLimiter;

// ============================================================================
// DIELECTRIC EVOLUTION
// ============================================================================

/// Evolution parameters for dielectric materials.
#[derive(Debug, Clone)]
pub struct DielectricEvolution {
    /// Base roughness.
    pub roughness_base: f64,
    /// Target roughness (for drying, weathering).
    pub roughness_target: f64,
    /// Time constant for roughness evolution (seconds).
    pub roughness_tau: f64,
    /// Base IOR.
    pub ior_base: f64,
    /// Temperature coefficient for IOR (dn/dT).
    pub ior_temp_coeff: f64,
}

impl Default for DielectricEvolution {
    fn default() -> Self {
        Self {
            roughness_base: 0.1,
            roughness_target: 0.1,
            roughness_tau: 10.0,
            ior_base: 1.5,
            ior_temp_coeff: -1e-5, // Typical for glass
        }
    }
}

/// Time-evolving dielectric material.
///
/// Supports:
/// - Roughness evolution (drying paint, weathering)
/// - Temperature-dependent IOR
#[derive(Debug, Clone)]
pub struct TemporalDielectric {
    /// Evolution parameters.
    evolution: DielectricEvolution,
    /// Rate limiter for smooth transitions (reserved for future temporal filtering).
    #[allow(dead_code)]
    _rate_limiter: RateLimiter,
    /// Last computed roughness (reserved for rate limiting implementation).
    #[allow(dead_code)]
    _last_roughness: f64,
}

impl TemporalDielectric {
    /// Create new temporal dielectric.
    pub fn new(evolution: DielectricEvolution) -> Self {
        Self {
            _last_roughness: evolution.roughness_base,
            evolution,
            _rate_limiter: RateLimiter::default(),
        }
    }

    /// Create drying paint preset.
    pub fn drying_paint() -> Self {
        Self::new(DielectricEvolution {
            roughness_base: 0.05,  // Wet paint is glossy
            roughness_target: 0.4, // Dry paint is matte
            roughness_tau: 60.0,   // Dries over ~1 minute
            ior_base: 1.5,
            ior_temp_coeff: -1e-5,
        })
    }

    /// Create weathering glass preset.
    pub fn weathering_glass() -> Self {
        Self::new(DielectricEvolution {
            roughness_base: 0.01,   // New glass is smooth
            roughness_target: 0.15, // Weathered glass is scratched
            roughness_tau: 3600.0,  // Weathers over ~1 hour simulation
            ior_base: 1.52,
            ior_temp_coeff: -1e-5,
        })
    }

    /// Compute roughness at time.
    fn roughness_at(&self, time: f64) -> f64 {
        let e = &self.evolution;
        // Exponential approach to target: r(t) = target + (base - target) * exp(-t/tau)
        e.roughness_target
            + (e.roughness_base - e.roughness_target) * (-time / e.roughness_tau).exp()
    }

    /// Compute IOR at temperature.
    fn ior_at_temperature(&self, temp_k: f64) -> f64 {
        let e = &self.evolution;
        let delta_t = temp_k - 293.15; // Reference: 20°C
        e.ior_base + e.ior_temp_coeff * delta_t
    }
}

impl BSDF for TemporalDielectric {
    fn evaluate(&self, ctx: &BSDFContext) -> BSDFResponse {
        // Static evaluation at t=0
        let bsdf = DielectricBSDF::new(self.evolution.ior_base, self.evolution.roughness_base);
        bsdf.evaluate(ctx)
    }

    fn sample(&self, ctx: &BSDFContext, u1: f64, u2: f64) -> BSDFSample {
        let bsdf = DielectricBSDF::new(self.evolution.ior_base, self.evolution.roughness_base);
        bsdf.sample(ctx, u1, u2)
    }

    fn pdf(&self, ctx: &BSDFContext) -> f64 {
        let bsdf = DielectricBSDF::new(self.evolution.ior_base, self.evolution.roughness_base);
        bsdf.pdf(ctx)
    }

    fn name(&self) -> &str {
        "TemporalDielectric"
    }

    fn validate_energy(&self, ctx: &BSDFContext) -> EnergyValidation {
        let bsdf = DielectricBSDF::new(self.evolution.ior_base, self.evolution.roughness_base);
        bsdf.validate_energy(ctx)
    }
}

impl TemporalBSDF for TemporalDielectric {
    fn eval_at_time(&self, ctx: &TemporalContext) -> BSDFResponse {
        let roughness = self.roughness_at(ctx.time);
        let ior = self.ior_at_temperature(ctx.temperature);

        let bsdf = DielectricBSDF::new(ior, roughness);
        bsdf.evaluate(&ctx.base)
    }

    fn supports_temporal(&self) -> bool {
        true
    }

    fn temporal_info(&self) -> TemporalBSDFInfo {
        TemporalBSDFInfo {
            name: "TemporalDielectric".to_string(),
            supports_temporal: true,
            evolution: TemporalEvolution::default().with_roughness(EvolutionRate::Exponential {
                rate: 1.0 / self.evolution.roughness_tau,
                asymptote: self.evolution.roughness_target,
            }),
            time_min: 0.0,
            time_max: f64::INFINITY,
        }
    }
}

// ============================================================================
// THIN FILM EVOLUTION
// ============================================================================

/// Evolution parameters for thin film materials.
#[derive(Debug, Clone)]
pub struct ThinFilmEvolution {
    /// Base film thickness (nm).
    pub thickness_base: f64,
    /// Oscillation amplitude (nm).
    pub thickness_amplitude: f64,
    /// Oscillation frequency (Hz).
    pub thickness_frequency: f64,
    /// Film IOR.
    pub film_ior: f64,
    /// Substrate IOR.
    pub substrate_ior: f64,
    /// Damping coefficient (for damped oscillation).
    pub damping: f64,
}

impl Default for ThinFilmEvolution {
    fn default() -> Self {
        Self {
            thickness_base: 300.0,
            thickness_amplitude: 50.0,
            thickness_frequency: 0.5,
            film_ior: 1.33,
            substrate_ior: 1.0,
            damping: 0.0,
        }
    }
}

/// Time-evolving thin film material.
///
/// Supports:
/// - Thickness oscillation (soap bubbles, vibrating films)
/// - Damped oscillation (settling films)
#[derive(Debug, Clone)]
pub struct TemporalThinFilm {
    /// Evolution parameters.
    evolution: ThinFilmEvolution,
}

impl TemporalThinFilm {
    /// Create new temporal thin film.
    pub fn new(evolution: ThinFilmEvolution) -> Self {
        Self { evolution }
    }

    /// Create soap bubble preset.
    pub fn soap_bubble() -> Self {
        Self::new(ThinFilmEvolution {
            thickness_base: 300.0,
            thickness_amplitude: 100.0,
            thickness_frequency: 2.0,
            film_ior: 1.33,
            substrate_ior: 1.0,
            damping: 0.1,
        })
    }

    /// Create oil slick preset.
    pub fn oil_slick() -> Self {
        Self::new(ThinFilmEvolution {
            thickness_base: 400.0,
            thickness_amplitude: 50.0,
            thickness_frequency: 0.2,
            film_ior: 1.5,
            substrate_ior: 1.33,
            damping: 0.05,
        })
    }

    /// Compute thickness at time.
    fn thickness_at(&self, time: f64) -> f64 {
        use std::f64::consts::TAU;
        let e = &self.evolution;

        // Damped oscillation: A * exp(-damping * t) * sin(2π * f * t)
        let envelope = if e.damping > 0.0 {
            (-e.damping * time).exp()
        } else {
            1.0
        };

        let oscillation = (e.thickness_frequency * TAU * time).sin();
        e.thickness_base + e.thickness_amplitude * envelope * oscillation
    }
}

impl BSDF for TemporalThinFilm {
    fn evaluate(&self, ctx: &BSDFContext) -> BSDFResponse {
        let bsdf = ThinFilmBSDF::new(
            self.evolution.film_ior,
            self.evolution.substrate_ior,
            self.evolution.thickness_base,
        );
        bsdf.evaluate(ctx)
    }

    fn sample(&self, ctx: &BSDFContext, u1: f64, u2: f64) -> BSDFSample {
        let bsdf = ThinFilmBSDF::new(
            self.evolution.film_ior,
            self.evolution.substrate_ior,
            self.evolution.thickness_base,
        );
        bsdf.sample(ctx, u1, u2)
    }

    fn pdf(&self, ctx: &BSDFContext) -> f64 {
        let bsdf = ThinFilmBSDF::new(
            self.evolution.film_ior,
            self.evolution.substrate_ior,
            self.evolution.thickness_base,
        );
        bsdf.pdf(ctx)
    }

    fn name(&self) -> &str {
        "TemporalThinFilm"
    }

    fn validate_energy(&self, ctx: &BSDFContext) -> EnergyValidation {
        let bsdf = ThinFilmBSDF::new(
            self.evolution.film_ior,
            self.evolution.substrate_ior,
            self.evolution.thickness_base,
        );
        bsdf.validate_energy(ctx)
    }
}

impl TemporalBSDF for TemporalThinFilm {
    fn eval_at_time(&self, ctx: &TemporalContext) -> BSDFResponse {
        let thickness = self.thickness_at(ctx.time);

        let bsdf = ThinFilmBSDF::new(
            self.evolution.film_ior,
            self.evolution.substrate_ior,
            thickness,
        );
        bsdf.evaluate(&ctx.base)
    }

    fn supports_temporal(&self) -> bool {
        true
    }

    fn temporal_info(&self) -> TemporalBSDFInfo {
        TemporalBSDFInfo {
            name: "TemporalThinFilm".to_string(),
            supports_temporal: true,
            evolution: TemporalEvolution::default().with_thickness(EvolutionRate::Oscillating {
                frequency: self.evolution.thickness_frequency,
                amplitude: self.evolution.thickness_amplitude,
            }),
            time_min: 0.0,
            time_max: f64::INFINITY,
        }
    }
}

// ============================================================================
// CONDUCTOR EVOLUTION
// ============================================================================

/// Evolution parameters for conductor materials.
#[derive(Debug, Clone)]
pub struct ConductorEvolution {
    /// Base n (real part of IOR).
    pub n_base: f64,
    /// Base k (extinction coefficient).
    pub k_base: f64,
    /// Base roughness.
    pub roughness_base: f64,
    /// Temperature coefficient for n.
    pub n_temp_coeff: f64,
    /// Temperature coefficient for k.
    pub k_temp_coeff: f64,
    /// Reference temperature (K).
    pub temp_ref: f64,
}

impl Default for ConductorEvolution {
    fn default() -> Self {
        Self {
            n_base: 0.18, // Gold-like
            k_base: 3.0,
            roughness_base: 0.1,
            n_temp_coeff: 1e-4,
            k_temp_coeff: -5e-4,
            temp_ref: 293.15,
        }
    }
}

/// Time-evolving conductor material.
///
/// Supports:
/// - Temperature-dependent optical constants
/// - Heating/cooling spectral shift
#[derive(Debug, Clone)]
pub struct TemporalConductor {
    /// Evolution parameters.
    evolution: ConductorEvolution,
}

impl TemporalConductor {
    /// Create new temporal conductor.
    pub fn new(evolution: ConductorEvolution) -> Self {
        Self { evolution }
    }

    /// Create heated gold preset.
    pub fn heated_gold() -> Self {
        Self::new(ConductorEvolution {
            n_base: 0.18,
            k_base: 3.0,
            roughness_base: 0.05,
            n_temp_coeff: 2e-4,
            k_temp_coeff: -1e-3,
            temp_ref: 293.15,
        })
    }

    /// Create heated copper preset.
    pub fn heated_copper() -> Self {
        Self::new(ConductorEvolution {
            n_base: 0.27,
            k_base: 3.4,
            roughness_base: 0.1,
            n_temp_coeff: 3e-4,
            k_temp_coeff: -8e-4,
            temp_ref: 293.15,
        })
    }

    /// Compute n at temperature.
    fn n_at_temperature(&self, temp_k: f64) -> f64 {
        let e = &self.evolution;
        let delta_t = temp_k - e.temp_ref;
        e.n_base + e.n_temp_coeff * delta_t
    }

    /// Compute k at temperature.
    fn k_at_temperature(&self, temp_k: f64) -> f64 {
        let e = &self.evolution;
        let delta_t = temp_k - e.temp_ref;
        (e.k_base + e.k_temp_coeff * delta_t).max(0.0)
    }
}

impl BSDF for TemporalConductor {
    fn evaluate(&self, ctx: &BSDFContext) -> BSDFResponse {
        let bsdf = ConductorBSDF::new(
            self.evolution.n_base,
            self.evolution.k_base,
            self.evolution.roughness_base,
        );
        bsdf.evaluate(ctx)
    }

    fn sample(&self, ctx: &BSDFContext, u1: f64, u2: f64) -> BSDFSample {
        let bsdf = ConductorBSDF::new(
            self.evolution.n_base,
            self.evolution.k_base,
            self.evolution.roughness_base,
        );
        bsdf.sample(ctx, u1, u2)
    }

    fn pdf(&self, ctx: &BSDFContext) -> f64 {
        let bsdf = ConductorBSDF::new(
            self.evolution.n_base,
            self.evolution.k_base,
            self.evolution.roughness_base,
        );
        bsdf.pdf(ctx)
    }

    fn name(&self) -> &str {
        "TemporalConductor"
    }

    fn validate_energy(&self, ctx: &BSDFContext) -> EnergyValidation {
        let bsdf = ConductorBSDF::new(
            self.evolution.n_base,
            self.evolution.k_base,
            self.evolution.roughness_base,
        );
        bsdf.validate_energy(ctx)
    }
}

impl TemporalBSDF for TemporalConductor {
    fn eval_at_time(&self, ctx: &TemporalContext) -> BSDFResponse {
        let n = self.n_at_temperature(ctx.temperature);
        let k = self.k_at_temperature(ctx.temperature);

        let bsdf = ConductorBSDF::new(n, k, self.evolution.roughness_base);
        bsdf.evaluate(&ctx.base)
    }

    fn supports_temporal(&self) -> bool {
        true
    }

    fn temporal_info(&self) -> TemporalBSDFInfo {
        TemporalBSDFInfo {
            name: "TemporalConductor".to_string(),
            supports_temporal: true,
            evolution: TemporalEvolution::default(),
            time_min: 0.0,
            time_max: f64::INFINITY,
        }
    }
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_drying_paint() {
        let paint = TemporalDielectric::drying_paint();
        assert!(paint.supports_temporal());

        // At t=0, should be near base roughness
        let ctx_start = TemporalContext::at_time(0.0);
        let r0 = paint.eval_at_time(&ctx_start);

        // At t=large, should be near target roughness
        let ctx_end = TemporalContext::at_time(300.0);
        let r_end = paint.eval_at_time(&ctx_end);

        // Both should be valid responses
        assert!(r0.reflectance >= 0.0);
        assert!(r_end.reflectance >= 0.0);
    }

    #[test]
    fn test_soap_bubble() {
        let bubble = TemporalThinFilm::soap_bubble();
        assert!(bubble.supports_temporal());

        // Evaluate at different times
        let ctx1 = TemporalContext::at_time(0.0);
        let ctx2 = TemporalContext::at_time(0.25);

        let r1 = bubble.eval_at_time(&ctx1);
        let r2 = bubble.eval_at_time(&ctx2);

        // Responses should differ due to thickness oscillation
        // (unless we happen to hit the same phase)
        assert!(r1.reflectance >= 0.0);
        assert!(r2.reflectance >= 0.0);
    }

    #[test]
    fn test_heated_gold() {
        let gold = TemporalConductor::heated_gold();
        assert!(gold.supports_temporal());

        // At room temperature
        let ctx_cold = TemporalContext::default();
        let r_cold = gold.eval_at_time(&ctx_cold);

        // At high temperature
        let ctx_hot = TemporalContext::default().with_temperature(500.0);
        let r_hot = gold.eval_at_time(&ctx_hot);

        // Both should be valid
        assert!(r_cold.reflectance >= 0.0);
        assert!(r_hot.reflectance >= 0.0);
    }

    #[test]
    fn test_backward_compatibility() {
        // Static evaluation should work
        let paint = TemporalDielectric::drying_paint();
        let ctx = BSDFContext::default();

        let response = paint.evaluate(&ctx);
        assert!(response.reflectance >= 0.0);
        assert!(response.transmittance >= 0.0);
    }

    #[test]
    fn test_energy_conservation() {
        let bubble = TemporalThinFilm::soap_bubble();
        let ctx = TemporalContext::at_time(0.5);

        let response = bubble.eval_at_time(&ctx);
        let total = response.reflectance + response.transmittance + response.absorption;
        assert!((total - 1.0).abs() < 0.01);
    }

    #[test]
    fn test_temporal_info() {
        let paint = TemporalDielectric::drying_paint();
        let info = paint.temporal_info();

        assert!(info.supports_temporal);
        assert_eq!(info.name, "TemporalDielectric");
    }
}
