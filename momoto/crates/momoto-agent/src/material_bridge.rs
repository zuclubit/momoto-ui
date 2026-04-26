// =============================================================================
// momoto-agent: Material → Color Bridge
// File: crates/momoto-agent/src/material_bridge.rs
//
// Converts BSDF spectral evaluation to a dominant OKLCH color via CIE CMFs.
// Implements the full spectral pipeline: material parameters → spectrum → XYZ → OKLCH.
//
// Architecture note: This module lives in momoto-agent because it depends on
// both momoto-materials and momoto-intelligence (harmony). Direct dependency of
// momoto-intelligence on momoto-materials would be circular.
// =============================================================================

use momoto_core::color::Color;
use momoto_core::space::oklch::OKLCH;

// =============================================================================
// Local Fresnel computation (avoids momoto-materials dependency in agent crate)
// Exact Fresnel equations for a dielectric interface (air → material).
// Reference: Born & Wolf, "Principles of Optics", §1.5.2
// =============================================================================

/// Local BSDF evaluation result (reflectance + transmittance for a dielectric).
struct DielectricResponse {
    reflectance: f64,
    transmittance: f64,
}

/// Exact Fresnel reflectance for an unpolarized plane wave at an air–dielectric interface.
///
/// Computes both Rs (s-polarized) and Rp (p-polarized) components then averages them.
/// Handles total internal reflection (TIR) when `ior < 1` and `cos_theta` is small.
///
/// # Arguments
/// * `ior` — real part of the index of refraction of the medium (n1 = 1 for air)
/// * `roughness` — surface roughness ∈ [0, 1]; reduces effective specular peak
///   via an empirical Beckmann width factor: `R_eff = R * exp(-4·k²)` where k = roughness.
/// * `cos_theta` — cosine of the angle of incidence (1 = normal, 0 = grazing)
///
/// # Returns
/// `DielectricResponse { reflectance, transmittance }` with R + T ≤ 1 (equality for lossless).
fn evaluate_dielectric(ior: f64, roughness: f64, cos_theta: f64) -> DielectricResponse {
    let cos_i = cos_theta.clamp(0.0, 1.0);
    let sin_i2 = (1.0 - cos_i * cos_i).max(0.0);
    let sin_t2 = sin_i2 / (ior * ior);

    if sin_t2 >= 1.0 {
        // Total internal reflection
        return DielectricResponse {
            reflectance: 1.0,
            transmittance: 0.0,
        };
    }

    let cos_t = (1.0 - sin_t2).sqrt();

    // s-polarization (TE): Rs = ((n1·cos_i − n2·cos_t) / (n1·cos_i + n2·cos_t))²
    let rs_num = cos_i - ior * cos_t;
    let rs_den = cos_i + ior * cos_t;
    let rs = if rs_den.abs() < 1e-15 {
        1.0
    } else {
        (rs_num / rs_den).powi(2)
    };

    // p-polarization (TM): Rp = ((n2·cos_i − n1·cos_t) / (n2·cos_i + n1·cos_t))²
    let rp_num = ior * cos_i - cos_t;
    let rp_den = ior * cos_i + cos_t;
    let rp = if rp_den.abs() < 1e-15 {
        1.0
    } else {
        (rp_num / rp_den).powi(2)
    };

    // Unpolarized reflectance (equal weighting)
    let r_spec = (rs + rp) * 0.5;

    // Roughness attenuation: Beckmann factor reduces coherent specular peak.
    // For rough surfaces energy redistributes into scattered lobes; total R+T preserved.
    let k = roughness.clamp(0.0, 1.0);
    let r = r_spec * (-4.0 * k * k).exp();

    DielectricResponse {
        reflectance: r.clamp(0.0, 1.0),
        transmittance: (1.0 - r).clamp(0.0, 1.0),
    }
}

// =============================================================================
// CIE 1931 2-degree Observer Color Matching Functions
// 10 nm intervals, 400–700 nm (31 bands)
// Source: CIE 1931 standard colorimetric observer (Stiles & Burch corrected)
// =============================================================================

const CIE_X_BAR: [f64; 31] = [
    0.01360, 0.04243, 0.13438, 0.28390, 0.34828, 0.33602, 0.29080, 0.19536, 0.09564, 0.03201,
    0.00490, 0.00930, 0.06327, 0.16768, 0.29012, 0.43382, 0.59450, 0.76210, 0.91620, 1.02630,
    1.06220, 1.00260, 0.85445, 0.64236, 0.44790, 0.28350, 0.16490, 0.08740, 0.04677, 0.02270,
    0.01136,
];

const CIE_Y_BAR: [f64; 31] = [
    0.00039, 0.00120, 0.00400, 0.01160, 0.02300, 0.03800, 0.06000, 0.09098, 0.13902, 0.20802,
    0.32300, 0.50300, 0.71000, 0.86200, 0.95400, 0.99495, 0.99500, 0.95200, 0.87000, 0.75700,
    0.63100, 0.50300, 0.38100, 0.26500, 0.17500, 0.10700, 0.06100, 0.03200, 0.01700, 0.00821,
    0.00410,
];

const CIE_Z_BAR: [f64; 31] = [
    0.06450, 0.20510, 0.67850, 1.38560, 1.74706, 1.77211, 1.66920, 1.28764, 0.81295, 0.46518,
    0.27200, 0.15820, 0.07825, 0.04216, 0.02030, 0.00875, 0.00390, 0.00210, 0.00165, 0.00110,
    0.00080, 0.00034, 0.00019, 0.00005, 0.00002, 0.00000, 0.00000, 0.00000, 0.00000, 0.00000,
    0.00000,
];

/// D65 illuminant spectral power distribution at 10 nm intervals, 400–700 nm.
/// Normalized to 100 at 560 nm. (CIE standard illuminant D65)
const ILLUMINANT_D65: [f64; 31] = [
    82.754, 91.486, 93.431, 86.682, 104.865, 117.008, 117.812, 114.861, 115.923, 108.811, 109.354,
    107.802, 104.790, 107.689, 104.405, 104.046, 100.000, 96.334, 95.788, 88.686, 90.006, 89.599,
    87.699, 83.288, 83.699, 80.026, 80.214, 82.277, 78.284, 69.721, 71.609,
];

/// Wavelengths (nm) corresponding to the 31 spectral bands.
const WAVELENGTHS: [f64; 31] = [
    400.0, 410.0, 420.0, 430.0, 440.0, 450.0, 460.0, 470.0, 480.0, 490.0, 500.0, 510.0, 520.0,
    530.0, 540.0, 550.0, 560.0, 570.0, 580.0, 590.0, 600.0, 610.0, 620.0, 630.0, 640.0, 650.0,
    660.0, 670.0, 680.0, 690.0, 700.0,
];

// =============================================================================
// Result type
// =============================================================================

/// Result of converting a material evaluation to a dominant color.
#[derive(Debug, Clone)]
pub struct MaterialColorResult {
    /// Dominant color in OKLCH (gamut-safe).
    pub dominant: OKLCH,
    /// Spectrally-integrated reflectance (0–1), averaged over 31 bands.
    pub reflectance: f64,
    /// Spectrally-integrated transmittance (0–1), averaged over 31 bands.
    pub transmittance: f64,
    /// Correlated Color Temperature in Kelvin (McCamy 1992 approximation).
    pub cct: f64,
}

// =============================================================================
// Bridge functions
// =============================================================================

/// Convert a dielectric material (IOR + roughness) to its dominant OKLCH color.
///
/// Uses the CIE 1931 2-degree observer with D65 illuminant.
/// Wavelength-dependent IOR is modeled via Cauchy dispersion:
/// `n(λ) = n_0 + B / λ²` with `B = 0.004 μm²` (typical glass).
///
/// # Arguments
/// * `ior` — base index of refraction at 589 nm (sodium D line)
/// * `roughness` — surface roughness in [0, 1]
/// * `cos_theta` — cosine of incidence angle (0 = grazing, 1 = normal)
///
/// # Returns
/// [`MaterialColorResult`] with dominant OKLCH color, reflectance, transmittance, and CCT.
pub fn bsdf_to_dominant_color(ior: f64, roughness: f64, cos_theta: f64) -> MaterialColorResult {
    let mut x_acc = 0.0_f64;
    let mut y_acc = 0.0_f64;
    let mut z_acc = 0.0_f64;
    let mut reflectance_acc = 0.0_f64;
    let mut transmittance_acc = 0.0_f64;
    let mut d65_y_norm = 0.0_f64;

    for i in 0..31 {
        let lambda = WAVELENGTHS[i];
        let d65 = ILLUMINANT_D65[i];

        // Cauchy dispersion: n(λ) = n₀ + B/λ²  (λ in μm, B = 0.004 μm² typical glass)
        let lambda_um = lambda / 1000.0;
        let spectral_ior = ior + 0.004 / (lambda_um * lambda_um);

        // Exact Fresnel for this wavelength's IOR
        let resp = evaluate_dielectric(spectral_ior, roughness, cos_theta);

        // Reflected spectrum under D65 illuminant
        let spectrum_r = resp.reflectance * d65;

        x_acc += spectrum_r * CIE_X_BAR[i];
        y_acc += spectrum_r * CIE_Y_BAR[i];
        z_acc += spectrum_r * CIE_Z_BAR[i];

        reflectance_acc += resp.reflectance;
        transmittance_acc += resp.transmittance;
        d65_y_norm += d65 * CIE_Y_BAR[i];
    }

    // Normalize XYZ by D65 Y normalization (so white = [1, 1, 1] approx)
    let norm = d65_y_norm.max(1e-10);
    let xyz = [x_acc / norm, y_acc / norm, z_acc / norm];

    // XYZ (D65) → linear sRGB (IEC 61966-2-1)
    let r_lin = 3.2404542 * xyz[0] - 1.5371385 * xyz[1] - 0.4985314 * xyz[2];
    let g_lin = -0.9692660 * xyz[0] + 1.8760108 * xyz[1] + 0.0415560 * xyz[2];
    let b_lin = 0.0556434 * xyz[0] - 0.2040259 * xyz[1] + 1.0572252 * xyz[2];

    // Gamma correction (IEC 61966-2-1 sRGB transfer function)
    fn to_srgb(x: f64) -> f64 {
        let c = x.clamp(0.0, 1.0);
        if c <= 0.0031308 {
            c * 12.92
        } else {
            1.055 * c.powf(1.0 / 2.4) - 0.055
        }
    }

    let color = Color::from_srgb(to_srgb(r_lin), to_srgb(g_lin), to_srgb(b_lin));
    let dominant = OKLCH::from_color(&color).map_to_gamut();

    // McCamy (1992) CCT approximation from chromaticity coordinates
    let x_chrom = xyz[0] / (xyz[0] + xyz[1] + xyz[2] + 1e-10);
    let y_chrom = xyz[1] / (xyz[0] + xyz[1] + xyz[2] + 1e-10);
    let n = (x_chrom - 0.3320) / (y_chrom - 0.1858 + 1e-10);
    let cct = (449.0 * n * n * n + 3525.0 * n * n + 6823.3 * n + 5520.33).clamp(1000.0, 20_000.0);

    MaterialColorResult {
        dominant,
        reflectance: (reflectance_acc / 31.0).clamp(0.0, 1.0),
        transmittance: (transmittance_acc / 31.0).clamp(0.0, 1.0),
        cct,
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_crown_glass_neutral() {
        // Crown glass (n=1.52), polished, near-normal incidence
        let result = bsdf_to_dominant_color(1.52, 0.0, 0.9);
        // L, C, H must be in valid ranges
        assert!(
            result.dominant.l > 0.0 && result.dominant.l <= 1.0,
            "L out of range: {}",
            result.dominant.l
        );
        assert!(
            result.dominant.c >= 0.0,
            "C must be non-negative: {}",
            result.dominant.c
        );
        assert!(result.reflectance >= 0.0 && result.reflectance <= 1.0);
        assert!(result.transmittance >= 0.0 && result.transmittance <= 1.0);
        assert!(
            result.cct > 1000.0 && result.cct < 20_000.0,
            "CCT out of range: {}",
            result.cct
        );
    }

    #[test]
    fn test_high_ior_more_reflective() {
        // Diamond (n≈2.42) is more reflective than glass at same angle
        let glass = bsdf_to_dominant_color(1.52, 0.0, 0.7);
        let diamond = bsdf_to_dominant_color(2.42, 0.0, 0.7);
        assert!(
            diamond.reflectance >= glass.reflectance,
            "Diamond ({:.4}) should be ≥ glass ({:.4})",
            diamond.reflectance,
            glass.reflectance
        );
    }

    #[test]
    fn test_energy_conservation() {
        // For a dielectric: R + T ≈ 1 (A ≈ 0)
        let result = bsdf_to_dominant_color(1.5, 0.1, 0.8);
        let sum = result.reflectance + result.transmittance;
        assert!(
            sum >= 0.85 && sum <= 1.05,
            "R+T should be ~1.0, got {:.4}",
            sum
        );
    }

    #[test]
    fn test_roughness_effect() {
        // Increasing roughness doesn't change total energy (only distribution)
        let smooth = bsdf_to_dominant_color(1.5, 0.0, 0.9);
        let rough = bsdf_to_dominant_color(1.5, 0.5, 0.9);
        let smooth_total = smooth.reflectance + smooth.transmittance;
        let rough_total = rough.reflectance + rough.transmittance;
        assert!(
            (smooth_total - rough_total).abs() < 0.2,
            "Roughness shouldn't change total energy dramatically"
        );
    }

    #[test]
    fn test_normal_incidence_fresnel() {
        // At normal incidence (cos_theta=1), glass n=1.5:
        // R = ((n-1)/(n+1))^2 = (0.5/2.5)^2 = 0.04
        let result = bsdf_to_dominant_color(1.5, 0.0, 1.0);
        assert!(
            (result.reflectance - 0.04).abs() < 0.01,
            "Fresnel at normal incidence: expected ~0.04, got {:.4}",
            result.reflectance
        );
    }
}
