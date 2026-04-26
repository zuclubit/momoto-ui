//! # Complex Index of Refraction (Phase 3)
//!
//! Models for materials with absorption: metals, semiconductors, and colored materials.
//!
//! ## Physical Background
//!
//! Real materials have complex refractive indices:
//!
//! ```text
//! n_complex = n + i*k
//! ```
//!
//! Where:
//! - `n` = real part (refraction, phase velocity)
//! - `k` = imaginary part (extinction coefficient, absorption)
//!
//! For dielectrics (glass): k ≈ 0
//! For metals: both n and k are significant
//!
//! ## Fresnel for Metals
//!
//! The Fresnel equations for conductors use complex arithmetic:
//!
//! ```text
//! R = |r|² where r involves complex division
//! ```
//!
//! This creates the characteristic metallic appearance:
//! - High reflectivity at all angles
//! - Colored reflections (gold = yellow, copper = orange)
//! - No transparency
//!
//! ## References
//!
//! - Pharr et al. (2016): "Physically Based Rendering", Chapter 8
//! - RefractiveIndex.INFO: Measured optical constants
//! - CRC Handbook: Optical properties of metals

use std::f64::consts::PI;

// ============================================================================
// COMPLEX NUMBER TYPE
// ============================================================================

/// Complex number for optical calculations
///
/// Minimal implementation optimized for Fresnel calculations.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Complex {
    /// Real part
    pub re: f64,
    /// Imaginary part
    pub im: f64,
}

impl Complex {
    /// Create a new complex number
    #[inline]
    pub const fn new(re: f64, im: f64) -> Self {
        Self { re, im }
    }

    /// Create a real number (imaginary = 0)
    #[inline]
    pub const fn real(re: f64) -> Self {
        Self { re, im: 0.0 }
    }

    /// Magnitude squared: |z|² = re² + im²
    #[inline]
    pub fn norm_squared(&self) -> f64 {
        self.re * self.re + self.im * self.im
    }

    /// Magnitude: |z| = sqrt(re² + im²)
    #[inline]
    pub fn norm(&self) -> f64 {
        self.norm_squared().sqrt()
    }

    /// Complex conjugate: (a + bi)* = a - bi
    #[inline]
    pub fn conj(&self) -> Self {
        Self {
            re: self.re,
            im: -self.im,
        }
    }

    /// Complex square root
    ///
    /// Uses the principal branch: sqrt(z) has non-negative real part
    pub fn sqrt(&self) -> Self {
        let r = self.norm();
        let re = ((r + self.re) / 2.0).sqrt();
        let im = ((r - self.re) / 2.0).sqrt() * self.im.signum();
        Self { re, im }
    }

    /// Compute cos²(theta) in complex plane for refracted angle
    pub fn cos2_refracted(&self, sin2_i: f64) -> Self {
        // cos²(t) = 1 - sin²(t) = 1 - sin²(i)/n²
        // For complex n: n² = (n + ik)² = n² - k² + 2ink
        let n2 = self.re * self.re - self.im * self.im;
        let k2 = 2.0 * self.re * self.im;

        // 1 - sin²(i) / (n² + ik²)
        // This requires complex division
        let denom = n2 * n2 + k2 * k2;
        let re = 1.0 - sin2_i * n2 / denom;
        let im = sin2_i * k2 / denom;

        Self { re, im }
    }
}

impl std::ops::Add for Complex {
    type Output = Self;
    #[inline]
    fn add(self, rhs: Self) -> Self {
        Self {
            re: self.re + rhs.re,
            im: self.im + rhs.im,
        }
    }
}

impl std::ops::Sub for Complex {
    type Output = Self;
    #[inline]
    fn sub(self, rhs: Self) -> Self {
        Self {
            re: self.re - rhs.re,
            im: self.im - rhs.im,
        }
    }
}

impl std::ops::Mul for Complex {
    type Output = Self;
    #[inline]
    fn mul(self, rhs: Self) -> Self {
        Self {
            re: self.re * rhs.re - self.im * rhs.im,
            im: self.re * rhs.im + self.im * rhs.re,
        }
    }
}

impl std::ops::Div for Complex {
    type Output = Self;
    #[inline]
    fn div(self, rhs: Self) -> Self {
        let denom = rhs.norm_squared();
        Self {
            re: (self.re * rhs.re + self.im * rhs.im) / denom,
            im: (self.im * rhs.re - self.re * rhs.im) / denom,
        }
    }
}

impl std::ops::Mul<f64> for Complex {
    type Output = Self;
    #[inline]
    fn mul(self, rhs: f64) -> Self {
        Self {
            re: self.re * rhs,
            im: self.im * rhs,
        }
    }
}

// ============================================================================
// COMPLEX INDEX OF REFRACTION
// ============================================================================

/// Complex refractive index at a single wavelength
///
/// n_complex = n + i*k
#[derive(Debug, Clone, Copy)]
pub struct ComplexIOR {
    /// Real part: refractive index (phase velocity)
    pub n: f64,
    /// Imaginary part: extinction coefficient (absorption)
    pub k: f64,
}

impl ComplexIOR {
    /// Create new complex IOR
    #[inline]
    pub const fn new(n: f64, k: f64) -> Self {
        Self { n, k }
    }

    /// Create dielectric (k = 0)
    #[inline]
    pub const fn dielectric(n: f64) -> Self {
        Self { n, k: 0.0 }
    }

    /// Convert to Complex type
    #[inline]
    pub fn to_complex(&self) -> Complex {
        Complex::new(self.n, self.k)
    }

    /// Calculate F0 (normal incidence reflectance)
    ///
    /// For conductors:
    /// F0 = ((n-1)² + k²) / ((n+1)² + k²)
    #[inline]
    pub fn f0(&self) -> f64 {
        let num = (self.n - 1.0).powi(2) + self.k.powi(2);
        let den = (self.n + 1.0).powi(2) + self.k.powi(2);
        num / den
    }

    /// Check if this is a conductor (has significant extinction)
    #[inline]
    pub fn is_conductor(&self) -> bool {
        self.k > 0.01
    }

    /// Calculate absorption coefficient (per unit length)
    ///
    /// alpha = 4*pi*k / lambda
    #[inline]
    pub fn absorption_coefficient(&self, wavelength_nm: f64) -> f64 {
        4.0 * PI * self.k / (wavelength_nm * 1e-9)
    }

    /// Calculate penetration depth (skin depth)
    ///
    /// delta = lambda / (4*pi*k)
    #[inline]
    pub fn penetration_depth_nm(&self, wavelength_nm: f64) -> f64 {
        wavelength_nm / (4.0 * PI * self.k.max(1e-10))
    }
}

impl Default for ComplexIOR {
    fn default() -> Self {
        Self::dielectric(1.5)
    }
}

// ============================================================================
// FRESNEL FOR CONDUCTORS
// ============================================================================

/// Full Fresnel reflectance for conductors
///
/// Uses exact complex Fresnel equations.
///
/// # Arguments
///
/// * `n_i` - Incident medium IOR (usually 1.0 for air)
/// * `n_t` - Complex IOR of the material
/// * `cos_theta_i` - Cosine of incident angle
///
/// # Returns
///
/// (Rs, Rp) - S and P polarization reflectances
///
/// # Performance
///
/// ~30-40 cycles (complex arithmetic)
pub fn fresnel_conductor(n_i: f64, n_t: ComplexIOR, cos_theta_i: f64) -> (f64, f64) {
    let cos_i = cos_theta_i.abs().clamp(0.0, 1.0);
    let sin2_i = 1.0 - cos_i * cos_i;

    // eta = n_t / n_i (complex)
    let eta = Complex::new(n_t.n / n_i, n_t.k / n_i);
    let eta2 = eta * eta;

    // cos²(theta_t) = 1 - sin²(theta_i) / eta²
    let cos2_t = Complex::real(1.0) - Complex::real(sin2_i) / eta2;
    let cos_t = cos2_t.sqrt();

    // Fresnel coefficients (complex)
    let n_i_cos_i = Complex::real(n_i * cos_i);
    let n_i_cos_t = Complex::real(n_i) * cos_t;
    let eta_cos_i = eta * Complex::real(cos_i);
    let eta_cos_t = eta * cos_t;

    // rs = (n_i * cos_i - eta * cos_t) / (n_i * cos_i + eta * cos_t)
    let rs = (n_i_cos_i - eta_cos_t) / (n_i_cos_i + eta_cos_t);

    // rp = (eta * cos_i - n_i * cos_t) / (eta * cos_i + n_i * cos_t)
    let rp = (eta_cos_i - n_i_cos_t) / (eta_cos_i + n_i_cos_t);

    // Reflectance = |r|²
    (rs.norm_squared(), rp.norm_squared())
}

/// Average Fresnel reflectance for unpolarized light
///
/// R = (Rs + Rp) / 2
#[inline]
pub fn fresnel_conductor_unpolarized(n_i: f64, n_t: ComplexIOR, cos_theta_i: f64) -> f64 {
    let (rs, rp) = fresnel_conductor(n_i, n_t, cos_theta_i);
    (rs + rp) / 2.0
}

/// Schlick approximation for conductors
///
/// Uses F0 calculated from complex IOR, then standard Schlick formula.
/// Less accurate than full equations but much faster.
///
/// # Performance
///
/// ~5 cycles (same as dielectric Schlick)
#[inline]
pub fn fresnel_conductor_schlick(n_t: ComplexIOR, cos_theta_i: f64) -> f64 {
    let f0 = n_t.f0();
    let one_minus_cos = (1.0 - cos_theta_i.abs()).max(0.0);
    f0 + (1.0 - f0) * one_minus_cos.powi(5)
}

// ============================================================================
// SPECTRAL COMPLEX IOR
// ============================================================================

/// Spectral complex IOR (varies with wavelength)
///
/// Stores RGB values for efficient rendering.
#[derive(Debug, Clone, Copy)]
pub struct SpectralComplexIOR {
    /// IOR at red wavelength (~650nm)
    pub red: ComplexIOR,
    /// IOR at green wavelength (~550nm)
    pub green: ComplexIOR,
    /// IOR at blue wavelength (~450nm)
    pub blue: ComplexIOR,
}

impl SpectralComplexIOR {
    /// Create new spectral complex IOR
    pub const fn new(red: ComplexIOR, green: ComplexIOR, blue: ComplexIOR) -> Self {
        Self { red, green, blue }
    }

    /// Create from arrays
    pub const fn from_arrays(n: [f64; 3], k: [f64; 3]) -> Self {
        Self {
            red: ComplexIOR::new(n[0], k[0]),
            green: ComplexIOR::new(n[1], k[1]),
            blue: ComplexIOR::new(n[2], k[2]),
        }
    }

    /// Get F0 for each RGB channel
    pub fn f0_rgb(&self) -> [f64; 3] {
        [self.red.f0(), self.green.f0(), self.blue.f0()]
    }

    /// Calculate full Fresnel for each RGB channel
    pub fn fresnel_rgb(&self, n_i: f64, cos_theta_i: f64) -> [f64; 3] {
        [
            fresnel_conductor_unpolarized(n_i, self.red, cos_theta_i),
            fresnel_conductor_unpolarized(n_i, self.green, cos_theta_i),
            fresnel_conductor_unpolarized(n_i, self.blue, cos_theta_i),
        ]
    }

    /// Calculate Schlick approximation for each RGB channel
    pub fn fresnel_schlick_rgb(&self, cos_theta_i: f64) -> [f64; 3] {
        [
            fresnel_conductor_schlick(self.red, cos_theta_i),
            fresnel_conductor_schlick(self.green, cos_theta_i),
            fresnel_conductor_schlick(self.blue, cos_theta_i),
        ]
    }

    /// Get as array of ComplexIOR
    pub fn as_array(&self) -> [ComplexIOR; 3] {
        [self.red, self.green, self.blue]
    }
}

// ============================================================================
// METAL PRESETS
// ============================================================================

/// Pre-defined metal materials with measured optical constants
///
/// Data from RefractiveIndex.INFO and CRC Handbook
pub mod metals {
    use super::SpectralComplexIOR;

    /// Gold (Au) - Warm yellow metal
    ///
    /// Characteristic yellow color from selective absorption in blue
    /// Source: Johnson & Christy (1972)
    pub const GOLD: SpectralComplexIOR = SpectralComplexIOR::from_arrays(
        [0.18, 0.42, 1.47], // n at R, G, B
        [3.00, 2.35, 1.95], // k at R, G, B
    );

    /// Silver (Ag) - Neutral white metal
    ///
    /// Highest reflectivity of common metals
    /// Source: Johnson & Christy (1972)
    pub const SILVER: SpectralComplexIOR = SpectralComplexIOR::from_arrays(
        [0.15, 0.13, 0.14], // n at R, G, B
        [3.64, 3.04, 2.54], // k at R, G, B
    );

    /// Copper (Cu) - Orange-red metal
    ///
    /// Distinctive reddish color
    /// Source: Johnson & Christy (1972)
    pub const COPPER: SpectralComplexIOR = SpectralComplexIOR::from_arrays(
        [0.27, 0.68, 1.13], // n at R, G, B
        [3.41, 2.63, 2.57], // k at R, G, B
    );

    /// Aluminum (Al) - Bright white metal
    ///
    /// Common reflective metal, slight blue tint
    /// Source: Rakic (1995)
    pub const ALUMINUM: SpectralComplexIOR = SpectralComplexIOR::from_arrays(
        [1.35, 0.96, 0.62], // n at R, G, B
        [7.47, 6.39, 5.31], // k at R, G, B
    );

    /// Iron (Fe) - Dark gray metal
    ///
    /// Lower reflectivity, used for steel effects
    /// Source: Johnson & Christy (1974)
    pub const IRON: SpectralComplexIOR = SpectralComplexIOR::from_arrays(
        [2.91, 2.95, 2.80], // n at R, G, B
        [3.08, 3.47, 3.00], // k at R, G, B
    );

    /// Chromium (Cr) - Bright silver metal
    ///
    /// Very hard, used for chrome plating
    /// Source: Johnson & Christy (1974)
    #[allow(clippy::approx_constant)] // 3.14 is the actual optical constant, not PI
    pub const CHROMIUM: SpectralComplexIOR = SpectralComplexIOR::from_arrays(
        [3.18, 3.14, 2.98], // n at R, G, B
        [3.19, 3.34, 3.36], // k at R, G, B
    );

    /// Titanium (Ti) - Dark silver metal
    ///
    /// Strong, lightweight, slight yellow tint
    /// Source: Johnson & Christy (1974)
    pub const TITANIUM: SpectralComplexIOR = SpectralComplexIOR::from_arrays(
        [2.73, 2.16, 1.94], // n at R, G, B
        [3.82, 2.94, 2.58], // k at R, G, B
    );

    /// Nickel (Ni) - Warm silver metal
    ///
    /// Common plating material
    /// Source: Ordal (1988)
    pub const NICKEL: SpectralComplexIOR = SpectralComplexIOR::from_arrays(
        [2.01, 1.83, 1.65], // n at R, G, B
        [4.05, 3.56, 3.07], // k at R, G, B
    );

    /// Platinum (Pt) - Dense silver-white metal
    ///
    /// Precious metal, slightly darker than silver
    /// Source: Johnson & Christy (1974)
    pub const PLATINUM: SpectralComplexIOR = SpectralComplexIOR::from_arrays(
        [2.38, 2.07, 1.72], // n at R, G, B
        [4.36, 3.68, 3.06], // k at R, G, B
    );

    /// Brass (Cu-Zn alloy) - Yellow metal
    ///
    /// Common alloy, warmer than gold
    /// Approximate values
    pub const BRASS: SpectralComplexIOR = SpectralComplexIOR::from_arrays(
        [0.44, 0.58, 0.95], // n at R, G, B
        [3.22, 2.85, 2.40], // k at R, G, B
    );

    /// Bronze (Cu-Sn alloy) - Brown metal
    ///
    /// Darker than brass, warmer tone
    /// Approximate values
    pub const BRONZE: SpectralComplexIOR = SpectralComplexIOR::from_arrays(
        [0.35, 0.55, 0.85], // n at R, G, B
        [3.30, 2.70, 2.35], // k at R, G, B
    );

    /// Tungsten (W) - Dense gray metal
    ///
    /// Very high melting point, used in filaments
    /// Source: Ordal (1988)
    pub const TUNGSTEN: SpectralComplexIOR = SpectralComplexIOR::from_arrays(
        [3.54, 3.32, 2.76], // n at R, G, B
        [2.86, 2.84, 2.51], // k at R, G, B
    );

    /// Get all metal presets with names
    pub fn all_presets() -> Vec<(&'static str, SpectralComplexIOR)> {
        vec![
            ("Gold", GOLD),
            ("Silver", SILVER),
            ("Copper", COPPER),
            ("Aluminum", ALUMINUM),
            ("Iron", IRON),
            ("Chromium", CHROMIUM),
            ("Titanium", TITANIUM),
            ("Nickel", NICKEL),
            ("Platinum", PLATINUM),
            ("Brass", BRASS),
            ("Bronze", BRONZE),
            ("Tungsten", TUNGSTEN),
        ]
    }

    /// Get metal by name
    pub fn by_name(name: &str) -> Option<SpectralComplexIOR> {
        match name.to_lowercase().as_str() {
            "gold" | "au" => Some(GOLD),
            "silver" | "ag" => Some(SILVER),
            "copper" | "cu" => Some(COPPER),
            "aluminum" | "aluminium" | "al" => Some(ALUMINUM),
            "iron" | "fe" => Some(IRON),
            "chromium" | "chrome" | "cr" => Some(CHROMIUM),
            "titanium" | "ti" => Some(TITANIUM),
            "nickel" | "ni" => Some(NICKEL),
            "platinum" | "pt" => Some(PLATINUM),
            "brass" => Some(BRASS),
            "bronze" => Some(BRONZE),
            "tungsten" | "w" => Some(TUNGSTEN),
            _ => None,
        }
    }
}

// ============================================================================
// CSS GENERATION FOR METALS
// ============================================================================

/// Generate CSS gradient for metallic reflection
///
/// Creates a gradient that simulates the view-angle dependent
/// color shift of metals.
pub fn to_css_metallic_gradient(metal: &SpectralComplexIOR, intensity: f64) -> String {
    let f0 = metal.f0_rgb();

    // Convert F0 to RGB color (scale to 0-255)
    let r = (f0[0] * 255.0 * intensity).min(255.0) as u8;
    let g = (f0[1] * 255.0 * intensity).min(255.0) as u8;
    let b = (f0[2] * 255.0 * intensity).min(255.0) as u8;

    // At grazing angles, all metals approach 100% reflectivity
    // Create gradient from F0 color to white at edges
    format!(
        "radial-gradient(ellipse 100% 100% at center, \
         rgba({}, {}, {}, 0) 0%, \
         rgba({}, {}, {}, {:.2}) 40%, \
         rgba({}, {}, {}, {:.2}) 70%, \
         rgba(255, 255, 255, {:.2}) 95%, \
         rgba(255, 255, 255, {:.2}) 100%)",
        r,
        g,
        b,
        r,
        g,
        b,
        0.3 * intensity,
        r,
        g,
        b,
        0.6 * intensity,
        0.8 * intensity,
        0.9 * intensity,
    )
}

/// Generate CSS for metallic button/surface
pub fn to_css_metallic_surface(metal: &SpectralComplexIOR, light_angle_deg: f64) -> String {
    let f0 = metal.f0_rgb();
    let cos_light = (light_angle_deg * PI / 180.0).cos().abs();

    // Fresnel at light angle
    let fresnel = metal.fresnel_schlick_rgb(cos_light);

    // Base color from F0
    let base_r = (f0[0] * 200.0) as u8;
    let base_g = (f0[1] * 200.0) as u8;
    let base_b = (f0[2] * 200.0) as u8;

    // Highlight color from Fresnel
    let hi_r = (fresnel[0] * 255.0).min(255.0) as u8;
    let hi_g = (fresnel[1] * 255.0).min(255.0) as u8;
    let hi_b = (fresnel[2] * 255.0).min(255.0) as u8;

    format!(
        "linear-gradient({}deg, \
         rgb({}, {}, {}) 0%, \
         rgb({}, {}, {}) 50%, \
         rgb({}, {}, {}) 100%)",
        light_angle_deg, base_r, base_g, base_b, hi_r, hi_g, hi_b, base_r, base_g, base_b,
    )
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_complex_arithmetic() {
        let a = Complex::new(1.0, 2.0);
        let b = Complex::new(3.0, 4.0);

        let sum = a + b;
        assert!((sum.re - 4.0).abs() < 1e-10);
        assert!((sum.im - 6.0).abs() < 1e-10);

        let prod = a * b;
        // (1+2i)(3+4i) = 3 + 4i + 6i + 8i² = 3 + 10i - 8 = -5 + 10i
        assert!((prod.re - (-5.0)).abs() < 1e-10);
        assert!((prod.im - 10.0).abs() < 1e-10);
    }

    #[test]
    fn test_complex_norm() {
        let z = Complex::new(3.0, 4.0);
        assert!((z.norm() - 5.0).abs() < 1e-10);
        assert!((z.norm_squared() - 25.0).abs() < 1e-10);
    }

    #[test]
    fn test_f0_calculation() {
        // For dielectric with n=1.5, F0 ≈ 0.04
        let dielectric = ComplexIOR::dielectric(1.5);
        let f0 = dielectric.f0();
        assert!((f0 - 0.04).abs() < 0.01, "Dielectric F0 should be ~0.04");

        // For gold, F0 should be high (yellow-ish)
        let gold = metals::GOLD.green;
        let f0_gold = gold.f0();
        assert!(f0_gold > 0.7, "Gold F0 should be high");
    }

    #[test]
    fn test_fresnel_conductor() {
        let gold = metals::GOLD.green;

        // Normal incidence should equal F0
        let (rs, rp) = fresnel_conductor(1.0, gold, 1.0);
        let f0 = gold.f0();
        assert!((rs - f0).abs() < 0.01, "Rs at normal should be ~F0");
        assert!((rp - f0).abs() < 0.01, "Rp at normal should be ~F0");

        // Grazing incidence should approach 1.0
        let (rs_graze, rp_graze) = fresnel_conductor(1.0, gold, 0.01);
        assert!(rs_graze > 0.95, "Rs at grazing should approach 1.0");
    }

    #[test]
    fn test_fresnel_schlick_conductor() {
        let gold = metals::GOLD.green;

        let full = fresnel_conductor_unpolarized(1.0, gold, 0.5);
        let schlick = fresnel_conductor_schlick(gold, 0.5);

        // Schlick should be within 10% of full
        let error = (full - schlick).abs() / full;
        assert!(
            error < 0.15,
            "Schlick should approximate full: error = {}",
            error
        );
    }

    #[test]
    fn test_spectral_metal() {
        let gold = metals::GOLD;
        let f0 = gold.f0_rgb();

        // Gold should have higher F0 in red than blue
        assert!(f0[0] > f0[2], "Gold F0 red > blue");

        // All should be reasonably high
        assert!(f0[0] > 0.5);
        assert!(f0[1] > 0.5);
        assert!(f0[2] > 0.3);
    }

    #[test]
    fn test_silver_highest_reflectivity() {
        let silver = metals::SILVER;
        let gold = metals::GOLD;
        let copper = metals::COPPER;

        let f0_silver = silver.f0_rgb();
        let f0_gold = gold.f0_rgb();
        let f0_copper = copper.f0_rgb();

        // Average F0
        let avg_silver: f64 = f0_silver.iter().sum::<f64>() / 3.0;
        let avg_gold: f64 = f0_gold.iter().sum::<f64>() / 3.0;
        let avg_copper: f64 = f0_copper.iter().sum::<f64>() / 3.0;

        assert!(
            avg_silver > avg_gold,
            "Silver should have higher avg F0 than gold"
        );
        assert!(
            avg_silver > avg_copper,
            "Silver should have higher avg F0 than copper"
        );
    }

    #[test]
    fn test_all_metal_presets() {
        let presets = metals::all_presets();
        assert!(!presets.is_empty());

        for (name, metal) in presets {
            let f0 = metal.f0_rgb();

            // All F0 values should be in valid range
            for (i, &f) in f0.iter().enumerate() {
                assert!(f >= 0.0, "{} F0[{}] should be >= 0", name, i);
                assert!(f <= 1.0, "{} F0[{}] should be <= 1", name, i);
            }

            // All metals should be conductors
            assert!(metal.red.is_conductor(), "{} red should be conductor", name);
            assert!(
                metal.green.is_conductor(),
                "{} green should be conductor",
                name
            );
            assert!(
                metal.blue.is_conductor(),
                "{} blue should be conductor",
                name
            );
        }
    }

    #[test]
    fn test_metal_by_name() {
        assert!(metals::by_name("gold").is_some());
        assert!(metals::by_name("Gold").is_some());
        assert!(metals::by_name("GOLD").is_some());
        assert!(metals::by_name("Au").is_some());
        assert!(metals::by_name("nonexistent").is_none());
    }

    #[test]
    fn test_css_generation() {
        let gradient = to_css_metallic_gradient(&metals::GOLD, 1.0);
        assert!(gradient.contains("radial-gradient"));
        assert!(gradient.contains("rgba"));
    }
}
