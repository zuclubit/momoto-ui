//! # Spectral Error Metrics Module
//!
//! Comprehensive error metrics for spectral and perceptual comparison.
//!
//! ## Features
//!
//! - **Spectral Metrics**: RMSE, MAE, spectral angle mapper
//! - **Perceptual Metrics**: Delta E variants (76, 94, 2000)
//! - **Energy Metrics**: Conservation error, reciprocity violation
//! - **Comprehensive Analysis**: Combined metrics with weighting
//!
//! ## Usage
//!
//! ```rust
//! use momoto_materials::glass_physics::spectral_error::{
//!     compute_spectral_metrics, compute_energy_metrics, SpectralErrorMetrics
//! };
//!
//! let measured = vec![0.1, 0.2, 0.3, 0.4];
//! let rendered = vec![0.11, 0.19, 0.31, 0.39];
//! let wavelengths = vec![400.0, 500.0, 600.0, 700.0];
//!
//! let metrics = compute_spectral_metrics(&measured, &rendered, &wavelengths);
//! println!("RMSE: {:.4}", metrics.rmse);
//! println!("Spectral Angle: {:.4} rad", metrics.spectral_angle);
//! ```

// ============================================================================
// SPECTRAL ERROR METRICS
// ============================================================================

/// Comprehensive spectral error metrics
#[derive(Debug, Clone)]
pub struct SpectralErrorMetrics {
    /// Root Mean Square Error
    pub rmse: f64,
    /// Mean Absolute Error
    pub mae: f64,
    /// Maximum absolute error
    pub max_error: f64,
    /// Spectral Angle Mapper (radians) - angle between spectral vectors
    pub spectral_angle: f64,
    /// Luminance-weighted RMSE (emphasizes visible sensitivity)
    pub weighted_rmse: f64,
    /// Per-band errors
    pub band_errors: Vec<f64>,
    /// Index of band with maximum error
    pub worst_band_index: usize,
    /// Number of bands compared
    pub band_count: usize,
}

impl SpectralErrorMetrics {
    /// Create default/empty metrics
    pub fn empty() -> Self {
        Self {
            rmse: 0.0,
            mae: 0.0,
            max_error: 0.0,
            spectral_angle: 0.0,
            weighted_rmse: 0.0,
            band_errors: Vec::new(),
            worst_band_index: 0,
            band_count: 0,
        }
    }

    /// Check if error is within tolerance
    pub fn within_tolerance(&self, rmse_tol: f64, max_error_tol: f64) -> bool {
        self.rmse <= rmse_tol && self.max_error <= max_error_tol
    }

    /// Get quality grade based on RMSE
    pub fn quality_grade(&self) -> SpectralQualityGrade {
        if self.rmse < 0.001 {
            SpectralQualityGrade::Excellent
        } else if self.rmse < 0.005 {
            SpectralQualityGrade::Good
        } else if self.rmse < 0.02 {
            SpectralQualityGrade::Acceptable
        } else if self.rmse < 0.05 {
            SpectralQualityGrade::Marginal
        } else {
            SpectralQualityGrade::Poor
        }
    }
}

/// Quality grade based on spectral error
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpectralQualityGrade {
    /// RMSE < 0.001 - Reference quality
    Excellent,
    /// RMSE < 0.005 - High quality
    Good,
    /// RMSE < 0.02 - Acceptable for most applications
    Acceptable,
    /// RMSE < 0.05 - Noticeable but usable
    Marginal,
    /// RMSE >= 0.05 - Significant deviation
    Poor,
}

// ============================================================================
// PERCEPTUAL ERROR METRICS
// ============================================================================

/// Perceptual error metrics using color difference formulas
#[derive(Debug, Clone)]
pub struct PerceptualErrorMetrics {
    /// CIE 1976 Delta E (simple Euclidean in Lab)
    pub delta_e_76: f64,
    /// CIE 1994 Delta E (graphic arts weights)
    pub delta_e_94: f64,
    /// CIEDE2000 Delta E (most accurate)
    pub delta_e_2000: f64,
    /// Just Noticeable Difference (JND) multiples (Delta E / 2.3)
    pub jnd_multiples: f64,
}

impl PerceptualErrorMetrics {
    /// Create default/zero metrics
    pub fn zero() -> Self {
        Self {
            delta_e_76: 0.0,
            delta_e_94: 0.0,
            delta_e_2000: 0.0,
            jnd_multiples: 0.0,
        }
    }

    /// Check if perceptually acceptable (Delta E 2000 < threshold)
    pub fn perceptually_acceptable(&self, threshold: f64) -> bool {
        self.delta_e_2000 <= threshold
    }

    /// Check if imperceptible (below JND)
    pub fn imperceptible(&self) -> bool {
        self.delta_e_2000 < 1.0
    }

    /// Get perceptual quality grade
    pub fn quality_grade(&self) -> PerceptualQualityGrade {
        if self.delta_e_2000 < 1.0 {
            PerceptualQualityGrade::Imperceptible
        } else if self.delta_e_2000 < 2.0 {
            PerceptualQualityGrade::Subtle
        } else if self.delta_e_2000 < 3.5 {
            PerceptualQualityGrade::Noticeable
        } else if self.delta_e_2000 < 5.0 {
            PerceptualQualityGrade::Significant
        } else {
            PerceptualQualityGrade::Obvious
        }
    }
}

/// Perceptual quality grade
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PerceptualQualityGrade {
    /// Delta E < 1.0 - Not perceivable by human eye
    Imperceptible,
    /// Delta E < 2.0 - Close observation needed
    Subtle,
    /// Delta E < 3.5 - Noticeable at a glance
    Noticeable,
    /// Delta E < 5.0 - Significant difference
    Significant,
    /// Delta E >= 5.0 - Obvious color difference
    Obvious,
}

// ============================================================================
// ENERGY METRICS
// ============================================================================

/// Energy conservation and physical consistency metrics
#[derive(Debug, Clone)]
pub struct EnergyMetrics {
    /// Total integrated reflectance
    pub total_reflectance: f64,
    /// Total integrated transmittance
    pub total_transmittance: f64,
    /// Total absorption (inferred or measured)
    pub total_absorption: f64,
    /// Energy conservation error: |1 - (R + T + A)|
    pub conservation_error: f64,
    /// Helmholtz reciprocity violation: |f(i→o) - f(o→i)|
    pub reciprocity_violation: f64,
    /// Passivity violation (negative values)
    pub passivity_violation: f64,
    /// Physical consistency score (0-1)
    pub physical_consistency: f64,
}

impl EnergyMetrics {
    /// Create default/ideal metrics
    pub fn ideal() -> Self {
        Self {
            total_reflectance: 0.0,
            total_transmittance: 0.0,
            total_absorption: 0.0,
            conservation_error: 0.0,
            reciprocity_violation: 0.0,
            passivity_violation: 0.0,
            physical_consistency: 1.0,
        }
    }

    /// Check if physically plausible
    pub fn physically_plausible(&self, tolerance: f64) -> bool {
        self.conservation_error < tolerance
            && self.passivity_violation < tolerance
            && self.total_reflectance >= 0.0
            && self.total_reflectance <= 1.0
            && self.total_transmittance >= 0.0
            && self.total_transmittance <= 1.0
    }
}

// ============================================================================
// COMPREHENSIVE METRICS
// ============================================================================

/// Combined metrics for full material validation
#[derive(Debug, Clone)]
pub struct ComprehensiveMetrics {
    /// Spectral error metrics
    pub spectral: SpectralErrorMetrics,
    /// Perceptual error metrics
    pub perceptual: PerceptualErrorMetrics,
    /// Energy conservation metrics
    pub energy: EnergyMetrics,
    /// Overall quality score (0-100)
    pub overall_score: f64,
    /// Timestamp (nanoseconds since epoch)
    pub timestamp_ns: u64,
}

impl ComprehensiveMetrics {
    /// Compute overall quality score
    pub fn compute_overall_score(&self) -> f64 {
        // Weight: spectral 30%, perceptual 50%, energy 20%
        let spectral_score = 100.0 * (1.0 - self.spectral.rmse.min(0.1) / 0.1);
        let perceptual_score = 100.0 * (1.0 - self.perceptual.delta_e_2000.min(10.0) / 10.0);
        let energy_score = 100.0 * self.energy.physical_consistency;

        0.30 * spectral_score + 0.50 * perceptual_score + 0.20 * energy_score
    }

    /// Get validation status
    pub fn validation_status(&self) -> ValidationStatus {
        if self.overall_score >= 95.0 {
            ValidationStatus::Pass
        } else if self.overall_score >= 80.0 {
            ValidationStatus::PassWithWarnings
        } else if self.overall_score >= 60.0 {
            ValidationStatus::Marginal
        } else {
            ValidationStatus::Fail
        }
    }
}

/// Validation status
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ValidationStatus {
    /// Score >= 95 - Excellent match
    Pass,
    /// Score >= 80 - Acceptable with minor issues
    PassWithWarnings,
    /// Score >= 60 - Borderline acceptable
    Marginal,
    /// Score < 60 - Significant issues
    Fail,
}

// ============================================================================
// METRIC COMPUTATION FUNCTIONS
// ============================================================================

/// Compute spectral error metrics between measured and rendered spectra
pub fn compute_spectral_metrics(
    measured: &[f64],
    rendered: &[f64],
    wavelengths: &[f64],
) -> SpectralErrorMetrics {
    if measured.len() != rendered.len() || measured.is_empty() {
        return SpectralErrorMetrics::empty();
    }

    let n = measured.len();
    let mut band_errors = Vec::with_capacity(n);
    let mut sum_sq = 0.0;
    let mut sum_abs = 0.0;
    let mut max_error = 0.0;
    let mut worst_band = 0;

    // Per-band errors
    for i in 0..n {
        let error = (rendered[i] - measured[i]).abs();
        band_errors.push(error);
        sum_sq += error * error;
        sum_abs += error;

        if error > max_error {
            max_error = error;
            worst_band = i;
        }
    }

    let rmse = (sum_sq / n as f64).sqrt();
    let mae = sum_abs / n as f64;

    // Spectral Angle Mapper (SAM)
    let spectral_angle = compute_spectral_angle(measured, rendered);

    // Luminance-weighted RMSE
    let weighted_rmse = compute_weighted_rmse(measured, rendered, wavelengths);

    SpectralErrorMetrics {
        rmse,
        mae,
        max_error,
        spectral_angle,
        weighted_rmse,
        band_errors,
        worst_band_index: worst_band,
        band_count: n,
    }
}

/// Compute Spectral Angle Mapper (SAM) between two spectra
pub fn compute_spectral_angle(a: &[f64], b: &[f64]) -> f64 {
    if a.len() != b.len() || a.is_empty() {
        return 0.0;
    }

    let mut dot = 0.0;
    let mut norm_a = 0.0;
    let mut norm_b = 0.0;

    for i in 0..a.len() {
        dot += a[i] * b[i];
        norm_a += a[i] * a[i];
        norm_b += b[i] * b[i];
    }

    let denom = (norm_a * norm_b).sqrt();
    if denom < 1e-10 {
        return 0.0;
    }

    // Clamp to [-1, 1] for numerical stability
    let cos_angle = (dot / denom).clamp(-1.0, 1.0);
    cos_angle.acos()
}

/// Compute luminance-weighted RMSE (CIE Y weighting)
pub fn compute_weighted_rmse(measured: &[f64], rendered: &[f64], wavelengths: &[f64]) -> f64 {
    if measured.len() != rendered.len() || wavelengths.len() != measured.len() {
        return 0.0;
    }

    let n = measured.len();
    let mut sum_weighted_sq = 0.0;
    let mut sum_weights = 0.0;

    for i in 0..n {
        let weight = cie_y_weight(wavelengths[i]);
        let error = rendered[i] - measured[i];
        sum_weighted_sq += weight * error * error;
        sum_weights += weight;
    }

    if sum_weights > 0.0 {
        (sum_weighted_sq / sum_weights).sqrt()
    } else {
        0.0
    }
}

/// CIE Y (luminance) weighting function
fn cie_y_weight(wavelength: f64) -> f64 {
    // Gaussian approximation of CIE Y bar
    let t1 = (wavelength - 568.8) * if wavelength < 568.8 { 0.0213 } else { 0.0247 };
    let t2 = (wavelength - 530.9) * if wavelength < 530.9 { 0.0613 } else { 0.0322 };

    0.821 * (-0.5 * t1 * t1).exp() + 0.286 * (-0.5 * t2 * t2).exp()
}

/// Compute perceptual error metrics between two Lab colors
pub fn compute_perceptual_metrics(
    lab_measured: [f64; 3],
    lab_rendered: [f64; 3],
) -> PerceptualErrorMetrics {
    let delta_e_76 = delta_e_76(lab_measured, lab_rendered);
    let delta_e_94 = delta_e_94(lab_measured, lab_rendered);
    let delta_e_2000 = delta_e_2000(lab_measured, lab_rendered);

    // JND (Just Noticeable Difference) is approximately 2.3 Delta E units
    let jnd_multiples = delta_e_2000 / 2.3;

    PerceptualErrorMetrics {
        delta_e_76,
        delta_e_94,
        delta_e_2000,
        jnd_multiples,
    }
}

/// Compute energy conservation metrics
pub fn compute_energy_metrics(
    reflectance: f64,
    transmittance: f64,
    absorption: f64,
) -> EnergyMetrics {
    let total_reflectance = reflectance.clamp(0.0, 1.0);
    let total_transmittance = transmittance.clamp(0.0, 1.0);
    let total_absorption = absorption.clamp(0.0, 1.0);

    // Energy conservation: R + T + A = 1
    let total = total_reflectance + total_transmittance + total_absorption;
    let conservation_error = (total - 1.0).abs();

    // Passivity: all values must be in [0, 1]
    let passivity_violation = {
        let mut violation = 0.0;
        if reflectance < 0.0 {
            violation += reflectance.abs();
        }
        if reflectance > 1.0 {
            violation += reflectance - 1.0;
        }
        if transmittance < 0.0 {
            violation += transmittance.abs();
        }
        if transmittance > 1.0 {
            violation += transmittance - 1.0;
        }
        violation
    };

    // Physical consistency score
    let physical_consistency = {
        let energy_score = 1.0 - conservation_error.min(1.0);
        let passivity_score = 1.0 - passivity_violation.min(1.0);
        (energy_score + passivity_score) / 2.0
    };

    EnergyMetrics {
        total_reflectance,
        total_transmittance,
        total_absorption,
        conservation_error,
        reciprocity_violation: 0.0, // Requires BRDF measurement
        passivity_violation,
        physical_consistency,
    }
}

/// Compute energy metrics with reciprocity check
pub fn compute_energy_metrics_with_reciprocity(
    reflectance: f64,
    transmittance: f64,
    absorption: f64,
    brdf_forward: f64,
    brdf_reverse: f64,
) -> EnergyMetrics {
    let mut metrics = compute_energy_metrics(reflectance, transmittance, absorption);
    metrics.reciprocity_violation = (brdf_forward - brdf_reverse).abs();

    // Update physical consistency to include reciprocity
    let reciprocity_score = 1.0 - metrics.reciprocity_violation.min(1.0);
    metrics.physical_consistency = (metrics.physical_consistency * 2.0 + reciprocity_score) / 3.0;

    metrics
}

/// Compute comprehensive metrics combining all error types
pub fn compute_comprehensive(
    measured_spectral: &[f64],
    rendered_spectral: &[f64],
    wavelengths: &[f64],
    lab_measured: [f64; 3],
    lab_rendered: [f64; 3],
    reflectance: f64,
    transmittance: f64,
    absorption: f64,
) -> ComprehensiveMetrics {
    let spectral = compute_spectral_metrics(measured_spectral, rendered_spectral, wavelengths);
    let perceptual = compute_perceptual_metrics(lab_measured, lab_rendered);
    let energy = compute_energy_metrics(reflectance, transmittance, absorption);

    let timestamp_ns = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos() as u64)
        .unwrap_or(0);

    let mut metrics = ComprehensiveMetrics {
        spectral,
        perceptual,
        energy,
        overall_score: 0.0,
        timestamp_ns,
    };

    metrics.overall_score = metrics.compute_overall_score();
    metrics
}

// ============================================================================
// DELTA E FORMULAS
// ============================================================================

/// CIE 1976 Delta E (simple Euclidean distance in Lab)
pub fn delta_e_76(lab1: [f64; 3], lab2: [f64; 3]) -> f64 {
    let dl = lab2[0] - lab1[0];
    let da = lab2[1] - lab1[1];
    let db = lab2[2] - lab1[2];

    (dl * dl + da * da + db * db).sqrt()
}

/// CIE 1994 Delta E (graphic arts weights)
pub fn delta_e_94(lab1: [f64; 3], lab2: [f64; 3]) -> f64 {
    let dl = lab2[0] - lab1[0];
    let da = lab2[1] - lab1[1];
    let db = lab2[2] - lab1[2];

    let c1 = (lab1[1] * lab1[1] + lab1[2] * lab1[2]).sqrt();
    let c2 = (lab2[1] * lab2[1] + lab2[2] * lab2[2]).sqrt();
    let dc = c2 - c1;

    let dh_sq = da * da + db * db - dc * dc;
    let dh = if dh_sq > 0.0 { dh_sq.sqrt() } else { 0.0 };

    // Graphic arts weights (kL=1, kC=1, kH=1, K1=0.045, K2=0.015)
    let sl = 1.0;
    let sc = 1.0 + 0.045 * c1;
    let sh = 1.0 + 0.015 * c1;

    let term_l = dl / sl;
    let term_c = dc / sc;
    let term_h = dh / sh;

    (term_l * term_l + term_c * term_c + term_h * term_h).sqrt()
}

/// CIEDE2000 Delta E (most perceptually uniform)
pub fn delta_e_2000(lab1: [f64; 3], lab2: [f64; 3]) -> f64 {
    let l1 = lab1[0];
    let a1 = lab1[1];
    let b1 = lab1[2];
    let l2 = lab2[0];
    let a2 = lab2[1];
    let b2 = lab2[2];

    // Parametric factors (default values)
    let k_l = 1.0;
    let k_c = 1.0;
    let k_h = 1.0;

    let c1 = (a1 * a1 + b1 * b1).sqrt();
    let c2 = (a2 * a2 + b2 * b2).sqrt();
    let c_bar = (c1 + c2) / 2.0;

    let c_bar_7 = c_bar.powi(7);
    let g = 0.5 * (1.0 - (c_bar_7 / (c_bar_7 + 6103515625.0)).sqrt());

    let a1_prime = a1 * (1.0 + g);
    let a2_prime = a2 * (1.0 + g);

    let c1_prime = (a1_prime * a1_prime + b1 * b1).sqrt();
    let c2_prime = (a2_prime * a2_prime + b2 * b2).sqrt();

    let h1_prime = if a1_prime.abs() < 1e-10 && b1.abs() < 1e-10 {
        0.0
    } else {
        let h = b1.atan2(a1_prime).to_degrees();
        if h < 0.0 {
            h + 360.0
        } else {
            h
        }
    };

    let h2_prime = if a2_prime.abs() < 1e-10 && b2.abs() < 1e-10 {
        0.0
    } else {
        let h = b2.atan2(a2_prime).to_degrees();
        if h < 0.0 {
            h + 360.0
        } else {
            h
        }
    };

    let dl_prime = l2 - l1;
    let dc_prime = c2_prime - c1_prime;

    let dh_prime = if c1_prime * c2_prime < 1e-10 {
        0.0
    } else {
        let mut dh = h2_prime - h1_prime;
        if dh > 180.0 {
            dh -= 360.0;
        } else if dh < -180.0 {
            dh += 360.0;
        }
        dh
    };

    let dh_prime_capital = 2.0 * (c1_prime * c2_prime).sqrt() * (dh_prime.to_radians() / 2.0).sin();

    let l_bar_prime = (l1 + l2) / 2.0;
    let c_bar_prime = (c1_prime + c2_prime) / 2.0;

    let h_bar_prime = if c1_prime * c2_prime < 1e-10 {
        h1_prime + h2_prime
    } else if (h1_prime - h2_prime).abs() <= 180.0 {
        (h1_prime + h2_prime) / 2.0
    } else if h1_prime + h2_prime < 360.0 {
        (h1_prime + h2_prime + 360.0) / 2.0
    } else {
        (h1_prime + h2_prime - 360.0) / 2.0
    };

    let t = 1.0 - 0.17 * (h_bar_prime - 30.0).to_radians().cos()
        + 0.24 * (2.0 * h_bar_prime).to_radians().cos()
        + 0.32 * (3.0 * h_bar_prime + 6.0).to_radians().cos()
        - 0.20 * (4.0 * h_bar_prime - 63.0).to_radians().cos();

    let sl =
        1.0 + (0.015 * (l_bar_prime - 50.0).powi(2)) / (20.0 + (l_bar_prime - 50.0).powi(2)).sqrt();
    let sc = 1.0 + 0.045 * c_bar_prime;
    let sh = 1.0 + 0.015 * c_bar_prime * t;

    let c_bar_prime_7 = c_bar_prime.powi(7);
    let rc = 2.0 * (c_bar_prime_7 / (c_bar_prime_7 + 6103515625.0)).sqrt();

    let dt = 30.0 * (-((h_bar_prime - 275.0) / 25.0).powi(2)).exp();
    let rt = -rc * (2.0 * dt.to_radians()).sin();

    let term_l = dl_prime / (k_l * sl);
    let term_c = dc_prime / (k_c * sc);
    let term_h = dh_prime_capital / (k_h * sh);

    (term_l * term_l + term_c * term_c + term_h * term_h + rt * term_c * term_h).sqrt()
}

// ============================================================================
// UTILITY FUNCTIONS
// ============================================================================

/// Compute GFC (Goodness of Fit Coefficient) between two spectra
pub fn goodness_of_fit(measured: &[f64], rendered: &[f64]) -> f64 {
    if measured.len() != rendered.len() || measured.is_empty() {
        return 0.0;
    }

    let mut sum_mr = 0.0;
    let mut sum_m2 = 0.0;
    let mut sum_r2 = 0.0;

    for i in 0..measured.len() {
        sum_mr += measured[i] * rendered[i];
        sum_m2 += measured[i] * measured[i];
        sum_r2 += rendered[i] * rendered[i];
    }

    let denom = (sum_m2 * sum_r2).sqrt();
    if denom < 1e-10 {
        return 0.0;
    }

    (sum_mr / denom).abs()
}

/// Compute R² (coefficient of determination) between measured and rendered
pub fn r_squared(measured: &[f64], rendered: &[f64]) -> f64 {
    if measured.len() != rendered.len() || measured.is_empty() {
        return 0.0;
    }

    let n = measured.len() as f64;
    let mean_m: f64 = measured.iter().sum::<f64>() / n;

    let mut ss_res = 0.0; // Residual sum of squares
    let mut ss_tot = 0.0; // Total sum of squares

    for i in 0..measured.len() {
        let residual = measured[i] - rendered[i];
        let deviation = measured[i] - mean_m;
        ss_res += residual * residual;
        ss_tot += deviation * deviation;
    }

    if ss_tot < 1e-10 {
        return 1.0; // Perfect fit if no variation
    }

    1.0 - (ss_res / ss_tot)
}

/// Compute relative error (percentage)
pub fn relative_error(measured: f64, rendered: f64) -> f64 {
    if measured.abs() < 1e-10 {
        return rendered.abs();
    }
    ((rendered - measured) / measured).abs() * 100.0
}

/// Compute mean relative error across spectra
pub fn mean_relative_error(measured: &[f64], rendered: &[f64]) -> f64 {
    if measured.len() != rendered.len() || measured.is_empty() {
        return 0.0;
    }

    let mut sum = 0.0;
    let mut count = 0;

    for i in 0..measured.len() {
        if measured[i].abs() > 1e-10 {
            sum += ((rendered[i] - measured[i]) / measured[i]).abs();
            count += 1;
        }
    }

    if count > 0 {
        (sum / count as f64) * 100.0
    } else {
        0.0
    }
}

// ============================================================================
// MEMORY ESTIMATION
// ============================================================================

/// Estimate memory usage for error metrics computation
pub fn total_error_memory() -> usize {
    // SpectralErrorMetrics: ~200 bytes (with band_errors vec)
    // PerceptualErrorMetrics: ~32 bytes
    // EnergyMetrics: ~64 bytes
    // ComprehensiveMetrics: ~300 bytes
    // Total working set: ~1KB
    1024
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use std::f64::consts::PI;

    #[test]
    fn test_spectral_metrics_identical() {
        let spectrum = vec![0.1, 0.2, 0.3, 0.4, 0.5];
        let wavelengths = vec![400.0, 475.0, 550.0, 625.0, 700.0];

        let metrics = compute_spectral_metrics(&spectrum, &spectrum, &wavelengths);

        assert!(metrics.rmse < 1e-10);
        assert!(metrics.mae < 1e-10);
        assert!(metrics.max_error < 1e-10);
        assert!(metrics.spectral_angle < 1e-10);
    }

    #[test]
    fn test_spectral_metrics_different() {
        let measured = vec![0.1, 0.2, 0.3, 0.4, 0.5];
        let rendered = vec![0.12, 0.18, 0.32, 0.38, 0.52];
        let wavelengths = vec![400.0, 475.0, 550.0, 625.0, 700.0];

        let metrics = compute_spectral_metrics(&measured, &rendered, &wavelengths);

        assert!(metrics.rmse > 0.0);
        assert!(metrics.rmse < 0.05);
        assert!(metrics.mae > 0.0);
    }

    #[test]
    fn test_spectral_angle() {
        // Identical spectra should have angle 0
        let a = vec![1.0, 2.0, 3.0];
        let angle_same = compute_spectral_angle(&a, &a);
        assert!(angle_same < 1e-10);

        // Orthogonal spectra should have angle π/2
        let b = vec![1.0, 0.0, 0.0];
        let c = vec![0.0, 1.0, 0.0];
        let angle_ortho = compute_spectral_angle(&b, &c);
        assert!((angle_ortho - PI / 2.0).abs() < 0.01);
    }

    #[test]
    fn test_delta_e_76() {
        // Same color
        let lab = [50.0, 0.0, 0.0];
        assert!(delta_e_76(lab, lab) < 1e-10);

        // Different colors
        let lab1 = [50.0, 0.0, 0.0];
        let lab2 = [53.0, 4.0, 0.0];
        let de = delta_e_76(lab1, lab2);
        assert!(de > 4.0 && de < 6.0);
    }

    #[test]
    fn test_delta_e_2000() {
        // Same color
        let lab = [50.0, 0.0, 0.0];
        assert!(delta_e_2000(lab, lab) < 1e-10);

        // Known test case (from CIEDE2000 paper)
        let lab1 = [50.0, 2.6772, -79.7751];
        let lab2 = [50.0, 0.0, -82.7485];
        let de = delta_e_2000(lab1, lab2);
        assert!(de > 2.0 && de < 3.0);
    }

    #[test]
    fn test_energy_conservation() {
        // Perfect conservation
        let metrics = compute_energy_metrics(0.3, 0.5, 0.2);
        assert!(metrics.conservation_error < 1e-10);
        assert!(metrics.physical_consistency > 0.99);

        // Violation
        let metrics_bad = compute_energy_metrics(0.5, 0.5, 0.5);
        assert!(metrics_bad.conservation_error > 0.4);
    }

    #[test]
    fn test_comprehensive_metrics() {
        let measured = vec![0.1, 0.2, 0.3];
        let rendered = vec![0.11, 0.19, 0.31];
        let wavelengths = vec![400.0, 550.0, 700.0];
        let lab1 = [50.0, 0.0, 0.0];
        let lab2 = [51.0, 1.0, 0.0];

        let metrics = compute_comprehensive(
            &measured,
            &rendered,
            &wavelengths,
            lab1,
            lab2,
            0.3,
            0.5,
            0.2,
        );

        assert!(metrics.overall_score > 0.0);
        assert!(metrics.overall_score <= 100.0);
        assert!(metrics.spectral.rmse < 0.05);
        assert!(metrics.perceptual.delta_e_2000 < 5.0);
    }

    #[test]
    fn test_goodness_of_fit() {
        let a = vec![1.0, 2.0, 3.0];

        // Perfect fit
        let gfc_same = goodness_of_fit(&a, &a);
        assert!((gfc_same - 1.0).abs() < 1e-10);

        // Scaled (still perfect correlation)
        let scaled = vec![2.0, 4.0, 6.0];
        let gfc_scaled = goodness_of_fit(&a, &scaled);
        assert!((gfc_scaled - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_r_squared() {
        let measured = vec![1.0, 2.0, 3.0, 4.0, 5.0];

        // Perfect prediction
        let r2_same = r_squared(&measured, &measured);
        assert!((r2_same - 1.0).abs() < 1e-10);

        // Poor prediction
        let poor = vec![3.0, 3.0, 3.0, 3.0, 3.0];
        let r2_poor = r_squared(&measured, &poor);
        assert!(r2_poor < 0.5);
    }

    #[test]
    fn test_quality_grades() {
        let good = SpectralErrorMetrics {
            rmse: 0.003,
            ..SpectralErrorMetrics::empty()
        };
        assert_eq!(good.quality_grade(), SpectralQualityGrade::Good);

        let perceptual = PerceptualErrorMetrics {
            delta_e_2000: 0.5,
            ..PerceptualErrorMetrics::zero()
        };
        assert_eq!(
            perceptual.quality_grade(),
            PerceptualQualityGrade::Imperceptible
        );
    }

    #[test]
    fn test_memory_estimate() {
        let mem = total_error_memory();
        assert!(mem > 0);
        assert!(mem < 10_000);
    }
}
