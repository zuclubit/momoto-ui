//! # Full Spectral Rendering Module (Phase 7)
//!
//! 31-point visible spectrum rendering with CIE color matching functions.
//! Enables accurate color reproduction under different illuminants.
//!
//! ## Features
//!
//! - Full visible spectrum evaluation (400-700nm, 10nm steps)
//! - CIE 1931 2-degree color matching functions
//! - Multiple illuminant support (D50, D65, A, F2)
//! - Spectral to XYZ to sRGB conversion pipeline
//! - Correlated Color Temperature (CCT) computation

use std::f64::consts::PI;

use super::combined_effects::CombinedMaterial;
use super::perceptual_loss::{xyz_to_rgb, Illuminant, XyzColor};

// ============================================================================
// CONSTANTS
// ============================================================================

/// Number of wavelength samples in visible spectrum
pub const WAVELENGTH_COUNT: usize = 31;

/// Minimum wavelength (nm)
pub const WAVELENGTH_MIN: f64 = 400.0;

/// Maximum wavelength (nm)
pub const WAVELENGTH_MAX: f64 = 700.0;

/// Wavelength step (nm)
pub const WAVELENGTH_STEP: f64 = 10.0;

// ============================================================================
// CIE 1931 COLOR MATCHING FUNCTIONS
// ============================================================================

/// CIE 1931 2-degree Standard Observer x̄(λ) values (400-700nm, 10nm steps)
pub const CIE_X_BAR: [f64; 31] = [
    0.01431, 0.04351, 0.13438, 0.28390, 0.34828, 0.33620, 0.29080, 0.19536, 0.09564, 0.03201,
    0.00490, 0.00930, 0.06327, 0.16550, 0.29040, 0.43345, 0.59450, 0.76210, 0.91630, 1.02630,
    1.06220, 1.00260, 0.85445, 0.64240, 0.44790, 0.28350, 0.16490, 0.08740, 0.04677, 0.02270,
    0.01135,
];

/// CIE 1931 2-degree Standard Observer ȳ(λ) values (400-700nm, 10nm steps)
pub const CIE_Y_BAR: [f64; 31] = [
    0.00040, 0.00120, 0.00400, 0.01160, 0.02300, 0.03800, 0.06000, 0.09100, 0.13902, 0.20802,
    0.32300, 0.50300, 0.71000, 0.86200, 0.95400, 0.99495, 0.99500, 0.95200, 0.87000, 0.75700,
    0.63100, 0.50300, 0.38100, 0.26500, 0.17500, 0.10700, 0.06100, 0.03200, 0.01700, 0.00821,
    0.00410,
];

/// CIE 1931 2-degree Standard Observer z̄(λ) values (400-700nm, 10nm steps)
pub const CIE_Z_BAR: [f64; 31] = [
    0.06785, 0.20740, 0.64560, 1.38560, 1.74706, 1.77211, 1.66920, 1.28764, 0.81295, 0.46518,
    0.27200, 0.15820, 0.07825, 0.04216, 0.02030, 0.00875, 0.00390, 0.00210, 0.00165, 0.00110,
    0.00080, 0.00034, 0.00019, 0.00005, 0.00002, 0.00000, 0.00000, 0.00000, 0.00000, 0.00000,
    0.00000,
];

// ============================================================================
// ILLUMINANT SPECTRA
// ============================================================================

/// CIE Standard Illuminant D65 (daylight) relative spectral power distribution
pub const ILLUMINANT_D65_SPD: [f64; 31] = [
    82.75, 91.49, 93.43, 86.68, 104.86, 117.01, 117.81, 114.86, 115.92, 108.81, 109.35, 107.80,
    104.79, 107.69, 104.41, 104.05, 100.00, 96.33, 95.79, 88.69, 90.01, 89.60, 87.70, 83.29, 83.70,
    80.03, 80.21, 82.28, 78.28, 69.72, 71.61,
];

/// CIE Standard Illuminant D50 (horizon daylight) relative SPD
pub const ILLUMINANT_D50_SPD: [f64; 31] = [
    49.31, 56.51, 60.03, 57.82, 74.82, 87.25, 90.61, 91.37, 95.11, 91.96, 95.72, 96.17, 97.03,
    102.10, 100.75, 102.32, 100.00, 97.74, 98.92, 93.50, 97.69, 99.27, 99.04, 95.72, 98.86, 95.67,
    98.19, 103.00, 99.13, 87.38, 91.60,
];

/// CIE Standard Illuminant A (incandescent) relative SPD
pub const ILLUMINANT_A_SPD: [f64; 31] = [
    14.71, 17.68, 20.99, 24.67, 28.70, 33.09, 37.81, 42.87, 48.25, 53.91, 59.86, 66.06, 72.50,
    79.13, 85.95, 92.91, 100.00, 107.18, 114.44, 121.73, 129.04, 136.35, 143.62, 150.84, 157.98,
    165.03, 171.96, 178.77, 185.43, 191.93, 198.26,
];

// ============================================================================
// CONFIGURATION
// ============================================================================

/// Spectral rendering configuration
#[derive(Debug, Clone)]
pub struct SpectralRenderConfig {
    /// Number of wavelength samples
    pub wavelength_count: usize,
    /// Minimum wavelength (nm)
    pub wavelength_min: f64,
    /// Maximum wavelength (nm)
    pub wavelength_max: f64,
    /// Illuminant for color rendering
    pub illuminant: Illuminant,
    /// Whether to apply chromatic adaptation
    pub chromatic_adaptation: bool,
}

impl SpectralRenderConfig {
    /// Create default configuration (D65, full visible spectrum)
    pub fn new() -> Self {
        Self {
            wavelength_count: WAVELENGTH_COUNT,
            wavelength_min: WAVELENGTH_MIN,
            wavelength_max: WAVELENGTH_MAX,
            illuminant: Illuminant::D65,
            chromatic_adaptation: true,
        }
    }

    /// Configure for D50 illuminant (print/prepress)
    pub fn d50() -> Self {
        Self {
            illuminant: Illuminant::D50,
            ..Self::new()
        }
    }

    /// Configure for incandescent illuminant
    pub fn incandescent() -> Self {
        Self {
            illuminant: Illuminant::A,
            ..Self::new()
        }
    }

    /// Get wavelength at index
    pub fn wavelength_at(&self, index: usize) -> f64 {
        self.wavelength_min + (index as f64) * WAVELENGTH_STEP
    }

    /// Get all wavelengths
    pub fn wavelengths(&self) -> [f64; WAVELENGTH_COUNT] {
        let mut wl = [0.0; WAVELENGTH_COUNT];
        for i in 0..WAVELENGTH_COUNT {
            wl[i] = self.wavelength_at(i);
        }
        wl
    }
}

impl Default for SpectralRenderConfig {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// SPECTRAL RADIANCE
// ============================================================================

/// Spectral radiance result from material evaluation
#[derive(Debug, Clone)]
pub struct SpectralRadiance {
    /// Wavelength samples (nm)
    pub wavelengths: [f64; WAVELENGTH_COUNT],
    /// Spectral values (reflectance or radiance)
    pub values: [f64; WAVELENGTH_COUNT],
    /// Integrated XYZ color
    pub xyz: XyzColor,
    /// Integrated sRGB color
    pub rgb: [f64; 3],
    /// Correlated Color Temperature (Kelvin)
    pub cct: f64,
    /// Luminance (Y value)
    pub luminance: f64,
}

impl SpectralRadiance {
    /// Create from spectral values
    pub fn from_spectrum(values: [f64; WAVELENGTH_COUNT], config: &SpectralRenderConfig) -> Self {
        let wavelengths = config.wavelengths();
        let xyz = spectral_to_xyz(&values, config);
        let rgb = xyz_to_rgb(xyz);
        let cct = compute_cct(&xyz);
        let luminance = xyz.y;

        Self {
            wavelengths,
            values,
            xyz,
            rgb,
            cct,
            luminance,
        }
    }

    /// Get dominant wavelength
    pub fn dominant_wavelength(&self) -> f64 {
        let mut max_idx = 0;
        let mut max_val = 0.0;

        for (i, &v) in self.values.iter().enumerate() {
            if v > max_val {
                max_val = v;
                max_idx = i;
            }
        }

        self.wavelengths[max_idx]
    }

    /// Get spectral centroid
    pub fn spectral_centroid(&self) -> f64 {
        let mut weighted_sum = 0.0;
        let mut total = 0.0;

        for (i, &v) in self.values.iter().enumerate() {
            weighted_sum += self.wavelengths[i] * v;
            total += v;
        }

        if total > 0.0 {
            weighted_sum / total
        } else {
            550.0 // Default to green
        }
    }

    /// Get spectral bandwidth (FWHM approximation)
    pub fn bandwidth(&self) -> f64 {
        let max_val = self.values.iter().cloned().fold(0.0, f64::max);
        let half_max = max_val * 0.5;

        let mut first_idx = 0;
        let mut last_idx = WAVELENGTH_COUNT - 1;

        for (i, &v) in self.values.iter().enumerate() {
            if v >= half_max {
                first_idx = i;
                break;
            }
        }

        for (i, &v) in self.values.iter().enumerate().rev() {
            if v >= half_max {
                last_idx = i;
                break;
            }
        }

        self.wavelengths[last_idx] - self.wavelengths[first_idx]
    }
}

// ============================================================================
// COLOR MATCHING FUNCTIONS
// ============================================================================

/// CIE Color Matching Function lookup table
#[derive(Debug, Clone)]
pub struct ColorMatchingLUT {
    /// x̄(λ) values
    pub x_bar: [f64; WAVELENGTH_COUNT],
    /// ȳ(λ) values
    pub y_bar: [f64; WAVELENGTH_COUNT],
    /// z̄(λ) values
    pub z_bar: [f64; WAVELENGTH_COUNT],
}

impl ColorMatchingLUT {
    /// Create CIE 1931 2-degree standard observer
    pub fn cie1931() -> Self {
        Self {
            x_bar: CIE_X_BAR,
            y_bar: CIE_Y_BAR,
            z_bar: CIE_Z_BAR,
        }
    }

    /// Get CMF values at wavelength index
    pub fn at(&self, index: usize) -> (f64, f64, f64) {
        (self.x_bar[index], self.y_bar[index], self.z_bar[index])
    }

    /// Interpolate CMF at arbitrary wavelength
    pub fn interpolate(&self, wavelength: f64) -> (f64, f64, f64) {
        let idx = ((wavelength - WAVELENGTH_MIN) / WAVELENGTH_STEP).floor() as usize;

        if idx >= WAVELENGTH_COUNT - 1 {
            return self.at(WAVELENGTH_COUNT - 1);
        }

        let t = (wavelength - WAVELENGTH_MIN - idx as f64 * WAVELENGTH_STEP) / WAVELENGTH_STEP;

        let x = self.x_bar[idx] * (1.0 - t) + self.x_bar[idx + 1] * t;
        let y = self.y_bar[idx] * (1.0 - t) + self.y_bar[idx + 1] * t;
        let z = self.z_bar[idx] * (1.0 - t) + self.z_bar[idx + 1] * t;

        (x, y, z)
    }
}

impl Default for ColorMatchingLUT {
    fn default() -> Self {
        Self::cie1931()
    }
}

// ============================================================================
// SPECTRAL MATERIAL EVALUATOR
// ============================================================================

/// Spectral material evaluator
#[derive(Debug, Clone)]
pub struct SpectralMaterialEvaluator {
    config: SpectralRenderConfig,
    cmf: ColorMatchingLUT,
}

impl SpectralMaterialEvaluator {
    /// Create new evaluator
    pub fn new(config: SpectralRenderConfig) -> Self {
        Self {
            config,
            cmf: ColorMatchingLUT::cie1931(),
        }
    }

    /// Evaluate material at full spectrum
    pub fn evaluate(&self, material: &CombinedMaterial, cos_theta: f64) -> SpectralRadiance {
        let mut values = [0.0; WAVELENGTH_COUNT];

        for i in 0..WAVELENGTH_COUNT {
            let wavelength = self.config.wavelength_at(i);
            values[i] = material.evaluate(wavelength, cos_theta);
        }

        SpectralRadiance::from_spectrum(values, &self.config)
    }

    /// Evaluate with illuminant weighting
    pub fn evaluate_illuminated(
        &self,
        material: &CombinedMaterial,
        cos_theta: f64,
    ) -> SpectralRadiance {
        let illuminant_spd = get_illuminant_spd(self.config.illuminant);
        let mut values = [0.0; WAVELENGTH_COUNT];

        for i in 0..WAVELENGTH_COUNT {
            let wavelength = self.config.wavelength_at(i);
            let reflectance = material.evaluate(wavelength, cos_theta);
            values[i] = reflectance * illuminant_spd[i] / 100.0; // Normalize
        }

        SpectralRadiance::from_spectrum(values, &self.config)
    }

    /// Get configuration
    pub fn config(&self) -> &SpectralRenderConfig {
        &self.config
    }
}

impl Default for SpectralMaterialEvaluator {
    fn default() -> Self {
        Self::new(SpectralRenderConfig::default())
    }
}

// ============================================================================
// SPECTRAL TO COLOR CONVERSION
// ============================================================================

/// Convert spectral values to CIE XYZ
pub fn spectral_to_xyz(
    spectrum: &[f64; WAVELENGTH_COUNT],
    config: &SpectralRenderConfig,
) -> XyzColor {
    let illuminant_spd = get_illuminant_spd(config.illuminant);
    let cmf = ColorMatchingLUT::cie1931();

    let mut x = 0.0;
    let mut y = 0.0;
    let mut z = 0.0;
    let mut n = 0.0; // Normalization factor

    for i in 0..WAVELENGTH_COUNT {
        let s = spectrum[i] * illuminant_spd[i];
        x += s * cmf.x_bar[i];
        y += s * cmf.y_bar[i];
        z += s * cmf.z_bar[i];
        n += illuminant_spd[i] * cmf.y_bar[i];
    }

    // Normalize
    if n > 0.0 {
        x = x * 100.0 / n;
        y = y * 100.0 / n;
        z = z * 100.0 / n;
    }

    XyzColor { x, y, z }
}

/// Convert spectral values to sRGB
pub fn spectral_to_srgb(
    spectrum: &[f64; WAVELENGTH_COUNT],
    config: &SpectralRenderConfig,
) -> [f64; 3] {
    let xyz = spectral_to_xyz(spectrum, config);
    xyz_to_rgb(xyz)
}

/// Get illuminant SPD array
fn get_illuminant_spd(illuminant: Illuminant) -> [f64; WAVELENGTH_COUNT] {
    match illuminant {
        Illuminant::D65 => ILLUMINANT_D65_SPD,
        Illuminant::D50 => ILLUMINANT_D50_SPD,
        Illuminant::A => ILLUMINANT_A_SPD,
        _ => ILLUMINANT_D65_SPD, // Default to D65 for custom illuminants
    }
}

/// Apply illuminant to spectrum (in-place)
pub fn apply_illuminant(spectrum: &mut [f64; WAVELENGTH_COUNT], illuminant: Illuminant) {
    let spd = get_illuminant_spd(illuminant);
    for i in 0..WAVELENGTH_COUNT {
        spectrum[i] *= spd[i] / 100.0;
    }
}

// ============================================================================
// CORRELATED COLOR TEMPERATURE
// ============================================================================

/// Compute Correlated Color Temperature from XYZ
pub fn compute_cct(xyz: &XyzColor) -> f64 {
    // McCamy's approximation
    let sum = xyz.x + xyz.y + xyz.z;
    if sum < 1e-6 {
        return 6500.0; // Default to D65
    }

    let x = xyz.x / sum;
    let y = xyz.y / sum;

    // Approximation constants
    let n = (x - 0.3320) / (0.1858 - y);
    let cct = 449.0 * n.powi(3) + 3525.0 * n.powi(2) + 6823.3 * n + 5520.33;

    cct.clamp(1000.0, 25000.0)
}

/// Get chromaticity coordinates from XYZ
pub fn xyz_to_chromaticity(xyz: &XyzColor) -> (f64, f64) {
    let sum = xyz.x + xyz.y + xyz.z;
    if sum < 1e-6 {
        return (0.3127, 0.3290); // D65 white point
    }

    (xyz.x / sum, xyz.y / sum)
}

// ============================================================================
// SPECTRAL COMPARISON
// ============================================================================

/// Compare two spectra using RMSE
pub fn spectral_rmse(a: &[f64; WAVELENGTH_COUNT], b: &[f64; WAVELENGTH_COUNT]) -> f64 {
    let mut sum_sq = 0.0;
    for i in 0..WAVELENGTH_COUNT {
        let diff = a[i] - b[i];
        sum_sq += diff * diff;
    }
    (sum_sq / WAVELENGTH_COUNT as f64).sqrt()
}

/// Compare two spectra using Spectral Angle Mapper (SAM)
pub fn spectral_angle(a: &[f64; WAVELENGTH_COUNT], b: &[f64; WAVELENGTH_COUNT]) -> f64 {
    let mut dot = 0.0;
    let mut mag_a = 0.0;
    let mut mag_b = 0.0;

    for i in 0..WAVELENGTH_COUNT {
        dot += a[i] * b[i];
        mag_a += a[i] * a[i];
        mag_b += b[i] * b[i];
    }

    mag_a = mag_a.sqrt();
    mag_b = mag_b.sqrt();

    if mag_a < 1e-10 || mag_b < 1e-10 {
        return PI / 2.0; // 90 degrees for zero vectors
    }

    (dot / (mag_a * mag_b)).clamp(-1.0, 1.0).acos()
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::super::combined_effects::presets as combined_presets;
    use super::*;

    #[test]
    fn test_wavelength_config() {
        let config = SpectralRenderConfig::new();
        assert_eq!(config.wavelength_at(0), 400.0);
        assert_eq!(config.wavelength_at(15), 550.0);
        assert_eq!(config.wavelength_at(30), 700.0);
    }

    #[test]
    fn test_cmf_sum() {
        let cmf = ColorMatchingLUT::cie1931();

        // Y_bar should sum to roughly luminous efficacy constant
        let y_sum: f64 = cmf.y_bar.iter().sum();
        assert!(y_sum > 10.0 && y_sum < 20.0);
    }

    #[test]
    fn test_spectral_to_xyz_white() {
        let config = SpectralRenderConfig::new();
        let white_spectrum = [1.0; WAVELENGTH_COUNT]; // Perfect reflector

        let xyz = spectral_to_xyz(&white_spectrum, &config);

        // Under D65, perfect reflector should give roughly equal X,Y,Z
        assert!(xyz.x > 90.0 && xyz.x < 110.0);
        assert!(xyz.y > 90.0 && xyz.y < 110.0);
        assert!(xyz.z > 90.0 && xyz.z < 110.0);
    }

    #[test]
    fn test_spectral_to_srgb_valid() {
        let config = SpectralRenderConfig::new();
        let spectrum = [0.5; WAVELENGTH_COUNT];

        let rgb = spectral_to_srgb(&spectrum, &config);

        // Should be valid gray
        for c in &rgb {
            assert!(*c >= 0.0 && *c <= 1.0);
        }
    }

    #[test]
    fn test_cct_d65() {
        let d65_xyz = XyzColor {
            x: 95.047,
            y: 100.0,
            z: 108.883,
        };
        let cct = compute_cct(&d65_xyz);

        // D65 is ~6500K
        assert!(cct > 6000.0 && cct < 7000.0);
    }

    #[test]
    fn test_spectral_evaluator() {
        let config = SpectralRenderConfig::new();
        let evaluator = SpectralMaterialEvaluator::new(config);
        let glass = combined_presets::glass();

        let result = evaluator.evaluate(&glass, 1.0);

        // Should have 31 wavelengths
        assert_eq!(result.wavelengths.len(), 31);
        assert_eq!(result.values.len(), 31);

        // Values should be in reasonable range
        for v in &result.values {
            assert!(*v >= 0.0 && *v <= 1.0);
        }

        // RGB should be valid
        for c in &result.rgb {
            assert!(*c >= 0.0 && *c <= 1.0);
        }
    }

    #[test]
    fn test_spectral_radiance_metrics() {
        let mut values = [0.0; WAVELENGTH_COUNT];
        // Peak at 550nm (green)
        for i in 0..WAVELENGTH_COUNT {
            let wavelength = 400.0 + i as f64 * 10.0;
            values[i] = (-(wavelength - 550.0).powi(2) / 2000.0).exp();
        }

        let config = SpectralRenderConfig::new();
        let radiance = SpectralRadiance::from_spectrum(values, &config);

        // Dominant should be around 550nm
        let dominant = radiance.dominant_wavelength();
        assert!(dominant > 540.0 && dominant < 560.0);

        // Centroid should be around 550nm
        let centroid = radiance.spectral_centroid();
        assert!(centroid > 540.0 && centroid < 560.0);
    }

    #[test]
    fn test_spectral_comparison() {
        let a = [0.5; WAVELENGTH_COUNT];
        let b = [0.5; WAVELENGTH_COUNT];
        let c = [0.6; WAVELENGTH_COUNT];

        // Same spectra should have 0 RMSE
        assert!(spectral_rmse(&a, &b) < 1e-10);

        // Different spectra should have non-zero RMSE
        let rmse = spectral_rmse(&a, &c);
        assert!((rmse - 0.1).abs() < 0.01);

        // Same direction should have 0 angle
        assert!(spectral_angle(&a, &b) < 1e-10);
    }

    #[test]
    fn test_illuminant_spectra() {
        // D65 should be relatively flat around 100
        let d65_mean: f64 = ILLUMINANT_D65_SPD.iter().sum::<f64>() / 31.0;
        assert!(d65_mean > 80.0 && d65_mean < 120.0);

        // A should increase with wavelength (warm)
        assert!(ILLUMINANT_A_SPD[30] > ILLUMINANT_A_SPD[0]);
    }

    #[test]
    fn test_chromaticity() {
        let d65_xyz = XyzColor {
            x: 95.047,
            y: 100.0,
            z: 108.883,
        };
        let (x, y) = xyz_to_chromaticity(&d65_xyz);

        // D65 white point
        assert!((x - 0.3127).abs() < 0.01);
        assert!((y - 0.3290).abs() < 0.01);
    }
}
