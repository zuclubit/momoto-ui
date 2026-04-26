//! # Anisotropic BSDF Materials
//!
//! This module provides anisotropic material models where surface roughness
//! varies with direction, enabling realistic simulation of brushed metals,
//! hair, fabric, and other directionally-dependent materials.
//!
//! ## Included Models
//!
//! - **AnisotropicBSDF**: GGX-based anisotropic microfacet model
//! - **AshikhminShirleyBSDF**: Classic anisotropic specular/diffuse model
//! - **HairBSDF**: Physically-based hair/fiber shading model
//!
//! ## Usage
//!
//! ```rust
//! use momoto_materials::glass_physics::anisotropic::{
//!     AnisotropicBSDF, HairBSDF, AshikhminShirleyBSDF
//! };
//!
//! // Brushed metal with anisotropic roughness
//! let brushed_metal = AnisotropicBSDF::new(0.1, 0.4, 1.5, 0.0);
//!
//! // Hair fiber
//! let hair = HairBSDF::new(0.5, 0.0, 0.3, 0.1, 0.0);
//! ```

use serde::{Deserialize, Serialize};
use std::f64::consts::PI;

use super::unified_bsdf::{BSDFContext, BSDFResponse, BSDFSample, Vector3, BSDF};

// ============================================================================
// Color Type for Material Responses
// ============================================================================

/// Simple RGB color representation.
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
pub struct Color {
    pub r: f64,
    pub g: f64,
    pub b: f64,
}

impl Color {
    /// Create a new color.
    pub const fn new(r: f64, g: f64, b: f64) -> Self {
        Self { r, g, b }
    }

    /// Create a grayscale color.
    pub const fn gray(v: f64) -> Self {
        Self { r: v, g: v, b: v }
    }

    /// Create white.
    pub const fn white() -> Self {
        Self::gray(1.0)
    }

    /// Create black.
    pub const fn black() -> Self {
        Self::gray(0.0)
    }

    /// Luminance (perceived brightness).
    pub fn luminance(&self) -> f64 {
        0.2126 * self.r + 0.7152 * self.g + 0.0722 * self.b
    }

    /// Scale by a factor.
    pub fn scale(&self, factor: f64) -> Self {
        Self {
            r: self.r * factor,
            g: self.g * factor,
            b: self.b * factor,
        }
    }

    /// Add two colors.
    pub fn add(&self, other: &Color) -> Self {
        Self {
            r: self.r + other.r,
            g: self.g + other.g,
            b: self.b + other.b,
        }
    }

    /// Multiply two colors component-wise.
    pub fn multiply(&self, other: &Color) -> Self {
        Self {
            r: self.r * other.r,
            g: self.g * other.g,
            b: self.b * other.b,
        }
    }
}

// ============================================================================
// Anisotropic GGX BSDF
// ============================================================================

/// Anisotropic microfacet BSDF using GGX distribution.
///
/// Suitable for brushed metals, machined surfaces, and other materials
/// with directionally-dependent roughness.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnisotropicBSDF {
    /// Roughness in tangent direction (U)
    pub alpha_x: f64,
    /// Roughness in bitangent direction (V)
    pub alpha_y: f64,
    /// Index of refraction
    pub ior: f64,
    /// Material rotation angle in radians
    pub rotation: f64,
    /// Base color (for metals)
    pub base_color: Color,
    /// Metallic factor (0 = dielectric, 1 = metal)
    pub metallic: f64,
}

impl AnisotropicBSDF {
    /// Create a new anisotropic BSDF.
    pub fn new(alpha_x: f64, alpha_y: f64, ior: f64, rotation: f64) -> Self {
        Self {
            alpha_x: alpha_x.max(0.001),
            alpha_y: alpha_y.max(0.001),
            ior,
            rotation,
            base_color: Color::white(),
            metallic: 0.0,
        }
    }

    /// Create brushed metal preset.
    pub fn brushed_metal(roughness_along: f64, roughness_across: f64) -> Self {
        Self {
            alpha_x: roughness_along.max(0.001),
            alpha_y: roughness_across.max(0.001),
            ior: 2.5,
            rotation: 0.0,
            base_color: Color::gray(0.9),
            metallic: 1.0,
        }
    }

    /// Set base color.
    pub fn with_color(mut self, color: Color) -> Self {
        self.base_color = color;
        self
    }

    /// Set metallic factor.
    pub fn with_metallic(mut self, metallic: f64) -> Self {
        self.metallic = metallic.clamp(0.0, 1.0);
        self
    }

    /// Rotate the anisotropy direction.
    fn rotate_tangent(&self, ctx: &BSDFContext) -> (Vector3, Vector3) {
        if self.rotation.abs() < 1e-6 {
            return (ctx.tangent, ctx.bitangent);
        }

        let cos_r = self.rotation.cos();
        let sin_r = self.rotation.sin();

        let new_tangent = Vector3::new(
            ctx.tangent.x * cos_r - ctx.bitangent.x * sin_r,
            ctx.tangent.y * cos_r - ctx.bitangent.y * sin_r,
            ctx.tangent.z * cos_r - ctx.bitangent.z * sin_r,
        );
        let new_bitangent = Vector3::new(
            ctx.tangent.x * sin_r + ctx.bitangent.x * cos_r,
            ctx.tangent.y * sin_r + ctx.bitangent.y * cos_r,
            ctx.tangent.z * sin_r + ctx.bitangent.z * cos_r,
        );

        (new_tangent, new_bitangent)
    }

    /// GGX anisotropic distribution function.
    fn ggx_d(&self, h: &Vector3, t: &Vector3, b: &Vector3, n: &Vector3) -> f64 {
        let h_dot_n = h.dot(n);
        if h_dot_n <= 0.0 {
            return 0.0;
        }

        let h_dot_t = h.dot(t);
        let h_dot_b = h.dot(b);

        let ax2 = self.alpha_x * self.alpha_x;
        let ay2 = self.alpha_y * self.alpha_y;

        let term = (h_dot_t * h_dot_t) / ax2 + (h_dot_b * h_dot_b) / ay2 + h_dot_n * h_dot_n;
        let denom = PI * self.alpha_x * self.alpha_y * term * term;

        if denom > 0.0 {
            1.0 / denom
        } else {
            0.0
        }
    }

    /// Smith G1 term for GGX.
    fn smith_g1(&self, v: &Vector3, h: &Vector3, t: &Vector3, b: &Vector3, n: &Vector3) -> f64 {
        let v_dot_h = v.dot(h);
        let v_dot_n = v.dot(n);

        if v_dot_h * v_dot_n <= 0.0 {
            return 0.0;
        }

        let v_dot_t = v.dot(t);
        let v_dot_b = v.dot(b);

        let ax2 = self.alpha_x * self.alpha_x;
        let ay2 = self.alpha_y * self.alpha_y;

        let tan2 = (v_dot_t * v_dot_t * ax2 + v_dot_b * v_dot_b * ay2) / (v_dot_n * v_dot_n);
        let lambda = 0.5 * (-1.0 + (1.0 + tan2).sqrt());

        1.0 / (1.0 + lambda)
    }

    /// Fresnel term using Schlick approximation.
    fn fresnel(&self, cos_theta: f64) -> f64 {
        let f0 = ((self.ior - 1.0) / (self.ior + 1.0)).powi(2);
        f0 + (1.0 - f0) * (1.0 - cos_theta).powi(5)
    }
}

impl BSDF for AnisotropicBSDF {
    fn evaluate(&self, ctx: &BSDFContext) -> BSDFResponse {
        let (t, b) = self.rotate_tangent(ctx);
        let n = &ctx.normal;
        let wo = &ctx.wo;
        let wi = &ctx.wi;

        let cos_o = wo.dot(n);
        let cos_i = wi.dot(n);

        if cos_o <= 0.0 || cos_i <= 0.0 {
            return BSDFResponse::new(0.0, 0.0, 0.0);
        }

        // Half vector
        let h = (*wo + *wi).normalize();

        // GGX distribution
        let d = self.ggx_d(&h, &t, &b, n);

        // Smith masking-shadowing
        let g1_o = self.smith_g1(wo, &h, &t, &b, n);
        let g1_i = self.smith_g1(wi, &h, &t, &b, n);
        let g = g1_o * g1_i;

        // Fresnel
        let f = self.fresnel(wo.dot(&h).abs());

        // Specular BRDF
        let denom = 4.0 * cos_o * cos_i;
        let reflectance = if denom > 1e-10 {
            (d * g * f / denom).min(1.0)
        } else {
            0.0
        };

        // For metals, modulate by base color
        let final_reflectance = if self.metallic > 0.5 {
            reflectance * self.base_color.luminance()
        } else {
            reflectance
        };

        BSDFResponse::pure_reflection(final_reflectance)
    }

    fn sample(&self, ctx: &BSDFContext, u1: f64, u2: f64) -> BSDFSample {
        // GGX importance sampling with anisotropic distribution
        let (t, b) = self.rotate_tangent(ctx);
        let n = &ctx.normal;

        // Sample half vector using GGX distribution
        let phi = 2.0 * PI * u1;
        let ax = self.alpha_x;
        let ay = self.alpha_y;

        let aspect = ay / ax;
        let phi_h = (aspect * phi.tan()).atan() + if phi > PI / 2.0 { PI } else { 0.0 };

        let cos_phi = phi_h.cos();
        let sin_phi = phi_h.sin();
        let alpha_h = 1.0 / ((cos_phi / ax).powi(2) + (sin_phi / ay).powi(2)).sqrt();

        let cos_theta_h = ((1.0 - u2) / (u2 * (alpha_h * alpha_h - 1.0) + 1.0)).sqrt();
        let sin_theta_h = (1.0 - cos_theta_h * cos_theta_h).sqrt();

        let h = Vector3::new(
            sin_theta_h * cos_phi * t.x + sin_theta_h * sin_phi * b.x + cos_theta_h * n.x,
            sin_theta_h * cos_phi * t.y + sin_theta_h * sin_phi * b.y + cos_theta_h * n.y,
            sin_theta_h * cos_phi * t.z + sin_theta_h * sin_phi * b.z + cos_theta_h * n.z,
        );

        // Reflect wo around h
        let wo_dot_h = ctx.wo.dot(&h);
        let sampled_wo = ctx.wo.reflect(&h);

        let pdf = self.ggx_d(&h, &t, &b, n) * cos_theta_h / (4.0 * wo_dot_h.abs());
        let value = self.evaluate(&BSDFContext {
            wi: sampled_wo,
            ..ctx.clone()
        });

        BSDFSample::new(sampled_wo, value, pdf.max(1e-10), false)
    }

    fn pdf(&self, ctx: &BSDFContext) -> f64 {
        let (t, b) = self.rotate_tangent(ctx);
        let n = &ctx.normal;
        let h = (ctx.wo + ctx.wi).normalize();

        let d = self.ggx_d(&h, &t, &b, n);
        let cos_theta_h = h.dot(n);
        let wo_dot_h = ctx.wo.dot(&h);

        d * cos_theta_h / (4.0 * wo_dot_h.abs())
    }
}

// ============================================================================
// Ashikhmin-Shirley BSDF
// ============================================================================

/// Ashikhmin-Shirley anisotropic BSDF.
///
/// A classic model combining anisotropic specular with diffuse reflection.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AshikhminShirleyBSDF {
    /// Diffuse color
    pub diffuse: Color,
    /// Specular coefficient
    pub specular: f64,
    /// Phong exponent in U direction
    pub nu: f64,
    /// Phong exponent in V direction
    pub nv: f64,
}

impl AshikhminShirleyBSDF {
    /// Create a new Ashikhmin-Shirley BSDF.
    pub fn new(diffuse: Color, specular: f64, nu: f64, nv: f64) -> Self {
        Self {
            diffuse,
            specular: specular.clamp(0.0, 1.0),
            nu: nu.max(1.0),
            nv: nv.max(1.0),
        }
    }

    /// Create silk-like material.
    pub fn silk(color: Color) -> Self {
        Self::new(color, 0.3, 100.0, 10.0)
    }

    /// Create satin-like material.
    pub fn satin(color: Color) -> Self {
        Self::new(color, 0.5, 50.0, 5.0)
    }

    /// Fresnel term.
    fn fresnel_schlick(&self, cos_theta: f64) -> f64 {
        self.specular + (1.0 - self.specular) * (1.0 - cos_theta).powi(5)
    }

    /// Anisotropic specular distribution.
    fn specular_d(&self, h: &Vector3, t: &Vector3, b: &Vector3, n: &Vector3) -> f64 {
        let h_dot_n = h.dot(n);
        if h_dot_n <= 0.0 {
            return 0.0;
        }

        let h_dot_t = h.dot(t);
        let h_dot_b = h.dot(b);

        let exponent = (self.nu * h_dot_t * h_dot_t + self.nv * h_dot_b * h_dot_b)
            / (1.0 - h_dot_n * h_dot_n + 1e-10);

        let norm = ((self.nu + 1.0) * (self.nv + 1.0)).sqrt() / (8.0 * PI);

        norm * h_dot_n.powf(exponent)
    }
}

impl BSDF for AshikhminShirleyBSDF {
    fn evaluate(&self, ctx: &BSDFContext) -> BSDFResponse {
        let n = &ctx.normal;
        let t = &ctx.tangent;
        let b = &ctx.bitangent;

        let cos_o = ctx.wo.dot(n);
        let cos_i = ctx.wi.dot(n);

        if cos_o <= 0.0 || cos_i <= 0.0 {
            return BSDFResponse::new(0.0, 0.0, 0.0);
        }

        // Half vector for specular
        let h = (ctx.wo + ctx.wi).normalize();
        let h_dot_i = h.dot(&ctx.wi).abs();

        // Specular term
        let d = self.specular_d(&h, t, b, n);
        let f = self.fresnel_schlick(h_dot_i);
        let spec = d * f / (h_dot_i * cos_o.max(cos_i));

        // Diffuse term (energy conserving)
        let diff_factor = 28.0 / (23.0 * PI);
        let diff = self.diffuse.luminance()
            * diff_factor
            * (1.0 - self.specular)
            * (1.0 - (1.0 - cos_i / 2.0).powi(5))
            * (1.0 - (1.0 - cos_o / 2.0).powi(5));

        let total = (spec + diff).min(1.0);

        BSDFResponse::pure_reflection(total)
    }

    fn sample(&self, ctx: &BSDFContext, u1: f64, u2: f64) -> BSDFSample {
        // Simple cosine-weighted hemisphere sampling
        let phi = 2.0 * PI * u1;
        let cos_theta = (1.0 - u2).sqrt();
        let sin_theta = (u2).sqrt();

        let wo = Vector3::new(sin_theta * phi.cos(), sin_theta * phi.sin(), cos_theta);

        let pdf = cos_theta / PI;
        let value = self.evaluate(&BSDFContext {
            wi: wo,
            ..ctx.clone()
        });

        BSDFSample::new(wo, value, pdf.max(1e-10), false)
    }

    fn pdf(&self, ctx: &BSDFContext) -> f64 {
        let cos_i = ctx.wi.dot(&ctx.normal);
        if cos_i > 0.0 {
            cos_i / PI
        } else {
            0.0
        }
    }
}

// ============================================================================
// Hair BSDF
// ============================================================================

/// Physically-based hair/fiber shading model.
///
/// Based on the Marschner hair model with extensions for melanin-based color.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HairBSDF {
    /// Melanin concentration (0 = white/blonde, 1 = black)
    pub melanin: f64,
    /// Melanin redness (0 = eumelanin/brown-black, 1 = pheomelanin/red)
    pub melanin_redness: f64,
    /// Longitudinal roughness
    pub roughness: f64,
    /// Radial roughness
    pub radial_roughness: f64,
    /// Coat/cuticle layer strength
    pub coat: f64,
}

impl HairBSDF {
    /// Create a new hair BSDF.
    pub fn new(
        melanin: f64,
        melanin_redness: f64,
        roughness: f64,
        radial_roughness: f64,
        coat: f64,
    ) -> Self {
        Self {
            melanin: melanin.clamp(0.0, 1.0),
            melanin_redness: melanin_redness.clamp(0.0, 1.0),
            roughness: roughness.clamp(0.01, 1.0),
            radial_roughness: radial_roughness.clamp(0.01, 1.0),
            coat: coat.clamp(0.0, 1.0),
        }
    }

    /// Create blonde hair.
    pub fn blonde() -> Self {
        Self::new(0.2, 0.3, 0.3, 0.15, 0.0)
    }

    /// Create brown hair.
    pub fn brown() -> Self {
        Self::new(0.5, 0.1, 0.3, 0.15, 0.0)
    }

    /// Create black hair.
    pub fn black() -> Self {
        Self::new(0.9, 0.0, 0.3, 0.15, 0.0)
    }

    /// Create red hair.
    pub fn red() -> Self {
        Self::new(0.4, 0.8, 0.3, 0.15, 0.0)
    }

    /// Create white/gray hair.
    pub fn white() -> Self {
        Self::new(0.0, 0.0, 0.4, 0.2, 0.0)
    }

    /// Calculate absorption coefficient from melanin.
    fn absorption_coefficient(&self) -> Color {
        // Melanin-based absorption
        let eumelanin = self.melanin * (1.0 - self.melanin_redness);
        let pheomelanin = self.melanin * self.melanin_redness;

        // Absorption coefficients at RGB wavelengths
        Color::new(
            0.419 * eumelanin + 0.187 * pheomelanin,
            0.697 * eumelanin + 0.145 * pheomelanin,
            1.37 * eumelanin + 0.0 * pheomelanin,
        )
    }

    /// Longitudinal scattering function (M).
    fn m_term(&self, theta_i: f64, theta_r: f64, v: f64) -> f64 {
        let theta_d = theta_r - theta_i;
        let theta_h = (theta_r + theta_i) / 2.0;

        let gaussian = |x: f64, sigma: f64| {
            (-x * x / (2.0 * sigma * sigma)).exp() / (sigma * (2.0 * PI).sqrt())
        };

        let v_scaled = v * self.roughness;
        gaussian(theta_h, v_scaled)
    }

    /// Azimuthal scattering function (N).
    fn n_term(&self, phi: f64, eta: f64, h: f64) -> f64 {
        // Simplified azimuthal distribution
        let gamma = phi / 2.0;
        let cos_gamma = gamma.cos();
        let fresnel = 0.04 + 0.96 * (1.0 - cos_gamma).powi(5);

        (1.0 - fresnel) * cos_gamma.abs().max(0.01)
    }
}

impl BSDF for HairBSDF {
    fn evaluate(&self, ctx: &BSDFContext) -> BSDFResponse {
        // Hair is essentially a cylinder - use tangent as hair direction
        let hair_dir = &ctx.tangent;

        // Project directions onto plane perpendicular to hair
        let sin_theta_i = ctx.wi.dot(hair_dir);
        let sin_theta_o = ctx.wo.dot(hair_dir);

        let theta_i = sin_theta_i.asin();
        let theta_o = sin_theta_o.asin();

        // R lobe (primary specular)
        let eta = 1.55; // IOR of keratin
        let h = (-1.0f64).max(-0.8); // Simplified

        // Longitudinal roughness
        let v = 0.726 * self.roughness + 0.812 * self.roughness * self.roughness + 3.7e-3;

        // R lobe contribution
        let m_r = self.m_term(theta_i, theta_o, v);
        let n_r = self.n_term(0.0, eta, h);
        let r_lobe = m_r * n_r;

        // TT lobe (transmission-transmission) - light through hair
        let absorption = self.absorption_coefficient();
        let transmittance = (1.0 - absorption.luminance()).powi(2);
        let tt_lobe = m_r * transmittance * 0.5;

        // TRT lobe (internal reflection)
        let trt_lobe = m_r * transmittance * 0.25;

        // Coat layer (if any)
        let coat_contrib = if self.coat > 0.0 {
            let fresnel = 0.04 + 0.96 * (1.0 - ctx.wo.dot(&ctx.normal).abs()).powi(5);
            self.coat * fresnel
        } else {
            0.0
        };

        let total = (r_lobe + tt_lobe + trt_lobe + coat_contrib).clamp(0.0, 1.0);

        BSDFResponse::pure_reflection(total)
    }

    fn sample(&self, ctx: &BSDFContext, u1: f64, u2: f64) -> BSDFSample {
        // Importance sample based on hair direction
        let _hair_dir = &ctx.tangent;

        // Sample around specular reflection
        let _phi = 2.0 * PI * u1;
        let v = 0.726 * self.roughness + 0.812 * self.roughness * self.roughness + 3.7e-3;
        let theta = v * (2.0 * u2 - 1.0);

        let sin_theta = theta.sin();
        let cos_theta = theta.cos();

        // Create sampled direction
        let reflected = ctx.wo.reflect(&ctx.normal);
        let wo = Vector3::new(
            reflected.x * cos_theta + ctx.normal.x * sin_theta,
            reflected.y * cos_theta + ctx.normal.y * sin_theta,
            reflected.z * cos_theta + ctx.normal.z * sin_theta,
        );

        let pdf = 1.0 / (2.0 * PI * v);
        let value = self.evaluate(&BSDFContext {
            wi: wo,
            ..ctx.clone()
        });

        BSDFSample::new(wo, value, pdf.max(1e-10), false)
    }

    fn pdf(&self, ctx: &BSDFContext) -> f64 {
        let v = 0.726 * self.roughness + 0.812 * self.roughness * self.roughness + 3.7e-3;
        1.0 / (2.0 * PI * v)
    }
}

// ============================================================================
// Presets
// ============================================================================

/// Get a brushed aluminum material.
pub fn brushed_aluminum() -> AnisotropicBSDF {
    AnisotropicBSDF::brushed_metal(0.05, 0.3)
        .with_color(Color::new(0.91, 0.92, 0.92))
        .with_metallic(1.0)
}

/// Get a brushed stainless steel material.
pub fn brushed_stainless() -> AnisotropicBSDF {
    AnisotropicBSDF::brushed_metal(0.08, 0.4)
        .with_color(Color::new(0.59, 0.55, 0.54))
        .with_metallic(1.0)
}

/// Get a brushed copper material.
pub fn brushed_copper() -> AnisotropicBSDF {
    AnisotropicBSDF::brushed_metal(0.1, 0.35)
        .with_color(Color::new(0.95, 0.64, 0.54))
        .with_metallic(1.0)
}

/// Get a CD surface (radial anisotropy).
pub fn cd_surface() -> AnisotropicBSDF {
    AnisotropicBSDF::new(0.01, 0.5, 1.5, 0.0)
        .with_color(Color::white())
        .with_metallic(0.9)
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_anisotropic_bsdf() {
        let bsdf = AnisotropicBSDF::new(0.1, 0.4, 1.5, 0.0);
        let ctx = BSDFContext::new_simple(0.7);
        let response = bsdf.evaluate(&ctx);

        assert!(response.reflectance >= 0.0);
        assert!(response.reflectance <= 1.0);
        assert!(response.is_energy_conserved(1e-6));
    }

    #[test]
    fn test_brushed_metal() {
        let bsdf = brushed_aluminum();
        let ctx = BSDFContext::new_simple(0.9);
        let response = bsdf.evaluate(&ctx);

        assert!(response.reflectance > 0.0);
        assert!(response.is_energy_conserved(1e-6));
    }

    #[test]
    fn test_ashikhmin_shirley() {
        let bsdf = AshikhminShirleyBSDF::silk(Color::new(0.8, 0.2, 0.2));
        let ctx = BSDFContext::new_simple(0.7);
        let response = bsdf.evaluate(&ctx);

        assert!(response.reflectance >= 0.0);
        assert!(response.is_energy_conserved(1e-6));
    }

    #[test]
    fn test_hair_bsdf() {
        let hair = HairBSDF::brown();
        let ctx = BSDFContext::new_simple(0.5);
        let response = hair.evaluate(&ctx);

        assert!(response.reflectance >= 0.0);
        assert!(response.reflectance <= 1.0);
    }

    #[test]
    fn test_hair_presets() {
        let presets = vec![
            HairBSDF::blonde(),
            HairBSDF::brown(),
            HairBSDF::black(),
            HairBSDF::red(),
            HairBSDF::white(),
        ];

        let ctx = BSDFContext::new_simple(0.6);
        for hair in presets {
            let response = hair.evaluate(&ctx);
            assert!(response.is_energy_conserved(1e-6));
        }
    }

    #[test]
    fn test_anisotropic_sampling() {
        let bsdf = AnisotropicBSDF::new(0.2, 0.4, 1.5, 0.0);
        let ctx = BSDFContext::new_simple(0.7);

        let sample = bsdf.sample(&ctx, 0.5, 0.5);
        assert!(sample.pdf > 0.0);
        assert!(sample.wo.length() > 0.99 && sample.wo.length() < 1.01);
    }
}

// ============================================================================
// Preset Module
// ============================================================================

/// Material presets for common anisotropic materials.
pub mod presets {
    use super::*;

    /// Brushed aluminum preset.
    pub fn brushed_aluminum() -> AnisotropicBSDF {
        super::brushed_aluminum()
    }

    /// Brushed stainless steel preset.
    pub fn brushed_stainless() -> AnisotropicBSDF {
        super::brushed_stainless()
    }

    /// Brushed copper preset.
    pub fn brushed_copper() -> AnisotropicBSDF {
        super::brushed_copper()
    }

    /// CD/DVD surface with radial anisotropy.
    pub fn cd_surface() -> AnisotropicBSDF {
        super::cd_surface()
    }

    /// Silk fabric preset.
    pub fn silk() -> AshikhminShirleyBSDF {
        AshikhminShirleyBSDF::silk(Color::new(0.95, 0.90, 0.85))
    }

    /// Red silk preset.
    pub fn red_silk() -> AshikhminShirleyBSDF {
        AshikhminShirleyBSDF::silk(Color::new(0.8, 0.1, 0.1))
    }

    /// Satin fabric preset.
    pub fn satin() -> AshikhminShirleyBSDF {
        AshikhminShirleyBSDF::satin(Color::new(0.85, 0.85, 0.9))
    }

    /// Blonde hair preset.
    pub fn hair_blonde() -> HairBSDF {
        HairBSDF::blonde()
    }

    /// Brown hair preset.
    pub fn hair_brown() -> HairBSDF {
        HairBSDF::brown()
    }

    /// Black hair preset.
    pub fn hair_black() -> HairBSDF {
        HairBSDF::black()
    }

    /// Red hair preset.
    pub fn hair_red() -> HairBSDF {
        HairBSDF::red()
    }

    /// White/gray hair preset.
    pub fn hair_white() -> HairBSDF {
        HairBSDF::white()
    }

    /// List all available preset names.
    pub fn list_presets() -> Vec<&'static str> {
        vec![
            "brushed_aluminum",
            "brushed_stainless",
            "brushed_copper",
            "cd_surface",
            "silk",
            "red_silk",
            "satin",
            "hair_blonde",
            "hair_brown",
            "hair_black",
            "hair_red",
            "hair_white",
        ]
    }
}

// ============================================================================
// Memory Estimation
// ============================================================================

/// Estimate memory usage for anisotropic material types.
pub fn estimate_anisotropic_memory() -> usize {
    // AnisotropicBSDF: 6 f64 fields + Color (3 f64)
    let anisotropic_size = std::mem::size_of::<AnisotropicBSDF>();
    // AshikhminShirleyBSDF: Color (3 f64) + 3 f64 fields
    let ashikhmin_size = std::mem::size_of::<AshikhminShirleyBSDF>();
    // HairBSDF: 5 f64 fields
    let hair_size = std::mem::size_of::<HairBSDF>();

    anisotropic_size + ashikhmin_size + hair_size
}
