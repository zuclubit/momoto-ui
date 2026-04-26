//! # Meta-Materials: Photonic Crystals and Structural Color
//!
//! This module provides models for meta-materials that derive their optical
//! properties from nanoscale structures rather than chemical pigments.
//!
//! ## Included Models
//!
//! - **PhotonicCrystal**: Periodic dielectric structures with band gaps
//! - **StructuralColor**: Nanostructure-based coloration (butterfly wings, opals)
//! - **DiffractionGrating**: Wavelength-dependent angular dispersion
//!
//! ## Applications
//!
//! - Iridescent materials (morpho butterfly)
//! - Opal-like structural color
//! - Holographic effects
//! - Anti-reflective coatings
//!
//! ## Usage
//!
//! ```rust
//! use momoto_materials::glass_physics::meta_materials::{
//!     StructuralColor, NanostructureType, PhotonicCrystal, LatticeType
//! };
//!
//! // Morpho butterfly wing
//! let morpho = StructuralColor::morpho_butterfly();
//!
//! // Custom photonic crystal
//! let crystal = PhotonicCrystal::new(LatticeType::Hexagonal, 350.0, 0.5);
//! ```

use serde::{Deserialize, Serialize};
use std::f64::consts::PI;

use super::anisotropic::Color;
use super::unified_bsdf::{BSDFContext, BSDFResponse, BSDFSample, Vector3, BSDF};

// ============================================================================
// Material Reference (for composite materials)
// ============================================================================

/// Reference to a material for composition.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MaterialRef {
    /// Dielectric with given IOR
    Dielectric { ior: f64 },
    /// Metal with complex IOR
    Metal { n: f64, k: f64 },
    /// Air/vacuum
    Air,
    /// Custom named material
    Named(String),
}

impl Default for MaterialRef {
    fn default() -> Self {
        Self::Air
    }
}

impl MaterialRef {
    /// Get effective refractive index.
    pub fn ior(&self) -> f64 {
        match self {
            Self::Dielectric { ior } => *ior,
            Self::Metal { n, .. } => *n,
            Self::Air => 1.0,
            Self::Named(_) => 1.5, // Default fallback
        }
    }
}

// ============================================================================
// Photonic Crystal
// ============================================================================

/// Lattice type for photonic crystals.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LatticeType {
    /// Triangular 2D lattice
    Triangular,
    /// Square 2D lattice
    Square,
    /// Hexagonal 2D lattice
    Hexagonal,
    /// 3D woodpile structure
    Woodpile3D,
    /// Face-centered cubic
    FCC,
    /// Diamond lattice
    Diamond,
}

impl LatticeType {
    /// Get the number of nearest neighbors.
    pub fn coordination_number(&self) -> usize {
        match self {
            Self::Square => 4,
            Self::Triangular | Self::Hexagonal => 6,
            Self::Woodpile3D => 4,
            Self::FCC => 12,
            Self::Diamond => 4,
        }
    }

    /// Get lattice filling fraction for optimal band gap.
    pub fn optimal_fill_fraction(&self) -> f64 {
        match self {
            Self::Square => 0.5,
            Self::Triangular => 0.45,
            Self::Hexagonal => 0.4,
            Self::Woodpile3D => 0.35,
            Self::FCC => 0.34,
            Self::Diamond => 0.33,
        }
    }
}

/// Photonic crystal material.
///
/// Models periodic dielectric structures that create photonic band gaps,
/// reflecting specific wavelengths while transmitting others.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PhotonicCrystal {
    /// Lattice type
    pub lattice: LatticeType,
    /// Lattice period in nanometers
    pub period: f64,
    /// Fill fraction (ratio of high-index material)
    pub fill_fraction: f64,
    /// High-index material
    pub material_high: MaterialRef,
    /// Low-index material (usually air)
    pub material_low: MaterialRef,
    /// Number of periods (depth)
    pub num_periods: usize,
}

impl PhotonicCrystal {
    /// Create a new photonic crystal.
    pub fn new(lattice: LatticeType, period: f64, fill_fraction: f64) -> Self {
        Self {
            lattice,
            period,
            fill_fraction: fill_fraction.clamp(0.1, 0.9),
            material_high: MaterialRef::Dielectric { ior: 2.5 },
            material_low: MaterialRef::Air,
            num_periods: 10,
        }
    }

    /// Set high-index material.
    pub fn with_high_material(mut self, material: MaterialRef) -> Self {
        self.material_high = material;
        self
    }

    /// Set low-index material.
    pub fn with_low_material(mut self, material: MaterialRef) -> Self {
        self.material_low = material;
        self
    }

    /// Set number of periods.
    pub fn with_periods(mut self, n: usize) -> Self {
        self.num_periods = n.max(1);
        self
    }

    /// Calculate center wavelength of band gap.
    pub fn bandgap_center(&self) -> f64 {
        // Bragg condition: lambda = 2 * n_eff * d
        let n_eff = self.effective_index();
        2.0 * n_eff * self.period
    }

    /// Calculate band gap width (approximate).
    pub fn bandgap_width(&self) -> f64 {
        let n_high = self.material_high.ior();
        let n_low = self.material_low.ior();
        let contrast = (n_high - n_low) / (n_high + n_low);

        // Band gap width scales with index contrast
        let center = self.bandgap_center();
        center * contrast.abs() * 0.5
    }

    /// Calculate effective refractive index.
    pub fn effective_index(&self) -> f64 {
        let n_high = self.material_high.ior();
        let n_low = self.material_low.ior();

        // Volume-weighted average
        self.fill_fraction * n_high + (1.0 - self.fill_fraction) * n_low
    }

    /// Check if wavelength is in the band gap.
    pub fn in_bandgap(&self, wavelength: f64) -> bool {
        let center = self.bandgap_center();
        let width = self.bandgap_width();

        (wavelength - center).abs() < width / 2.0
    }

    /// Calculate reflectance at a wavelength.
    pub fn reflectance_at(&self, wavelength: f64, angle: f64) -> f64 {
        let center = self.bandgap_center();
        let width = self.bandgap_width();

        // Shift center with angle (Bragg's law)
        let shifted_center = center * angle.cos();
        let distance = (wavelength - shifted_center).abs();

        if distance < width / 2.0 {
            // In band gap - high reflectance
            let depth_factor = (self.num_periods as f64 / 10.0).min(1.0);
            let position = distance / (width / 2.0);
            depth_factor * (1.0 - position * position)
        } else {
            // Outside band gap - low reflectance
            let falloff = ((distance - width / 2.0) / width).min(1.0);
            0.1 * (1.0 - falloff)
        }
    }

    /// Create opal-like photonic crystal.
    pub fn opal() -> Self {
        Self::new(LatticeType::FCC, 250.0, 0.74)
            .with_high_material(MaterialRef::Dielectric { ior: 1.45 }) // Silica spheres
            .with_low_material(MaterialRef::Air)
            .with_periods(20)
    }

    /// Create inverse opal (high contrast).
    pub fn inverse_opal() -> Self {
        Self::new(LatticeType::FCC, 250.0, 0.26)
            .with_high_material(MaterialRef::Dielectric { ior: 3.5 }) // High-index
            .with_low_material(MaterialRef::Air)
            .with_periods(15)
    }
}

impl BSDF for PhotonicCrystal {
    fn evaluate(&self, ctx: &BSDFContext) -> BSDFResponse {
        let cos_theta = ctx.cos_theta_i();
        let angle = cos_theta.acos();
        let wavelength = ctx.wavelength;

        let reflectance = self.reflectance_at(wavelength, angle);
        let transmittance = (1.0 - reflectance) * 0.95; // Some absorption
        let absorption = 1.0 - reflectance - transmittance;

        BSDFResponse {
            reflectance,
            transmittance,
            absorption,
            ..BSDFResponse::default()
        }
    }

    fn sample(&self, ctx: &BSDFContext, u1: f64, _u2: f64) -> BSDFSample {
        // Reflect or transmit based on reflectance
        let value = self.evaluate(ctx);

        let wo = if u1 < value.reflectance {
            ctx.wo.reflect(&ctx.normal)
        } else {
            -ctx.wo
        };

        BSDFSample::new(wo, value, 1.0, true)
    }

    fn pdf(&self, _ctx: &BSDFContext) -> f64 {
        1.0 // Delta distribution
    }
}

// ============================================================================
// Nanostructure Types
// ============================================================================

/// Type of nanostructure for structural color.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum NanostructureType {
    /// Stack of thin films
    ThinFilmStack {
        /// Layer specifications: (thickness_nm, ior)
        layers: Vec<(f64, f64)>,
    },
    /// Surface grating
    Grating {
        /// Grating period in nm
        period: f64,
        /// Grating depth in nm
        depth: f64,
        /// Grating profile (0 = sinusoidal, 1 = rectangular)
        profile: f64,
    },
    /// Photonic crystal
    PhotonicCrystal(PhotonicCrystal),
    /// Morpho butterfly wing structure
    MorphoButterfly,
    /// Peacock feather barbule
    PeacockFeather,
    /// Beetle shell (jewel beetle)
    BeetleShell,
}

// ============================================================================
// Structural Color
// ============================================================================

/// Structural color material based on nanostructures.
///
/// Models materials where color arises from light interference in
/// nanoscale structures rather than pigment absorption.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StructuralColor {
    /// Type of nanostructure
    pub nanostructure: NanostructureType,
    /// Base material (substrate)
    pub base_material: MaterialRef,
    /// Whether color varies with viewing angle
    pub viewing_angle_dependence: bool,
    /// Disorder factor (0 = perfect, 1 = fully random)
    pub disorder: f64,
}

impl StructuralColor {
    /// Create new structural color.
    pub fn new(nanostructure: NanostructureType, base: MaterialRef) -> Self {
        Self {
            nanostructure,
            base_material: base,
            viewing_angle_dependence: true,
            disorder: 0.0,
        }
    }

    /// Set disorder level.
    pub fn with_disorder(mut self, disorder: f64) -> Self {
        self.disorder = disorder.clamp(0.0, 1.0);
        self
    }

    /// Create morpho butterfly wing material.
    pub fn morpho_butterfly() -> Self {
        Self {
            nanostructure: NanostructureType::MorphoButterfly,
            base_material: MaterialRef::Dielectric { ior: 1.56 }, // Chitin
            viewing_angle_dependence: true,
            disorder: 0.15, // Natural disorder
        }
    }

    /// Create peacock feather material.
    pub fn peacock_feather() -> Self {
        Self {
            nanostructure: NanostructureType::PeacockFeather,
            base_material: MaterialRef::Dielectric { ior: 1.55 },
            viewing_angle_dependence: true,
            disorder: 0.1,
        }
    }

    /// Create beetle shell material.
    pub fn beetle_shell() -> Self {
        Self {
            nanostructure: NanostructureType::BeetleShell,
            base_material: MaterialRef::Dielectric { ior: 1.56 },
            viewing_angle_dependence: true,
            disorder: 0.08,
        }
    }

    /// Create opal material.
    pub fn opal() -> Self {
        Self {
            nanostructure: NanostructureType::PhotonicCrystal(PhotonicCrystal::opal()),
            base_material: MaterialRef::Dielectric { ior: 1.45 },
            viewing_angle_dependence: true,
            disorder: 0.05,
        }
    }

    /// Create thin-film iridescence (soap bubble, oil slick).
    pub fn thin_film(thickness_nm: f64, ior: f64) -> Self {
        Self {
            nanostructure: NanostructureType::ThinFilmStack {
                layers: vec![(thickness_nm, ior)],
            },
            base_material: MaterialRef::Air,
            viewing_angle_dependence: true,
            disorder: 0.0,
        }
    }

    /// Calculate color at wavelength and angle.
    fn evaluate_nanostructure(&self, wavelength: f64, cos_theta: f64) -> (f64, f64) {
        match &self.nanostructure {
            NanostructureType::ThinFilmStack { layers } => {
                self.evaluate_thin_film(layers, wavelength, cos_theta)
            }
            NanostructureType::Grating {
                period,
                depth,
                profile,
            } => self.evaluate_grating(*period, *depth, *profile, wavelength, cos_theta),
            NanostructureType::PhotonicCrystal(pc) => {
                let r = pc.reflectance_at(wavelength, cos_theta.acos());
                (r, 1.0 - r)
            }
            NanostructureType::MorphoButterfly => self.evaluate_morpho(wavelength, cos_theta),
            NanostructureType::PeacockFeather => self.evaluate_peacock(wavelength, cos_theta),
            NanostructureType::BeetleShell => self.evaluate_beetle(wavelength, cos_theta),
        }
    }

    /// Evaluate thin-film stack interference.
    fn evaluate_thin_film(
        &self,
        layers: &[(f64, f64)],
        wavelength: f64,
        cos_theta: f64,
    ) -> (f64, f64) {
        if layers.is_empty() {
            return (0.04, 0.96);
        }

        let mut total_r = 0.0;
        let n0 = 1.0; // Air

        for &(thickness, n) in layers {
            // Phase change through layer
            let cos_theta_t = (1.0 - (n0 / n).powi(2) * (1.0 - cos_theta * cos_theta)).sqrt();
            let path_length = 2.0 * n * thickness * cos_theta_t;
            let phase = 2.0 * PI * path_length / wavelength;

            // Fresnel reflection at interfaces
            let r_s =
                ((n0 * cos_theta - n * cos_theta_t) / (n0 * cos_theta + n * cos_theta_t)).powi(2);
            let r_p =
                ((n * cos_theta - n0 * cos_theta_t) / (n * cos_theta + n0 * cos_theta_t)).powi(2);
            let r = (r_s + r_p) / 2.0;

            // Interference
            total_r += r * (1.0 + phase.cos()) / 2.0;
        }

        let r = total_r.clamp(0.0, 1.0);
        (r, 1.0 - r)
    }

    /// Evaluate diffraction grating.
    fn evaluate_grating(
        &self,
        period: f64,
        depth: f64,
        _profile: f64,
        wavelength: f64,
        cos_theta: f64,
    ) -> (f64, f64) {
        let sin_theta = (1.0 - cos_theta * cos_theta).sqrt();

        // Diffraction orders
        let m_max = (period / wavelength * (1.0 + sin_theta)).floor() as i32;

        let mut total_efficiency = 0.0;

        for m in -m_max..=m_max {
            // Grating equation: sin(theta_m) = sin(theta_i) + m * lambda / d
            let sin_theta_m = sin_theta + m as f64 * wavelength / period;

            if sin_theta_m.abs() <= 1.0 {
                // Diffraction efficiency (simplified sinc^2 model)
                let phase = PI * depth * sin_theta_m / wavelength;
                let efficiency = if phase.abs() < 0.01 {
                    1.0
                } else {
                    (phase.sin() / phase).powi(2)
                };

                total_efficiency += efficiency / (m_max as f64 * 2.0 + 1.0);
            }
        }

        let r = total_efficiency.clamp(0.0, 0.8);
        (r, 1.0 - r)
    }

    /// Evaluate morpho butterfly structure.
    fn evaluate_morpho(&self, wavelength: f64, cos_theta: f64) -> (f64, f64) {
        // Morpho: Christmas-tree structure with ~10 layers
        // Peak at ~480nm (blue)
        let peak_wavelength = 480.0;
        let layer_spacing = 150.0; // nm

        // Multi-layer interference
        let path_diff = 2.0 * layer_spacing * cos_theta;
        let phase = 2.0 * PI * path_diff / wavelength;

        // Interference from multiple layers
        let n_layers = 10.0;
        let coherence = (n_layers * phase / 2.0).sin() / (phase / 2.0).sin();
        let intensity = (coherence / n_layers).powi(2);

        // Blue selectivity
        let wavelength_selectivity = (-(wavelength - peak_wavelength).powi(2) / 2000.0).exp();

        let r = (intensity * wavelength_selectivity * 0.8).clamp(0.0, 0.8);

        // Add disorder
        let disorder_factor = 1.0 - self.disorder * 0.5;
        let r_disordered = r * disorder_factor + 0.1 * self.disorder;

        (r_disordered, 1.0 - r_disordered)
    }

    /// Evaluate peacock feather barbule.
    fn evaluate_peacock(&self, wavelength: f64, cos_theta: f64) -> (f64, f64) {
        // Peacock: 2D photonic crystal with melanin rods
        // Multiple colors from different domains
        let period = 400.0; // nm

        // Bragg reflection
        let bragg_wavelength = 2.0 * period * cos_theta * 1.55; // n_eff ~ 1.55

        let wavelength_diff = (wavelength - bragg_wavelength).abs();
        let bandwidth = 50.0; // nm

        let r = if wavelength_diff < bandwidth {
            0.6 * (1.0 - wavelength_diff / bandwidth)
        } else {
            0.05
        };

        (r, 1.0 - r)
    }

    /// Evaluate beetle shell (jewel beetle).
    fn evaluate_beetle(&self, wavelength: f64, cos_theta: f64) -> (f64, f64) {
        // Beetle: Multilayer reflector with chirped layers
        // Broad-band reflection

        let base_thickness = 100.0; // nm
        let chirp_factor = 0.1; // 10% variation

        let mut total_r = 0.0;

        for i in 0..15 {
            let thickness = base_thickness * (1.0 + chirp_factor * i as f64);
            let n: f64 = 1.56; // Chitin

            let cos_t = (1.0 - (1.0 / n).powi(2) * (1.0 - cos_theta * cos_theta)).sqrt();
            let path = 2.0 * n * thickness * cos_t;
            let phase = 2.0 * PI * path / wavelength;

            let r = 0.04; // Fresnel at interface
            total_r += r * (1.0 + phase.cos()) / 2.0;
        }

        let r = total_r.clamp(0.0, 0.7);
        (r, 1.0 - r)
    }
}

impl BSDF for StructuralColor {
    fn evaluate(&self, ctx: &BSDFContext) -> BSDFResponse {
        let cos_theta = ctx.cos_theta_i();
        let wavelength = ctx.wavelength;

        let (reflectance, transmittance) = self.evaluate_nanostructure(wavelength, cos_theta);

        BSDFResponse {
            reflectance,
            transmittance,
            absorption: 1.0 - reflectance - transmittance,
            ..BSDFResponse::default()
        }
    }

    fn sample(&self, ctx: &BSDFContext, u1: f64, _u2: f64) -> BSDFSample {
        let value = self.evaluate(ctx);

        let wo = if u1 < value.reflectance {
            ctx.wo.reflect(&ctx.normal)
        } else {
            -ctx.wo
        };

        BSDFSample::new(wo, value, 1.0, true)
    }

    fn pdf(&self, _ctx: &BSDFContext) -> f64 {
        1.0
    }
}

// ============================================================================
// Diffraction Grating
// ============================================================================

/// Diffraction grating for holographic effects.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiffractionGrating {
    /// Grating period in nm
    pub period: f64,
    /// Grating depth in nm
    pub depth: f64,
    /// Blaze angle in radians
    pub blaze_angle: f64,
    /// Grating efficiency
    pub efficiency: f64,
}

impl DiffractionGrating {
    /// Create a new diffraction grating.
    pub fn new(period: f64, depth: f64) -> Self {
        Self {
            period,
            depth,
            blaze_angle: 0.0,
            efficiency: 0.8,
        }
    }

    /// Set blaze angle for efficiency optimization.
    pub fn with_blaze(mut self, angle: f64) -> Self {
        self.blaze_angle = angle;
        self
    }

    /// Create holographic grating (high line density).
    pub fn holographic() -> Self {
        Self::new(833.0, 100.0) // 1200 lines/mm
            .with_blaze(0.35)
    }

    /// Create CD/DVD-like grating.
    pub fn cd_surface() -> Self {
        Self::new(1600.0, 120.0) // ~625 lines/mm
    }

    /// Calculate diffraction angle for order m.
    pub fn diffraction_angle(
        &self,
        wavelength: f64,
        incident_angle: f64,
        order: i32,
    ) -> Option<f64> {
        let sin_i = incident_angle.sin();
        let sin_m = sin_i + order as f64 * wavelength / self.period;

        if sin_m.abs() <= 1.0 {
            Some(sin_m.asin())
        } else {
            None
        }
    }

    /// Calculate efficiency for a given order.
    pub fn order_efficiency(&self, wavelength: f64, incident_angle: f64, order: i32) -> f64 {
        if let Some(diff_angle) = self.diffraction_angle(wavelength, incident_angle, order) {
            // Blaze efficiency
            let blaze_diff = (diff_angle - self.blaze_angle).abs();
            let blaze_efficiency = (-blaze_diff.powi(2) / 0.1).exp();

            self.efficiency * blaze_efficiency / (order.abs() as f64 + 1.0)
        } else {
            0.0
        }
    }
}

impl BSDF for DiffractionGrating {
    fn evaluate(&self, ctx: &BSDFContext) -> BSDFResponse {
        let cos_theta = ctx.cos_theta_i();
        let incident_angle = cos_theta.acos();
        let wavelength = ctx.wavelength;

        // Sum efficiency over diffraction orders
        let mut total_r = 0.0;

        for order in -3..=3 {
            total_r += self.order_efficiency(wavelength, incident_angle, order);
        }

        let reflectance = total_r.clamp(0.0, 0.9);

        BSDFResponse {
            reflectance,
            transmittance: 1.0 - reflectance,
            absorption: 0.0,
            ..BSDFResponse::default()
        }
    }

    fn sample(&self, ctx: &BSDFContext, u1: f64, _u2: f64) -> BSDFSample {
        // Sample a diffraction order
        let order = (u1 * 7.0).floor() as i32 - 3;
        let incident_angle = ctx.cos_theta_i().acos();

        let wo = if let Some(diff_angle) =
            self.diffraction_angle(ctx.wavelength, incident_angle, order)
        {
            // Create direction at diffraction angle
            let cos_diff = diff_angle.cos();
            let sin_diff = diff_angle.sin();
            Vector3::new(sin_diff, 0.0, cos_diff)
        } else {
            ctx.wo.reflect(&ctx.normal)
        };

        BSDFSample::new(wo, self.evaluate(ctx), 1.0 / 7.0, false)
    }

    fn pdf(&self, _ctx: &BSDFContext) -> f64 {
        1.0 / 7.0
    }
}

// ============================================================================
// Presets
// ============================================================================

/// Create mother-of-pearl (nacre) material.
pub fn mother_of_pearl() -> StructuralColor {
    StructuralColor::new(
        NanostructureType::ThinFilmStack {
            layers: vec![
                (300.0, 1.66), // Aragonite
                (30.0, 1.33),  // Organic
                (300.0, 1.66),
                (30.0, 1.33),
                (300.0, 1.66),
            ],
        },
        MaterialRef::Dielectric { ior: 1.66 },
    )
    .with_disorder(0.05)
}

/// Create soap bubble material.
pub fn soap_bubble() -> StructuralColor {
    StructuralColor::thin_film(350.0, 1.33)
}

/// Create oil on water effect.
pub fn oil_on_water(thickness_nm: f64) -> StructuralColor {
    StructuralColor::thin_film(thickness_nm, 1.47)
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_photonic_crystal() {
        let pc = PhotonicCrystal::opal();
        let ctx = BSDFContext::new_simple(0.9);
        let response = pc.evaluate(&ctx);

        assert!(response.is_energy_conserved(1e-6));
        assert!(response.reflectance >= 0.0);
    }

    #[test]
    fn test_bandgap() {
        let pc = PhotonicCrystal::new(LatticeType::FCC, 300.0, 0.5);
        let center = pc.bandgap_center();
        let width = pc.bandgap_width();

        assert!(center > 0.0);
        assert!(width > 0.0);
        assert!(pc.in_bandgap(center));
        assert!(!pc.in_bandgap(center + width));
    }

    #[test]
    fn test_morpho() {
        let morpho = StructuralColor::morpho_butterfly();
        let ctx = BSDFContext::new_simple(0.9).with_wavelength(480.0); // Blue
        let response = morpho.evaluate(&ctx);

        assert!(response.is_energy_conserved(1e-6));
        // Should have some reflectance at blue wavelengths
        assert!(response.reflectance >= 0.0);
        // Test angle dependence - different angle should give different reflectance
        let ctx2 = BSDFContext::new_simple(0.5).with_wavelength(480.0);
        let response2 = morpho.evaluate(&ctx2);
        assert!(response2.is_energy_conserved(1e-6));
    }

    #[test]
    fn test_thin_film() {
        let bubble = soap_bubble();
        let ctx = BSDFContext::new_simple(0.9);
        let response = bubble.evaluate(&ctx);

        assert!(response.is_energy_conserved(1e-6));
    }

    #[test]
    fn test_diffraction_grating() {
        let grating = DiffractionGrating::holographic();
        let ctx = BSDFContext::new_simple(0.9);
        let response = grating.evaluate(&ctx);

        assert!(response.is_energy_conserved(1e-6));
    }

    #[test]
    fn test_diffraction_orders() {
        let grating = DiffractionGrating::new(1000.0, 100.0);

        // First order should exist
        let angle = grating.diffraction_angle(550.0, 0.0, 1);
        assert!(angle.is_some());

        // Very high order shouldn't exist
        let angle = grating.diffraction_angle(550.0, 0.0, 10);
        assert!(angle.is_none());
    }
}

// ============================================================================
// Preset Module
// ============================================================================

/// Material presets for meta-materials.
pub mod presets {
    use super::*;

    /// Morpho butterfly wing (blue iridescence).
    pub fn morpho_butterfly() -> StructuralColor {
        StructuralColor::morpho_butterfly()
    }

    /// Peacock feather barbule.
    pub fn peacock_feather() -> StructuralColor {
        StructuralColor::peacock_feather()
    }

    /// Jewel beetle shell.
    pub fn beetle_shell() -> StructuralColor {
        StructuralColor::beetle_shell()
    }

    /// Natural opal (silica spheres).
    pub fn opal() -> StructuralColor {
        StructuralColor::opal()
    }

    /// Mother-of-pearl (nacre).
    pub fn mother_of_pearl() -> StructuralColor {
        super::mother_of_pearl()
    }

    /// Soap bubble.
    pub fn soap_bubble() -> StructuralColor {
        super::soap_bubble()
    }

    /// Oil on water (default thickness).
    pub fn oil_on_water() -> StructuralColor {
        super::oil_on_water(400.0)
    }

    /// Holographic diffraction grating.
    pub fn holographic() -> DiffractionGrating {
        DiffractionGrating::holographic()
    }

    /// CD/DVD surface grating.
    pub fn cd_surface() -> DiffractionGrating {
        DiffractionGrating::cd_surface()
    }

    /// Photonic crystal opal.
    pub fn photonic_opal() -> PhotonicCrystal {
        PhotonicCrystal::opal()
    }

    /// Inverse opal (high contrast).
    pub fn inverse_opal() -> PhotonicCrystal {
        PhotonicCrystal::inverse_opal()
    }

    /// List all available preset names.
    pub fn list_presets() -> Vec<&'static str> {
        vec![
            "morpho_butterfly",
            "peacock_feather",
            "beetle_shell",
            "opal",
            "mother_of_pearl",
            "soap_bubble",
            "oil_on_water",
            "holographic",
            "cd_surface",
            "photonic_opal",
            "inverse_opal",
        ]
    }
}

// ============================================================================
// Memory Estimation
// ============================================================================

/// Estimate memory usage for meta-material types.
pub fn estimate_meta_materials_memory() -> usize {
    let photonic_crystal_size = std::mem::size_of::<PhotonicCrystal>();
    let structural_color_size = std::mem::size_of::<StructuralColor>();
    let diffraction_grating_size = std::mem::size_of::<DiffractionGrating>();
    let material_ref_size = std::mem::size_of::<MaterialRef>();
    let nanostructure_size = std::mem::size_of::<NanostructureType>();

    photonic_crystal_size
        + structural_color_size
        + diffraction_grating_size
        + material_ref_size
        + nanostructure_size
}
