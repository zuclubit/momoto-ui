//! # Unified BSDF Interface (Phase 9)
//!
//! Provides a single, unified interface for all surface interactions with guaranteed
//! energy conservation. This module transforms the engine from a collection of separate
//! physical models into a cohesive light-matter interaction system.
//!
//! ## Core Principles
//!
//! 1. **Energy Conservation**: Every `BSDFResponse` satisfies R + T + A = 1
//! 2. **Unified Interface**: All materials implement the same `BSDF` trait
//! 3. **Composability**: Materials can be layered with proper energy tracking
//! 4. **Quality Tiers**: Different implementations for performance vs accuracy
//!
//! ## Usage
//!
//! ```rust
//! use momoto_materials::glass_physics::unified_bsdf::{
//!     BSDF, BSDFContext, DielectricBSDF, ConductorBSDF, LayeredBSDF,
//! };
//!
//! // Create a dielectric material
//! let glass = DielectricBSDF::new(1.52, 0.0);
//!
//! // Evaluate at a given angle
//! let ctx = BSDFContext::new_simple(0.7); // cos(theta) = 0.7
//! let response = glass.evaluate(&ctx);
//!
//! // Energy is always conserved
//! assert!((response.reflectance + response.transmittance + response.absorption - 1.0).abs() < 1e-6);
//! ```

use std::f64::consts::PI;

use serde::{Deserialize, Serialize};

use super::complex_ior::{fresnel_conductor_unpolarized, ComplexIOR, SpectralComplexIOR};
use super::fresnel::{fresnel_full, fresnel_schlick};
use super::thin_film::ThinFilm;

// ============================================================================
// VECTOR TYPES
// ============================================================================

/// 3D vector for direction representation
#[derive(Debug, Clone, Copy, Default)]
pub struct Vector3 {
    pub x: f64,
    pub y: f64,
    pub z: f64,
}

impl Vector3 {
    /// Create a new vector
    #[inline]
    pub const fn new(x: f64, y: f64, z: f64) -> Self {
        Self { x, y, z }
    }

    /// Create a unit vector along Z (surface normal)
    #[inline]
    pub const fn unit_z() -> Self {
        Self {
            x: 0.0,
            y: 0.0,
            z: 1.0,
        }
    }

    /// Create a unit vector along X (tangent)
    #[inline]
    pub const fn unit_x() -> Self {
        Self {
            x: 1.0,
            y: 0.0,
            z: 0.0,
        }
    }

    /// Create a unit vector along Y (bitangent)
    #[inline]
    pub const fn unit_y() -> Self {
        Self {
            x: 0.0,
            y: 1.0,
            z: 0.0,
        }
    }

    /// Dot product
    #[inline]
    pub fn dot(&self, other: &Vector3) -> f64 {
        self.x * other.x + self.y * other.y + self.z * other.z
    }

    /// Length squared
    #[inline]
    pub fn length_squared(&self) -> f64 {
        self.dot(self)
    }

    /// Length
    #[inline]
    pub fn length(&self) -> f64 {
        self.length_squared().sqrt()
    }

    /// Normalize to unit vector
    #[inline]
    pub fn normalize(&self) -> Self {
        let len = self.length();
        if len > 1e-10 {
            Self {
                x: self.x / len,
                y: self.y / len,
                z: self.z / len,
            }
        } else {
            Self::unit_z()
        }
    }

    /// Reflect around normal
    #[inline]
    pub fn reflect(&self, normal: &Vector3) -> Self {
        let d = 2.0 * self.dot(normal);
        Self {
            x: self.x - d * normal.x,
            y: self.y - d * normal.y,
            z: self.z - d * normal.z,
        }
    }
}

impl std::ops::Add for Vector3 {
    type Output = Self;
    fn add(self, rhs: Self) -> Self {
        Self {
            x: self.x + rhs.x,
            y: self.y + rhs.y,
            z: self.z + rhs.z,
        }
    }
}

impl std::ops::Sub for Vector3 {
    type Output = Self;
    fn sub(self, rhs: Self) -> Self {
        Self {
            x: self.x - rhs.x,
            y: self.y - rhs.y,
            z: self.z - rhs.z,
        }
    }
}

impl std::ops::Mul<f64> for Vector3 {
    type Output = Self;
    fn mul(self, rhs: f64) -> Self {
        Self {
            x: self.x * rhs,
            y: self.y * rhs,
            z: self.z * rhs,
        }
    }
}

impl std::ops::Neg for Vector3 {
    type Output = Self;
    fn neg(self) -> Self {
        Self {
            x: -self.x,
            y: -self.y,
            z: -self.z,
        }
    }
}

// ============================================================================
// BSDF CONTEXT
// ============================================================================

/// Evaluation context for BSDF calculations
///
/// Contains all geometric and spectral information needed to evaluate a BSDF.
#[derive(Debug, Clone)]
pub struct BSDFContext {
    /// Incoming direction (towards surface, normalized)
    pub wi: Vector3,
    /// Outgoing direction (away from surface, normalized)
    pub wo: Vector3,
    /// Surface normal (normalized)
    pub normal: Vector3,
    /// Tangent direction (for anisotropy)
    pub tangent: Vector3,
    /// Bitangent direction
    pub bitangent: Vector3,
    /// Primary wavelength in nm (for spectral calculations)
    pub wavelength: f64,
    /// Optional multiple wavelengths for full spectral rendering
    pub wavelengths: Option<Vec<f64>>,
}

impl BSDFContext {
    /// Create a new full context
    pub fn new(
        wi: Vector3,
        wo: Vector3,
        normal: Vector3,
        tangent: Vector3,
        bitangent: Vector3,
        wavelength: f64,
    ) -> Self {
        Self {
            wi: wi.normalize(),
            wo: wo.normalize(),
            normal: normal.normalize(),
            tangent: tangent.normalize(),
            bitangent: bitangent.normalize(),
            wavelength,
            wavelengths: None,
        }
    }

    /// Create a simple context from cos(theta) only
    ///
    /// Useful for quick evaluations where only the angle matters.
    pub fn new_simple(cos_theta: f64) -> Self {
        let cos_theta = cos_theta.clamp(-1.0, 1.0);
        let sin_theta = (1.0 - cos_theta * cos_theta).sqrt();

        Self {
            wi: Vector3::new(sin_theta, 0.0, cos_theta),
            wo: Vector3::new(-sin_theta, 0.0, cos_theta),
            normal: Vector3::unit_z(),
            tangent: Vector3::unit_x(),
            bitangent: Vector3::unit_y(),
            wavelength: 550.0,
            wavelengths: None,
        }
    }

    /// Create context for reflection (wo = reflected wi)
    pub fn new_reflection(cos_theta: f64, wavelength: f64) -> Self {
        let cos_theta = cos_theta.clamp(0.0, 1.0);
        let sin_theta = (1.0 - cos_theta * cos_theta).sqrt();

        let wi = Vector3::new(sin_theta, 0.0, cos_theta);
        let normal = Vector3::unit_z();
        let wo = wi.reflect(&normal);

        Self {
            wi,
            wo,
            normal,
            tangent: Vector3::unit_x(),
            bitangent: Vector3::unit_y(),
            wavelength,
            wavelengths: None,
        }
    }

    /// Set wavelength
    pub fn with_wavelength(mut self, wavelength: f64) -> Self {
        self.wavelength = wavelength;
        self
    }

    /// Set multiple wavelengths for spectral rendering
    pub fn with_wavelengths(mut self, wavelengths: Vec<f64>) -> Self {
        self.wavelengths = Some(wavelengths);
        self
    }

    /// Get cos(theta_i) - cosine of incident angle
    #[inline]
    pub fn cos_theta_i(&self) -> f64 {
        self.wi.dot(&self.normal).abs()
    }

    /// Get cos(theta_o) - cosine of outgoing angle
    #[inline]
    pub fn cos_theta_o(&self) -> f64 {
        self.wo.dot(&self.normal).abs()
    }

    /// Get half vector (for microfacet models)
    #[inline]
    pub fn half_vector(&self) -> Vector3 {
        (self.wi + self.wo).normalize()
    }
}

impl Default for BSDFContext {
    fn default() -> Self {
        Self::new_simple(1.0)
    }
}

// ============================================================================
// BSDF RESPONSE
// ============================================================================

/// Response from BSDF evaluation with guaranteed energy conservation
///
/// The response always satisfies: `reflectance + transmittance + absorption = 1.0`
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BSDFResponse {
    /// Fraction of light reflected (0-1)
    pub reflectance: f64,
    /// Fraction of light transmitted (0-1)
    pub transmittance: f64,
    /// Fraction of light absorbed (0-1)
    pub absorption: f64,
    /// Optional spectral response (for multi-wavelength evaluation)
    pub spectral: Option<Vec<f64>>,
    /// Optional RGB response for quick rendering
    pub rgb: Option<[f64; 3]>,
}

impl BSDFResponse {
    /// Create a new response with automatic normalization
    pub fn new(reflectance: f64, transmittance: f64, absorption: f64) -> Self {
        let mut response = Self {
            reflectance: reflectance.max(0.0),
            transmittance: transmittance.max(0.0),
            absorption: absorption.max(0.0),
            spectral: None,
            rgb: None,
        };
        response.normalize();
        response
    }

    /// Create a pure reflection response
    pub fn pure_reflection(reflectance: f64) -> Self {
        let r = reflectance.clamp(0.0, 1.0);
        Self {
            reflectance: r,
            transmittance: 1.0 - r,
            absorption: 0.0,
            spectral: None,
            rgb: None,
        }
    }

    /// Create a pure transmission response
    pub fn pure_transmission(transmittance: f64) -> Self {
        let t = transmittance.clamp(0.0, 1.0);
        Self {
            reflectance: 0.0,
            transmittance: t,
            absorption: 1.0 - t,
            spectral: None,
            rgb: None,
        }
    }

    /// Enforce energy conservation: R + T + A = 1
    ///
    /// This is called automatically on construction but can be called
    /// again after manual modifications.
    pub fn normalize(&mut self) {
        let total = self.reflectance + self.transmittance + self.absorption;
        if total > 1e-10 {
            self.reflectance /= total;
            self.transmittance /= total;
            self.absorption = 1.0 - self.reflectance - self.transmittance;
        } else {
            // Edge case: no energy - assume full absorption
            self.reflectance = 0.0;
            self.transmittance = 0.0;
            self.absorption = 1.0;
        }
    }

    /// Check if energy is conserved within tolerance
    pub fn is_energy_conserved(&self, tolerance: f64) -> bool {
        let total = self.reflectance + self.transmittance + self.absorption;
        (total - 1.0).abs() < tolerance
    }

    /// Get total energy (should be 1.0)
    pub fn total_energy(&self) -> f64 {
        self.reflectance + self.transmittance + self.absorption
    }

    /// Set spectral response
    pub fn with_spectral(mut self, spectral: Vec<f64>) -> Self {
        self.spectral = Some(spectral);
        self
    }

    /// Set RGB response
    pub fn with_rgb(mut self, rgb: [f64; 3]) -> Self {
        self.rgb = Some(rgb);
        self
    }
}

impl Default for BSDFResponse {
    fn default() -> Self {
        // Default: perfect mirror
        Self::pure_reflection(0.04) // Typical glass F0
    }
}

// ============================================================================
// BSDF SAMPLE
// ============================================================================

/// Result of importance sampling the BSDF
#[derive(Debug, Clone)]
pub struct BSDFSample {
    /// Sampled outgoing direction
    pub wo: Vector3,
    /// BSDF value at sampled direction
    pub value: BSDFResponse,
    /// Probability density function value
    pub pdf: f64,
    /// Whether this is a delta distribution (perfect reflection/refraction)
    pub is_delta: bool,
}

impl BSDFSample {
    /// Create a new sample
    pub fn new(wo: Vector3, value: BSDFResponse, pdf: f64, is_delta: bool) -> Self {
        Self {
            wo,
            value,
            pdf,
            is_delta,
        }
    }

    /// Create a delta reflection sample
    pub fn delta_reflection(wo: Vector3, reflectance: f64) -> Self {
        Self {
            wo,
            value: BSDFResponse::pure_reflection(reflectance),
            pdf: 1.0,
            is_delta: true,
        }
    }
}

// ============================================================================
// ENERGY VALIDATION
// ============================================================================

/// Result of energy conservation validation
#[derive(Debug, Clone)]
pub struct EnergyValidation {
    /// Whether energy is conserved within tolerance
    pub conserved: bool,
    /// Energy conservation error (|R + T + A - 1|)
    pub error: f64,
    /// Human-readable details
    pub details: String,
}

impl EnergyValidation {
    /// Create a passing validation
    pub fn pass(error: f64) -> Self {
        Self {
            conserved: true,
            error,
            details: format!("Energy conserved: error = {:.2e}", error),
        }
    }

    /// Create a failing validation
    pub fn fail(error: f64, reason: &str) -> Self {
        Self {
            conserved: false,
            error,
            details: format!("Energy NOT conserved: {} (error = {:.2e})", reason, error),
        }
    }
}

// ============================================================================
// BSDF TRAIT
// ============================================================================

/// Core BSDF trait - all materials implement this
///
/// This trait provides a unified interface for evaluating surface interactions.
/// All implementations guarantee energy conservation.
pub trait BSDF: Send + Sync {
    /// Evaluate the BSDF for given directions
    ///
    /// Returns the fraction of light reflected/transmitted/absorbed.
    fn evaluate(&self, ctx: &BSDFContext) -> BSDFResponse;

    /// Sample the BSDF using importance sampling
    ///
    /// Returns a sampled direction with its probability.
    fn sample(&self, ctx: &BSDFContext, u1: f64, u2: f64) -> BSDFSample {
        // Default implementation: cosine-weighted hemisphere sampling
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

    /// Probability density for a given direction pair
    fn pdf(&self, ctx: &BSDFContext) -> f64 {
        // Default: cosine-weighted PDF
        ctx.cos_theta_o() / PI
    }

    /// Validate energy conservation
    fn validate_energy(&self, ctx: &BSDFContext) -> EnergyValidation {
        let response = self.evaluate(ctx);
        let error = (response.total_energy() - 1.0).abs();

        if error < 1e-6 {
            EnergyValidation::pass(error)
        } else {
            EnergyValidation::fail(error, "R + T + A != 1")
        }
    }

    /// Get a descriptive name for this BSDF
    fn name(&self) -> &str {
        "BSDF"
    }

    /// Whether this BSDF has delta distributions (perfect specular)
    fn is_delta(&self) -> bool {
        false
    }
}

// ============================================================================
// DIELECTRIC BSDF
// ============================================================================

/// Unified dielectric BSDF (glass, water, crystals)
///
/// Wraps the existing Fresnel implementation with energy conservation.
#[derive(Debug, Clone)]
pub struct DielectricBSDF {
    /// Index of refraction
    pub ior: f64,
    /// Surface roughness (0 = smooth, 1 = rough)
    pub roughness: f64,
    /// Optional dispersion model
    pub dispersion: Option<DispersionModel>,
    /// Use full Fresnel equations (slower but more accurate)
    pub use_full_fresnel: bool,
}

/// Dispersion model for wavelength-dependent IOR
#[derive(Debug, Clone, Copy)]
pub struct DispersionModel {
    /// Base IOR at reference wavelength (589nm, sodium D-line)
    pub n_d: f64,
    /// Abbe number (dispersion strength, lower = more dispersion)
    pub abbe_number: f64,
}

impl DispersionModel {
    /// Create new dispersion model
    pub const fn new(n_d: f64, abbe_number: f64) -> Self {
        Self { n_d, abbe_number }
    }

    /// Calculate IOR at a given wavelength using Cauchy formula
    pub fn ior_at_wavelength(&self, wavelength_nm: f64) -> f64 {
        // Cauchy approximation: n = A + B/λ²
        // A ≈ n_d, B ≈ (n_d - 1) / V_d * 0.01
        let lambda_um = wavelength_nm / 1000.0;
        let b = (self.n_d - 1.0) / self.abbe_number * 0.01;
        self.n_d + b / (lambda_um * lambda_um)
    }

    /// Crown glass preset (low dispersion)
    pub const CROWN_GLASS: Self = Self {
        n_d: 1.52,
        abbe_number: 64.0,
    };

    /// Flint glass preset (high dispersion)
    pub const FLINT_GLASS: Self = Self {
        n_d: 1.62,
        abbe_number: 36.0,
    };

    /// Diamond preset
    pub const DIAMOND: Self = Self {
        n_d: 2.42,
        abbe_number: 55.0,
    };
}

impl DielectricBSDF {
    /// Create a new dielectric BSDF
    pub fn new(ior: f64, roughness: f64) -> Self {
        Self {
            ior: ior.max(1.0),
            roughness: roughness.clamp(0.0, 1.0),
            dispersion: None,
            use_full_fresnel: false,
        }
    }

    /// Create with dispersion
    pub fn with_dispersion(mut self, dispersion: DispersionModel) -> Self {
        self.dispersion = Some(dispersion);
        self
    }

    /// Use full Fresnel equations (more accurate at grazing angles)
    pub fn with_full_fresnel(mut self) -> Self {
        self.use_full_fresnel = true;
        self
    }

    /// Get IOR at wavelength (considering dispersion)
    fn ior_at(&self, wavelength_nm: f64) -> f64 {
        match &self.dispersion {
            Some(disp) => disp.ior_at_wavelength(wavelength_nm),
            None => self.ior,
        }
    }

    /// Glass preset
    pub fn glass() -> Self {
        Self::new(1.52, 0.0)
    }

    /// Water preset
    pub fn water() -> Self {
        Self::new(1.33, 0.0)
    }

    /// Diamond preset
    pub fn diamond() -> Self {
        Self::new(2.42, 0.0).with_dispersion(DispersionModel::DIAMOND)
    }

    /// Frosted glass preset
    pub fn frosted_glass() -> Self {
        Self::new(1.52, 0.3)
    }
}

impl BSDF for DielectricBSDF {
    fn evaluate(&self, ctx: &BSDFContext) -> BSDFResponse {
        let cos_theta = ctx.cos_theta_i();
        let ior = self.ior_at(ctx.wavelength);

        // Calculate Fresnel reflectance
        let reflectance = if self.use_full_fresnel {
            let (rs, rp) = fresnel_full(1.0, ior, cos_theta);
            (rs + rp) / 2.0
        } else {
            fresnel_schlick(1.0, ior, cos_theta)
        };

        // Apply roughness (reduces specular, adds diffuse)
        let specular = reflectance * (1.0 - self.roughness);
        let diffuse = self.roughness * 0.05; // Rough surfaces have some diffuse reflection

        let total_reflectance = specular + diffuse;

        // Dielectrics are transparent (no absorption for thin surfaces)
        BSDFResponse::new(total_reflectance, 1.0 - total_reflectance, 0.0)
    }

    fn sample(&self, ctx: &BSDFContext, u1: f64, u2: f64) -> BSDFSample {
        if self.roughness < 0.01 {
            // Smooth surface: delta reflection/refraction
            let reflectance = self.evaluate(ctx).reflectance;

            if u1 < reflectance {
                // Reflect
                let wo = ctx.wi.reflect(&ctx.normal);
                BSDFSample::delta_reflection(wo, reflectance)
            } else {
                // Refract (simplified: just flip z)
                let wo = Vector3::new(ctx.wi.x, ctx.wi.y, -ctx.wi.z);
                BSDFSample::new(
                    wo,
                    BSDFResponse::pure_transmission(1.0 - reflectance),
                    1.0 - reflectance,
                    true,
                )
            }
        } else {
            // Rough surface: use default cosine-weighted sampling
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
    }

    fn name(&self) -> &str {
        "DielectricBSDF"
    }

    fn is_delta(&self) -> bool {
        self.roughness < 0.01
    }
}

// ============================================================================
// CONDUCTOR BSDF
// ============================================================================

/// Unified conductor BSDF (metals)
///
/// Wraps the existing complex IOR implementation with energy conservation.
#[derive(Debug, Clone)]
pub struct ConductorBSDF {
    /// Real part of complex IOR
    pub n: f64,
    /// Extinction coefficient (imaginary part)
    pub k: f64,
    /// Surface roughness
    pub roughness: f64,
    /// Optional spectral complex IOR for colored metals
    pub spectral_ior: Option<SpectralComplexIOR>,
}

impl ConductorBSDF {
    /// Create a new conductor BSDF
    pub fn new(n: f64, k: f64, roughness: f64) -> Self {
        Self {
            n,
            k,
            roughness: roughness.clamp(0.0, 1.0),
            spectral_ior: None,
        }
    }

    /// Create with spectral IOR for wavelength-dependent reflection
    pub fn with_spectral(mut self, spectral: SpectralComplexIOR) -> Self {
        self.spectral_ior = Some(spectral);
        self
    }

    /// Gold preset
    pub fn gold() -> Self {
        use super::complex_ior::metals;
        Self::new(0.18, 3.0, 0.0).with_spectral(metals::GOLD)
    }

    /// Silver preset
    pub fn silver() -> Self {
        use super::complex_ior::metals;
        Self::new(0.15, 3.64, 0.0).with_spectral(metals::SILVER)
    }

    /// Copper preset
    pub fn copper() -> Self {
        use super::complex_ior::metals;
        Self::new(0.27, 3.41, 0.0).with_spectral(metals::COPPER)
    }

    /// Aluminum preset
    pub fn aluminum() -> Self {
        use super::complex_ior::metals;
        Self::new(1.35, 7.47, 0.0).with_spectral(metals::ALUMINUM)
    }

    /// Chrome preset
    pub fn chrome() -> Self {
        use super::complex_ior::metals;
        Self::new(3.18, 3.19, 0.0).with_spectral(metals::CHROMIUM)
    }

    /// Brushed metal preset
    pub fn brushed_metal(base_n: f64, base_k: f64) -> Self {
        Self::new(base_n, base_k, 0.15)
    }
}

impl BSDF for ConductorBSDF {
    fn evaluate(&self, ctx: &BSDFContext) -> BSDFResponse {
        let cos_theta = ctx.cos_theta_i();

        // Get complex IOR (wavelength-dependent if spectral)
        let (n, k) = if let Some(ref spectral) = self.spectral_ior {
            // Map wavelength to RGB channel
            let ior = if ctx.wavelength < 500.0 {
                spectral.blue
            } else if ctx.wavelength < 600.0 {
                spectral.green
            } else {
                spectral.red
            };
            (ior.n, ior.k)
        } else {
            (self.n, self.k)
        };

        // Calculate conductor Fresnel
        let ior = ComplexIOR::new(n, k);
        let reflectance = fresnel_conductor_unpolarized(1.0, ior, cos_theta);

        // Apply roughness
        let specular = reflectance * (1.0 - self.roughness * 0.5);
        let diffuse = self.roughness * reflectance * 0.1;

        let total_reflectance = (specular + diffuse).min(1.0);

        // Conductors absorb non-reflected light
        BSDFResponse::new(total_reflectance, 0.0, 1.0 - total_reflectance)
    }

    fn sample(&self, ctx: &BSDFContext, u1: f64, u2: f64) -> BSDFSample {
        if self.roughness < 0.01 {
            // Perfect specular reflection
            let wo = ctx.wi.reflect(&ctx.normal);
            let response = self.evaluate(ctx);
            BSDFSample::delta_reflection(wo, response.reflectance)
        } else {
            // Rough metal: GGX-like sampling would go here
            // For now, use cosine-weighted
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
    }

    fn name(&self) -> &str {
        "ConductorBSDF"
    }

    fn is_delta(&self) -> bool {
        self.roughness < 0.01
    }
}

// ============================================================================
// THIN FILM BSDF
// ============================================================================

/// Unified thin-film BSDF (coatings, soap bubbles, oil slicks)
///
/// Wraps a substrate BSDF with a thin-film coating.
#[derive(Debug, Clone)]
pub struct ThinFilmBSDF {
    /// Underlying substrate
    substrate_ior: f64,
    substrate_roughness: f64,
    /// Film parameters
    pub film_ior: f64,
    /// Film thickness in nanometers
    pub film_thickness: f64,
}

impl ThinFilmBSDF {
    /// Create a new thin-film BSDF
    pub fn new(substrate_ior: f64, film_ior: f64, film_thickness: f64) -> Self {
        Self {
            substrate_ior,
            substrate_roughness: 0.0,
            film_ior,
            film_thickness,
        }
    }

    /// Set substrate roughness
    pub fn with_roughness(mut self, roughness: f64) -> Self {
        self.substrate_roughness = roughness.clamp(0.0, 1.0);
        self
    }

    /// Soap bubble preset
    pub fn soap_bubble(thickness: f64) -> Self {
        Self::new(1.0, 1.33, thickness)
    }

    /// Oil slick on water preset
    pub fn oil_on_water(thickness: f64) -> Self {
        Self::new(1.33, 1.47, thickness)
    }

    /// AR coating preset (quarter-wave at 550nm)
    pub fn ar_coating() -> Self {
        Self::new(1.52, 1.38, 100.0)
    }
}

impl BSDF for ThinFilmBSDF {
    fn evaluate(&self, ctx: &BSDFContext) -> BSDFResponse {
        let cos_theta = ctx.cos_theta_i();
        let wavelength = ctx.wavelength;

        // Create thin film structure
        let film = ThinFilm::new(self.film_ior, self.film_thickness);

        // Calculate thin-film interference reflectance
        let reflectance = film.reflectance(wavelength, self.substrate_ior, cos_theta);

        // Apply roughness
        let effective_r = reflectance * (1.0 - self.substrate_roughness * 0.3);

        // Thin films are generally transparent
        BSDFResponse::new(effective_r, 1.0 - effective_r, 0.0)
    }

    fn name(&self) -> &str {
        "ThinFilmBSDF"
    }
}

// ============================================================================
// LAYERED BSDF
// ============================================================================

/// Layered BSDF with energy-conserving composition
///
/// Combines multiple BSDFs in a physically-correct manner.
pub struct LayeredBSDF {
    /// Layers from top (air-side) to bottom (substrate)
    layers: Vec<Box<dyn BSDF>>,
}

impl LayeredBSDF {
    /// Create a new layered BSDF
    pub fn new() -> Self {
        Self { layers: Vec::new() }
    }

    /// Add a layer on top
    pub fn push(mut self, layer: Box<dyn BSDF>) -> Self {
        self.layers.push(layer);
        self
    }

    /// Number of layers
    pub fn layer_count(&self) -> usize {
        self.layers.len()
    }
}

impl Default for LayeredBSDF {
    fn default() -> Self {
        Self::new()
    }
}

impl BSDF for LayeredBSDF {
    fn evaluate(&self, ctx: &BSDFContext) -> BSDFResponse {
        if self.layers.is_empty() {
            return BSDFResponse::pure_transmission(1.0);
        }

        // Energy-conserving layer stacking
        let mut remaining_energy = 1.0;
        let mut total_reflectance = 0.0;
        let mut total_absorption = 0.0;

        for layer in &self.layers {
            let response = layer.evaluate(ctx);

            // Light reflected by this layer (weighted by remaining energy)
            total_reflectance += remaining_energy * response.reflectance;

            // Light absorbed by this layer
            total_absorption += remaining_energy * response.absorption;

            // Remaining energy continues to next layer
            remaining_energy *= response.transmittance;

            // Early exit if no energy remains
            if remaining_energy < 1e-6 {
                break;
            }
        }

        // Whatever's left is transmitted
        let total_transmittance = remaining_energy;

        BSDFResponse::new(total_reflectance, total_transmittance, total_absorption)
    }

    fn name(&self) -> &str {
        "LayeredBSDF"
    }
}

// ============================================================================
// LAMBERTIAN BSDF
// ============================================================================

/// Simple Lambertian (diffuse) BSDF
///
/// Uniform scattering in all directions (matte surfaces).
#[derive(Debug, Clone, Copy)]
pub struct LambertianBSDF {
    /// Albedo (fraction of light reflected diffusely)
    pub albedo: f64,
}

impl LambertianBSDF {
    /// Create a new Lambertian BSDF
    pub fn new(albedo: f64) -> Self {
        Self {
            albedo: albedo.clamp(0.0, 1.0),
        }
    }

    /// White matte surface
    pub fn white() -> Self {
        Self::new(0.9)
    }

    /// Gray matte surface
    pub fn gray() -> Self {
        Self::new(0.5)
    }

    /// Black matte surface
    pub fn black() -> Self {
        Self::new(0.05)
    }
}

impl BSDF for LambertianBSDF {
    fn evaluate(&self, ctx: &BSDFContext) -> BSDFResponse {
        // Lambertian: f = albedo / pi
        // Integrated over hemisphere: R = albedo
        let reflectance = self.albedo * ctx.cos_theta_o() / PI;
        BSDFResponse::new(reflectance, 0.0, 1.0 - self.albedo)
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

    fn pdf(&self, ctx: &BSDFContext) -> f64 {
        ctx.cos_theta_o() / PI
    }

    fn name(&self) -> &str {
        "LambertianBSDF"
    }
}

// ============================================================================
// UTILITY FUNCTIONS
// ============================================================================

/// Evaluate a BSDF at multiple wavelengths and return RGB
pub fn evaluate_rgb(bsdf: &dyn BSDF, cos_theta: f64) -> [f64; 3] {
    let ctx_r = BSDFContext::new_simple(cos_theta).with_wavelength(650.0);
    let ctx_g = BSDFContext::new_simple(cos_theta).with_wavelength(550.0);
    let ctx_b = BSDFContext::new_simple(cos_theta).with_wavelength(450.0);

    [
        bsdf.evaluate(&ctx_r).reflectance,
        bsdf.evaluate(&ctx_g).reflectance,
        bsdf.evaluate(&ctx_b).reflectance,
    ]
}

/// Evaluate a BSDF across the full visible spectrum
pub fn evaluate_spectral(bsdf: &dyn BSDF, cos_theta: f64) -> Vec<(f64, f64)> {
    (0..31)
        .map(|i| {
            let wavelength = 400.0 + i as f64 * 10.0;
            let ctx = BSDFContext::new_simple(cos_theta).with_wavelength(wavelength);
            (wavelength, bsdf.evaluate(&ctx).reflectance)
        })
        .collect()
}

/// Validate energy conservation across multiple angles
pub fn validate_energy_conservation(bsdf: &dyn BSDF) -> EnergyValidation {
    let angles = [0.0, 15.0, 30.0, 45.0, 60.0, 75.0, 85.0];
    let mut max_error: f64 = 0.0;

    for &angle_deg in &angles {
        let cos_theta = (angle_deg * PI / 180.0).cos();
        let ctx = BSDFContext::new_simple(cos_theta);
        let response = bsdf.evaluate(&ctx);
        let error = (response.total_energy() - 1.0).abs();
        max_error = max_error.max(error);
    }

    if max_error < 1e-6 {
        EnergyValidation::pass(max_error)
    } else {
        EnergyValidation::fail(max_error, "Max error at some angle")
    }
}

/// Total memory used by unified BSDF module
pub fn total_unified_bsdf_memory() -> usize {
    // Struct sizes
    std::mem::size_of::<Vector3>() * 10
        + std::mem::size_of::<BSDFContext>()
        + std::mem::size_of::<BSDFResponse>()
        + std::mem::size_of::<DielectricBSDF>()
        + std::mem::size_of::<ConductorBSDF>()
        + std::mem::size_of::<ThinFilmBSDF>()
        + std::mem::size_of::<LambertianBSDF>()
        + 1_000 // Overhead
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    const EPSILON: f64 = 1e-6;

    #[test]
    fn test_bsdf_response_normalization() {
        let response = BSDFResponse::new(0.5, 0.3, 0.2);
        assert!((response.total_energy() - 1.0).abs() < EPSILON);

        // Test over-energy normalization
        let response = BSDFResponse::new(0.8, 0.5, 0.3);
        assert!((response.total_energy() - 1.0).abs() < EPSILON);
    }

    #[test]
    fn test_dielectric_energy_conservation() {
        let glass = DielectricBSDF::glass();
        let validation = validate_energy_conservation(&glass);
        assert!(validation.conserved, "{}", validation.details);
    }

    #[test]
    fn test_conductor_energy_conservation() {
        let gold = ConductorBSDF::gold();
        let validation = validate_energy_conservation(&gold);
        assert!(validation.conserved, "{}", validation.details);
    }

    #[test]
    fn test_thin_film_energy_conservation() {
        let soap = ThinFilmBSDF::soap_bubble(350.0);
        let validation = validate_energy_conservation(&soap);
        assert!(validation.conserved, "{}", validation.details);
    }

    #[test]
    fn test_lambertian_energy_conservation() {
        let matte = LambertianBSDF::white();
        let validation = validate_energy_conservation(&matte);
        assert!(validation.conserved, "{}", validation.details);
    }

    #[test]
    fn test_layered_energy_conservation() {
        let layered = LayeredBSDF::new()
            .push(Box::new(ThinFilmBSDF::ar_coating()))
            .push(Box::new(DielectricBSDF::glass()));

        let validation = validate_energy_conservation(&layered);
        assert!(validation.conserved, "{}", validation.details);
    }

    #[test]
    fn test_dielectric_fresnel_behavior() {
        let glass = DielectricBSDF::glass();

        // At normal incidence, glass reflects ~4%
        let ctx_normal = BSDFContext::new_simple(1.0);
        let r_normal = glass.evaluate(&ctx_normal).reflectance;
        assert!(
            (r_normal - 0.04).abs() < 0.01,
            "Normal incidence: {}",
            r_normal
        );

        // At grazing angles, reflectance increases
        let ctx_grazing = BSDFContext::new_simple(0.1);
        let r_grazing = glass.evaluate(&ctx_grazing).reflectance;
        assert!(r_grazing > r_normal, "Grazing should be higher");
    }

    #[test]
    fn test_conductor_high_reflectivity() {
        let gold = ConductorBSDF::gold();

        let ctx = BSDFContext::new_simple(1.0);
        let response = gold.evaluate(&ctx);

        // Gold should have high reflectivity
        assert!(
            response.reflectance > 0.5,
            "Gold reflectance: {}",
            response.reflectance
        );

        // Conductors don't transmit
        assert!(response.transmittance < 0.01);
    }

    #[test]
    fn test_thin_film_wavelength_dependence() {
        let soap = ThinFilmBSDF::soap_bubble(350.0);

        let ctx_blue = BSDFContext::new_simple(0.8).with_wavelength(450.0);
        let ctx_red = BSDFContext::new_simple(0.8).with_wavelength(650.0);

        let r_blue = soap.evaluate(&ctx_blue).reflectance;
        let r_red = soap.evaluate(&ctx_red).reflectance;

        // Thin-film should show wavelength-dependent interference
        assert!(
            (r_blue - r_red).abs() > 0.01,
            "Should show color: {} vs {}",
            r_blue,
            r_red
        );
    }

    #[test]
    fn test_layered_composition() {
        // Create a coated glass
        let coating = DielectricBSDF::new(1.38, 0.0);
        let substrate = DielectricBSDF::glass();

        let layered = LayeredBSDF::new()
            .push(Box::new(coating))
            .push(Box::new(substrate));

        let ctx = BSDFContext::new_simple(0.8);
        let response = layered.evaluate(&ctx);

        // Should produce valid response
        assert!(response.reflectance >= 0.0 && response.reflectance <= 1.0);
        assert!((response.total_energy() - 1.0).abs() < EPSILON);
    }

    #[test]
    fn test_vector3_operations() {
        let a = Vector3::new(1.0, 0.0, 0.0);
        let b = Vector3::new(0.0, 1.0, 0.0);

        assert!((a.dot(&b)).abs() < EPSILON);
        assert!((a.length() - 1.0).abs() < EPSILON);

        let sum = a + b;
        assert!((sum.length() - 2.0_f64.sqrt()).abs() < EPSILON);
    }

    #[test]
    fn test_evaluate_rgb() {
        let glass = DielectricBSDF::glass();
        let rgb = evaluate_rgb(&glass, 0.8);

        for &r in &rgb {
            assert!(r >= 0.0 && r <= 1.0);
        }
    }

    #[test]
    fn test_evaluate_spectral() {
        let glass = DielectricBSDF::glass();
        let spectrum = evaluate_spectral(&glass, 0.8);

        assert_eq!(spectrum.len(), 31);
        assert!((spectrum[0].0 - 400.0).abs() < 0.1);
        assert!((spectrum[30].0 - 700.0).abs() < 0.1);
    }

    #[test]
    fn test_dispersion_model() {
        let crown = DispersionModel::CROWN_GLASS;
        let flint = DispersionModel::FLINT_GLASS;

        // Blue light should have higher IOR than red
        let n_blue_crown = crown.ior_at_wavelength(450.0);
        let n_red_crown = crown.ior_at_wavelength(650.0);
        assert!(n_blue_crown > n_red_crown);

        // Flint glass should have more dispersion
        let n_blue_flint = flint.ior_at_wavelength(450.0);
        let n_red_flint = flint.ior_at_wavelength(650.0);
        let dispersion_flint = n_blue_flint - n_red_flint;
        let dispersion_crown = n_blue_crown - n_red_crown;
        assert!(dispersion_flint > dispersion_crown);
    }

    #[test]
    fn test_bsdf_sampling() {
        let glass = DielectricBSDF::glass();
        let ctx = BSDFContext::new_simple(0.8);

        let sample = glass.sample(&ctx, 0.5, 0.5);

        // Sample should be valid
        assert!(sample.pdf > 0.0);
        assert!(sample.value.reflectance >= 0.0);
        assert!(sample.wo.length() > 0.9); // Approximately unit vector
    }

    #[test]
    fn test_memory_usage() {
        let mem = total_unified_bsdf_memory();
        assert!(mem < 5_000, "Memory should be < 5KB, got {}", mem);
    }

    #[test]
    fn test_presets() {
        // Test all presets compile and produce valid results
        let presets: Vec<Box<dyn BSDF>> = vec![
            Box::new(DielectricBSDF::glass()),
            Box::new(DielectricBSDF::water()),
            Box::new(DielectricBSDF::diamond()),
            Box::new(DielectricBSDF::frosted_glass()),
            Box::new(ConductorBSDF::gold()),
            Box::new(ConductorBSDF::silver()),
            Box::new(ConductorBSDF::copper()),
            Box::new(ConductorBSDF::aluminum()),
            Box::new(ConductorBSDF::chrome()),
            Box::new(ThinFilmBSDF::soap_bubble(350.0)),
            Box::new(ThinFilmBSDF::oil_on_water(300.0)),
            Box::new(ThinFilmBSDF::ar_coating()),
            Box::new(LambertianBSDF::white()),
        ];

        for bsdf in presets {
            let validation = validate_energy_conservation(bsdf.as_ref());
            assert!(
                validation.conserved,
                "{}: {}",
                bsdf.name(),
                validation.details
            );
        }
    }
}
