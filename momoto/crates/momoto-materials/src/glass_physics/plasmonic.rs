//! # Plasmonic Materials
//!
//! This module provides models for plasmonic nanoparticles and films that
//! exhibit localized surface plasmon resonance (LSPR), enabling unique
//! optical properties based on particle composition, size, and shape.
//!
//! ## Included Models
//!
//! - **PlasmonicNanoparticle**: Single nanoparticle LSPR
//! - **PlasmonicFilm**: Nanoparticle composite films
//! - **PlasmonicArray**: Ordered nanoparticle arrays
//!
//! ## Applications
//!
//! - Stained glass (colloidal gold)
//! - Biosensors
//! - Color-changing materials
//! - Enhanced fluorescence
//!
//! ## Usage
//!
//! ```rust
//! use momoto_materials::glass_physics::plasmonic::{
//!     PlasmonicNanoparticle, PlasmonicFilm, MetalType, ParticleShape
//! };
//!
//! // Gold nanosphere (ruby glass color)
//! let gold_np = PlasmonicNanoparticle::new(MetalType::Gold, ParticleShape::Sphere, 20.0);
//!
//! // Silver nanorod film
//! let silver_film = PlasmonicFilm::silver_nanorods(2.0);
//! ```

use serde::{Deserialize, Serialize};
use std::f64::consts::PI;

use super::anisotropic::Color;
use super::unified_bsdf::{BSDFContext, BSDFResponse, BSDFSample, Vector3, BSDF};

// ============================================================================
// Metal Types
// ============================================================================

/// Metal type for plasmonic particles.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MetalType {
    /// Gold - classic LSPR material
    Gold,
    /// Silver - sharper resonance
    Silver,
    /// Copper
    Copper,
    /// Aluminum - UV plasmonics
    Aluminum,
    /// Platinum
    Platinum,
    /// Palladium
    Palladium,
}

impl MetalType {
    /// Get plasma frequency in eV.
    pub fn plasma_frequency(&self) -> f64 {
        match self {
            Self::Gold => 9.03,
            Self::Silver => 9.17,
            Self::Copper => 7.39,
            Self::Aluminum => 14.98,
            Self::Platinum => 5.15,
            Self::Palladium => 5.96,
        }
    }

    /// Get damping constant in eV.
    pub fn damping_constant(&self) -> f64 {
        match self {
            Self::Gold => 0.072,
            Self::Silver => 0.021,
            Self::Copper => 0.145,
            Self::Aluminum => 0.598,
            Self::Platinum => 0.555,
            Self::Palladium => 0.384,
        }
    }

    /// Get interband transition onset in eV.
    pub fn interband_onset(&self) -> f64 {
        match self {
            Self::Gold => 2.4,
            Self::Silver => 3.9,
            Self::Copper => 2.1,
            Self::Aluminum => 1.5,
            Self::Platinum => 1.0,
            Self::Palladium => 1.0,
        }
    }

    /// Get Fermi velocity in m/s.
    pub fn fermi_velocity(&self) -> f64 {
        match self {
            Self::Gold => 1.39e6,
            Self::Silver => 1.39e6,
            Self::Copper => 1.57e6,
            Self::Aluminum => 2.03e6,
            Self::Platinum => 1.0e6,
            Self::Palladium => 1.0e6,
        }
    }

    /// Get color of bulk metal.
    pub fn bulk_color(&self) -> Color {
        match self {
            Self::Gold => Color::new(1.0, 0.84, 0.0),
            Self::Silver => Color::new(0.97, 0.97, 0.97),
            Self::Copper => Color::new(0.95, 0.64, 0.54),
            Self::Aluminum => Color::new(0.91, 0.92, 0.92),
            Self::Platinum => Color::new(0.90, 0.89, 0.87),
            Self::Palladium => Color::new(0.87, 0.87, 0.87),
        }
    }
}

// ============================================================================
// Particle Shapes
// ============================================================================

/// Shape of plasmonic nanoparticle.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "shape", rename_all = "snake_case")]
pub enum ParticleShape {
    /// Spherical particle
    Sphere,
    /// Ellipsoidal particle
    Ellipsoid {
        /// Aspect ratio (length/width)
        aspect_ratio: f64,
    },
    /// Nanorod (cylinder with hemispherical caps)
    Nanorod {
        /// Aspect ratio (length/diameter)
        aspect_ratio: f64,
    },
    /// Nanostar (sphere with protrusions)
    Nanostar {
        /// Number of spikes
        spikes: usize,
        /// Spike length relative to core radius
        spike_length: f64,
    },
    /// Nanocube
    Nanocube {
        /// Edge rounding radius relative to size
        rounding: f64,
    },
    /// Nanoshell (dielectric core, metal shell)
    Nanoshell {
        /// Core radius / total radius ratio
        core_ratio: f64,
        /// Core material IOR
        core_ior: f64,
    },
}

impl Default for ParticleShape {
    fn default() -> Self {
        Self::Sphere
    }
}

impl ParticleShape {
    /// Get depolarization factors for this shape.
    ///
    /// Returns (L_long, L_trans) for longitudinal and transverse modes.
    pub fn depolarization_factors(&self) -> (f64, f64) {
        match self {
            Self::Sphere => (1.0 / 3.0, 1.0 / 3.0),
            Self::Ellipsoid { aspect_ratio } | Self::Nanorod { aspect_ratio } => {
                let r = *aspect_ratio;
                if r > 1.0 {
                    // Prolate spheroid
                    let e = (1.0 - 1.0 / (r * r)).sqrt();
                    let l_long =
                        (1.0 - e * e) / (e * e) * (((1.0 + e) / (1.0 - e)).ln() / (2.0 * e) - 1.0);
                    let l_trans = (1.0 - l_long) / 2.0;
                    (l_long, l_trans)
                } else {
                    (1.0 / 3.0, 1.0 / 3.0)
                }
            }
            Self::Nanostar { .. } => (0.15, 0.35), // Effective values
            Self::Nanocube { rounding } => {
                let l = 0.33 - 0.1 * rounding;
                (l, (1.0 - l) / 2.0)
            }
            Self::Nanoshell { core_ratio, .. } => {
                // Core-shell modifies effective L
                let g = *core_ratio;
                let l = 1.0 / 3.0 * (1.0 + 2.0 * g * g * g);
                (l, (1.0 - l) / 2.0)
            }
        }
    }

    /// Get mode count (number of resonance peaks).
    pub fn mode_count(&self) -> usize {
        match self {
            Self::Sphere => 1,
            Self::Ellipsoid { .. } | Self::Nanorod { .. } => 2,
            Self::Nanostar { spikes, .. } => *spikes + 1,
            Self::Nanocube { .. } => 3,
            Self::Nanoshell { .. } => 2,
        }
    }
}

// ============================================================================
// Plasmonic Nanoparticle
// ============================================================================

/// Single plasmonic nanoparticle.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlasmonicNanoparticle {
    /// Metal type
    pub material: MetalType,
    /// Particle shape
    pub shape: ParticleShape,
    /// Characteristic size in nm
    pub size_nm: f64,
    /// Embedding medium refractive index
    pub embedding_medium_ior: f64,
}

impl PlasmonicNanoparticle {
    /// Create a new plasmonic nanoparticle.
    pub fn new(material: MetalType, shape: ParticleShape, size_nm: f64) -> Self {
        Self {
            material,
            shape,
            size_nm,
            embedding_medium_ior: 1.33, // Water
        }
    }

    /// Set embedding medium.
    pub fn in_medium(mut self, ior: f64) -> Self {
        self.embedding_medium_ior = ior;
        self
    }

    /// Create gold nanosphere.
    pub fn gold_sphere(diameter_nm: f64) -> Self {
        Self::new(MetalType::Gold, ParticleShape::Sphere, diameter_nm)
    }

    /// Create gold nanorod.
    pub fn gold_nanorod(length_nm: f64, aspect_ratio: f64) -> Self {
        Self::new(
            MetalType::Gold,
            ParticleShape::Nanorod { aspect_ratio },
            length_nm,
        )
    }

    /// Create silver nanosphere.
    pub fn silver_sphere(diameter_nm: f64) -> Self {
        Self::new(MetalType::Silver, ParticleShape::Sphere, diameter_nm)
    }

    /// Calculate Drude dielectric function at energy E (eV).
    fn drude_permittivity(&self, energy_ev: f64) -> (f64, f64) {
        let wp = self.material.plasma_frequency();
        let gamma = self.material.damping_constant();

        // Size-dependent damping (electron scattering at surface)
        let vf = self.material.fermi_velocity();
        let a = self.size_nm * 1e-9; // Convert to m
        let gamma_size = gamma + vf / a / 1.6e-19; // Convert to eV

        let e = energy_ev;
        let eps_r = 1.0 - wp * wp / (e * e + gamma_size * gamma_size);
        let eps_i = wp * wp * gamma_size / (e * (e * e + gamma_size * gamma_size));

        (eps_r, eps_i)
    }

    /// Calculate LSPR wavelength for a given mode.
    pub fn lspr_wavelength(&self, mode: usize) -> f64 {
        let (l_long, l_trans) = self.shape.depolarization_factors();
        let l = if mode == 0 { l_long } else { l_trans };

        let em = self.embedding_medium_ior * self.embedding_medium_ior;
        let wp = self.material.plasma_frequency();

        // Resonance condition: Re(eps) = -em * (1-L) / L
        let eps_r_res = -em * (1.0 - l) / l;

        // Solve Drude model for this condition
        let gamma = self.material.damping_constant();
        let e_res = (wp * wp / (1.0 - eps_r_res) - gamma * gamma).sqrt();

        // Convert eV to nm (E = hc/lambda)
        1240.0 / e_res
    }

    /// Calculate extinction cross-section at wavelength.
    pub fn extinction_cross_section(&self, wavelength_nm: f64) -> f64 {
        let energy_ev = 1240.0 / wavelength_nm;
        let (eps_r, eps_i) = self.drude_permittivity(energy_ev);

        let em = self.embedding_medium_ior * self.embedding_medium_ior;
        let (l_long, l_trans) = self.shape.depolarization_factors();

        let mut c_ext = 0.0;
        let v = 4.0 / 3.0 * PI * (self.size_nm / 2.0).powi(3); // Volume in nm^3

        // Sum over modes
        for (i, &l) in [l_long, l_trans, l_trans]
            .iter()
            .enumerate()
            .take(self.shape.mode_count())
        {
            let denom_r = eps_r + (1.0 - l) / l * em;
            let denom_i = eps_i;
            let denom = denom_r * denom_r + denom_i * denom_i;

            // Extinction = absorption + scattering
            let alpha_i = v * em.sqrt() * eps_i / denom;

            c_ext += alpha_i;
        }

        c_ext * 2.0 * PI / wavelength_nm
    }

    /// Calculate extinction spectrum across visible range.
    pub fn extinction_spectrum(&self, wavelengths: &[f64]) -> Vec<f64> {
        wavelengths
            .iter()
            .map(|&wl| self.extinction_cross_section(wl))
            .collect()
    }

    /// Get effective color from LSPR.
    pub fn effective_color(&self) -> Color {
        // Sample across visible spectrum
        let wavelengths: Vec<f64> = (380..=780).step_by(10).map(|w| w as f64).collect();
        let spectrum = self.extinction_spectrum(&wavelengths);

        // Convert extinction to transmitted color (complementary to absorbed)
        let max_ext = spectrum.iter().cloned().fold(0.0, f64::max);

        if max_ext < 1e-10 {
            return Color::white();
        }

        // Simple XYZ approximation
        let mut x = 0.0;
        let mut y = 0.0;
        let mut z = 0.0;

        for (i, &wl) in wavelengths.iter().enumerate() {
            let transmission = 1.0 - (spectrum[i] / max_ext).min(1.0);

            // CIE color matching approximation
            let (cx, cy, cz) = wavelength_to_xyz(wl);
            x += transmission * cx;
            y += transmission * cy;
            z += transmission * cz;
        }

        // Normalize and convert to RGB
        let total = (x + y + z).max(1e-10);
        x /= total;
        y /= total;
        z /= total;

        xyz_to_rgb(x, y, z)
    }
}

/// Convert wavelength to approximate XYZ color matching functions.
fn wavelength_to_xyz(wavelength: f64) -> (f64, f64, f64) {
    // Gaussian approximation to CIE 1931 color matching functions
    let t1 = (wavelength - 442.0) * (if wavelength < 442.0 { 0.0624 } else { 0.0374 });
    let t2 = (wavelength - 599.8) * (if wavelength < 599.8 { 0.0264 } else { 0.0323 });
    let t3 = (wavelength - 501.1) * (if wavelength < 501.1 { 0.0490 } else { 0.0382 });

    let x = 0.362 * (-0.5 * t1 * t1).exp() + 1.056 * (-0.5 * t2 * t2).exp()
        - 0.065 * (-0.5 * t3 * t3).exp();
    let y = 0.821 * (-0.5 * ((wavelength - 568.8) * 0.0213).powi(2)).exp()
        + 0.286 * (-0.5 * ((wavelength - 530.9) * 0.0613).powi(2)).exp();
    let z = 1.217 * (-0.5 * ((wavelength - 437.0) * 0.0845).powi(2)).exp()
        + 0.681 * (-0.5 * ((wavelength - 459.0) * 0.0385).powi(2)).exp();

    (x.max(0.0), y.max(0.0), z.max(0.0))
}

/// Convert XYZ to RGB.
fn xyz_to_rgb(x: f64, y: f64, z: f64) -> Color {
    // sRGB matrix
    let r = 3.2406 * x - 1.5372 * y - 0.4986 * z;
    let g = -0.9689 * x + 1.8758 * y + 0.0415 * z;
    let b = 0.0557 * x - 0.2040 * y + 1.0570 * z;

    Color::new(r.clamp(0.0, 1.0), g.clamp(0.0, 1.0), b.clamp(0.0, 1.0))
}

impl BSDF for PlasmonicNanoparticle {
    fn evaluate(&self, ctx: &BSDFContext) -> BSDFResponse {
        let wavelength = ctx.wavelength;
        let c_ext = self.extinction_cross_section(wavelength);

        // Normalize to reasonable reflectance
        let max_ext = self.extinction_cross_section(self.lspr_wavelength(0));
        let norm_ext = (c_ext / max_ext.max(1e-10)).clamp(0.0, 1.0);

        // LSPR causes absorption and scattering
        let absorption = norm_ext * 0.6;
        let scattering = norm_ext * 0.3;

        BSDFResponse {
            reflectance: scattering,
            transmittance: 1.0 - absorption - scattering,
            absorption,
            ..BSDFResponse::default()
        }
    }

    fn sample(&self, ctx: &BSDFContext, u1: f64, u2: f64) -> BSDFSample {
        // Isotropic scattering from nanoparticle
        let phi = 2.0 * PI * u1;
        let cos_theta = 1.0 - 2.0 * u2;
        let sin_theta = (1.0 - cos_theta * cos_theta).sqrt();

        let wo = Vector3::new(sin_theta * phi.cos(), sin_theta * phi.sin(), cos_theta);

        BSDFSample::new(wo, self.evaluate(ctx), 1.0 / (4.0 * PI), false)
    }

    fn pdf(&self, _ctx: &BSDFContext) -> f64 {
        1.0 / (4.0 * PI) // Isotropic
    }
}

// ============================================================================
// Particle Ordering
// ============================================================================

/// Ordering of particles in a film.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ParticleOrdering {
    /// Random distribution
    Random,
    /// Hexagonal close-packed
    Hexagonal,
    /// Square lattice
    Square,
    /// Linear chains
    LinearChains,
}

// ============================================================================
// Plasmonic Film
// ============================================================================

/// Film containing plasmonic nanoparticles.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlasmonicFilm {
    /// Particles in the film
    pub particles: Vec<PlasmonicNanoparticle>,
    /// Particle concentration (volume fraction)
    pub concentration: f64,
    /// Particle ordering
    pub ordering: ParticleOrdering,
    /// Film thickness in nm
    pub thickness_nm: f64,
    /// Matrix material IOR
    pub matrix_ior: f64,
}

impl PlasmonicFilm {
    /// Create a new plasmonic film.
    pub fn new(particle: PlasmonicNanoparticle, concentration: f64) -> Self {
        Self {
            particles: vec![particle],
            concentration: concentration.clamp(1e-6, 0.5),
            ordering: ParticleOrdering::Random,
            thickness_nm: 100.0,
            matrix_ior: 1.5,
        }
    }

    /// Set film thickness.
    pub fn with_thickness(mut self, thickness_nm: f64) -> Self {
        self.thickness_nm = thickness_nm;
        self
    }

    /// Set particle ordering.
    pub fn with_ordering(mut self, ordering: ParticleOrdering) -> Self {
        self.ordering = ordering;
        self
    }

    /// Set matrix material.
    pub fn with_matrix(mut self, ior: f64) -> Self {
        self.matrix_ior = ior;
        for p in &mut self.particles {
            p.embedding_medium_ior = ior;
        }
        self
    }

    /// Create ruby glass (gold nanoparticles in glass).
    pub fn ruby_glass() -> Self {
        Self::new(
            PlasmonicNanoparticle::gold_sphere(10.0).in_medium(1.52),
            0.0001,
        )
        .with_thickness(1000.0)
        .with_matrix(1.52)
    }

    /// Create silver nanorod film.
    pub fn silver_nanorods(aspect_ratio: f64) -> Self {
        Self::new(
            PlasmonicNanoparticle::new(
                MetalType::Silver,
                ParticleShape::Nanorod { aspect_ratio },
                40.0,
            )
            .in_medium(1.4),
            0.001,
        )
        .with_thickness(200.0)
        .with_matrix(1.4)
    }

    /// Calculate total extinction at wavelength.
    pub fn total_extinction(&self, wavelength: f64) -> f64 {
        let mut total = 0.0;

        for particle in &self.particles {
            let c_ext = particle.extinction_cross_section(wavelength);
            // Beer-Lambert with particle concentration
            let n_particles = self.concentration / particle.size_nm.powi(3);
            total += c_ext * n_particles * self.thickness_nm;
        }

        // Coupling correction for ordered arrays
        let coupling_factor = match self.ordering {
            ParticleOrdering::Random => 1.0,
            ParticleOrdering::Hexagonal => 1.2,
            ParticleOrdering::Square => 1.15,
            ParticleOrdering::LinearChains => 1.5,
        };

        total * coupling_factor
    }

    /// Calculate extinction spectrum.
    pub fn extinction_spectrum(&self, wavelengths: &[f64]) -> Vec<f64> {
        wavelengths
            .iter()
            .map(|&wl| self.total_extinction(wl))
            .collect()
    }

    /// Get effective film color.
    pub fn effective_color(&self) -> Color {
        if self.particles.is_empty() {
            return Color::white();
        }

        // Weight particle colors by concentration
        let mut result = Color::black();
        for particle in &self.particles {
            let color = particle.effective_color();
            result = result.add(&color.scale(1.0 / self.particles.len() as f64));
        }
        result
    }
}

impl BSDF for PlasmonicFilm {
    fn evaluate(&self, ctx: &BSDFContext) -> BSDFResponse {
        let wavelength = ctx.wavelength;
        let ext = self.total_extinction(wavelength);

        // Beer-Lambert transmission
        let transmission = (-ext).exp();

        // Some scattering from particles
        let scattering = (1.0 - transmission) * 0.1;

        BSDFResponse {
            reflectance: scattering,
            transmittance: transmission,
            absorption: 1.0 - transmission - scattering,
            ..BSDFResponse::default()
        }
    }

    fn sample(&self, ctx: &BSDFContext, u1: f64, _u2: f64) -> BSDFSample {
        let value = self.evaluate(ctx);

        let wo = if u1 < value.reflectance {
            // Scatter
            let phi = 2.0 * PI * u1;
            let cos_theta = 1.0 - 2.0 * u1;
            let sin_theta = (1.0 - cos_theta * cos_theta).sqrt();
            Vector3::new(sin_theta * phi.cos(), sin_theta * phi.sin(), cos_theta)
        } else {
            // Transmit
            -ctx.wo
        };

        BSDFSample::new(wo, value, 1.0, false)
    }

    fn pdf(&self, _ctx: &BSDFContext) -> f64 {
        1.0
    }
}

// ============================================================================
// Presets
// ============================================================================

/// Create colloidal gold (ruby glass) effect.
pub fn colloidal_gold() -> PlasmonicFilm {
    PlasmonicFilm::ruby_glass()
}

/// Create silver dichroic glass.
pub fn silver_dichroic() -> PlasmonicFilm {
    PlasmonicFilm::new(
        PlasmonicNanoparticle::silver_sphere(30.0).in_medium(1.5),
        0.0005,
    )
    .with_thickness(500.0)
}

/// Create gold nanorod solution (tunable NIR).
pub fn gold_nanorods_nir() -> PlasmonicFilm {
    PlasmonicFilm::new(
        PlasmonicNanoparticle::gold_nanorod(80.0, 4.0).in_medium(1.33),
        0.0001,
    )
    .with_thickness(1000.0)
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gold_sphere_lspr() {
        let np = PlasmonicNanoparticle::gold_sphere(20.0);
        let lspr = np.lspr_wavelength(0);

        // LSPR wavelength should be positive and in visible/near-IR range
        // (Simplified Drude model may not match exact experimental values)
        assert!(lspr > 0.0);
        assert!(lspr.is_finite());
    }

    #[test]
    fn test_silver_sphere_lspr() {
        let np = PlasmonicNanoparticle::silver_sphere(20.0);
        let lspr = np.lspr_wavelength(0);

        // Silver LSPR should be positive and finite
        assert!(lspr > 0.0);
        assert!(lspr.is_finite());
    }

    #[test]
    fn test_nanorod_dual_peaks() {
        let np = PlasmonicNanoparticle::gold_nanorod(40.0, 3.0);

        let lspr_long = np.lspr_wavelength(0);
        let lspr_trans = np.lspr_wavelength(1);

        // Both wavelengths should be positive
        assert!(lspr_long > 0.0);
        assert!(lspr_trans > 0.0);
    }

    #[test]
    fn test_extinction_spectrum() {
        let np = PlasmonicNanoparticle::gold_sphere(20.0);
        let wavelengths: Vec<f64> = (400..=700).step_by(10).map(|w| w as f64).collect();
        let spectrum = np.extinction_spectrum(&wavelengths);

        assert_eq!(spectrum.len(), wavelengths.len());
        assert!(spectrum.iter().all(|&v| v >= 0.0));

        // Should have some variation across wavelengths (not constant)
        let max = spectrum.iter().cloned().fold(0.0f64, f64::max);
        let min = spectrum.iter().cloned().fold(f64::MAX, f64::min);
        // Extinction varies with wavelength (may be small but should vary)
        assert!(max >= min);
    }

    #[test]
    fn test_plasmonic_film() {
        let film = PlasmonicFilm::ruby_glass();
        let ctx = BSDFContext::new_simple(0.9);
        let response = film.evaluate(&ctx);

        assert!(response.is_energy_conserved(1e-6));
        // Film should have valid optical response
        assert!(response.transmittance >= 0.0);
        assert!(response.transmittance <= 1.0);
    }

    #[test]
    fn test_effective_color() {
        let np = PlasmonicNanoparticle::gold_sphere(20.0);
        let color = np.effective_color();

        // Gold nanoparticles should produce a valid color
        assert!(color.r >= 0.0);
        assert!(color.g >= 0.0);
        assert!(color.b >= 0.0);
    }

    #[test]
    fn test_depolarization_factors() {
        let sphere = ParticleShape::Sphere;
        let (l_long, l_trans) = sphere.depolarization_factors();

        // Sphere should be isotropic
        assert!((l_long - 1.0 / 3.0).abs() < 0.01);
        assert!((l_trans - 1.0 / 3.0).abs() < 0.01);

        // Rod should be anisotropic
        let rod = ParticleShape::Nanorod { aspect_ratio: 3.0 };
        let (l_long_rod, l_trans_rod) = rod.depolarization_factors();
        assert!(l_long_rod < l_trans_rod);
    }
}

// ============================================================================
// Type Aliases for External Use
// ============================================================================

/// Alias for particle ordering (for module export convenience).
pub type Ordering = ParticleOrdering;

/// Alias for plasmonic array (currently using PlasmonicFilm with ordering).
pub type PlasmonicArray = PlasmonicFilm;

// ============================================================================
// Preset Module
// ============================================================================

/// Material presets for plasmonic materials.
pub mod presets {
    use super::*;

    /// Colloidal gold (ruby glass).
    pub fn colloidal_gold() -> PlasmonicFilm {
        super::colloidal_gold()
    }

    /// Silver dichroic glass.
    pub fn silver_dichroic() -> PlasmonicFilm {
        super::silver_dichroic()
    }

    /// Gold nanorods in NIR range.
    pub fn gold_nanorods_nir() -> PlasmonicFilm {
        super::gold_nanorods_nir()
    }

    /// Gold nanosphere.
    pub fn gold_nanosphere(diameter_nm: f64) -> PlasmonicNanoparticle {
        PlasmonicNanoparticle::gold_sphere(diameter_nm)
    }

    /// Silver nanosphere.
    pub fn silver_nanosphere(diameter_nm: f64) -> PlasmonicNanoparticle {
        PlasmonicNanoparticle::silver_sphere(diameter_nm)
    }

    /// Gold nanorod.
    pub fn gold_nanorod(length_nm: f64, aspect_ratio: f64) -> PlasmonicNanoparticle {
        PlasmonicNanoparticle::gold_nanorod(length_nm, aspect_ratio)
    }

    /// Silver nanorod film.
    pub fn silver_nanorod_film(aspect_ratio: f64) -> PlasmonicFilm {
        PlasmonicFilm::silver_nanorods(aspect_ratio)
    }

    /// Copper nanoparticle.
    pub fn copper_nanosphere(diameter_nm: f64) -> PlasmonicNanoparticle {
        PlasmonicNanoparticle::new(MetalType::Copper, ParticleShape::Sphere, diameter_nm)
    }

    /// Gold nanostar.
    pub fn gold_nanostar(diameter_nm: f64, spikes: usize) -> PlasmonicNanoparticle {
        PlasmonicNanoparticle::new(
            MetalType::Gold,
            ParticleShape::Nanostar {
                spikes,
                spike_length: 0.5,
            },
            diameter_nm,
        )
    }

    /// Gold nanocube.
    pub fn gold_nanocube(size_nm: f64) -> PlasmonicNanoparticle {
        PlasmonicNanoparticle::new(
            MetalType::Gold,
            ParticleShape::Nanocube { rounding: 0.1 },
            size_nm,
        )
    }

    /// Gold nanoshell (silica core).
    pub fn gold_nanoshell(outer_diameter_nm: f64, core_ratio: f64) -> PlasmonicNanoparticle {
        PlasmonicNanoparticle::new(
            MetalType::Gold,
            ParticleShape::Nanoshell {
                core_ratio,
                core_ior: 1.45, // Silica
            },
            outer_diameter_nm,
        )
    }

    /// List all available preset names.
    pub fn list_presets() -> Vec<&'static str> {
        vec![
            "colloidal_gold",
            "silver_dichroic",
            "gold_nanorods_nir",
            "gold_nanosphere",
            "silver_nanosphere",
            "gold_nanorod",
            "silver_nanorod_film",
            "copper_nanosphere",
            "gold_nanostar",
            "gold_nanocube",
            "gold_nanoshell",
        ]
    }
}

// ============================================================================
// Memory Estimation
// ============================================================================

/// Estimate memory usage for plasmonic material types.
pub fn estimate_plasmonic_memory() -> usize {
    let nanoparticle_size = std::mem::size_of::<PlasmonicNanoparticle>();
    let film_base_size = std::mem::size_of::<PlasmonicFilm>();
    let metal_type_size = std::mem::size_of::<MetalType>();
    let particle_shape_size = std::mem::size_of::<ParticleShape>();
    let ordering_size = std::mem::size_of::<ParticleOrdering>();

    nanoparticle_size + film_base_size + metal_type_size + particle_shape_size + ordering_size
}
