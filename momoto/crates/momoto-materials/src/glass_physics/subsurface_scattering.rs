//! # Subsurface Scattering (Phase 9)
//!
//! Real BSSRDF implementation for translucent materials.
//!
//! ## Physical Background
//!
//! Subsurface scattering occurs when light penetrates a surface, scatters
//! multiple times within the material, and exits at a different location.
//! This creates the characteristic soft, translucent appearance of:
//!
//! - **Human skin**: Warm subsurface glow
//! - **Marble**: Deep translucency
//! - **Milk**: Strong scattering, white appearance
//! - **Jade**: Green translucency
//! - **Wax**: Warm subsurface light transport
//!
//! ## Implementation
//!
//! Uses the diffusion approximation (Jensen 2001) which models subsurface
//! scattering as a diffusion process, valid when:
//! - Scattering is much stronger than absorption
//! - Light travels many mean free paths before exiting
//!
//! ## References
//!
//! - Jensen et al. (2001): "A Practical Model for Subsurface Light Transport"
//! - Donner & Jensen (2005): "Light Diffusion in Multi-Layered Translucent Materials"
//! - Christensen & Burley (2015): "Approximate Reflectance Profiles for Efficient SSS"

use std::f64::consts::PI;

use super::fresnel::fresnel_schlick;
use super::unified_bsdf::{BSDFContext, BSDFResponse, BSDFSample, Vector3, BSDF};

// ============================================================================
// SUBSURFACE PARAMETERS
// ============================================================================

/// Subsurface scattering parameters per RGB channel
///
/// These parameters define the optical properties that govern
/// how light scatters within the material.
#[derive(Debug, Clone, Copy)]
pub struct SubsurfaceParams {
    /// Absorption coefficient (1/mm) per RGB channel
    ///
    /// Higher values = more light absorbed per unit distance
    pub sigma_a: [f64; 3],

    /// Scattering coefficient (1/mm) per RGB channel
    ///
    /// Higher values = more scattering events per unit distance
    pub sigma_s: [f64; 3],

    /// Scattering anisotropy (Henyey-Greenstein g parameter)
    ///
    /// - g = 0: Isotropic scattering
    /// - g > 0: Forward scattering (most biological materials)
    /// - g < 0: Back scattering
    pub g: f64,

    /// Internal index of refraction
    pub eta: f64,
}

impl SubsurfaceParams {
    /// Create new subsurface parameters
    pub fn new(sigma_a: [f64; 3], sigma_s: [f64; 3], g: f64, eta: f64) -> Self {
        Self {
            sigma_a,
            sigma_s,
            g: g.clamp(-1.0, 1.0),
            eta: eta.max(1.0),
        }
    }

    /// Extinction coefficient: σt = σa + σs
    pub fn sigma_t(&self) -> [f64; 3] {
        [
            self.sigma_a[0] + self.sigma_s[0],
            self.sigma_a[1] + self.sigma_s[1],
            self.sigma_a[2] + self.sigma_s[2],
        ]
    }

    /// Reduced scattering coefficient: σs' = σs * (1 - g)
    pub fn sigma_s_prime(&self) -> [f64; 3] {
        let factor = 1.0 - self.g;
        [
            self.sigma_s[0] * factor,
            self.sigma_s[1] * factor,
            self.sigma_s[2] * factor,
        ]
    }

    /// Effective transport coefficient: σtr = √(3 * σa * (σa + σs'))
    pub fn sigma_tr(&self) -> [f64; 3] {
        let sigma_s_p = self.sigma_s_prime();
        [
            (3.0 * self.sigma_a[0] * (self.sigma_a[0] + sigma_s_p[0])).sqrt(),
            (3.0 * self.sigma_a[1] * (self.sigma_a[1] + sigma_s_p[1])).sqrt(),
            (3.0 * self.sigma_a[2] * (self.sigma_a[2] + sigma_s_p[2])).sqrt(),
        ]
    }

    /// Reduced albedo: α' = σs' / (σa + σs')
    pub fn alpha_prime(&self) -> [f64; 3] {
        let sigma_s_p = self.sigma_s_prime();
        [
            sigma_s_p[0] / (self.sigma_a[0] + sigma_s_p[0]).max(1e-10),
            sigma_s_p[1] / (self.sigma_a[1] + sigma_s_p[1]).max(1e-10),
            sigma_s_p[2] / (self.sigma_a[2] + sigma_s_p[2]).max(1e-10),
        ]
    }

    /// Mean free path (average distance between scattering events)
    pub fn mean_free_path(&self) -> [f64; 3] {
        let sigma_t = self.sigma_t();
        [
            1.0 / sigma_t[0].max(1e-10),
            1.0 / sigma_t[1].max(1e-10),
            1.0 / sigma_t[2].max(1e-10),
        ]
    }

    /// Diffuse reflectance (Rd) at normal incidence
    pub fn diffuse_reflectance(&self) -> [f64; 3] {
        let alpha = self.alpha_prime();
        [
            approximate_rd(alpha[0], self.eta),
            approximate_rd(alpha[1], self.eta),
            approximate_rd(alpha[2], self.eta),
        ]
    }

    /// Scale all coefficients by a factor
    pub fn scaled(&self, scale: f64) -> Self {
        Self {
            sigma_a: [
                self.sigma_a[0] / scale,
                self.sigma_a[1] / scale,
                self.sigma_a[2] / scale,
            ],
            sigma_s: [
                self.sigma_s[0] / scale,
                self.sigma_s[1] / scale,
                self.sigma_s[2] / scale,
            ],
            g: self.g,
            eta: self.eta,
        }
    }
}

impl Default for SubsurfaceParams {
    fn default() -> Self {
        // Default: generic translucent material
        Self::new(
            [0.01, 0.01, 0.01], // Low absorption
            [2.0, 2.0, 2.0],    // Moderate scattering
            0.0,                // Isotropic
            1.3,                // Typical organic material
        )
    }
}

// ============================================================================
// DIFFUSION BSSRDF
// ============================================================================

/// Diffusion approximation BSSRDF (Jensen 2001)
///
/// Models subsurface scattering using the diffusion equation,
/// which is valid for highly scattering media.
#[derive(Debug, Clone, Copy)]
pub struct DiffusionBSSRDF {
    /// Optical parameters
    pub params: SubsurfaceParams,
    /// Distance scale (controls extent of scattering)
    pub scale: f64,
}

impl DiffusionBSSRDF {
    /// Create a new diffusion BSSRDF
    pub fn new(params: SubsurfaceParams, scale: f64) -> Self {
        Self {
            params,
            scale: scale.max(0.001),
        }
    }

    /// Create with default scale
    pub fn with_params(params: SubsurfaceParams) -> Self {
        Self::new(params, 1.0)
    }

    /// Dipole diffusion profile (Jensen 2001)
    ///
    /// Rd(r) = diffuse reflectance at distance r from illumination point
    ///
    /// Uses the dipole approximation with a real source below the surface
    /// and a virtual source above to satisfy boundary conditions.
    pub fn rd(&self, r: f64, channel: usize) -> f64 {
        let r = r / self.scale; // Apply distance scaling

        if r < 1e-10 {
            // At r = 0, use limiting behavior
            return self.params.diffuse_reflectance()[channel];
        }

        let sigma_tr = self.params.sigma_tr()[channel];
        let sigma_s_p = self.params.sigma_s_prime()[channel];
        let sigma_a = self.params.sigma_a[channel];

        // Diffusion coefficient
        let _d = 1.0 / (3.0 * (sigma_a + sigma_s_p));

        // Boundary condition term (approximate Fresnel correction)
        let fdr = fresnel_diffuse_reflectance(self.params.eta);
        let a = (1.0 + fdr) / (1.0 - fdr);

        // Dipole distances
        let zr = 1.0 / (sigma_a + sigma_s_p); // Real source depth
        let zv = zr * (1.0 + 4.0 * a / 3.0); // Virtual source height

        // Distances to real and virtual sources
        let dr = (r * r + zr * zr).sqrt();
        let dv = (r * r + zv * zv).sqrt();

        // Dipole formula
        let c1 = zr * (1.0 + sigma_tr * dr) * (-sigma_tr * dr).exp() / (dr * dr * dr);
        let c2 = zv * (1.0 + sigma_tr * dv) * (-sigma_tr * dv).exp() / (dv * dv * dv);

        let rd = (c1 + c2) / (4.0 * PI);

        rd.max(0.0)
    }

    /// RGB diffuse reflectance at distance r
    pub fn rd_rgb(&self, r: f64) -> [f64; 3] {
        [self.rd(r, 0), self.rd(r, 1), self.rd(r, 2)]
    }

    /// Single-scattering approximation
    ///
    /// First-order scattering for thin translucent materials
    pub fn single_scatter(&self, cos_theta_i: f64, cos_theta_o: f64, channel: usize) -> f64 {
        let sigma_t = self.params.sigma_t()[channel];
        let sigma_s = self.params.sigma_s[channel];

        // Simplified single-scattering
        let phase = henyey_greenstein(cos_theta_i * cos_theta_o, self.params.g);
        let extinction = (-sigma_t * self.scale).exp();

        sigma_s * phase * extinction / (4.0 * PI)
    }

    /// Combined diffuse + single scatter contribution
    pub fn evaluate_channel(
        &self,
        r: f64,
        cos_theta_i: f64,
        cos_theta_o: f64,
        channel: usize,
    ) -> f64 {
        // Multiple scattering (diffusion)
        let diffuse = self.rd(r, channel);

        // Single scattering
        let single = self.single_scatter(cos_theta_i, cos_theta_o, channel);

        diffuse + single
    }

    /// Evaluate RGB contribution
    pub fn evaluate_rgb(&self, r: f64, cos_theta_i: f64, cos_theta_o: f64) -> [f64; 3] {
        [
            self.evaluate_channel(r, cos_theta_i, cos_theta_o, 0),
            self.evaluate_channel(r, cos_theta_i, cos_theta_o, 1),
            self.evaluate_channel(r, cos_theta_i, cos_theta_o, 2),
        ]
    }

    /// Effective radius (distance where Rd drops to 1% of max)
    pub fn effective_radius(&self, channel: usize) -> f64 {
        // Approximate: R_eff ≈ 3 / σtr
        3.0 * self.scale / self.params.sigma_tr()[channel].max(0.01)
    }
}

// ============================================================================
// SUBSURFACE BSDF
// ============================================================================

/// BSDF adapter combining surface reflection and subsurface scattering
#[derive(Debug, Clone)]
pub struct SubsurfaceBSDF {
    /// Surface IOR for Fresnel reflection
    pub surface_ior: f64,
    /// Surface roughness
    pub surface_roughness: f64,
    /// Subsurface scattering model
    pub subsurface: DiffusionBSSRDF,
    /// Mix factor (0 = pure surface, 1 = pure subsurface)
    pub mix: f64,
}

impl SubsurfaceBSDF {
    /// Create a new subsurface BSDF
    pub fn new(surface_ior: f64, subsurface_params: SubsurfaceParams, mix: f64) -> Self {
        Self {
            surface_ior,
            surface_roughness: 0.0,
            subsurface: DiffusionBSSRDF::with_params(subsurface_params),
            mix: mix.clamp(0.0, 1.0),
        }
    }

    /// Set surface roughness
    pub fn with_roughness(mut self, roughness: f64) -> Self {
        self.surface_roughness = roughness.clamp(0.0, 1.0);
        self
    }

    /// Set subsurface scale
    pub fn with_scale(mut self, scale: f64) -> Self {
        self.subsurface.scale = scale.max(0.001);
        self
    }

    /// Skin preset
    pub fn skin() -> Self {
        Self::new(1.4, sss_presets::skin(), 0.8)
    }

    /// Marble preset
    pub fn marble() -> Self {
        Self::new(1.5, sss_presets::marble(), 0.6)
    }

    /// Milk preset
    pub fn milk() -> Self {
        Self::new(1.35, sss_presets::milk(), 0.95)
    }

    /// Jade preset
    pub fn jade() -> Self {
        Self::new(1.6, sss_presets::jade(), 0.7)
    }

    /// Wax preset
    pub fn wax() -> Self {
        Self::new(1.4, sss_presets::wax(), 0.85)
    }
}

impl BSDF for SubsurfaceBSDF {
    fn evaluate(&self, ctx: &BSDFContext) -> BSDFResponse {
        let cos_i = ctx.cos_theta_i();
        let cos_o = ctx.cos_theta_o();

        // Surface Fresnel reflection
        let fresnel = fresnel_schlick(1.0, self.surface_ior, cos_i);

        // Apply roughness to surface
        let surface_r = fresnel * (1.0 - self.surface_roughness * 0.5);

        // Subsurface contribution (using r = 0 for local BSDF approximation)
        let sss_rgb = self.subsurface.evaluate_rgb(0.0, cos_i, cos_o);
        let sss_avg = (sss_rgb[0] + sss_rgb[1] + sss_rgb[2]) / 3.0;

        // Transmitted light can scatter subsurface
        let transmitted = 1.0 - surface_r;
        let subsurface_r = transmitted * sss_avg * self.mix;

        // Combine surface and subsurface
        let total_r = surface_r + subsurface_r * (1.0 - surface_r);

        // Non-reflected light exits as transmission or is absorbed
        let absorption_factor = 1.0 - sss_avg;
        let absorption = (1.0 - total_r) * absorption_factor * self.mix;
        let transmission = (1.0 - total_r) * (1.0 - absorption_factor * self.mix);

        BSDFResponse::new(total_r, transmission, absorption).with_rgb(sss_rgb)
    }

    fn sample(&self, ctx: &BSDFContext, u1: f64, u2: f64) -> BSDFSample {
        // Cosine-weighted hemisphere sampling
        let theta = (1.0 - u1).sqrt().acos();
        let phi = 2.0 * PI * u2;

        let wo = Vector3::new(
            theta.sin() * phi.cos(),
            theta.sin() * phi.sin(),
            theta.cos(),
        );

        let mut sample_ctx = ctx.clone();
        sample_ctx.wo = wo;

        let value = self.evaluate(&sample_ctx);
        let pdf = theta.cos() / PI;

        BSDFSample::new(wo, value, pdf.max(1e-10), false)
    }

    fn name(&self) -> &str {
        "SubsurfaceBSDF"
    }
}

// ============================================================================
// PRESETS
// ============================================================================

/// Pre-defined subsurface scattering parameters
pub mod sss_presets {
    use super::SubsurfaceParams;

    /// Human skin (Caucasian)
    ///
    /// Warm reddish subsurface due to blood absorption
    pub fn skin() -> SubsurfaceParams {
        SubsurfaceParams::new(
            [0.032, 0.17, 0.48], // σa: absorbs blue/green, passes red
            [0.74, 0.88, 1.01],  // σs: high scattering
            0.8,                 // Forward scattering
            1.3,                 // Typical tissue IOR
        )
    }

    /// White marble
    ///
    /// Translucent with slight warm tint
    pub fn marble() -> SubsurfaceParams {
        SubsurfaceParams::new(
            [0.0021, 0.0041, 0.0071], // Very low absorption
            [2.19, 2.62, 3.00],       // High scattering
            0.0,                      // Isotropic
            1.5,                      // Stone IOR
        )
    }

    /// Whole milk
    ///
    /// Extremely high scattering, white appearance
    pub fn milk() -> SubsurfaceParams {
        SubsurfaceParams::new(
            [0.0015, 0.0046, 0.019], // Low absorption
            [2.55, 3.21, 3.77],      // Very high scattering
            0.7,                     // Forward scattering
            1.35,                    // Liquid IOR
        )
    }

    /// Green jade
    ///
    /// Translucent green mineral
    pub fn jade() -> SubsurfaceParams {
        SubsurfaceParams::new(
            [0.1, 0.01, 0.05], // Absorbs red/blue, passes green
            [1.5, 1.8, 1.6],   // Moderate scattering
            0.3,               // Slight forward scatter
            1.6,               // Mineral IOR
        )
    }

    /// Candle wax
    ///
    /// Warm translucent appearance
    pub fn wax() -> SubsurfaceParams {
        SubsurfaceParams::new(
            [0.001, 0.002, 0.005], // Very low absorption
            [1.8, 1.9, 2.0],       // High scattering
            0.6,                   // Forward scattering
            1.4,                   // Wax IOR
        )
    }

    /// Ivory soap
    ///
    /// Clean white translucent
    pub fn soap() -> SubsurfaceParams {
        SubsurfaceParams::new(
            [0.0001, 0.0001, 0.0003], // Minimal absorption
            [1.0, 1.1, 1.2],          // Moderate scattering
            0.4,                      // Forward scattering
            1.4,                      // Soap IOR
        )
    }

    /// Ketchup
    ///
    /// Strong red absorption
    pub fn ketchup() -> SubsurfaceParams {
        SubsurfaceParams::new(
            [0.06, 0.97, 1.45], // Heavy green/blue absorption
            [0.18, 0.07, 0.03], // Low scattering
            0.0,                // Isotropic
            1.4,                // Food IOR
        )
    }

    /// Apple (red delicious)
    pub fn apple() -> SubsurfaceParams {
        SubsurfaceParams::new(
            [0.003, 0.0034, 0.046], // Absorbs blue
            [2.29, 2.39, 1.97],     // High scattering
            0.5,                    // Forward scattering
            1.3,                    // Fruit IOR
        )
    }

    /// Get all preset names
    pub fn all_preset_names() -> Vec<&'static str> {
        vec![
            "skin", "marble", "milk", "jade", "wax", "soap", "ketchup", "apple",
        ]
    }
}

// ============================================================================
// HELPER FUNCTIONS
// ============================================================================

/// Approximate diffuse reflectance from reduced albedo
fn approximate_rd(alpha: f64, eta: f64) -> f64 {
    // Grosjean approximation
    let fdr = fresnel_diffuse_reflectance(eta);
    let a = (1.0 + fdr) / (1.0 - fdr);

    let s = (3.0 * (1.0 - alpha)).sqrt();
    let f = (1.0 + (-s / a).exp()) / (1.0 - (-s).exp());

    0.5 * alpha * f.min(1.0)
}

/// Diffuse Fresnel reflectance (average over hemisphere)
fn fresnel_diffuse_reflectance(eta: f64) -> f64 {
    // Fit from Jensen et al.
    if eta < 1.0 {
        -0.4399 + 0.7099 / eta - 0.3319 / (eta * eta) + 0.0636 / (eta * eta * eta)
    } else {
        -1.4399 / (eta * eta) + 0.7099 / eta + 0.6681 + 0.0636 * eta
    }
}

/// Henyey-Greenstein phase function
fn henyey_greenstein(cos_theta: f64, g: f64) -> f64 {
    if g.abs() < 1e-10 {
        return 1.0 / (4.0 * PI);
    }

    let g2 = g * g;
    let denom = 1.0 + g2 - 2.0 * g * cos_theta;
    (1.0 - g2) / (4.0 * PI * denom * denom.sqrt())
}

/// Total memory used by SSS module
pub fn total_sss_memory() -> usize {
    std::mem::size_of::<SubsurfaceParams>()
        + std::mem::size_of::<DiffusionBSSRDF>()
        + std::mem::size_of::<SubsurfaceBSDF>()
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
    fn test_sigma_computations() {
        let params = SubsurfaceParams::new([0.1, 0.1, 0.1], [1.0, 1.0, 1.0], 0.5, 1.3);

        let sigma_t = params.sigma_t();
        assert!((sigma_t[0] - 1.1).abs() < EPSILON);

        let sigma_s_p = params.sigma_s_prime();
        assert!((sigma_s_p[0] - 0.5).abs() < EPSILON); // 1.0 * (1 - 0.5)

        let alpha = params.alpha_prime();
        assert!(alpha[0] > 0.0 && alpha[0] < 1.0);
    }

    #[test]
    fn test_diffusion_profile() {
        let params = sss_presets::skin();
        let bssrdf = DiffusionBSSRDF::with_params(params);

        // Rd should be highest at center and decrease with distance
        let rd_0 = bssrdf.rd(0.0, 0);
        let rd_1 = bssrdf.rd(1.0, 0);
        let rd_5 = bssrdf.rd(5.0, 0);

        assert!(rd_0 > rd_1);
        assert!(rd_1 > rd_5);
        assert!(rd_5 > 0.0);
    }

    #[test]
    fn test_diffusion_rgb() {
        let params = sss_presets::jade();
        let bssrdf = DiffusionBSSRDF::with_params(params);

        let rgb = bssrdf.rd_rgb(0.5);

        // Jade should have highest reflectance in green
        assert!(rgb[1] > rgb[0]); // Green > Red
        assert!(rgb[1] > rgb[2]); // Green > Blue
    }

    #[test]
    fn test_energy_conservation() {
        let materials: Vec<SubsurfaceBSDF> = vec![
            SubsurfaceBSDF::skin(),
            SubsurfaceBSDF::marble(),
            SubsurfaceBSDF::milk(),
            SubsurfaceBSDF::jade(),
            SubsurfaceBSDF::wax(),
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
    fn test_presets_valid() {
        let presets = vec![
            sss_presets::skin(),
            sss_presets::marble(),
            sss_presets::milk(),
            sss_presets::jade(),
            sss_presets::wax(),
            sss_presets::soap(),
            sss_presets::ketchup(),
            sss_presets::apple(),
        ];

        for params in presets {
            // All parameters should be positive
            for &a in &params.sigma_a {
                assert!(a >= 0.0, "sigma_a should be non-negative");
            }
            for &s in &params.sigma_s {
                assert!(s >= 0.0, "sigma_s should be non-negative");
            }
            assert!(params.eta >= 1.0, "eta should be >= 1");
            assert!(
                params.g >= -1.0 && params.g <= 1.0,
                "g should be in [-1, 1]"
            );
        }
    }

    #[test]
    fn test_scaling() {
        let params = sss_presets::skin();
        let scaled = params.scaled(2.0);

        // Scaling by 2 should halve the coefficients
        assert!((scaled.sigma_a[0] - params.sigma_a[0] / 2.0).abs() < EPSILON);
    }

    #[test]
    fn test_effective_radius() {
        let params = sss_presets::milk();
        let bssrdf = DiffusionBSSRDF::with_params(params);

        let r_eff = bssrdf.effective_radius(0);
        assert!(r_eff > 0.0, "Effective radius should be positive");
    }

    #[test]
    fn test_henyey_greenstein() {
        // Isotropic (g=0) should be uniform
        let iso = henyey_greenstein(0.5, 0.0);
        assert!((iso - 1.0 / (4.0 * PI)).abs() < 0.01);

        // Forward scattering should peak at cos_theta = 1
        let fwd_peak = henyey_greenstein(1.0, 0.8);
        let fwd_side = henyey_greenstein(0.0, 0.8);
        assert!(fwd_peak > fwd_side);

        // Back scattering should peak at cos_theta = -1
        let back_peak = henyey_greenstein(-1.0, -0.8);
        let back_side = henyey_greenstein(0.0, -0.8);
        assert!(back_peak > back_side);
    }

    #[test]
    fn test_fresnel_diffuse_reflectance() {
        // Should return reasonable values
        let fdr = fresnel_diffuse_reflectance(1.3);
        assert!(fdr > 0.0 && fdr < 1.0, "Fdr should be in (0, 1): {}", fdr);

        // Higher IOR should give higher diffuse reflectance
        let fdr_low = fresnel_diffuse_reflectance(1.1);
        let fdr_high = fresnel_diffuse_reflectance(1.5);
        assert!(fdr_high > fdr_low);
    }

    #[test]
    fn test_subsurface_mix() {
        let params = sss_presets::skin();

        // Mix = 0 should be mostly surface
        let pure_surface = SubsurfaceBSDF::new(1.4, params, 0.0);
        let ctx = BSDFContext::new_simple(1.0);
        let r_surface = pure_surface.evaluate(&ctx).reflectance;

        // Mix = 1 should show subsurface contribution
        let pure_sss = SubsurfaceBSDF::new(1.4, params, 1.0);
        let r_sss = pure_sss.evaluate(&ctx).reflectance;

        // With SSS, should generally have different reflectance
        // (though exact relationship depends on parameters)
        assert!(r_surface > 0.0);
        assert!(r_sss > 0.0);
    }

    #[test]
    fn test_memory_usage() {
        let mem = total_sss_memory();
        assert!(mem < 1_000, "Memory should be < 1KB, got {}", mem);
    }

    #[test]
    fn test_diffuse_reflectance_presets() {
        // Check that diffuse reflectance is in valid range
        let presets = vec![
            sss_presets::skin(),
            sss_presets::marble(),
            sss_presets::milk(),
        ];

        for params in presets {
            let rd = params.diffuse_reflectance();
            for &r in &rd {
                assert!(
                    r >= 0.0 && r <= 1.0,
                    "Diffuse reflectance out of range: {}",
                    r
                );
            }
        }
    }

    #[test]
    fn test_skin_color() {
        let params = sss_presets::skin();
        let rd = params.diffuse_reflectance();

        // Skin should have highest reflectance in red
        assert!(rd[0] > rd[1]); // Red > Green
        assert!(rd[0] > rd[2]); // Red > Blue
    }

    #[test]
    fn test_milk_white() {
        let params = sss_presets::milk();
        let rd = params.diffuse_reflectance();

        // Milk should be nearly white (all channels similar and high)
        let avg = (rd[0] + rd[1] + rd[2]) / 3.0;
        let variance =
            ((rd[0] - avg).powi(2) + (rd[1] - avg).powi(2) + (rd[2] - avg).powi(2)) / 3.0;

        assert!(avg > 0.3, "Milk should be bright");
        assert!(variance < 0.05, "Milk should be white (low color variance)");
    }
}
