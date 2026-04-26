//! # RGB Spectral Fresnel Evaluation
//!
//! Fresnel calculations with per-channel wavelength-dependent IOR.
//!
//! ## Physical Background
//!
//! Different wavelengths of light have different refractive indices in glass.
//! This causes:
//! - **Chromatic edge glow**: Different colors reflect differently at edges
//! - **Color fringing**: Visible separation at high-contrast boundaries
//! - **Enhanced realism**: Subtle color shifts make glass look more natural
//!
//! ## Implementation
//!
//! This module evaluates Fresnel equations at three wavelengths:
//! - Red: 656.3nm (C-line)
//! - Green: 587.6nm (d-line)
//! - Blue: 486.1nm (F-line)
//!
//! Each channel gets its own reflectance value, creating RGB chromatic effects.
//!
//! ## Performance
//!
//! RGB evaluation is ~3x the cost of single evaluation, but provides
//! significantly more realistic edge effects.

use super::dispersion::{f0_from_ior, CauchyDispersion, Dispersion};
use super::fresnel::fresnel_schlick;
use super::lut::FresnelLUT;

// ============================================================================
// SPECTRAL FRESNEL CALCULATIONS
// ============================================================================

/// Calculate Fresnel reflectance for RGB channels using dispersion model
///
/// Evaluates Fresnel at red, green, and blue wavelengths for chromatic effects.
///
/// # Arguments
///
/// * `dispersion` - Dispersion model for wavelength-dependent IOR
/// * `cos_theta` - Cosine of incident angle
///
/// # Returns
///
/// [R, G, B] Fresnel reflectance values
///
/// # Example
///
/// ```rust
/// use momoto_materials::glass_physics::spectral_fresnel::fresnel_rgb;
/// use momoto_materials::glass_physics::dispersion::CauchyDispersion;
///
/// let crown = CauchyDispersion::crown_glass();
/// let [r, g, b] = fresnel_rgb(&crown, 0.5);
///
/// // Blue has higher IOR, so higher reflectance
/// assert!(b > g && g > r);
/// ```
pub fn fresnel_rgb<D: Dispersion>(dispersion: &D, cos_theta: f64) -> [f64; 3] {
    let n_rgb = dispersion.n_rgb();

    [
        fresnel_schlick(1.0, n_rgb[0], cos_theta), // Red
        fresnel_schlick(1.0, n_rgb[1], cos_theta), // Green
        fresnel_schlick(1.0, n_rgb[2], cos_theta), // Blue
    ]
}

/// Fast Fresnel RGB using LUT
///
/// Uses precomputed LUT for each channel.
pub fn fresnel_rgb_fast<D: Dispersion>(dispersion: &D, cos_theta: f64) -> [f64; 3] {
    let n_rgb = dispersion.n_rgb();
    let lut = FresnelLUT::global();

    [
        lut.lookup(n_rgb[0], cos_theta),
        lut.lookup(n_rgb[1], cos_theta),
        lut.lookup(n_rgb[2], cos_theta),
    ]
}

/// Calculate edge intensity for RGB channels
///
/// Provides per-channel edge glow intensity for chromatic edge effects.
///
/// # Arguments
///
/// * `dispersion` - Dispersion model
/// * `cos_theta` - Cosine of view angle
/// * `edge_power` - Edge falloff power (1.5-4.0)
///
/// # Returns
///
/// [R, G, B] edge intensity values
pub fn edge_intensity_rgb<D: Dispersion>(
    dispersion: &D,
    cos_theta: f64,
    edge_power: f64,
) -> [f64; 3] {
    let fresnel = fresnel_rgb(dispersion, cos_theta);
    let cos_clamped = cos_theta.clamp(0.0, 1.0);
    let edge_factor = (1.0 - cos_clamped).powf(edge_power);

    [
        fresnel[0] * edge_factor,
        fresnel[1] * edge_factor,
        fresnel[2] * edge_factor,
    ]
}

// ============================================================================
// SPECTRAL FRESNEL RESULT
// ============================================================================

/// RGB Fresnel evaluation result
///
/// Contains per-channel reflectance and derived values.
#[derive(Debug, Clone, Copy)]
pub struct SpectralFresnelResult {
    /// Red channel reflectance
    pub r: f64,
    /// Green channel reflectance
    pub g: f64,
    /// Blue channel reflectance
    pub b: f64,

    /// Average reflectance (for compatibility)
    pub average: f64,

    /// Chromatic spread (b - r)
    pub chromatic_spread: f64,
}

impl SpectralFresnelResult {
    /// Create from RGB values
    pub fn from_rgb(rgb: [f64; 3]) -> Self {
        Self {
            r: rgb[0],
            g: rgb[1],
            b: rgb[2],
            average: (rgb[0] + rgb[1] + rgb[2]) / 3.0,
            chromatic_spread: rgb[2] - rgb[0],
        }
    }

    /// Convert to array
    pub fn to_array(&self) -> [f64; 3] {
        [self.r, self.g, self.b]
    }

    /// Check if chromatic effect is significant
    pub fn is_chromatic(&self) -> bool {
        self.chromatic_spread.abs() > 0.001
    }
}

// ============================================================================
// CSS GENERATION (Chromatic Edge Glow)
// ============================================================================

/// Generate CSS radial gradient for chromatic Fresnel edge glow
///
/// Creates a multi-layer gradient with per-channel color shifts for
/// realistic chromatic aberration at glass edges.
///
/// # Arguments
///
/// * `dispersion` - Dispersion model
/// * `intensity` - Overall glow intensity (0.0-1.0)
/// * `light_mode` - Whether to use light mode colors
///
/// # Returns
///
/// CSS background string with multiple gradient layers
pub fn to_css_chromatic_fresnel<D: Dispersion>(
    dispersion: &D,
    intensity: f64,
    light_mode: bool,
) -> String {
    let intensity = intensity.clamp(0.0, 1.0);

    // Get RGB IORs
    let n_rgb = dispersion.n_rgb();

    // Calculate chromatic spread
    let spread = n_rgb[2] - n_rgb[0]; // Blue - Red

    // Boost for visibility
    let boost = if light_mode { 1.8 } else { 1.4 };
    let boosted = (intensity * boost).min(1.0);

    // If dispersion is negligible, use white
    if spread < 0.001 {
        return format!(
            "radial-gradient(ellipse 100% 100% at center, \
             rgba(255, 255, 255, 0) 0%, \
             rgba(255, 255, 255, 0) 40%, \
             rgba(255, 255, 255, {:.3}) 60%, \
             rgba(255, 255, 255, {:.3}) 75%, \
             rgba(255, 255, 255, {:.3}) 88%, \
             rgba(255, 255, 255, {:.3}) 100%)",
            boosted * 0.15,
            boosted * 0.35,
            boosted * 0.65,
            boosted * 0.85,
        );
    }

    // Calculate per-channel edge strengths
    // Higher IOR = stronger edge effect
    let f0_r = f0_from_ior(n_rgb[0]);
    let f0_g = f0_from_ior(n_rgb[1]);
    let f0_b = f0_from_ior(n_rgb[2]);

    // Normalize relative to green
    let r_factor = f0_r / f0_g;
    let b_factor = f0_b / f0_g;

    // Chromatic fringe: blue shows first (inner), red last (outer)
    // This creates the characteristic "rainbow" edge
    let chromatic_offset = (spread * 50.0).min(8.0); // Max 8% offset

    format!(
        "radial-gradient(ellipse 100% 100% at center, \
         rgba(100, 150, 255, 0) 0%, \
         rgba(100, 150, 255, 0) {}%, \
         rgba(100, 150, 255, {:.3}) {}%, \
         rgba(255, 255, 255, {:.3}) 75%, \
         rgba(255, 200, 150, {:.3}) {}%, \
         rgba(255, 200, 150, {:.3}) 100%)",
        40.0 - chromatic_offset,       // Blue transparent zone start
        boosted * 0.2 * b_factor,      // Blue alpha
        60.0 - chromatic_offset / 2.0, // Blue zone end
        boosted * 0.5,                 // White center alpha
        85.0 + chromatic_offset / 2.0, // Red zone start
        boosted * 0.4 * r_factor,      // Red inner alpha
        boosted * 0.2 * r_factor,      // Red edge alpha (fade out)
    )
}

/// Generate CSS for chromatic edge border
///
/// Creates subtle colored inner glow at edges.
pub fn to_css_chromatic_border<D: Dispersion>(
    dispersion: &D,
    intensity: f64,
    border_radius: f64,
) -> String {
    let intensity = intensity.clamp(0.0, 1.0);
    let n_rgb = dispersion.n_rgb();
    let spread = n_rgb[2] - n_rgb[0];

    let inner_spread = (border_radius * 0.1).max(1.0).min(4.0);
    let blur = inner_spread * 2.0;

    // Slight blue tint on inside, slight red on outside
    let blue_opacity = intensity * 0.3 * (1.0 + spread * 10.0);
    let red_opacity = intensity * 0.2;

    format!(
        "inset 0 0 {:.1}px {:.1}px rgba(150, 180, 255, {:.3}), \
         inset 0 0 {:.1}px {:.1}px rgba(255, 220, 200, {:.3})",
        blur * 0.8,
        inner_spread * 0.8,
        blue_opacity,
        blur * 1.2,
        inner_spread * 1.2,
        red_opacity,
    )
}

// ============================================================================
// SPECTRAL FRESNEL LUT
// ============================================================================

use std::sync::OnceLock;

/// Pre-computed spectral Fresnel LUT
///
/// Stores Fresnel values for multiple wavelengths.
///
/// ## Dimensions
///
/// - IOR base: 32 values (1.0 to 2.5)
/// - Dispersion: 16 levels (0 to high)
/// - Angle: 64 values (cos 0 to 1)
/// - Channels: 3 (R, G, B)
///
/// ## Memory
///
/// 32 * 16 * 64 * 3 * 4 bytes = ~384KB
pub struct SpectralFresnelLUT {
    /// Table[ior][dispersion][angle] = [r, g, b]
    table: Box<[[[[f32; 3]; 64]; 16]; 32]>,
}

impl SpectralFresnelLUT {
    const IOR_MIN: f64 = 1.0;
    const IOR_MAX: f64 = 2.5;
    const IOR_COUNT: usize = 32;
    const IOR_STEP: f64 = (Self::IOR_MAX - Self::IOR_MIN) / (Self::IOR_COUNT - 1) as f64;

    const DISP_MIN: f64 = 0.0;
    const DISP_MAX: f64 = 20000.0; // Cauchy B coefficient range
    const DISP_COUNT: usize = 16;
    const DISP_STEP: f64 = (Self::DISP_MAX - Self::DISP_MIN) / (Self::DISP_COUNT - 1) as f64;

    const ANGLE_COUNT: usize = 64;

    fn build() -> Self {
        let mut table = Box::new([[[[0.0f32; 3]; 64]; 16]; 32]);

        for i in 0..Self::IOR_COUNT {
            let ior_base = Self::IOR_MIN + i as f64 * Self::IOR_STEP;

            for d in 0..Self::DISP_COUNT {
                let disp_b = Self::DISP_MIN + d as f64 * Self::DISP_STEP;
                let dispersion = CauchyDispersion::new(ior_base, disp_b, 0.0);

                for a in 0..Self::ANGLE_COUNT {
                    let cos_theta = a as f64 / (Self::ANGLE_COUNT - 1) as f64;
                    let rgb = fresnel_rgb(&dispersion, cos_theta);

                    table[i][d][a] = [rgb[0] as f32, rgb[1] as f32, rgb[2] as f32];
                }
            }
        }

        Self { table }
    }

    /// Get global instance
    pub fn global() -> &'static SpectralFresnelLUT {
        static LUT: OnceLock<SpectralFresnelLUT> = OnceLock::new();
        LUT.get_or_init(SpectralFresnelLUT::build)
    }

    /// Fast lookup with trilinear interpolation
    pub fn lookup(&self, ior_base: f64, dispersion_b: f64, cos_theta: f64) -> [f64; 3] {
        let ior_clamped = ior_base.clamp(Self::IOR_MIN, Self::IOR_MAX);
        let disp_clamped = dispersion_b.clamp(Self::DISP_MIN, Self::DISP_MAX);
        let cos_clamped = cos_theta.clamp(0.0, 1.0);

        // Map to indices
        let ior_idx_f = (ior_clamped - Self::IOR_MIN) / Self::IOR_STEP;
        let ior_i0 = (ior_idx_f.floor() as usize).min(Self::IOR_COUNT - 2);
        let ior_t = ior_idx_f - ior_i0 as f64;

        let disp_idx_f = (disp_clamped - Self::DISP_MIN) / Self::DISP_STEP;
        let disp_i0 = (disp_idx_f.floor() as usize).min(Self::DISP_COUNT - 2);
        let disp_t = disp_idx_f - disp_i0 as f64;

        let angle_idx_f = cos_clamped * (Self::ANGLE_COUNT - 1) as f64;
        let angle_i0 = (angle_idx_f.floor() as usize).min(Self::ANGLE_COUNT - 2);
        let angle_t = angle_idx_f - angle_i0 as f64;

        // Trilinear interpolation (simplified: 2 bilinear interpolations)
        let mut result = [0.0f64; 3];

        for ch in 0..3 {
            let v000 = self.table[ior_i0][disp_i0][angle_i0][ch] as f64;
            let v001 = self.table[ior_i0][disp_i0][angle_i0 + 1][ch] as f64;
            let v010 = self.table[ior_i0][disp_i0 + 1][angle_i0][ch] as f64;
            let v011 = self.table[ior_i0][disp_i0 + 1][angle_i0 + 1][ch] as f64;
            let v100 = self.table[ior_i0 + 1][disp_i0][angle_i0][ch] as f64;
            let v101 = self.table[ior_i0 + 1][disp_i0][angle_i0 + 1][ch] as f64;
            let v110 = self.table[ior_i0 + 1][disp_i0 + 1][angle_i0][ch] as f64;
            let v111 = self.table[ior_i0 + 1][disp_i0 + 1][angle_i0 + 1][ch] as f64;

            // Interpolate along angle
            let v00 = v000 + (v001 - v000) * angle_t;
            let v01 = v010 + (v011 - v010) * angle_t;
            let v10 = v100 + (v101 - v100) * angle_t;
            let v11 = v110 + (v111 - v110) * angle_t;

            // Interpolate along dispersion
            let v0 = v00 + (v01 - v00) * disp_t;
            let v1 = v10 + (v11 - v10) * disp_t;

            // Interpolate along IOR
            result[ch] = v0 + (v1 - v0) * ior_t;
        }

        result
    }

    /// Memory size
    pub fn memory_size(&self) -> usize {
        Self::IOR_COUNT * Self::DISP_COUNT * Self::ANGLE_COUNT * 3 * std::mem::size_of::<f32>()
    }
}

/// Fast spectral Fresnel using LUT
pub fn fresnel_rgb_lut(ior_base: f64, dispersion_b: f64, cos_theta: f64) -> [f64; 3] {
    SpectralFresnelLUT::global().lookup(ior_base, dispersion_b, cos_theta)
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::super::dispersion::wavelengths;
    use super::*;

    #[test]
    fn test_fresnel_rgb_ordering() {
        let crown = CauchyDispersion::crown_glass();
        let [r, g, b] = fresnel_rgb(&crown, 0.5);

        // Higher IOR = higher F0 = higher reflectance
        // Blue wavelength has highest IOR
        assert!(r < g, "Red should have lower reflectance than green");
        assert!(g < b, "Green should have lower reflectance than blue");
    }

    #[test]
    fn test_fresnel_rgb_vs_scalar() {
        let crown = CauchyDispersion::crown_glass();
        let cos_theta = 0.8;

        let rgb = fresnel_rgb(&crown, cos_theta);
        let n_d = crown.n(wavelengths::SODIUM_D);
        let scalar = fresnel_schlick(1.0, n_d, cos_theta);

        // Average of RGB should be close to scalar at d-line
        let avg = (rgb[0] + rgb[1] + rgb[2]) / 3.0;
        assert!(
            (avg - scalar).abs() < 0.01,
            "RGB average {} should be close to scalar {}",
            avg,
            scalar
        );
    }

    #[test]
    fn test_spectral_result() {
        let crown = CauchyDispersion::crown_glass();
        let rgb = fresnel_rgb(&crown, 0.5);
        let result = SpectralFresnelResult::from_rgb(rgb);

        assert!(
            result.is_chromatic(),
            "Crown glass should show chromatic effect"
        );
        assert!(
            result.chromatic_spread > 0.0,
            "Blue should be higher than red"
        );
    }

    #[test]
    fn test_constant_ior_no_chromatic() {
        let constant = CauchyDispersion::constant(1.5);
        let [r, g, b] = fresnel_rgb(&constant, 0.5);

        // All channels should be equal
        assert!((r - g).abs() < 0.0001);
        assert!((g - b).abs() < 0.0001);
    }

    #[test]
    fn test_high_dispersion_large_spread() {
        let flint = CauchyDispersion::flint_glass();
        let crown = CauchyDispersion::crown_glass();

        let spread_flint = {
            let rgb = fresnel_rgb(&flint, 0.5);
            rgb[2] - rgb[0]
        };

        let spread_crown = {
            let rgb = fresnel_rgb(&crown, 0.5);
            rgb[2] - rgb[0]
        };

        assert!(
            spread_flint > spread_crown,
            "Flint should have larger chromatic spread"
        );
    }

    #[test]
    fn test_spectral_lut() {
        let lut = SpectralFresnelLUT::global();

        // Test against direct calculation
        let crown = CauchyDispersion::crown_glass();
        let direct = fresnel_rgb(&crown, 0.5);
        let from_lut = lut.lookup(crown.a, crown.b, 0.5);

        for ch in 0..3 {
            let error = (direct[ch] - from_lut[ch]).abs();
            assert!(error < 0.02, "LUT error {} in channel {}", error, ch);
        }
    }

    #[test]
    fn test_lut_memory() {
        let lut = SpectralFresnelLUT::global();
        let size = lut.memory_size();

        // Should be ~384KB
        assert!(
            size > 350_000 && size < 400_000,
            "Spectral LUT should be ~384KB, got {}",
            size
        );
    }

    #[test]
    fn test_css_generation() {
        let crown = CauchyDispersion::crown_glass();
        let css = to_css_chromatic_fresnel(&crown, 0.3, true);

        assert!(css.contains("radial-gradient"));
        // Should have color values for chromatic effect
        assert!(css.contains("rgba"));
    }
}
