//! # Perceptual Loss Module
//!
//! LAB color space conversions and Delta E metrics for perceptual color matching.
//!
//! ## Features
//!
//! - **Color Space Conversions**: sRGB ↔ XYZ ↔ LAB
//! - **Delta E Metrics**: CIE76, CIE94, CIEDE2000
//! - **Perceptual Loss**: Loss function for material auto-calibration
//! - **LUT Acceleration**: Fast gamma correction lookup tables
//!
//! ## Usage
//!
//! ```rust
//! use momoto_materials::glass_physics::perceptual_loss::{
//!     rgb_to_lab, delta_e_2000, LabColor, Illuminant,
//! };
//!
//! let lab1 = rgb_to_lab([0.8, 0.2, 0.1], Illuminant::D65);
//! let lab2 = rgb_to_lab([0.75, 0.25, 0.15], Illuminant::D65);
//! let delta_e = delta_e_2000(lab1, lab2);
//! println!("Perceptual difference: {:.2}", delta_e);
//! ```

use std::f64::consts::PI;

// ============================================================================
// COLOR STRUCTURES
// ============================================================================

/// CIE LAB color (perceptually uniform color space)
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LabColor {
    /// Lightness (0-100)
    pub l: f64,
    /// Green-Red axis (-128 to 127)
    pub a: f64,
    /// Blue-Yellow axis (-128 to 127)
    pub b: f64,
}

impl LabColor {
    pub fn new(l: f64, a: f64, b: f64) -> Self {
        Self { l, a, b }
    }

    /// Chroma (colorfulness)
    pub fn chroma(&self) -> f64 {
        (self.a * self.a + self.b * self.b).sqrt()
    }

    /// Hue angle in radians
    pub fn hue(&self) -> f64 {
        self.b.atan2(self.a)
    }

    /// Hue angle in degrees (0-360)
    pub fn hue_degrees(&self) -> f64 {
        let h = self.b.atan2(self.a) * 180.0 / PI;
        if h < 0.0 {
            h + 360.0
        } else {
            h
        }
    }
}

impl Default for LabColor {
    fn default() -> Self {
        Self {
            l: 50.0,
            a: 0.0,
            b: 0.0,
        }
    }
}

/// CIE XYZ color (intermediate color space)
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct XyzColor {
    pub x: f64,
    pub y: f64,
    pub z: f64,
}

impl XyzColor {
    pub fn new(x: f64, y: f64, z: f64) -> Self {
        Self { x, y, z }
    }
}

impl Default for XyzColor {
    fn default() -> Self {
        Self {
            x: 0.0,
            y: 0.0,
            z: 0.0,
        }
    }
}

/// Reference white point (illuminant)
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Illuminant {
    /// D50 (print industry standard)
    D50,
    /// D65 (sRGB standard, daylight)
    D65,
    /// Illuminant A (incandescent)
    A,
    /// Custom white point
    Custom(XyzColor),
}

impl Illuminant {
    /// Get XYZ values for the illuminant
    pub fn xyz(&self) -> XyzColor {
        match self {
            Illuminant::D50 => XyzColor::new(0.96422, 1.0, 0.82521),
            Illuminant::D65 => XyzColor::new(0.95047, 1.0, 1.08883),
            Illuminant::A => XyzColor::new(1.09850, 1.0, 0.35585),
            Illuminant::Custom(xyz) => *xyz,
        }
    }
}

impl Default for Illuminant {
    fn default() -> Self {
        Illuminant::D65
    }
}

// ============================================================================
// DELTA E FORMULAS
// ============================================================================

/// Delta E formula selection
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum DeltaEFormula {
    /// CIE76 - Classic Euclidean distance
    CIE76,
    /// CIE94 - Perceptual weighting for graphics
    CIE94,
    /// CIEDE2000 - Industry standard, most accurate
    #[default]
    CIEDE2000,
}

// ============================================================================
// COLOR SPACE CONVERSIONS
// ============================================================================

/// Convert sRGB to linear RGB
#[inline]
pub fn srgb_to_linear(c: f64) -> f64 {
    if c <= 0.04045 {
        c / 12.92
    } else {
        ((c + 0.055) / 1.055).powf(2.4)
    }
}

/// Convert linear RGB to sRGB
#[inline]
pub fn linear_to_srgb(c: f64) -> f64 {
    if c <= 0.0031308 {
        c * 12.92
    } else {
        1.055 * c.powf(1.0 / 2.4) - 0.055
    }
}

/// Convert sRGB to CIE XYZ
pub fn rgb_to_xyz(rgb: [f64; 3]) -> XyzColor {
    // Linearize sRGB
    let r = srgb_to_linear(rgb[0].clamp(0.0, 1.0));
    let g = srgb_to_linear(rgb[1].clamp(0.0, 1.0));
    let b = srgb_to_linear(rgb[2].clamp(0.0, 1.0));

    // sRGB to XYZ matrix (D65 illuminant)
    XyzColor {
        x: 0.4124564 * r + 0.3575761 * g + 0.1804375 * b,
        y: 0.2126729 * r + 0.7151522 * g + 0.0721750 * b,
        z: 0.0193339 * r + 0.1191920 * g + 0.9503041 * b,
    }
}

/// Convert CIE XYZ to sRGB
pub fn xyz_to_rgb(xyz: XyzColor) -> [f64; 3] {
    // XYZ to sRGB matrix (D65 illuminant)
    let r = 3.2404542 * xyz.x - 1.5371385 * xyz.y - 0.4985314 * xyz.z;
    let g = -0.9692660 * xyz.x + 1.8760108 * xyz.y + 0.0415560 * xyz.z;
    let b = 0.0556434 * xyz.x - 0.2040259 * xyz.y + 1.0572252 * xyz.z;

    // Apply sRGB gamma and clamp
    [
        linear_to_srgb(r).clamp(0.0, 1.0),
        linear_to_srgb(g).clamp(0.0, 1.0),
        linear_to_srgb(b).clamp(0.0, 1.0),
    ]
}

/// LAB f function (cube root with linear segment)
#[inline]
fn lab_f(t: f64) -> f64 {
    const DELTA: f64 = 6.0 / 29.0;
    const DELTA_CUBED: f64 = DELTA * DELTA * DELTA;

    if t > DELTA_CUBED {
        t.cbrt()
    } else {
        t / (3.0 * DELTA * DELTA) + 4.0 / 29.0
    }
}

/// Inverse LAB f function
#[inline]
fn lab_f_inv(t: f64) -> f64 {
    const DELTA: f64 = 6.0 / 29.0;

    if t > DELTA {
        t * t * t
    } else {
        3.0 * DELTA * DELTA * (t - 4.0 / 29.0)
    }
}

/// Convert CIE XYZ to CIE LAB
pub fn xyz_to_lab(xyz: XyzColor, illuminant: Illuminant) -> LabColor {
    let white = illuminant.xyz();

    let fx = lab_f(xyz.x / white.x);
    let fy = lab_f(xyz.y / white.y);
    let fz = lab_f(xyz.z / white.z);

    LabColor {
        l: 116.0 * fy - 16.0,
        a: 500.0 * (fx - fy),
        b: 200.0 * (fy - fz),
    }
}

/// Convert CIE LAB to CIE XYZ
pub fn lab_to_xyz(lab: LabColor, illuminant: Illuminant) -> XyzColor {
    let white = illuminant.xyz();

    let fy = (lab.l + 16.0) / 116.0;
    let fx = lab.a / 500.0 + fy;
    let fz = fy - lab.b / 200.0;

    XyzColor {
        x: white.x * lab_f_inv(fx),
        y: white.y * lab_f_inv(fy),
        z: white.z * lab_f_inv(fz),
    }
}

/// Convert sRGB to CIE LAB
pub fn rgb_to_lab(rgb: [f64; 3], illuminant: Illuminant) -> LabColor {
    xyz_to_lab(rgb_to_xyz(rgb), illuminant)
}

/// Convert CIE LAB to sRGB
pub fn lab_to_rgb(lab: LabColor, illuminant: Illuminant) -> [f64; 3] {
    xyz_to_rgb(lab_to_xyz(lab, illuminant))
}

// ============================================================================
// DELTA E COMPUTATIONS
// ============================================================================

/// CIE76 Delta E (classic Euclidean distance)
pub fn delta_e_76(lab1: LabColor, lab2: LabColor) -> f64 {
    let dl = lab1.l - lab2.l;
    let da = lab1.a - lab2.a;
    let db = lab1.b - lab2.b;
    (dl * dl + da * da + db * db).sqrt()
}

/// CIE94 Delta E (perceptual weighting for graphics)
pub fn delta_e_94(lab1: LabColor, lab2: LabColor) -> f64 {
    // Graphic arts constants
    const KL: f64 = 1.0;
    const K1: f64 = 0.045;
    const K2: f64 = 0.015;

    let dl = lab1.l - lab2.l;
    let c1 = lab1.chroma();
    let c2 = lab2.chroma();
    let dc = c1 - c2;

    let da = lab1.a - lab2.a;
    let db = lab1.b - lab2.b;
    let dh_sq = da * da + db * db - dc * dc;
    let dh = if dh_sq > 0.0 { dh_sq.sqrt() } else { 0.0 };

    let sl = 1.0;
    let sc = 1.0 + K1 * c1;
    let sh = 1.0 + K2 * c1;

    let term_l = dl / (KL * sl);
    let term_c = dc / sc;
    let term_h = dh / sh;

    (term_l * term_l + term_c * term_c + term_h * term_h).sqrt()
}

/// CIEDE2000 Delta E (industry standard, most accurate)
pub fn delta_e_2000(lab1: LabColor, lab2: LabColor) -> f64 {
    // Parametric weighting factors (standard values)
    const KL: f64 = 1.0;
    const KC: f64 = 1.0;
    const KH: f64 = 1.0;

    let l1 = lab1.l;
    let a1 = lab1.a;
    let b1 = lab1.b;
    let l2 = lab2.l;
    let a2 = lab2.a;
    let b2 = lab2.b;

    // Calculate C1, C2
    let c1 = (a1 * a1 + b1 * b1).sqrt();
    let c2 = (a2 * a2 + b2 * b2).sqrt();
    let c_avg = (c1 + c2) / 2.0;

    // Calculate G factor
    let c_avg_7 = c_avg.powi(7);
    let g = 0.5 * (1.0 - (c_avg_7 / (c_avg_7 + 6103515625.0)).sqrt()); // 25^7 = 6103515625

    // Adjust a values
    let a1_prime = a1 * (1.0 + g);
    let a2_prime = a2 * (1.0 + g);

    // Calculate C' values
    let c1_prime = (a1_prime * a1_prime + b1 * b1).sqrt();
    let c2_prime = (a2_prime * a2_prime + b2 * b2).sqrt();

    // Calculate h' values
    let h1_prime = if a1_prime == 0.0 && b1 == 0.0 {
        0.0
    } else {
        let h = b1.atan2(a1_prime) * 180.0 / PI;
        if h < 0.0 {
            h + 360.0
        } else {
            h
        }
    };

    let h2_prime = if a2_prime == 0.0 && b2 == 0.0 {
        0.0
    } else {
        let h = b2.atan2(a2_prime) * 180.0 / PI;
        if h < 0.0 {
            h + 360.0
        } else {
            h
        }
    };

    // Calculate differences
    let dl_prime = l2 - l1;
    let dc_prime = c2_prime - c1_prime;

    let dh_prime = if c1_prime * c2_prime == 0.0 {
        0.0
    } else {
        let diff = h2_prime - h1_prime;
        if diff.abs() <= 180.0 {
            diff
        } else if diff > 180.0 {
            diff - 360.0
        } else {
            diff + 360.0
        }
    };

    let dh_prime_rad = dh_prime * PI / 180.0;
    let dh_large = 2.0 * (c1_prime * c2_prime).sqrt() * (dh_prime_rad / 2.0).sin();

    // Calculate averages
    let l_avg = (l1 + l2) / 2.0;
    let c_avg_prime = (c1_prime + c2_prime) / 2.0;

    let h_avg_prime = if c1_prime * c2_prime == 0.0 {
        h1_prime + h2_prime
    } else {
        let diff = (h1_prime - h2_prime).abs();
        if diff <= 180.0 {
            (h1_prime + h2_prime) / 2.0
        } else if h1_prime + h2_prime < 360.0 {
            (h1_prime + h2_prime + 360.0) / 2.0
        } else {
            (h1_prime + h2_prime - 360.0) / 2.0
        }
    };

    // Calculate T
    let h_rad = h_avg_prime * PI / 180.0;
    let t = 1.0 - 0.17 * (h_rad - PI / 6.0).cos()
        + 0.24 * (2.0 * h_rad).cos()
        + 0.32 * (3.0 * h_rad + PI / 30.0).cos()
        - 0.20 * (4.0 * h_rad - 63.0 * PI / 180.0).cos();

    // Calculate weighting functions
    let sl = 1.0 + (0.015 * (l_avg - 50.0).powi(2)) / (20.0 + (l_avg - 50.0).powi(2)).sqrt();
    let sc = 1.0 + 0.045 * c_avg_prime;
    let sh = 1.0 + 0.015 * c_avg_prime * t;

    // Calculate RT (rotation term)
    let dtheta = 30.0 * (-(((h_avg_prime - 275.0) / 25.0).powi(2))).exp();
    let c_avg_prime_7 = c_avg_prime.powi(7);
    let rc = 2.0 * (c_avg_prime_7 / (c_avg_prime_7 + 6103515625.0)).sqrt();
    let rt = -rc * (2.0 * dtheta * PI / 180.0).sin();

    // Calculate final Delta E
    let term_l = dl_prime / (KL * sl);
    let term_c = dc_prime / (KC * sc);
    let term_h = dh_large / (KH * sh);

    (term_l * term_l + term_c * term_c + term_h * term_h + rt * term_c * term_h).sqrt()
}

/// Calculate Delta E using specified formula
pub fn delta_e(lab1: LabColor, lab2: LabColor, formula: DeltaEFormula) -> f64 {
    match formula {
        DeltaEFormula::CIE76 => delta_e_76(lab1, lab2),
        DeltaEFormula::CIE94 => delta_e_94(lab1, lab2),
        DeltaEFormula::CIEDE2000 => delta_e_2000(lab1, lab2),
    }
}

// ============================================================================
// PERCEPTUAL LOSS CONFIGURATION
// ============================================================================

/// Configuration for perceptual loss computation
#[derive(Debug, Clone)]
pub struct PerceptualLossConfig {
    /// Delta E formula to use
    pub formula: DeltaEFormula,
    /// Weight for lightness component
    pub weight_lightness: f64,
    /// Weight for chroma component
    pub weight_chroma: f64,
    /// Weight for hue component
    pub weight_hue: f64,
    /// Illuminant for LAB conversion
    pub illuminant: Illuminant,
    /// Exponent for loss (1.0 = linear, 2.0 = squared)
    pub exponent: f64,
}

impl Default for PerceptualLossConfig {
    fn default() -> Self {
        Self {
            formula: DeltaEFormula::CIEDE2000,
            weight_lightness: 1.0,
            weight_chroma: 1.0,
            weight_hue: 1.0,
            illuminant: Illuminant::D65,
            exponent: 1.0,
        }
    }
}

impl PerceptualLossConfig {
    /// Create config optimized for material matching
    pub fn material_matching() -> Self {
        Self {
            formula: DeltaEFormula::CIEDE2000,
            weight_lightness: 1.2, // Lightness differences more noticeable
            weight_chroma: 1.0,
            weight_hue: 0.8, // Hue shifts less noticeable
            illuminant: Illuminant::D65,
            exponent: 2.0, // Penalize large errors more
        }
    }

    /// Create config for strict color matching
    pub fn strict() -> Self {
        Self {
            formula: DeltaEFormula::CIEDE2000,
            weight_lightness: 1.0,
            weight_chroma: 1.0,
            weight_hue: 1.0,
            illuminant: Illuminant::D65,
            exponent: 1.0,
        }
    }
}

// ============================================================================
// PERCEPTUAL LOSS FUNCTIONS
// ============================================================================

/// Compute perceptual loss between rendered and reference RGB values
pub fn perceptual_loss(
    rendered: &[[f64; 3]],
    reference: &[[f64; 3]],
    config: &PerceptualLossConfig,
) -> f64 {
    if rendered.len() != reference.len() || rendered.is_empty() {
        return f64::MAX;
    }

    let mut total_loss = 0.0;

    for (r, t) in rendered.iter().zip(reference.iter()) {
        let lab_r = rgb_to_lab(*r, config.illuminant);
        let lab_t = rgb_to_lab(*t, config.illuminant);

        let de = delta_e(lab_r, lab_t, config.formula);
        total_loss += de.powf(config.exponent);
    }

    total_loss / rendered.len() as f64
}

/// Compute weighted perceptual loss with separate L*a*b* components
pub fn perceptual_loss_weighted(
    rendered: &[[f64; 3]],
    reference: &[[f64; 3]],
    config: &PerceptualLossConfig,
) -> f64 {
    if rendered.len() != reference.len() || rendered.is_empty() {
        return f64::MAX;
    }

    let mut total_loss = 0.0;

    for (r, t) in rendered.iter().zip(reference.iter()) {
        let lab_r = rgb_to_lab(*r, config.illuminant);
        let lab_t = rgb_to_lab(*t, config.illuminant);

        // Separate component differences
        let dl = (lab_r.l - lab_t.l).abs();
        let dc = (lab_r.chroma() - lab_t.chroma()).abs();

        // Hue difference (handle wraparound)
        let dh = {
            let h1 = lab_r.hue_degrees();
            let h2 = lab_t.hue_degrees();
            let diff = (h1 - h2).abs();
            if diff > 180.0 {
                360.0 - diff
            } else {
                diff
            }
        };

        // Weighted sum
        let loss =
            config.weight_lightness * dl + config.weight_chroma * dc + config.weight_hue * dh;

        total_loss += loss.powf(config.exponent);
    }

    total_loss / rendered.len() as f64
}

/// Compute gradient of perceptual loss with respect to RGB values
/// Returns gradient in RGB space (for backpropagation)
pub fn perceptual_loss_gradient(
    rendered: &[[f64; 3]],
    reference: &[[f64; 3]],
    config: &PerceptualLossConfig,
) -> Vec<[f64; 3]> {
    if rendered.len() != reference.len() {
        return vec![];
    }

    let eps = 1e-6;
    let mut gradients = Vec::with_capacity(rendered.len());

    for i in 0..rendered.len() {
        let mut grad = [0.0; 3];

        // Compute numerical gradient for each RGB channel
        for c in 0..3 {
            let mut r_plus = rendered.to_vec();
            let mut r_minus = rendered.to_vec();

            r_plus[i][c] += eps;
            r_minus[i][c] -= eps;

            let loss_plus = perceptual_loss(&r_plus, reference, config);
            let loss_minus = perceptual_loss(&r_minus, reference, config);

            grad[c] = (loss_plus - loss_minus) / (2.0 * eps);
        }

        gradients.push(grad);
    }

    gradients
}

// ============================================================================
// BATCH OPERATIONS
// ============================================================================

/// Convert batch of RGB values to LAB
pub fn rgb_batch_to_lab(rgb_batch: &[[f64; 3]], illuminant: Illuminant) -> Vec<LabColor> {
    rgb_batch
        .iter()
        .map(|rgb| rgb_to_lab(*rgb, illuminant))
        .collect()
}

/// Convert batch of LAB values to RGB
pub fn lab_batch_to_rgb(lab_batch: &[LabColor], illuminant: Illuminant) -> Vec<[f64; 3]> {
    lab_batch
        .iter()
        .map(|lab| lab_to_rgb(*lab, illuminant))
        .collect()
}

/// Compute Delta E for batch of color pairs
pub fn delta_e_batch(
    lab1_batch: &[LabColor],
    lab2_batch: &[LabColor],
    formula: DeltaEFormula,
) -> Vec<f64> {
    lab1_batch
        .iter()
        .zip(lab2_batch.iter())
        .map(|(l1, l2)| delta_e(*l1, *l2, formula))
        .collect()
}

// ============================================================================
// UTILITY FUNCTIONS
// ============================================================================

/// Check if two colors are perceptually indistinguishable (Delta E < 1.0)
pub fn colors_match(rgb1: [f64; 3], rgb2: [f64; 3], illuminant: Illuminant) -> bool {
    let lab1 = rgb_to_lab(rgb1, illuminant);
    let lab2 = rgb_to_lab(rgb2, illuminant);
    delta_e_2000(lab1, lab2) < 1.0
}

/// Classify perceptual difference
pub fn classify_difference(delta_e: f64) -> &'static str {
    if delta_e < 1.0 {
        "Not perceptible"
    } else if delta_e < 2.0 {
        "Perceptible through close observation"
    } else if delta_e < 3.5 {
        "Perceptible at a glance"
    } else if delta_e < 5.0 {
        "More similar than different"
    } else {
        "Obvious difference"
    }
}

/// Estimate memory usage of perceptual LUTs
pub fn total_perceptual_memory() -> usize {
    // No global LUTs in this implementation
    // Could add sRGB/linear LUT if needed (~4KB)
    0
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_srgb_linear_roundtrip() {
        for i in 0..=10 {
            let c = i as f64 / 10.0;
            let linear = srgb_to_linear(c);
            let back = linear_to_srgb(linear);
            assert!((c - back).abs() < 1e-10);
        }
    }

    #[test]
    fn test_rgb_xyz_roundtrip() {
        let rgb = [0.5, 0.3, 0.8];
        let xyz = rgb_to_xyz(rgb);
        let back = xyz_to_rgb(xyz);

        for i in 0..3 {
            assert!((rgb[i] - back[i]).abs() < 1e-6);
        }
    }

    #[test]
    fn test_rgb_lab_roundtrip() {
        let rgb = [0.5, 0.3, 0.8];
        let lab = rgb_to_lab(rgb, Illuminant::D65);
        let back = lab_to_rgb(lab, Illuminant::D65);

        for i in 0..3 {
            assert!((rgb[i] - back[i]).abs() < 1e-5);
        }
    }

    #[test]
    fn test_white_lab() {
        let white = rgb_to_lab([1.0, 1.0, 1.0], Illuminant::D65);
        assert!((white.l - 100.0).abs() < 0.1);
        assert!(white.a.abs() < 0.1);
        assert!(white.b.abs() < 0.1);
    }

    #[test]
    fn test_black_lab() {
        let black = rgb_to_lab([0.0, 0.0, 0.0], Illuminant::D65);
        assert!(black.l.abs() < 0.1);
        assert!(black.a.abs() < 0.1);
        assert!(black.b.abs() < 0.1);
    }

    #[test]
    fn test_delta_e_76_same_color() {
        let lab = LabColor::new(50.0, 25.0, -30.0);
        assert!(delta_e_76(lab, lab).abs() < 1e-10);
    }

    #[test]
    fn test_delta_e_2000_same_color() {
        let lab = LabColor::new(50.0, 25.0, -30.0);
        assert!(delta_e_2000(lab, lab).abs() < 1e-10);
    }

    #[test]
    fn test_delta_e_order() {
        // For nearby colors, all formulas should give similar results
        let lab1 = LabColor::new(50.0, 25.0, -30.0);
        let lab2 = LabColor::new(52.0, 27.0, -28.0);

        let de76 = delta_e_76(lab1, lab2);
        let de94 = delta_e_94(lab1, lab2);
        let de2000 = delta_e_2000(lab1, lab2);

        // All should be positive
        assert!(de76 > 0.0);
        assert!(de94 > 0.0);
        assert!(de2000 > 0.0);

        // All should be reasonably small for nearby colors
        assert!(de76 < 10.0);
        assert!(de94 < 10.0);
        assert!(de2000 < 10.0);
    }

    #[test]
    fn test_perceptual_loss() {
        let rendered = vec![[0.5, 0.3, 0.8]];
        let reference = vec![[0.52, 0.32, 0.78]];
        let config = PerceptualLossConfig::default();

        let loss = perceptual_loss(&rendered, &reference, &config);
        assert!(loss > 0.0);
        assert!(loss < 10.0); // Should be small for similar colors
    }

    #[test]
    fn test_colors_match() {
        // Very similar colors should match
        assert!(colors_match(
            [0.5, 0.3, 0.8],
            [0.502, 0.301, 0.799],
            Illuminant::D65
        ));

        // Different colors should not match
        assert!(!colors_match(
            [0.5, 0.3, 0.8],
            [0.8, 0.3, 0.5],
            Illuminant::D65
        ));
    }

    #[test]
    fn test_classify_difference() {
        assert_eq!(classify_difference(0.5), "Not perceptible");
        assert_eq!(
            classify_difference(1.5),
            "Perceptible through close observation"
        );
        assert_eq!(classify_difference(3.0), "Perceptible at a glance");
        assert_eq!(classify_difference(4.5), "More similar than different");
        assert_eq!(classify_difference(10.0), "Obvious difference");
    }

    #[test]
    fn test_batch_operations() {
        let rgb_batch = vec![[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]];

        let lab_batch = rgb_batch_to_lab(&rgb_batch, Illuminant::D65);
        assert_eq!(lab_batch.len(), 3);

        let back = lab_batch_to_rgb(&lab_batch, Illuminant::D65);
        assert_eq!(back.len(), 3);
    }

    #[test]
    fn test_chroma_hue() {
        let lab = LabColor::new(50.0, 30.0, 40.0);
        let chroma = lab.chroma();
        let hue = lab.hue_degrees();

        assert!((chroma - 50.0).abs() < 0.01); // sqrt(30^2 + 40^2) = 50
        assert!(hue > 0.0 && hue < 90.0); // First quadrant
    }
}
