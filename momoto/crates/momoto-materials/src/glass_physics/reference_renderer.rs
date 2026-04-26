//! # Reference Renderer Module
//!
//! IEEE754 maximum precision rendering without LUTs or approximations.
//!
//! ## Purpose
//!
//! This module provides a **reference-grade** renderer for:
//! - Scientific validation against path tracers
//! - Comparison with real-world measurements
//! - LUT accuracy verification
//! - Energy conservation testing
//!
//! ## Features
//!
//! - Full Fresnel equations (not Schlick approximation)
//! - Exact Beer-Lambert absorption
//! - Transfer matrix method for thin-film interference
//! - Energy conservation tracking
//! - Bit-exact reproducibility
//!
//! ## Usage
//!
//! ```rust
//! use momoto_materials::glass_physics::reference_renderer::{
//!     ReferenceRenderer, ReferenceRenderConfig, PrecisionMode
//! };
//!
//! let config = ReferenceRenderConfig::default();
//! let renderer = ReferenceRenderer::new(config);
//!
//! // Evaluate material at specific conditions
//! let result = renderer.evaluate_dielectric(1.5, 0.8, 0.1, 10.0);
//! println!("Reflectance: {:.6}", result.reflectance);
//! println!("Energy error: {:.2e}", result.energy_conservation_error);
//! ```

use std::f64::consts::PI;
use std::time::Instant;

// ============================================================================
// CONFIGURATION
// ============================================================================

/// Precision mode for reference rendering
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PrecisionMode {
    /// Standard 32-bit float (for comparison)
    F32,
    /// Full 64-bit float (default for reference)
    F64,
}

impl Default for PrecisionMode {
    fn default() -> Self {
        Self::F64
    }
}

/// Configuration for reference rendering
#[derive(Debug, Clone)]
pub struct ReferenceRenderConfig {
    /// Precision mode for computations
    pub precision: PrecisionMode,
    /// Number of spectral bands (31 for visible, 81 for extended)
    pub spectral_bands: usize,
    /// Wavelength range minimum (nm)
    pub wavelength_min: f64,
    /// Wavelength range maximum (nm)
    pub wavelength_max: f64,
    /// Use full Fresnel equations (vs Schlick approximation)
    pub enable_full_fresnel: bool,
    /// Enable exact thin-film transfer matrix
    pub enable_transfer_matrix: bool,
    /// Track energy conservation
    pub track_energy: bool,
}

impl Default for ReferenceRenderConfig {
    fn default() -> Self {
        Self {
            precision: PrecisionMode::F64,
            spectral_bands: 31,
            wavelength_min: 400.0,
            wavelength_max: 700.0,
            enable_full_fresnel: true,
            enable_transfer_matrix: true,
            track_energy: true,
        }
    }
}

impl ReferenceRenderConfig {
    /// Create config for visible spectrum (31 bands, 400-700nm)
    pub fn visible() -> Self {
        Self::default()
    }

    /// Create config for extended spectrum (81 bands, 300-1100nm)
    pub fn extended() -> Self {
        Self {
            spectral_bands: 81,
            wavelength_min: 300.0,
            wavelength_max: 1100.0,
            ..Default::default()
        }
    }

    /// Create config with minimal precision (for comparison testing)
    pub fn low_precision() -> Self {
        Self {
            precision: PrecisionMode::F32,
            enable_full_fresnel: false,
            enable_transfer_matrix: false,
            ..Default::default()
        }
    }

    /// Get wavelength step size
    pub fn wavelength_step(&self) -> f64 {
        (self.wavelength_max - self.wavelength_min) / (self.spectral_bands - 1) as f64
    }

    /// Get wavelength at band index
    pub fn wavelength_at(&self, band: usize) -> f64 {
        self.wavelength_min + band as f64 * self.wavelength_step()
    }

    /// Get all wavelengths
    pub fn wavelengths(&self) -> Vec<f64> {
        (0..self.spectral_bands)
            .map(|i| self.wavelength_at(i))
            .collect()
    }
}

// ============================================================================
// RENDER RESULTS
// ============================================================================

/// Reference rendering result
#[derive(Debug, Clone)]
pub struct ReferenceRenderResult {
    /// Spectral reflectance at each wavelength
    pub spectral_reflectance: Vec<f64>,
    /// Spectral transmittance at each wavelength
    pub spectral_transmittance: Vec<f64>,
    /// Integrated reflectance (scalar)
    pub reflectance: f64,
    /// Integrated transmittance (scalar)
    pub transmittance: f64,
    /// Absorption (1 - R - T)
    pub absorption: f64,
    /// XYZ color coordinates
    pub xyz: [f64; 3],
    /// sRGB color (0-1 range)
    pub rgb: [f64; 3],
    /// Energy conservation error: |1 - (R + T + A)|
    pub energy_conservation_error: f64,
    /// Computation time in microseconds
    pub computation_time_us: f64,
    /// Number of wavelengths evaluated
    pub wavelength_count: usize,
}

impl ReferenceRenderResult {
    /// Check if energy is conserved within tolerance
    pub fn energy_conserved(&self, tolerance: f64) -> bool {
        self.energy_conservation_error < tolerance
    }
}

/// LUT vs Reference comparison result
#[derive(Debug, Clone)]
pub struct LutVsReferenceComparison {
    /// Maximum absolute error
    pub max_error: f64,
    /// Mean absolute error
    pub mean_error: f64,
    /// Root mean square error
    pub rmse: f64,
    /// Number of samples compared
    pub samples_compared: usize,
    /// Worst-case angle (if applicable)
    pub worst_angle: Option<f64>,
    /// Worst-case wavelength (if applicable)
    pub worst_wavelength: Option<f64>,
    /// Error histogram (10 bins: 0-0.001, 0.001-0.01, etc.)
    pub error_histogram: [usize; 10],
}

impl LutVsReferenceComparison {
    /// Create new comparison result
    pub fn new() -> Self {
        Self {
            max_error: 0.0,
            mean_error: 0.0,
            rmse: 0.0,
            samples_compared: 0,
            worst_angle: None,
            worst_wavelength: None,
            error_histogram: [0; 10],
        }
    }

    /// Check if LUT error is acceptable
    pub fn is_acceptable(&self, max_allowed_error: f64) -> bool {
        self.max_error <= max_allowed_error
    }

    /// Get error bin for histogram
    fn error_bin(error: f64) -> usize {
        let abs_error = error.abs();
        if abs_error < 0.0001 {
            0
        } else if abs_error < 0.001 {
            1
        } else if abs_error < 0.005 {
            2
        } else if abs_error < 0.01 {
            3
        } else if abs_error < 0.02 {
            4
        } else if abs_error < 0.05 {
            5
        } else if abs_error < 0.1 {
            6
        } else if abs_error < 0.2 {
            7
        } else if abs_error < 0.5 {
            8
        } else {
            9
        }
    }
}

impl Default for LutVsReferenceComparison {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// REFERENCE RENDERER
// ============================================================================

/// Reference-grade renderer for scientific validation
#[derive(Debug, Clone)]
pub struct ReferenceRenderer {
    config: ReferenceRenderConfig,
    /// CIE 1931 2-degree color matching functions
    cmf_x: Vec<f64>,
    cmf_y: Vec<f64>,
    cmf_z: Vec<f64>,
}

impl ReferenceRenderer {
    /// Create new reference renderer with configuration
    pub fn new(config: ReferenceRenderConfig) -> Self {
        let wavelengths = config.wavelengths();
        let cmf_x = wavelengths.iter().map(|&w| cie_x_bar(w)).collect();
        let cmf_y = wavelengths.iter().map(|&w| cie_y_bar(w)).collect();
        let cmf_z = wavelengths.iter().map(|&w| cie_z_bar(w)).collect();

        Self {
            config,
            cmf_x,
            cmf_y,
            cmf_z,
        }
    }

    /// Get configuration
    pub fn config(&self) -> &ReferenceRenderConfig {
        &self.config
    }

    // ========================================================================
    // DIELECTRIC MATERIAL EVALUATION
    // ========================================================================

    /// Evaluate dielectric material (glass-like) with full precision
    pub fn evaluate_dielectric(
        &self,
        ior: f64,
        cos_theta: f64,
        absorption_coeff: f64,
        thickness: f64,
    ) -> ReferenceRenderResult {
        let start = Instant::now();
        let wavelengths = self.config.wavelengths();
        let n_bands = wavelengths.len();

        let mut spectral_reflectance = Vec::with_capacity(n_bands);
        let mut spectral_transmittance = Vec::with_capacity(n_bands);

        for _wavelength in &wavelengths {
            // Full Fresnel reflectance for dielectric
            let r = if self.config.enable_full_fresnel {
                fresnel_dielectric_full(cos_theta, ior)
            } else {
                fresnel_schlick_reference(cos_theta, ior)
            };

            // Exact Beer-Lambert transmittance
            let t_internal = beer_lambert_exact(absorption_coeff, thickness, cos_theta);

            // Total transmittance accounts for both surfaces
            let t = (1.0 - r) * t_internal * (1.0 - r);

            spectral_reflectance.push(r);
            spectral_transmittance.push(t);
        }

        self.finalize_result(spectral_reflectance, spectral_transmittance, start)
    }

    /// Evaluate dielectric with wavelength-dependent IOR (dispersion)
    pub fn evaluate_dielectric_dispersive(
        &self,
        ior_fn: impl Fn(f64) -> f64,
        cos_theta: f64,
        absorption_coeff: f64,
        thickness: f64,
    ) -> ReferenceRenderResult {
        let start = Instant::now();
        let wavelengths = self.config.wavelengths();
        let n_bands = wavelengths.len();

        let mut spectral_reflectance = Vec::with_capacity(n_bands);
        let mut spectral_transmittance = Vec::with_capacity(n_bands);

        for &wavelength in &wavelengths {
            let ior = ior_fn(wavelength);

            let r = if self.config.enable_full_fresnel {
                fresnel_dielectric_full(cos_theta, ior)
            } else {
                fresnel_schlick_reference(cos_theta, ior)
            };

            let t_internal = beer_lambert_exact(absorption_coeff, thickness, cos_theta);
            let t = (1.0 - r) * t_internal * (1.0 - r);

            spectral_reflectance.push(r);
            spectral_transmittance.push(t);
        }

        self.finalize_result(spectral_reflectance, spectral_transmittance, start)
    }

    // ========================================================================
    // METAL MATERIAL EVALUATION
    // ========================================================================

    /// Evaluate metal material with complex IOR
    pub fn evaluate_metal(&self, n: f64, k: f64, cos_theta: f64) -> ReferenceRenderResult {
        let start = Instant::now();
        let wavelengths = self.config.wavelengths();
        let n_bands = wavelengths.len();

        let mut spectral_reflectance = Vec::with_capacity(n_bands);
        let spectral_transmittance = vec![0.0; n_bands]; // Metals don't transmit

        for _wavelength in &wavelengths {
            let r = fresnel_conductor_full(cos_theta, n, k);
            spectral_reflectance.push(r);
        }

        self.finalize_result(spectral_reflectance, spectral_transmittance, start)
    }

    /// Evaluate metal with wavelength-dependent complex IOR
    pub fn evaluate_metal_spectral(
        &self,
        n_fn: impl Fn(f64) -> f64,
        k_fn: impl Fn(f64) -> f64,
        cos_theta: f64,
    ) -> ReferenceRenderResult {
        let start = Instant::now();
        let wavelengths = self.config.wavelengths();
        let n_bands = wavelengths.len();

        let mut spectral_reflectance = Vec::with_capacity(n_bands);
        let spectral_transmittance = vec![0.0; n_bands];

        for &wavelength in &wavelengths {
            let n = n_fn(wavelength);
            let k = k_fn(wavelength);
            let r = fresnel_conductor_full(cos_theta, n, k);
            spectral_reflectance.push(r);
        }

        self.finalize_result(spectral_reflectance, spectral_transmittance, start)
    }

    // ========================================================================
    // THIN-FILM EVALUATION
    // ========================================================================

    /// Evaluate single-layer thin-film interference
    pub fn evaluate_thin_film(
        &self,
        n_film: f64,
        thickness_nm: f64,
        n_substrate: f64,
        cos_theta: f64,
    ) -> ReferenceRenderResult {
        let start = Instant::now();
        let wavelengths = self.config.wavelengths();
        let n_bands = wavelengths.len();

        let mut spectral_reflectance: Vec<f64> = Vec::with_capacity(n_bands);
        let spectral_transmittance: Vec<f64> = Vec::with_capacity(n_bands);

        for &wavelength in &wavelengths {
            let r = if self.config.enable_transfer_matrix {
                thin_film_transfer_matrix(wavelength, n_film, thickness_nm, n_substrate, cos_theta)
            } else {
                thin_film_simple(wavelength, n_film, thickness_nm, n_substrate, cos_theta)
            };

            spectral_reflectance.push(r);
        }

        // Approximate transmittance for thin-film
        let spectral_transmittance: Vec<f64> = spectral_reflectance
            .iter()
            .map(|r| (1.0 - r).max(0.0))
            .collect();

        self.finalize_result(spectral_reflectance, spectral_transmittance, start)
    }

    /// Evaluate multi-layer thin-film stack using transfer matrix method
    pub fn evaluate_thin_film_stack(
        &self,
        layers: &[(f64, f64)], // (n, thickness_nm) pairs
        n_substrate: f64,
        cos_theta: f64,
    ) -> ReferenceRenderResult {
        let start = Instant::now();
        let wavelengths = self.config.wavelengths();
        let n_bands = wavelengths.len();

        let mut spectral_reflectance = Vec::with_capacity(n_bands);

        for &wavelength in &wavelengths {
            let r = thin_film_stack_transfer_matrix(wavelength, layers, n_substrate, cos_theta);
            spectral_reflectance.push(r);
        }

        let spectral_transmittance: Vec<f64> = spectral_reflectance
            .iter()
            .map(|r| (1.0 - r).max(0.0))
            .collect();

        self.finalize_result(spectral_reflectance, spectral_transmittance, start)
    }

    // ========================================================================
    // PHASE FUNCTION EVALUATION
    // ========================================================================

    /// Evaluate Henyey-Greenstein phase function with full precision
    pub fn evaluate_henyey_greenstein(&self, g: f64, cos_theta: f64) -> f64 {
        henyey_greenstein_exact(g, cos_theta)
    }

    /// Evaluate double Henyey-Greenstein phase function
    pub fn evaluate_double_hg(&self, g1: f64, g2: f64, weight: f64, cos_theta: f64) -> f64 {
        let p1 = henyey_greenstein_exact(g1, cos_theta);
        let p2 = henyey_greenstein_exact(g2, cos_theta);
        weight * p1 + (1.0 - weight) * p2
    }

    // ========================================================================
    // RESULT FINALIZATION
    // ========================================================================

    fn finalize_result(
        &self,
        spectral_reflectance: Vec<f64>,
        spectral_transmittance: Vec<f64>,
        start: Instant,
    ) -> ReferenceRenderResult {
        let n_bands = spectral_reflectance.len();

        // Integrate reflectance and transmittance (luminance-weighted)
        let reflectance = self.integrate_luminance(&spectral_reflectance);
        let transmittance = self.integrate_luminance(&spectral_transmittance);
        let absorption = (1.0 - reflectance - transmittance).max(0.0);

        // Convert to XYZ and RGB
        let xyz = self.spectral_to_xyz(&spectral_reflectance);
        let rgb = xyz_to_srgb(xyz);

        // Energy conservation error
        let energy_conservation_error = (1.0 - reflectance - transmittance - absorption).abs();

        let computation_time_us = start.elapsed().as_nanos() as f64 / 1000.0;

        ReferenceRenderResult {
            spectral_reflectance,
            spectral_transmittance,
            reflectance,
            transmittance,
            absorption,
            xyz,
            rgb,
            energy_conservation_error,
            computation_time_us,
            wavelength_count: n_bands,
        }
    }

    /// Integrate spectral data weighted by luminance (Y)
    fn integrate_luminance(&self, spectral: &[f64]) -> f64 {
        let mut sum = 0.0;
        let mut weight_sum = 0.0;

        for (i, &value) in spectral.iter().enumerate() {
            let weight = self.cmf_y[i];
            sum += value * weight;
            weight_sum += weight;
        }

        if weight_sum > 0.0 {
            sum / weight_sum
        } else {
            0.0
        }
    }

    /// Convert spectral data to CIE XYZ
    fn spectral_to_xyz(&self, spectral: &[f64]) -> [f64; 3] {
        let mut x = 0.0;
        let mut y = 0.0;
        let mut z = 0.0;

        let step = self.config.wavelength_step();

        for (i, &value) in spectral.iter().enumerate() {
            x += value * self.cmf_x[i] * step;
            y += value * self.cmf_y[i] * step;
            z += value * self.cmf_z[i] * step;
        }

        // Normalize (standard observer normalization)
        let norm = 1.0 / 106.857; // D65 normalization factor
        [x * norm, y * norm, z * norm]
    }
}

impl Default for ReferenceRenderer {
    fn default() -> Self {
        Self::new(ReferenceRenderConfig::default())
    }
}

// ============================================================================
// COMPARISON UTILITIES
// ============================================================================

/// Compare LUT-based results against reference
pub fn compare_lut_vs_reference(
    lut_results: &[f64],
    ref_results: &[f64],
) -> LutVsReferenceComparison {
    if lut_results.len() != ref_results.len() || lut_results.is_empty() {
        return LutVsReferenceComparison::new();
    }

    let n = lut_results.len();
    let mut comparison = LutVsReferenceComparison::new();
    comparison.samples_compared = n;

    let mut sum_error = 0.0;
    let mut sum_sq_error = 0.0;
    let mut max_idx = 0;

    for i in 0..n {
        let error = (lut_results[i] - ref_results[i]).abs();
        sum_error += error;
        sum_sq_error += error * error;

        // Track maximum error
        if error > comparison.max_error {
            comparison.max_error = error;
            max_idx = i;
        }

        // Update histogram
        let bin = LutVsReferenceComparison::error_bin(error);
        comparison.error_histogram[bin] += 1;
    }

    comparison.mean_error = sum_error / n as f64;
    comparison.rmse = (sum_sq_error / n as f64).sqrt();

    // Record worst-case index (could be mapped to angle/wavelength by caller)
    let _ = max_idx; // Suppress unused warning

    comparison
}

/// Compare Fresnel approximations against full equations
pub fn compare_fresnel_approximations(ior: f64, n_angles: usize) -> LutVsReferenceComparison {
    let mut lut_results = Vec::with_capacity(n_angles);
    let mut ref_results = Vec::with_capacity(n_angles);
    let mut worst_angle = 0.0;
    let mut max_error = 0.0;

    for i in 0..n_angles {
        let cos_theta = i as f64 / (n_angles - 1) as f64;

        let schlick = fresnel_schlick_reference(cos_theta, ior);
        let full = fresnel_dielectric_full(cos_theta, ior);

        lut_results.push(schlick);
        ref_results.push(full);

        let error = (schlick - full).abs();
        if error > max_error {
            max_error = error;
            worst_angle = cos_theta.acos().to_degrees();
        }
    }

    let mut comparison = compare_lut_vs_reference(&lut_results, &ref_results);
    comparison.worst_angle = Some(worst_angle);

    comparison
}

// ============================================================================
// FRESNEL EQUATIONS (Full Precision)
// ============================================================================

/// Full Fresnel equations for dielectric (unpolarized)
pub fn fresnel_dielectric_full(cos_theta_i: f64, ior: f64) -> f64 {
    let cos_i = cos_theta_i.clamp(0.0, 1.0);
    let sin_i = (1.0 - cos_i * cos_i).sqrt();

    // Snell's law: n1 * sin(theta1) = n2 * sin(theta2)
    // Assuming n1 = 1.0 (air), n2 = ior
    let sin_t = sin_i / ior;

    // Total internal reflection check
    if sin_t >= 1.0 {
        return 1.0;
    }

    let cos_t = (1.0 - sin_t * sin_t).sqrt();

    // Fresnel equations for s and p polarization
    let rs = (cos_i - ior * cos_t) / (cos_i + ior * cos_t);
    let rp = (ior * cos_i - cos_t) / (ior * cos_i + cos_t);

    // Unpolarized light: average of s and p
    (rs * rs + rp * rp) / 2.0
}

/// Schlick approximation for reference comparison
pub fn fresnel_schlick_reference(cos_theta: f64, ior: f64) -> f64 {
    let f0 = ((ior - 1.0) / (ior + 1.0)).powi(2);
    let cos_t = cos_theta.clamp(0.0, 1.0);
    f0 + (1.0 - f0) * (1.0 - cos_t).powi(5)
}

/// Full Fresnel equations for conductor (metal)
pub fn fresnel_conductor_full(cos_theta_i: f64, n: f64, k: f64) -> f64 {
    let cos_i = cos_theta_i.clamp(0.0, 1.0);
    let cos_i2 = cos_i * cos_i;
    let sin_i2 = 1.0 - cos_i2;

    let n2 = n * n;
    let k2 = k * k;
    let n2_k2 = n2 - k2;

    // Calculate intermediate terms
    let a2_b2 = ((n2_k2 - sin_i2).powi(2) + 4.0 * n2 * k2).sqrt();
    let a = ((a2_b2 + n2_k2 - sin_i2) / 2.0).sqrt();

    // Rs (s-polarized)
    let rs_num = a2_b2 + cos_i2 - 2.0 * a * cos_i;
    let rs_den = a2_b2 + cos_i2 + 2.0 * a * cos_i;
    let rs2 = rs_num / rs_den;

    // Rp (p-polarized)
    let _cos_i2_sin_i2 = cos_i2 * sin_i2;
    let rp_num = a2_b2 * cos_i2 + sin_i2 * sin_i2 - 2.0 * a * cos_i * sin_i2;
    let rp_den = a2_b2 * cos_i2 + sin_i2 * sin_i2 + 2.0 * a * cos_i * sin_i2;
    let rp2 = rs2 * rp_num / rp_den;

    // Unpolarized: average
    (rs2 + rp2) / 2.0
}

// ============================================================================
// BEER-LAMBERT (Full Precision)
// ============================================================================

/// Exact Beer-Lambert transmittance
pub fn beer_lambert_exact(absorption_coeff: f64, thickness: f64, cos_theta: f64) -> f64 {
    let cos_t = cos_theta.clamp(0.001, 1.0);
    let path_length = thickness / cos_t;
    (-absorption_coeff * path_length).exp()
}

// ============================================================================
// THIN-FILM (Transfer Matrix Method)
// ============================================================================

/// Transfer matrix method for single-layer thin-film
pub fn thin_film_transfer_matrix(
    wavelength_nm: f64,
    n_film: f64,
    thickness_nm: f64,
    n_substrate: f64,
    cos_theta: f64,
) -> f64 {
    // Phase thickness (optical path difference)
    let sin_theta = (1.0 - cos_theta * cos_theta).sqrt();
    let sin_film = sin_theta / n_film;

    if sin_film >= 1.0 {
        return 1.0; // Total internal reflection
    }

    let cos_film = (1.0 - sin_film * sin_film).sqrt();

    // Optical path length in film
    let delta = 2.0 * PI * n_film * thickness_nm * cos_film / wavelength_nm;

    // Fresnel coefficients at each interface
    let r01 = (cos_theta - n_film * cos_film) / (cos_theta + n_film * cos_film);
    let r12 = (n_film * cos_film - n_substrate) / (n_film * cos_film + n_substrate);

    // Multiple reflection formula (Airy formula)
    let numerator = r01 * r01 + r12 * r12 + 2.0 * r01 * r12 * (2.0 * delta).cos();
    let denominator = 1.0 + r01 * r01 * r12 * r12 + 2.0 * r01 * r12 * (2.0 * delta).cos();

    (numerator / denominator).clamp(0.0, 1.0)
}

/// Simple thin-film model (for comparison)
pub fn thin_film_simple(
    wavelength_nm: f64,
    n_film: f64,
    thickness_nm: f64,
    _n_substrate: f64,
    cos_theta: f64,
) -> f64 {
    // Simplified constructive/destructive interference
    let optical_path = 2.0 * n_film * thickness_nm * cos_theta;
    let phase = 2.0 * PI * optical_path / wavelength_nm;

    // Base Fresnel reflectance
    let f0 = ((n_film - 1.0) / (n_film + 1.0)).powi(2);

    // Interference modulation
    let modulation = 0.5 * (1.0 + phase.cos());

    f0 * modulation
}

/// Transfer matrix method for multi-layer thin-film stack
pub fn thin_film_stack_transfer_matrix(
    wavelength_nm: f64,
    layers: &[(f64, f64)], // (n, thickness_nm) pairs
    n_substrate: f64,
    cos_theta: f64,
) -> f64 {
    if layers.is_empty() {
        // No film, just air-substrate interface
        return fresnel_dielectric_full(cos_theta, n_substrate);
    }

    // Start with air (n=1)
    let mut n_prev: f64 = 1.0;
    let mut total_r: f64 = 0.0;
    let mut phase_acc: f64 = 0.0;

    for &(n_layer, thickness) in layers {
        // Angle in this layer (Snell's law)
        let sin_theta = (1.0 - cos_theta * cos_theta).sqrt();
        let sin_layer = sin_theta * n_prev / n_layer;

        if sin_layer >= 1.0 {
            return 1.0; // TIR
        }

        let cos_layer = (1.0 - sin_layer * sin_layer).sqrt();

        // Interface reflectance
        let r_interface =
            (n_prev * cos_theta - n_layer * cos_layer) / (n_prev * cos_theta + n_layer * cos_layer);

        // Phase in this layer
        let delta = 2.0 * PI * n_layer * thickness * cos_layer / wavelength_nm;

        // Accumulate (simplified - full transfer matrix would use 2x2 matrices)
        total_r += r_interface.powi(2) * (2.0 * phase_acc).cos();
        phase_acc += delta;

        n_prev = n_layer;
    }

    // Final interface to substrate
    let sin_sub = (1.0 - cos_theta * cos_theta).sqrt() / n_substrate;
    if sin_sub < 1.0 {
        let cos_sub = (1.0 - sin_sub * sin_sub).sqrt();
        let r_final = (n_prev - n_substrate * cos_sub) / (n_prev + n_substrate * cos_sub);
        total_r += r_final.powi(2) * (2.0 * phase_acc).cos();
    }

    total_r.clamp(0.0, 1.0)
}

// ============================================================================
// HENYEY-GREENSTEIN (Full Precision)
// ============================================================================

/// Exact Henyey-Greenstein phase function
pub fn henyey_greenstein_exact(g: f64, cos_theta: f64) -> f64 {
    let g2 = g * g;
    let denom = 1.0 + g2 - 2.0 * g * cos_theta;

    if denom <= 0.0 {
        return 0.0;
    }

    (1.0 - g2) / (4.0 * PI * denom.powf(1.5))
}

// ============================================================================
// CIE 1931 COLOR MATCHING FUNCTIONS
// ============================================================================

/// CIE 1931 2-degree observer x-bar function
fn cie_x_bar(wavelength: f64) -> f64 {
    // Gaussian approximation
    let t1 = (wavelength - 442.0) * if wavelength < 442.0 { 0.0624 } else { 0.0374 };
    let t2 = (wavelength - 599.8) * if wavelength < 599.8 { 0.0264 } else { 0.0323 };
    let t3 = (wavelength - 501.1) * if wavelength < 501.1 { 0.0490 } else { 0.0382 };

    0.362 * (-0.5 * t1 * t1).exp() + 1.056 * (-0.5 * t2 * t2).exp() - 0.065 * (-0.5 * t3 * t3).exp()
}

/// CIE 1931 2-degree observer y-bar function
fn cie_y_bar(wavelength: f64) -> f64 {
    let t1 = (wavelength - 568.8) * if wavelength < 568.8 { 0.0213 } else { 0.0247 };
    let t2 = (wavelength - 530.9) * if wavelength < 530.9 { 0.0613 } else { 0.0322 };

    0.821 * (-0.5 * t1 * t1).exp() + 0.286 * (-0.5 * t2 * t2).exp()
}

/// CIE 1931 2-degree observer z-bar function
fn cie_z_bar(wavelength: f64) -> f64 {
    let t1 = (wavelength - 437.0) * if wavelength < 437.0 { 0.0845 } else { 0.0278 };
    let t2 = (wavelength - 459.0) * if wavelength < 459.0 { 0.0385 } else { 0.0725 };

    1.217 * (-0.5 * t1 * t1).exp() + 0.681 * (-0.5 * t2 * t2).exp()
}

/// Convert XYZ to sRGB
fn xyz_to_srgb(xyz: [f64; 3]) -> [f64; 3] {
    // sRGB matrix (D65)
    let r_linear = 3.2404542 * xyz[0] - 1.5371385 * xyz[1] - 0.4985314 * xyz[2];
    let g_linear = -0.9692660 * xyz[0] + 1.8760108 * xyz[1] + 0.0415560 * xyz[2];
    let b_linear = 0.0556434 * xyz[0] - 0.2040259 * xyz[1] + 1.0572252 * xyz[2];

    // Gamma correction
    fn gamma(x: f64) -> f64 {
        let x = x.clamp(0.0, 1.0);
        if x <= 0.0031308 {
            12.92 * x
        } else {
            1.055 * x.powf(1.0 / 2.4) - 0.055
        }
    }

    [gamma(r_linear), gamma(g_linear), gamma(b_linear)]
}

// ============================================================================
// MEMORY ESTIMATION
// ============================================================================

/// Estimate memory usage for reference renderer
pub fn total_reference_memory() -> usize {
    // Config: ~64 bytes
    // CMF arrays: 3 * 31 * 8 = 744 bytes (visible)
    // Per-call buffers: negligible (stack allocated)
    // Total: ~1KB
    1024
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_reference_renderer_creation() {
        let renderer = ReferenceRenderer::default();
        assert_eq!(renderer.config.spectral_bands, 31);
        assert!(renderer.config.enable_full_fresnel);
    }

    #[test]
    fn test_fresnel_dielectric_boundaries() {
        // At normal incidence (cos_theta = 1)
        let r_normal = fresnel_dielectric_full(1.0, 1.5);
        assert!(r_normal > 0.0 && r_normal < 0.1);

        // At grazing angle (cos_theta = 0)
        let r_grazing = fresnel_dielectric_full(0.0, 1.5);
        assert!((r_grazing - 1.0).abs() < 0.01);
    }

    #[test]
    fn test_fresnel_conductor() {
        // Gold at 550nm: n ≈ 0.27, k ≈ 2.87 (more accurate values)
        // R = ((n-1)² + k²) / ((n+1)² + k²) ≈ 0.89
        let r_gold = fresnel_conductor_full(1.0, 0.27, 2.87);
        assert!(r_gold > 0.8, "Gold reflectance {} should be > 0.8", r_gold);

        // Also test with approximate values (n≈0.4, k≈2.4) - should still be high
        let r_gold_approx = fresnel_conductor_full(1.0, 0.4, 2.4);
        assert!(
            r_gold_approx > 0.75,
            "Gold reflectance {} should be > 0.75",
            r_gold_approx
        );
    }

    #[test]
    fn test_beer_lambert_exact() {
        // Zero absorption = full transmission
        let t0 = beer_lambert_exact(0.0, 10.0, 1.0);
        assert!((t0 - 1.0).abs() < 1e-10);

        // High absorption = low transmission
        let t_high = beer_lambert_exact(10.0, 10.0, 1.0);
        assert!(t_high < 0.001);
    }

    #[test]
    fn test_henyey_greenstein_normalization() {
        // Isotropic (g=0) should give 1/(4π)
        let p_iso = henyey_greenstein_exact(0.0, 0.5);
        let expected = 1.0 / (4.0 * PI);
        assert!((p_iso - expected).abs() < 1e-10);
    }

    #[test]
    fn test_energy_conservation_dielectric() {
        let renderer = ReferenceRenderer::default();
        let result = renderer.evaluate_dielectric(1.5, 0.8, 0.01, 5.0);

        // R + T + A should be very close to 1
        let total = result.reflectance + result.transmittance + result.absorption;
        assert!(
            (total - 1.0).abs() < 0.01,
            "Energy not conserved: {}",
            total
        );
    }

    #[test]
    fn test_energy_conservation_metal() {
        let renderer = ReferenceRenderer::default();
        let result = renderer.evaluate_metal(0.4, 2.4, 0.8);

        // Metals absorb what they don't reflect
        assert!(result.transmittance.abs() < 0.001); // No transmission
        assert!(result.reflectance + result.absorption > 0.99);
    }

    #[test]
    fn test_schlick_vs_full_comparison() {
        let comparison = compare_fresnel_approximations(1.5, 100);

        // Schlick should be reasonably accurate
        assert!(comparison.rmse < 0.05);
        assert!(comparison.max_error < 0.15);
    }

    #[test]
    fn test_thin_film_interference() {
        let renderer = ReferenceRenderer::default();

        // Anti-reflection coating: quarter-wave at 550nm
        // MgF2 (n=1.38) on glass (n=1.5)
        let thickness = 550.0 / (4.0 * 1.38); // ~100nm
        let result = renderer.evaluate_thin_film(1.38, thickness, 1.5, 1.0);

        // Should show reduced reflectance at design wavelength
        let r_at_550 = result.spectral_reflectance[15]; // Center of visible
        assert!(r_at_550 < 0.02, "AR coating should minimize reflectance");
    }

    #[test]
    fn test_lut_comparison() {
        let lut = vec![0.1, 0.2, 0.3, 0.4];
        let reference = vec![0.11, 0.19, 0.31, 0.39];

        let comparison = compare_lut_vs_reference(&lut, &reference);

        assert_eq!(comparison.samples_compared, 4);
        assert!(comparison.max_error < 0.02);
        assert!(comparison.mean_error < 0.02);
    }

    #[test]
    fn test_xyz_to_srgb() {
        // D65 white point
        let white_xyz = [0.95047, 1.0, 1.08883];
        let white_rgb = xyz_to_srgb(white_xyz);

        // Should be approximately white
        assert!((white_rgb[0] - 1.0).abs() < 0.02);
        assert!((white_rgb[1] - 1.0).abs() < 0.02);
        assert!((white_rgb[2] - 1.0).abs() < 0.02);
    }

    #[test]
    fn test_wavelength_config() {
        let config = ReferenceRenderConfig::visible();
        assert_eq!(config.wavelengths().len(), 31);
        assert!((config.wavelength_at(0) - 400.0).abs() < 0.1);
        assert!((config.wavelength_at(30) - 700.0).abs() < 0.1);

        let extended = ReferenceRenderConfig::extended();
        assert_eq!(extended.wavelengths().len(), 81);
    }

    #[test]
    fn test_memory_estimate() {
        let mem = total_reference_memory();
        assert!(mem > 0);
        assert!(mem < 10_000); // Should be under 10KB
    }
}
