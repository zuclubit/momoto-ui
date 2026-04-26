//! # Chromatic Dispersion Models
//!
//! Models for wavelength-dependent refractive index.
//!
//! ## Physical Background
//!
//! Real glass materials have refractive indices that vary with wavelength.
//! This creates chromatic effects:
//! - **Chromatic aberration**: Color fringing at edges
//! - **Rainbows/prisms**: Spectral separation of white light
//! - **Fire in diamonds**: Dispersion creates colored sparkles
//!
//! ## Models Implemented
//!
//! - **Cauchy**: Simple polynomial approximation (P0 priority)
//! - **Sellmeier**: Resonance-based, high accuracy (P1 priority)
//!
//! ## References
//!
//! - Cauchy (1836): "Memoire sur la dispersion de la lumiere"
//! - Born & Wolf (1999): "Principles of Optics", Chapter 2
//! - RefractiveIndex.INFO database: https://refractiveindex.info/

// ============================================================================
// WAVELENGTH CONSTANTS
// ============================================================================

/// Standard wavelengths for RGB spectral sampling (in nanometers)
pub mod wavelengths {
    /// Red channel dominant wavelength (C-line, Hydrogen)
    pub const RED: f64 = 656.3;

    /// Green channel dominant wavelength (d-line, Helium)
    pub const GREEN: f64 = 587.6;

    /// Blue channel dominant wavelength (F-line, Hydrogen)
    pub const BLUE: f64 = 486.1;

    /// Yellow sodium D-line (reference for Abbe number)
    pub const SODIUM_D: f64 = 589.3;

    /// Visible spectrum range
    pub const VISIBLE_MIN: f64 = 380.0;
    pub const VISIBLE_MAX: f64 = 780.0;
}

// ============================================================================
// DISPERSION TRAIT
// ============================================================================

/// Trait for dispersion models
///
/// All dispersion models implement this trait for consistent interface.
pub trait Dispersion: Send + Sync {
    /// Calculate refractive index at given wavelength
    ///
    /// # Arguments
    /// * `wavelength_nm` - Wavelength in nanometers (380-780 visible)
    ///
    /// # Returns
    /// Refractive index n (typically 1.0 to 2.5)
    fn n(&self, wavelength_nm: f64) -> f64;

    /// Calculate refractive indices for RGB channels
    ///
    /// Uses standard wavelengths: R=656.3nm, G=587.6nm, B=486.1nm
    fn n_rgb(&self) -> [f64; 3] {
        [
            self.n(wavelengths::RED),
            self.n(wavelengths::GREEN),
            self.n(wavelengths::BLUE),
        ]
    }

    /// Calculate Abbe number (dispersion strength)
    ///
    /// V_d = (n_d - 1) / (n_F - n_C)
    ///
    /// Higher values = less dispersion (crown glass ~60)
    /// Lower values = more dispersion (flint glass ~30)
    fn abbe_number(&self) -> f64 {
        let n_d = self.n(wavelengths::SODIUM_D);
        let n_f = self.n(wavelengths::BLUE); // F-line
        let n_c = self.n(wavelengths::RED); // C-line

        (n_d - 1.0) / (n_f - n_c)
    }

    /// Get base refractive index (at d-line)
    fn n_base(&self) -> f64 {
        self.n(wavelengths::SODIUM_D)
    }
}

// ============================================================================
// CAUCHY DISPERSION (P0 Priority)
// ============================================================================

/// Cauchy's dispersion equation
///
/// Simple polynomial approximation for most transparent materials.
///
/// # Formula
///
/// ```text
/// n(λ) = A + B/λ² + C/λ⁴
/// ```
///
/// Where:
/// - A ≈ n_d (index at d-line)
/// - B = first dispersion coefficient (nm²)
/// - C = second coefficient (nm⁴, often 0)
///
/// # Performance
///
/// ~5 cycles (2 divisions, 2 additions)
///
/// # Example
///
/// ```rust
/// use momoto_materials::glass_physics::dispersion::{CauchyDispersion, Dispersion};
///
/// // Crown glass (BK7)
/// let crown = CauchyDispersion::crown_glass();
/// let n_green = crown.n(550.0);
/// assert!(n_green > 1.51 && n_green < 1.53);
/// ```
#[derive(Debug, Clone, Copy)]
pub struct CauchyDispersion {
    /// Base refractive index (A coefficient)
    pub a: f64,
    /// First dispersion coefficient in nm² (B coefficient)
    pub b: f64,
    /// Second dispersion coefficient in nm⁴ (C coefficient)
    pub c: f64,
}

impl CauchyDispersion {
    /// Create new Cauchy dispersion model
    pub const fn new(a: f64, b: f64, c: f64) -> Self {
        Self { a, b, c }
    }

    /// Create from base IOR with default dispersion
    ///
    /// Uses empirical relationship between IOR and dispersion.
    /// Higher IOR typically means higher dispersion.
    pub fn from_ior(ior: f64) -> Self {
        // Empirical: B coefficient scales with IOR
        // Typical glass: B ~ 4000-8000 nm²
        let b = (ior - 1.0) * 6000.0;
        Self { a: ior, b, c: 0.0 }
    }

    /// Create non-dispersive model (constant IOR)
    pub const fn constant(ior: f64) -> Self {
        Self {
            a: ior,
            b: 0.0,
            c: 0.0,
        }
    }

    // ========================================================================
    // MATERIAL PRESETS
    // ========================================================================

    /// Crown glass (BK7) - Low dispersion, common optical glass
    /// Abbe number ~64
    /// Coefficients derived to give n_d = 1.5168 at sodium D-line
    pub const fn crown_glass() -> Self {
        // A is set so that A + B/λ_d² ≈ 1.517
        // With B = 4200 nm², at λ_d = 589.3nm: 1.5047 + 4200/347354 ≈ 1.5168
        Self::new(1.5047, 4200.0, 0.0)
    }

    /// Flint glass (SF11) - High dispersion, dense glass
    /// Abbe number ~25
    pub const fn flint_glass() -> Self {
        Self::new(1.7847, 14800.0, 0.0)
    }

    /// Fused silica - Very low dispersion, pure SiO2
    /// Abbe number ~68
    pub const fn fused_silica() -> Self {
        Self::new(1.4585, 3540.0, 0.0)
    }

    /// Water at 20°C
    /// Abbe number ~56
    pub const fn water() -> Self {
        Self::new(1.333, 3100.0, 0.0)
    }

    /// Diamond - Very high dispersion ("fire")
    /// Abbe number ~44
    pub const fn diamond() -> Self {
        Self::new(2.417, 27000.0, 0.0)
    }

    /// Polycarbonate (PC) - High dispersion plastic
    /// Abbe number ~30
    pub const fn polycarbonate() -> Self {
        Self::new(1.585, 12000.0, 0.0)
    }

    /// PMMA (Acrylic) - Low dispersion plastic
    /// Abbe number ~57
    pub const fn pmma() -> Self {
        Self::new(1.492, 5000.0, 0.0)
    }
}

impl Dispersion for CauchyDispersion {
    #[inline]
    fn n(&self, wavelength_nm: f64) -> f64 {
        let lambda2 = wavelength_nm * wavelength_nm;
        let lambda4 = lambda2 * lambda2;

        self.a + self.b / lambda2 + self.c / lambda4
    }
}

impl Default for CauchyDispersion {
    fn default() -> Self {
        Self::crown_glass()
    }
}

// ============================================================================
// SELLMEIER DISPERSION (P1 Priority)
// ============================================================================

/// Sellmeier's dispersion equation
///
/// Resonance-based model with higher accuracy than Cauchy,
/// especially in UV and IR ranges.
///
/// # Formula
///
/// ```text
/// n²(λ) = 1 + Σᵢ (Bᵢ * λ²) / (λ² - Cᵢ)
/// ```
///
/// Where:
/// - Bᵢ = oscillator strengths
/// - Cᵢ = resonance wavelengths squared (μm²)
///
/// # Performance
///
/// ~15 cycles (3 divisions, sqrt)
///
/// # Example
///
/// ```rust
/// use momoto_materials::glass_physics::dispersion::{SellmeierDispersion, Dispersion};
///
/// let silica = SellmeierDispersion::fused_silica();
/// let n = silica.n(550.0);
/// assert!((n - 1.4599).abs() < 0.001);
/// ```
#[derive(Debug, Clone, Copy)]
pub struct SellmeierDispersion {
    /// Oscillator strengths (dimensionless)
    pub b: [f64; 3],
    /// Resonance wavelengths squared in μm²
    pub c: [f64; 3],
}

impl SellmeierDispersion {
    /// Create new Sellmeier dispersion model
    pub const fn new(b: [f64; 3], c: [f64; 3]) -> Self {
        Self { b, c }
    }

    // ========================================================================
    // MATERIAL PRESETS (from RefractiveIndex.INFO)
    // ========================================================================

    /// Fused silica (SiO2)
    /// Source: Malitson 1965
    pub const fn fused_silica() -> Self {
        Self::new(
            [0.6961663, 0.4079426, 0.8974794],
            [
                0.0684043 * 0.0684043,
                0.1162414 * 0.1162414,
                9.896161 * 9.896161,
            ],
        )
    }

    /// BK7 optical glass (Schott)
    /// Common crown glass
    pub const fn bk7() -> Self {
        Self::new(
            [1.03961212, 0.231792344, 1.01046945],
            [0.00600069867, 0.0200179144, 103.560653],
        )
    }

    /// SF11 flint glass (Schott)
    /// High dispersion glass
    pub const fn sf11() -> Self {
        Self::new(
            [1.73759695, 0.313747346, 1.89878101],
            [0.013188707, 0.0623068142, 155.23629],
        )
    }

    /// Sapphire (Al2O3)
    /// Ordinary ray
    pub const fn sapphire() -> Self {
        Self::new(
            [1.4313493, 0.65054713, 5.3414021],
            [0.0052799261, 0.0142382647, 325.01783],
        )
    }

    /// Diamond (C)
    pub const fn diamond() -> Self {
        Self::new(
            [0.3306, 4.3356, 0.0],
            [0.0175 * 0.0175, 0.1060 * 0.1060, 0.0],
        )
    }
}

impl Dispersion for SellmeierDispersion {
    #[inline]
    fn n(&self, wavelength_nm: f64) -> f64 {
        // Convert nm to μm for Sellmeier coefficients
        let lambda_um = wavelength_nm / 1000.0;
        let lambda2 = lambda_um * lambda_um;

        let mut n2 = 1.0;
        for i in 0..3 {
            if self.b[i] != 0.0 {
                n2 += self.b[i] * lambda2 / (lambda2 - self.c[i]);
            }
        }

        n2.sqrt()
    }
}

impl Default for SellmeierDispersion {
    fn default() -> Self {
        Self::bk7()
    }
}

// ============================================================================
// DISPERSION ENUM (Unified Interface)
// ============================================================================

/// Unified dispersion model enum
///
/// Allows runtime selection of dispersion model while maintaining
/// performance for the common case (Cauchy).
#[derive(Debug, Clone)]
pub enum DispersionModel {
    /// No dispersion (constant IOR)
    None(f64),
    /// Cauchy polynomial (fast, good for most cases)
    Cauchy(CauchyDispersion),
    /// Sellmeier resonance (accurate, slower)
    Sellmeier(SellmeierDispersion),
}

impl DispersionModel {
    /// Create non-dispersive model
    pub const fn constant(ior: f64) -> Self {
        Self::None(ior)
    }

    /// Create from base IOR with automatic Cauchy coefficients
    pub fn from_ior(ior: f64) -> Self {
        Self::Cauchy(CauchyDispersion::from_ior(ior))
    }

    /// Check if model has wavelength-dependent dispersion
    pub fn is_dispersive(&self) -> bool {
        match self {
            Self::None(_) => false,
            Self::Cauchy(c) => c.b != 0.0 || c.c != 0.0,
            Self::Sellmeier(_) => true,
        }
    }
}

impl Dispersion for DispersionModel {
    #[inline]
    fn n(&self, wavelength_nm: f64) -> f64 {
        match self {
            Self::None(ior) => *ior,
            Self::Cauchy(cauchy) => cauchy.n(wavelength_nm),
            Self::Sellmeier(sellmeier) => sellmeier.n(wavelength_nm),
        }
    }
}

impl Default for DispersionModel {
    fn default() -> Self {
        Self::Cauchy(CauchyDispersion::crown_glass())
    }
}

// ============================================================================
// F0 CALCULATION (for Fresnel)
// ============================================================================

/// Calculate Fresnel F0 (reflectance at normal incidence) from IOR
///
/// F0 = ((n - 1) / (n + 1))²
#[inline]
pub fn f0_from_ior(ior: f64) -> f64 {
    let term = (ior - 1.0) / (ior + 1.0);
    term * term
}

/// Calculate F0 for RGB channels from dispersion model
pub fn f0_rgb<D: Dispersion>(dispersion: &D) -> [f64; 3] {
    let n_rgb = dispersion.n_rgb();
    [
        f0_from_ior(n_rgb[0]),
        f0_from_ior(n_rgb[1]),
        f0_from_ior(n_rgb[2]),
    ]
}

// ============================================================================
// SPECTRAL UTILITIES
// ============================================================================

/// Calculate chromatic aberration strength
///
/// Returns the difference in IOR between red and blue light.
/// Higher values = more visible chromatic effects.
pub fn chromatic_aberration_strength<D: Dispersion>(dispersion: &D) -> f64 {
    let n_blue = dispersion.n(wavelengths::BLUE);
    let n_red = dispersion.n(wavelengths::RED);
    n_blue - n_red
}

/// Estimate visual chromatic aberration (in degrees)
///
/// Returns the angular separation between red and blue rays
/// at a given incident angle.
pub fn chromatic_angle_separation<D: Dispersion>(dispersion: &D, incident_angle_rad: f64) -> f64 {
    let n_red = dispersion.n(wavelengths::RED);
    let n_blue = dispersion.n(wavelengths::BLUE);

    let sin_i = incident_angle_rad.sin();

    // Snell's law: sin(θ_t) = sin(θ_i) / n
    let sin_t_red = sin_i / n_red;
    let sin_t_blue = sin_i / n_blue;

    // Clamp to avoid domain errors at TIR
    let theta_red = sin_t_red.clamp(-1.0, 1.0).asin();
    let theta_blue = sin_t_blue.clamp(-1.0, 1.0).asin();

    (theta_red - theta_blue).abs()
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cauchy_crown_glass() {
        let crown = CauchyDispersion::crown_glass();

        // Check d-line index
        let n_d = crown.n(wavelengths::SODIUM_D);
        assert!(
            (n_d - 1.517).abs() < 0.01,
            "Crown glass n_d should be ~1.517"
        );

        // Check dispersion (blue should be higher than red)
        let n_blue = crown.n(wavelengths::BLUE);
        let n_red = crown.n(wavelengths::RED);
        assert!(n_blue > n_red, "Blue light should have higher n than red");

        // Check Abbe number (~64 for crown glass)
        let abbe = crown.abbe_number();
        assert!(
            abbe > 55.0 && abbe < 75.0,
            "Crown glass Abbe ~60-70, got {}",
            abbe
        );
    }

    #[test]
    fn test_cauchy_flint_glass() {
        let flint = CauchyDispersion::flint_glass();

        // Flint glass should have higher dispersion (lower Abbe number)
        let abbe = flint.abbe_number();
        assert!(
            abbe > 20.0 && abbe < 35.0,
            "Flint glass Abbe ~25-30, got {}",
            abbe
        );

        // Higher IOR than crown
        assert!(flint.n_base() > 1.7);
    }

    #[test]
    fn test_sellmeier_fused_silica() {
        let silica = SellmeierDispersion::fused_silica();

        // Check against known values
        let n_546 = silica.n(546.1);
        assert!(
            (n_546 - 1.4601).abs() < 0.001,
            "Fused silica n at 546nm should be ~1.4601"
        );

        let n_633 = silica.n(632.8); // HeNe laser line
        assert!(
            (n_633 - 1.4570).abs() < 0.001,
            "Fused silica n at 633nm should be ~1.4570"
        );
    }

    #[test]
    fn test_sellmeier_bk7() {
        let bk7 = SellmeierDispersion::bk7();

        let n_d = bk7.n(wavelengths::SODIUM_D);
        assert!((n_d - 1.5168).abs() < 0.002, "BK7 n_d should be ~1.5168");
    }

    #[test]
    fn test_dispersion_model_enum() {
        // Test unified interface
        let models: Vec<DispersionModel> = vec![
            DispersionModel::constant(1.5),
            DispersionModel::Cauchy(CauchyDispersion::crown_glass()),
            DispersionModel::Sellmeier(SellmeierDispersion::bk7()),
        ];

        for model in &models {
            let n = model.n(550.0);
            assert!(n > 1.0 && n < 3.0, "IOR should be reasonable");
        }

        // Non-dispersive should return constant
        let constant = DispersionModel::constant(1.5);
        assert!(!constant.is_dispersive());
        assert_eq!(constant.n(400.0), constant.n(700.0));
    }

    #[test]
    fn test_f0_calculation() {
        // Air-glass F0 should be ~4%
        let f0_glass = f0_from_ior(1.5);
        assert!((f0_glass - 0.04).abs() < 0.01);

        // Diamond F0 should be ~17%
        let f0_diamond = f0_from_ior(2.4);
        assert!((f0_diamond - 0.17).abs() < 0.02);
    }

    #[test]
    fn test_rgb_sampling() {
        let crown = CauchyDispersion::crown_glass();
        let n_rgb = crown.n_rgb();

        // RGB should be in order: red < green < blue
        assert!(n_rgb[0] < n_rgb[1], "n_red should be less than n_green");
        assert!(n_rgb[1] < n_rgb[2], "n_green should be less than n_blue");
    }

    #[test]
    fn test_chromatic_aberration() {
        let crown = CauchyDispersion::crown_glass();
        let flint = CauchyDispersion::flint_glass();

        let ca_crown = chromatic_aberration_strength(&crown);
        let ca_flint = chromatic_aberration_strength(&flint);

        // Flint should have stronger chromatic aberration
        assert!(
            ca_flint > ca_crown,
            "Flint should have more chromatic aberration"
        );
    }

    #[test]
    fn test_from_ior() {
        let model = CauchyDispersion::from_ior(1.5);

        // Should give reasonable values
        let n_d = model.n(wavelengths::SODIUM_D);
        assert!((n_d - 1.5).abs() < 0.1);

        // Should have some dispersion
        assert!(model.b > 0.0);
    }
}
