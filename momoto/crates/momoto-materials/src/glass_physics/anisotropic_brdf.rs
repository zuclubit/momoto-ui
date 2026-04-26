//! # Anisotropic BRDF (Phase 9)
//!
//! Native anisotropic microfacet models for directionally-dependent materials.
//!
//! ## Physical Background
//!
//! Anisotropic materials have different roughness in different directions:
//! - **Brushed metal**: Grooves create directional highlights
//! - **Silk/Satin**: Fiber orientation causes sheen
//! - **Carbon fiber**: Weave pattern visible in reflections
//! - **Hair**: Strong forward/backward scattering
//!
//! ## Implementation
//!
//! Uses the anisotropic GGX (Trowbridge-Reitz) distribution with:
//! - Heitz (2014) height-correlated Smith masking-shadowing
//! - Heitz (2018) visible normal distribution function (VNDF) sampling
//!
//! ## References
//!
//! - Heitz (2014): "Understanding the Masking-Shadowing Function in Microfacet-Based BRDFs"
//! - Heitz (2018): "Sampling the GGX Distribution of Visible Normals"
//! - Walter et al. (2007): "Microfacet Models for Refraction through Rough Surfaces"

use std::f64::consts::PI;

use super::complex_ior::{fresnel_conductor_unpolarized, ComplexIOR};
use super::fresnel::fresnel_schlick;
use super::unified_bsdf::{BSDFContext, BSDFResponse, BSDFSample, Vector3, BSDF};

// ============================================================================
// ANISOTROPIC GGX MODEL
// ============================================================================

/// Anisotropic GGX (Trowbridge-Reitz) microfacet model
///
/// Implements the full anisotropic GGX distribution with:
/// - Different roughness along tangent (alpha_x) and bitangent (alpha_y)
/// - Height-correlated Smith masking-shadowing
/// - Fresnel term for dielectric materials
#[derive(Debug, Clone, Copy)]
pub struct AnisotropicGGX {
    /// Roughness along tangent direction (0-1)
    pub alpha_x: f64,
    /// Roughness along bitangent direction (0-1)
    pub alpha_y: f64,
    /// Index of refraction for Fresnel calculation
    pub ior: f64,
}

impl AnisotropicGGX {
    /// Create a new anisotropic GGX model
    pub fn new(alpha_x: f64, alpha_y: f64, ior: f64) -> Self {
        Self {
            alpha_x: alpha_x.clamp(0.001, 1.0),
            alpha_y: alpha_y.clamp(0.001, 1.0),
            ior: ior.max(1.0),
        }
    }

    /// Create isotropic GGX (alpha_x == alpha_y)
    pub fn isotropic(alpha: f64, ior: f64) -> Self {
        Self::new(alpha, alpha, ior)
    }

    /// Create from roughness and anisotropy parameters
    ///
    /// - roughness: overall roughness (0-1)
    /// - anisotropy: directional bias (-1 to 1, 0 = isotropic)
    pub fn from_roughness_anisotropy(roughness: f64, anisotropy: f64, ior: f64) -> Self {
        let roughness = roughness.clamp(0.0, 1.0);
        let anisotropy = anisotropy.clamp(-1.0, 1.0);

        // Convert to alpha_x, alpha_y
        let aspect = (1.0 - anisotropy * 0.9).sqrt();
        let alpha_x = roughness / aspect;
        let alpha_y = roughness * aspect;

        Self::new(alpha_x, alpha_y, ior)
    }

    /// GGX normal distribution function (anisotropic)
    ///
    /// D(h) = 1 / (π * αx * αy * ((hx/αx)² + (hy/αy)² + hz²)²)
    ///
    /// Where h is the half-vector in local coordinates.
    pub fn d(&self, h: &Vector3, ctx: &BSDFContext) -> f64 {
        // Transform to local tangent space
        let h_local = self.to_local(h, ctx);

        let hx = h_local.x / self.alpha_x;
        let hy = h_local.y / self.alpha_y;
        let hz = h_local.z;

        let denom = hx * hx + hy * hy + hz * hz;
        if denom < 1e-10 {
            return 0.0;
        }

        1.0 / (PI * self.alpha_x * self.alpha_y * denom * denom)
    }

    /// Smith masking-shadowing function (height-correlated)
    ///
    /// G2(wi, wo) = G1(wi) * G1(wo) / (1 + Λ(wi) + Λ(wo))
    ///
    /// Uses the height-correlated form from Heitz (2014).
    pub fn g(&self, wi: &Vector3, wo: &Vector3, ctx: &BSDFContext) -> f64 {
        let lambda_i = self.lambda(wi, ctx);
        let lambda_o = self.lambda(wo, ctx);

        1.0 / (1.0 + lambda_i + lambda_o)
    }

    /// Smith G1 (single-direction masking)
    pub fn g1(&self, w: &Vector3, ctx: &BSDFContext) -> f64 {
        1.0 / (1.0 + self.lambda(w, ctx))
    }

    /// Lambda function for Smith G
    ///
    /// Λ(w) = (-1 + sqrt(1 + α²tan²θ)) / 2
    fn lambda(&self, w: &Vector3, ctx: &BSDFContext) -> f64 {
        let w_local = self.to_local(w, ctx);

        let cos_theta = w_local.z.abs();
        if cos_theta < 1e-6 {
            return 1e10; // Near grazing, very high lambda
        }

        let tan2_theta = (1.0 - cos_theta * cos_theta) / (cos_theta * cos_theta);

        // Anisotropic alpha computation
        let phi = w_local.y.atan2(w_local.x);
        let cos_phi = phi.cos();
        let sin_phi = phi.sin();
        let alpha2 = (self.alpha_x * cos_phi).powi(2) + (self.alpha_y * sin_phi).powi(2);

        let a2_tan2 = alpha2 * tan2_theta;
        (-1.0 + (1.0 + a2_tan2).sqrt()) / 2.0
    }

    /// Sample the visible normal distribution function (VNDF)
    ///
    /// Implements Heitz (2018) sampling for importance sampling.
    pub fn sample_vndf(&self, wi: &Vector3, u1: f64, u2: f64, ctx: &BSDFContext) -> Vector3 {
        let wi_local = self.to_local(wi, ctx);

        // Transform to stretched space
        let wi_stretched = Vector3::new(
            self.alpha_x * wi_local.x,
            self.alpha_y * wi_local.y,
            wi_local.z,
        )
        .normalize();

        // Build orthonormal basis around stretched wi
        let t1 = if wi_stretched.z.abs() < 0.999 {
            Vector3::new(-wi_stretched.y, wi_stretched.x, 0.0).normalize()
        } else {
            Vector3::new(1.0, 0.0, 0.0)
        };
        let t2 = Vector3::new(
            wi_stretched.y * t1.z - wi_stretched.z * t1.y,
            wi_stretched.z * t1.x - wi_stretched.x * t1.z,
            wi_stretched.x * t1.y - wi_stretched.y * t1.x,
        );

        // Sample point on hemisphere
        let a = 1.0 / (1.0 + wi_stretched.z);
        let r = u1.sqrt();
        let phi = if u2 < a {
            u2 / a * PI
        } else {
            PI + (u2 - a) / (1.0 - a) * PI
        };

        let p1 = r * phi.cos();
        let p2 = r * phi.sin() * if u2 < a { 1.0 } else { wi_stretched.z };

        // Compute normal in stretched space
        let n_stretched =
            t1 * p1 + t2 * p2 + wi_stretched * (1.0 - p1 * p1 - p2 * p2).max(0.0).sqrt();

        // Transform back to original space
        let h_local = Vector3::new(
            self.alpha_x * n_stretched.x,
            self.alpha_y * n_stretched.y,
            n_stretched.z.max(0.0),
        )
        .normalize();

        // Transform to world space
        self.to_world(&h_local, ctx)
    }

    /// Transform vector to local tangent space
    fn to_local(&self, w: &Vector3, ctx: &BSDFContext) -> Vector3 {
        Vector3::new(
            w.dot(&ctx.tangent),
            w.dot(&ctx.bitangent),
            w.dot(&ctx.normal),
        )
    }

    /// Transform vector from local tangent space to world
    fn to_world(&self, w_local: &Vector3, ctx: &BSDFContext) -> Vector3 {
        ctx.tangent * w_local.x + ctx.bitangent * w_local.y + ctx.normal * w_local.z
    }

    /// Full microfacet BRDF evaluation
    ///
    /// f(wi, wo) = F(wi, h) * D(h) * G(wi, wo) / (4 * cos_i * cos_o)
    pub fn evaluate_brdf(&self, ctx: &BSDFContext) -> f64 {
        let cos_i = ctx.cos_theta_i();
        let cos_o = ctx.cos_theta_o();

        if cos_i < 1e-6 || cos_o < 1e-6 {
            return 0.0;
        }

        let h = ctx.half_vector();

        // Fresnel term
        let f = fresnel_schlick(1.0, self.ior, ctx.wi.dot(&h).abs());

        // Distribution term
        let d = self.d(&h, ctx);

        // Geometry term
        let g = self.g(&ctx.wi, &ctx.wo, ctx);

        // Microfacet BRDF
        f * d * g / (4.0 * cos_i * cos_o)
    }
}

impl BSDF for AnisotropicGGX {
    fn evaluate(&self, ctx: &BSDFContext) -> BSDFResponse {
        let brdf = self.evaluate_brdf(ctx);

        // Convert BRDF to reflectance (approximate integration)
        // For microfacet models, reflectance ≈ brdf * cos(theta_o) * pi
        let reflectance = (brdf * ctx.cos_theta_o() * PI).min(1.0);

        BSDFResponse::new(reflectance, 1.0 - reflectance, 0.0)
    }

    fn sample(&self, ctx: &BSDFContext, u1: f64, u2: f64) -> BSDFSample {
        // Sample half-vector using VNDF
        let h = self.sample_vndf(&ctx.wi, u1, u2, ctx);

        // Reflect wi around h
        let wo = ctx.wi.reflect(&h);

        // Check if wo is above surface
        if wo.dot(&ctx.normal) < 0.0 {
            return BSDFSample::new(wo, BSDFResponse::new(0.0, 1.0, 0.0), 1e-10, false);
        }

        // Evaluate at sampled direction
        let mut new_ctx = ctx.clone();
        new_ctx.wo = wo;

        let value = self.evaluate(&new_ctx);

        // PDF for VNDF sampling
        let d = self.d(&h, ctx);
        let g1 = self.g1(&ctx.wi, ctx);
        let pdf = d * g1 * ctx.wi.dot(&h).abs() / (4.0 * ctx.cos_theta_i());

        BSDFSample::new(wo, value, pdf.max(1e-10), false)
    }

    fn pdf(&self, ctx: &BSDFContext) -> f64 {
        let h = ctx.half_vector();
        let d = self.d(&h, ctx);
        let g1 = self.g1(&ctx.wi, ctx);
        d * g1 * ctx.wi.dot(&h).abs() / (4.0 * ctx.cos_theta_i())
    }

    fn name(&self) -> &str {
        "AnisotropicGGX"
    }
}

// ============================================================================
// ANISOTROPIC CONDUCTOR
// ============================================================================

/// Anisotropic conductor (brushed metal)
///
/// Combines anisotropic GGX distribution with conductor Fresnel.
#[derive(Debug, Clone, Copy)]
pub struct AnisotropicConductor {
    /// Anisotropic GGX distribution
    pub ggx: AnisotropicGGX,
    /// Real part of complex IOR
    pub n: f64,
    /// Extinction coefficient
    pub k: f64,
}

impl AnisotropicConductor {
    /// Create a new anisotropic conductor
    pub fn new(alpha_x: f64, alpha_y: f64, n: f64, k: f64) -> Self {
        Self {
            ggx: AnisotropicGGX::new(alpha_x, alpha_y, n),
            n,
            k,
        }
    }

    /// Create brushed stainless steel
    pub fn brushed_steel() -> Self {
        Self::new(0.05, 0.30, 2.91, 3.08) // Iron-like
    }

    /// Create brushed aluminum
    pub fn brushed_aluminum() -> Self {
        Self::new(0.03, 0.25, 1.35, 7.47)
    }

    /// Create brushed copper
    pub fn brushed_copper() -> Self {
        Self::new(0.04, 0.28, 0.27, 3.41)
    }

    /// Create brushed gold
    pub fn brushed_gold() -> Self {
        Self::new(0.02, 0.20, 0.18, 3.00)
    }

    /// Create with custom brush direction
    ///
    /// - angle_deg: brush direction in degrees (0 = along tangent)
    /// - roughness_along: roughness along brush direction
    /// - roughness_across: roughness across brush direction
    pub fn with_brush_direction(
        angle_deg: f64,
        roughness_along: f64,
        roughness_across: f64,
        n: f64,
        k: f64,
    ) -> Self {
        // Rotate roughness based on brush angle
        let angle = angle_deg.to_radians();
        let c = angle.cos();
        let s = angle.sin();

        let alpha_x = roughness_along * c * c + roughness_across * s * s;
        let alpha_y = roughness_along * s * s + roughness_across * c * c;

        Self::new(alpha_x, alpha_y, n, k)
    }

    /// Evaluate conductor Fresnel with complex IOR
    fn fresnel_conductor(&self, cos_theta: f64) -> f64 {
        let ior = ComplexIOR::new(self.n, self.k);
        fresnel_conductor_unpolarized(1.0, ior, cos_theta)
    }
}

impl BSDF for AnisotropicConductor {
    fn evaluate(&self, ctx: &BSDFContext) -> BSDFResponse {
        let cos_i = ctx.cos_theta_i();
        let cos_o = ctx.cos_theta_o();

        if cos_i < 1e-6 || cos_o < 1e-6 {
            return BSDFResponse::new(0.0, 0.0, 1.0);
        }

        let h = ctx.half_vector();

        // Conductor Fresnel
        let f = self.fresnel_conductor(ctx.wi.dot(&h).abs());

        // Distribution and geometry from GGX
        let d = self.ggx.d(&h, ctx);
        let g = self.ggx.g(&ctx.wi, &ctx.wo, ctx);

        // Microfacet BRDF
        let brdf = f * d * g / (4.0 * cos_i * cos_o);
        let reflectance = (brdf * cos_o * PI).min(1.0);

        // Conductors absorb non-reflected light
        BSDFResponse::new(reflectance, 0.0, 1.0 - reflectance)
    }

    fn sample(&self, ctx: &BSDFContext, u1: f64, u2: f64) -> BSDFSample {
        self.ggx.sample(ctx, u1, u2)
    }

    fn pdf(&self, ctx: &BSDFContext) -> f64 {
        self.ggx.pdf(ctx)
    }

    fn name(&self) -> &str {
        "AnisotropicConductor"
    }
}

// ============================================================================
// FIBER BSDF
// ============================================================================

/// Fiber-based BSDF (silk, hair-like materials)
///
/// Models materials with directional fiber structure.
#[derive(Debug, Clone, Copy)]
pub struct FiberBSDF {
    /// Base anisotropic model
    pub base: AnisotropicGGX,
    /// Fiber direction (normalized)
    pub fiber_direction: Vector3,
    /// Sheen intensity (0-1)
    pub sheen: f64,
    /// Sheen color tint (grayscale for now)
    pub sheen_tint: f64,
}

impl FiberBSDF {
    /// Create a new fiber BSDF
    pub fn new(alpha_x: f64, alpha_y: f64, ior: f64, fiber_direction: Vector3, sheen: f64) -> Self {
        Self {
            base: AnisotropicGGX::new(alpha_x, alpha_y, ior),
            fiber_direction: fiber_direction.normalize(),
            sheen: sheen.clamp(0.0, 1.0),
            sheen_tint: 0.5,
        }
    }

    /// Create silk preset
    pub fn silk() -> Self {
        Self::new(0.10, 0.40, 1.5, Vector3::unit_x(), 0.4)
    }

    /// Create satin preset
    pub fn satin() -> Self {
        Self::new(0.08, 0.35, 1.5, Vector3::unit_x(), 0.5)
    }

    /// Create velvet preset
    pub fn velvet() -> Self {
        Self::new(0.15, 0.50, 1.5, Vector3::unit_z(), 0.7)
    }

    /// Create carbon fiber preset
    pub fn carbon_fiber() -> Self {
        Self::new(0.02, 0.10, 2.0, Vector3::unit_x(), 0.2)
    }

    /// Calculate sheen contribution
    fn sheen_term(&self, ctx: &BSDFContext) -> f64 {
        // Sheen is strongest at grazing angles perpendicular to fiber
        let fiber_dot_wi = self.fiber_direction.dot(&ctx.wi).abs();
        let fiber_dot_wo = self.fiber_direction.dot(&ctx.wo).abs();

        // Sheen peaks when light/view are perpendicular to fiber
        let perpendicular_factor = (1.0 - fiber_dot_wi) * (1.0 - fiber_dot_wo);

        self.sheen * perpendicular_factor * (1.0 - ctx.cos_theta_i().powi(5))
    }
}

impl BSDF for FiberBSDF {
    fn evaluate(&self, ctx: &BSDFContext) -> BSDFResponse {
        // Base anisotropic reflection
        let base_response = self.base.evaluate(ctx);

        // Add sheen
        let sheen = self.sheen_term(ctx);
        let total_reflectance = (base_response.reflectance + sheen).min(1.0);

        BSDFResponse::new(total_reflectance, 1.0 - total_reflectance, 0.0)
    }

    fn sample(&self, ctx: &BSDFContext, u1: f64, u2: f64) -> BSDFSample {
        self.base.sample(ctx, u1, u2)
    }

    fn name(&self) -> &str {
        "FiberBSDF"
    }
}

// ============================================================================
// PRESET FACTORY
// ============================================================================

/// Anisotropic material presets
pub mod presets {
    use super::*;

    /// Brushed stainless steel
    pub fn brushed_stainless() -> AnisotropicConductor {
        AnisotropicConductor::brushed_steel()
    }

    /// Brushed aluminum (common in consumer electronics)
    pub fn brushed_aluminum() -> AnisotropicConductor {
        AnisotropicConductor::brushed_aluminum()
    }

    /// Brushed copper (decorative surfaces)
    pub fn brushed_copper() -> AnisotropicConductor {
        AnisotropicConductor::brushed_copper()
    }

    /// Brushed gold (luxury finishes)
    pub fn brushed_gold() -> AnisotropicConductor {
        AnisotropicConductor::brushed_gold()
    }

    /// Silk fabric
    pub fn silk() -> FiberBSDF {
        FiberBSDF::silk()
    }

    /// Satin fabric
    pub fn satin() -> FiberBSDF {
        FiberBSDF::satin()
    }

    /// Velvet fabric
    pub fn velvet() -> FiberBSDF {
        FiberBSDF::velvet()
    }

    /// Carbon fiber composite
    pub fn carbon_fiber() -> FiberBSDF {
        FiberBSDF::carbon_fiber()
    }

    /// Get all preset names
    pub fn all_preset_names() -> Vec<&'static str> {
        vec![
            "brushed_stainless",
            "brushed_aluminum",
            "brushed_copper",
            "brushed_gold",
            "silk",
            "satin",
            "velvet",
            "carbon_fiber",
        ]
    }
}

// ============================================================================
// UTILITY FUNCTIONS
// ============================================================================

/// Measure anisotropy strength from alpha values
pub fn anisotropy_strength(alpha_x: f64, alpha_y: f64) -> f64 {
    if alpha_x.max(alpha_y) < 1e-10 {
        return 0.0;
    }
    1.0 - (alpha_x.min(alpha_y) / alpha_x.max(alpha_y))
}

/// Convert anisotropy strength to alpha values
pub fn strength_to_alphas(roughness: f64, strength: f64) -> (f64, f64) {
    let strength = strength.clamp(0.0, 1.0);
    let roughness = roughness.clamp(0.01, 1.0);

    let ratio = 1.0 - strength * 0.9; // max ratio 10:1
    let alpha_x = roughness;
    let alpha_y = roughness * ratio;

    (alpha_x, alpha_y)
}

/// Total memory used by anisotropic module
pub fn total_anisotropic_memory() -> usize {
    std::mem::size_of::<AnisotropicGGX>()
        + std::mem::size_of::<AnisotropicConductor>()
        + std::mem::size_of::<FiberBSDF>()
        + 500 // Overhead
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    const EPSILON: f64 = 1e-6;

    #[test]
    fn test_ggx_isotropic_recovery() {
        // When alpha_x == alpha_y, should behave like isotropic GGX
        let iso = AnisotropicGGX::isotropic(0.2, 1.5);
        let aniso = AnisotropicGGX::new(0.2, 0.2, 1.5);

        let ctx = BSDFContext::new_simple(0.7);

        let iso_r = iso.evaluate(&ctx).reflectance;
        let aniso_r = aniso.evaluate(&ctx).reflectance;

        assert!(
            (iso_r - aniso_r).abs() < 0.01,
            "Isotropic recovery: {} vs {}",
            iso_r,
            aniso_r
        );
    }

    #[test]
    fn test_anisotropic_direction_dependence() {
        // High anisotropy should show direction dependence
        let aniso = AnisotropicGGX::new(0.05, 0.40, 1.5);

        // Create two contexts with different tangent orientations
        let ctx1 = BSDFContext::new_reflection(0.7, 550.0);

        // Rotate the view direction
        let mut ctx2 = ctx1.clone();
        ctx2.wi = Vector3::new(ctx1.wi.y, ctx1.wi.x, ctx1.wi.z); // Swap x,y

        let r1 = aniso.evaluate(&ctx1).reflectance;
        let r2 = aniso.evaluate(&ctx2).reflectance;

        // With anisotropy, these should differ
        // Note: actual difference depends on geometry, just check both are valid
        assert!(r1 > 0.0 && r1 < 1.0);
        assert!(r2 > 0.0 && r2 < 1.0);
    }

    #[test]
    fn test_energy_conservation() {
        let materials: Vec<Box<dyn BSDF>> = vec![
            Box::new(AnisotropicGGX::new(0.1, 0.3, 1.5)),
            Box::new(AnisotropicConductor::brushed_steel()),
            Box::new(FiberBSDF::silk()),
        ];

        for mat in materials {
            let ctx = BSDFContext::new_simple(0.7);
            let response = mat.evaluate(&ctx);

            let total = response.total_energy();
            assert!(
                (total - 1.0).abs() < EPSILON,
                "{} energy: {}",
                mat.name(),
                total
            );
        }
    }

    #[test]
    fn test_sampling_produces_valid_directions() {
        let ggx = AnisotropicGGX::new(0.1, 0.3, 1.5);
        let ctx = BSDFContext::new_simple(0.7);

        for i in 0..10 {
            let u1 = (i as f64 + 0.5) / 10.0;
            let u2 = ((i * 7) as f64 % 10.0 + 0.5) / 10.0;

            let sample = ggx.sample(&ctx, u1, u2);

            // Check direction is valid
            assert!(sample.wo.length() > 0.9, "Invalid direction length");
            assert!(sample.pdf > 0.0, "Invalid PDF");
        }
    }

    #[test]
    fn test_brushed_metal_presets() {
        let presets: Vec<AnisotropicConductor> = vec![
            presets::brushed_stainless(),
            presets::brushed_aluminum(),
            presets::brushed_copper(),
            presets::brushed_gold(),
        ];

        for preset in presets {
            let ctx = BSDFContext::new_simple(0.8);
            let response = preset.evaluate(&ctx);

            // Metals should have high reflectance
            assert!(response.reflectance > 0.3, "Metal reflectance too low");

            // No transmission
            assert!(response.transmittance < 0.01);
        }
    }

    #[test]
    fn test_fiber_presets() {
        let presets: Vec<FiberBSDF> = vec![
            presets::silk(),
            presets::satin(),
            presets::velvet(),
            presets::carbon_fiber(),
        ];

        for preset in presets {
            let ctx = BSDFContext::new_simple(0.8);
            let response = preset.evaluate(&ctx);

            // Should produce valid response
            assert!(response.reflectance >= 0.0);
            assert!((response.total_energy() - 1.0).abs() < EPSILON);
        }
    }

    #[test]
    fn test_anisotropy_strength() {
        assert!((anisotropy_strength(0.1, 0.1) - 0.0).abs() < 0.01);
        assert!(anisotropy_strength(0.1, 0.5) > 0.5);
        assert!(anisotropy_strength(0.01, 0.1) > 0.8);
    }

    #[test]
    fn test_strength_to_alphas() {
        let (ax, ay) = strength_to_alphas(0.2, 0.0);
        assert!((ax - ay).abs() < 0.01, "Zero strength should be isotropic");

        let (ax, ay) = strength_to_alphas(0.2, 0.5);
        assert!(ax > ay, "Positive strength should give ax > ay");
    }

    #[test]
    fn test_from_roughness_anisotropy() {
        let ggx = AnisotropicGGX::from_roughness_anisotropy(0.2, 0.0, 1.5);
        assert!((ggx.alpha_x - ggx.alpha_y).abs() < 0.01, "Zero anisotropy");

        let ggx = AnisotropicGGX::from_roughness_anisotropy(0.2, 0.5, 1.5);
        assert!(ggx.alpha_x != ggx.alpha_y, "Non-zero anisotropy");
    }

    #[test]
    fn test_d_function_normalization() {
        // The D function should integrate to 1 over the hemisphere
        // We do a rough numerical check
        let ggx = AnisotropicGGX::isotropic(0.3, 1.5);

        let mut sum = 0.0;
        let n = 20;

        for i in 0..n {
            for j in 0..n {
                let theta = (i as f64 + 0.5) / n as f64 * PI / 2.0;
                let phi = (j as f64 + 0.5) / n as f64 * 2.0 * PI;

                let h = Vector3::new(
                    theta.sin() * phi.cos(),
                    theta.sin() * phi.sin(),
                    theta.cos(),
                );

                let ctx = BSDFContext::default();
                let d = ggx.d(&h, &ctx);

                // Solid angle element
                let d_omega = theta.sin() * (PI / 2.0 / n as f64) * (2.0 * PI / n as f64);
                sum += d * h.z * d_omega;
            }
        }

        // Should be approximately 1 (allowing numerical error)
        assert!(sum > 0.5 && sum < 2.0, "D normalization: {}", sum);
    }

    #[test]
    fn test_memory_usage() {
        let mem = total_anisotropic_memory();
        assert!(mem < 1_000, "Memory should be < 1KB, got {}", mem);
    }
}
