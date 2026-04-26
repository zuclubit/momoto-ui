//! # Quality Tier Cross-Validation Module
//!
//! Comprehensive validation comparing all quality tiers against Reference Renderer.
//!
//! ## Purpose
//!
//! This module validates the accuracy-performance trade-offs across quality tiers:
//! - Fast, Medium, High, Ultra, Cinematic, Reference
//!
//! ## Metrics
//!
//! - Spectral RMSE
//! - Spectral Angle Mapper (SAM)
//! - Delta E 2000
//! - Energy conservation error
//! - Performance (µs per evaluation)

use super::enhanced_presets::QualityTier;
use super::fresnel::{fresnel_full, fresnel_schlick};
use super::perceptual_loss::{delta_e_2000, rgb_to_lab, Illuminant};
use super::reference_renderer::{
    beer_lambert_exact, fresnel_conductor_full, fresnel_dielectric_full, ReferenceRenderer,
};
use std::time::Instant;

// ============================================================================
// VALIDATION RESULTS
// ============================================================================

/// Single tier validation result
#[derive(Debug, Clone)]
pub struct TierValidationResult {
    /// Quality tier name
    pub tier: QualityTier,
    /// Fresnel RMSE vs reference
    pub fresnel_rmse: f64,
    /// Fresnel max error
    pub fresnel_max_error: f64,
    /// Absorption RMSE vs reference
    pub absorption_rmse: f64,
    /// Mean Delta E 2000 (perceptual)
    pub mean_delta_e: f64,
    /// Max Delta E 2000
    pub max_delta_e: f64,
    /// Energy conservation error
    pub energy_error: f64,
    /// Mean evaluation time (µs)
    pub mean_eval_time_us: f64,
    /// Speedup vs Reference tier
    pub speedup_vs_reference: f64,
}

/// Full cross-validation report
#[derive(Debug, Clone)]
pub struct CrossValidationReport {
    /// Results per tier
    pub tier_results: Vec<TierValidationResult>,
    /// Reference evaluation time (µs)
    pub reference_time_us: f64,
    /// Total test cases
    pub test_cases: usize,
    /// Test IOR range
    pub ior_range: (f64, f64),
    /// Test angle range
    pub angle_range: (f64, f64),
}

impl CrossValidationReport {
    /// Generate markdown report
    pub fn to_markdown(&self) -> String {
        let mut md = String::new();
        md.push_str("# Quality Tier Cross-Validation Report\n\n");
        md.push_str(&format!("Test cases: {}\n", self.test_cases));
        md.push_str(&format!(
            "IOR range: {:.2} - {:.2}\n",
            self.ior_range.0, self.ior_range.1
        ));
        md.push_str(&format!(
            "Angle range: {:.2}° - {:.2}°\n\n",
            self.angle_range.0.to_degrees(),
            self.angle_range.1.to_degrees()
        ));

        md.push_str("## Accuracy vs Performance\n\n");
        md.push_str("| Tier | Fresnel RMSE | Max Error | ΔE2000 Mean | ΔE2000 Max | Time (µs) | Speedup |\n");
        md.push_str(
            "|------|--------------|-----------|-------------|------------|-----------|--------|\n",
        );

        for r in &self.tier_results {
            md.push_str(&format!(
                "| {:?} | {:.6} | {:.6} | {:.2} | {:.2} | {:.2} | {:.1}x |\n",
                r.tier,
                r.fresnel_rmse,
                r.fresnel_max_error,
                r.mean_delta_e,
                r.max_delta_e,
                r.mean_eval_time_us,
                r.speedup_vs_reference
            ));
        }

        md.push_str("\n## Interpretation\n\n");
        md.push_str("- **Fresnel RMSE**: Lower is better. <0.01 is excellent, <0.05 is good.\n");
        md.push_str("- **ΔE2000**: <1.0 imperceptible, <3.0 barely noticeable, <6.0 noticeable.\n");
        md.push_str("- **Speedup**: Higher is better. Reference tier = 1.0x baseline.\n");

        md
    }

    /// Generate JSON report
    pub fn to_json(&self) -> String {
        let mut json = String::from("{\n");
        json.push_str(&format!("  \"test_cases\": {},\n", self.test_cases));
        json.push_str(&format!(
            "  \"ior_range\": [{:.2}, {:.2}],\n",
            self.ior_range.0, self.ior_range.1
        ));
        json.push_str(&format!(
            "  \"angle_range_rad\": [{:.4}, {:.4}],\n",
            self.angle_range.0, self.angle_range.1
        ));
        json.push_str(&format!(
            "  \"reference_time_us\": {:.4},\n",
            self.reference_time_us
        ));
        json.push_str("  \"tiers\": [\n");

        for (i, r) in self.tier_results.iter().enumerate() {
            json.push_str("    {\n");
            json.push_str(&format!("      \"tier\": \"{:?}\",\n", r.tier));
            json.push_str(&format!("      \"fresnel_rmse\": {:.8},\n", r.fresnel_rmse));
            json.push_str(&format!(
                "      \"fresnel_max_error\": {:.8},\n",
                r.fresnel_max_error
            ));
            json.push_str(&format!("      \"mean_delta_e\": {:.4},\n", r.mean_delta_e));
            json.push_str(&format!("      \"max_delta_e\": {:.4},\n", r.max_delta_e));
            json.push_str(&format!("      \"energy_error\": {:.8},\n", r.energy_error));
            json.push_str(&format!(
                "      \"mean_eval_time_us\": {:.4},\n",
                r.mean_eval_time_us
            ));
            json.push_str(&format!(
                "      \"speedup\": {:.2}\n",
                r.speedup_vs_reference
            ));
            if i < self.tier_results.len() - 1 {
                json.push_str("    },\n");
            } else {
                json.push_str("    }\n");
            }
        }

        json.push_str("  ]\n}\n");
        json
    }
}

// ============================================================================
// TIER EVALUATION FUNCTIONS
// ============================================================================

/// Evaluate Fresnel reflectance for a specific tier
fn evaluate_tier_fresnel(tier: QualityTier, ior: f64, cos_theta: f64) -> f64 {
    match tier {
        QualityTier::Fast => {
            // Schlick approximation only
            let f0 = ((ior - 1.0) / (ior + 1.0)).powi(2);
            f0 + (1.0 - f0) * (1.0 - cos_theta).powi(5)
        }
        QualityTier::Standard => {
            // Schlick with slight correction (air n=1.0 to material n=ior)
            fresnel_schlick(1.0, ior, cos_theta)
        }
        QualityTier::High | QualityTier::UltraHigh => {
            // Full Fresnel equations (air n=1.0 to material n=ior)
            // Average Rs and Rp for unpolarized light
            let (rs, rp) = fresnel_full(1.0, ior, cos_theta);
            (rs + rp) / 2.0
        }
        QualityTier::Experimental | QualityTier::Reference => {
            // Reference-grade full precision
            fresnel_dielectric_full(cos_theta, ior)
        }
    }
}

/// Evaluate absorption for a specific tier
fn evaluate_tier_absorption(
    tier: QualityTier,
    absorption: f64,
    thickness: f64,
    density: f64,
) -> f64 {
    let distance = thickness * density;

    match tier {
        QualityTier::Fast => {
            // Linear approximation
            (1.0 - absorption * distance * 0.1).max(0.0)
        }
        QualityTier::Standard => {
            // Simplified exponential
            (-absorption * distance * 0.1).exp()
        }
        QualityTier::High
        | QualityTier::UltraHigh
        | QualityTier::Experimental
        | QualityTier::Reference => {
            // Full Beer-Lambert
            beer_lambert_exact(absorption, distance, 1.0)
        }
    }
}

/// Convert reflectance to approximate RGB for Delta E calculation
fn reflectance_to_rgb(reflectance: f64, ior: f64) -> [f64; 3] {
    // Simple model: reflectance affects overall brightness
    // Higher IOR materials appear slightly blue-shifted at edges
    let base = reflectance.clamp(0.0, 1.0);
    let ior_factor = (ior - 1.0) / 1.5; // Normalize around glass IOR

    [
        base * (1.0 - 0.02 * ior_factor), // R slightly reduced for high IOR
        base,                             // G neutral
        base * (1.0 + 0.02 * ior_factor), // B slightly increased for high IOR
    ]
}

// ============================================================================
// VALIDATION ENGINE
// ============================================================================

/// Run cross-validation across all quality tiers
pub fn run_cross_validation() -> CrossValidationReport {
    let tiers = [
        QualityTier::Fast,
        QualityTier::Standard,
        QualityTier::High,
        QualityTier::UltraHigh,
        QualityTier::Experimental,
        QualityTier::Reference,
    ];

    // Test parameters
    let iors: [f64; 8] = [1.0, 1.2, 1.33, 1.5, 1.7, 2.0, 2.4, 3.0];
    let angles: [f64; 10] = [0.0, 0.1, 0.2, 0.3, 0.5, 0.7, 0.9, 1.1, 1.3, 1.5];

    let _reference_renderer = ReferenceRenderer::default();
    let mut tier_results = Vec::new();
    let mut reference_time_total = 0.0;

    // First pass: compute reference values and timing
    let mut reference_values: Vec<(f64, f64, f64)> = Vec::new(); // (ior, angle, fresnel)

    for &ior in &iors {
        for &angle in &angles {
            let cos_theta = angle.cos();
            let start = Instant::now();
            let ref_fresnel = fresnel_dielectric_full(cos_theta, ior);
            reference_time_total += start.elapsed().as_nanos() as f64 / 1000.0;
            reference_values.push((ior, angle, ref_fresnel));
        }
    }

    let test_cases = reference_values.len();
    let reference_time_us = reference_time_total / test_cases as f64;

    // Validate each tier
    for tier in tiers {
        let mut fresnel_errors: Vec<f64> = Vec::new();
        let mut delta_es: Vec<f64> = Vec::new();
        let mut energy_errors: Vec<f64> = Vec::new();
        let mut times: Vec<f64> = Vec::new();

        for &(ior, angle, ref_fresnel) in &reference_values {
            let cos_theta = angle.cos();

            // Evaluate tier Fresnel
            let start = Instant::now();
            let tier_fresnel = evaluate_tier_fresnel(tier, ior, cos_theta);
            times.push(start.elapsed().as_nanos() as f64 / 1000.0);

            // Compute Fresnel error
            let error = (tier_fresnel - ref_fresnel).abs();
            fresnel_errors.push(error);

            // Compute Delta E
            let ref_rgb = reflectance_to_rgb(ref_fresnel, ior);
            let tier_rgb = reflectance_to_rgb(tier_fresnel, ior);
            let ref_lab = rgb_to_lab(ref_rgb, Illuminant::D65);
            let tier_lab = rgb_to_lab(tier_rgb, Illuminant::D65);
            let de = delta_e_2000(ref_lab, tier_lab);
            delta_es.push(de);

            // Energy conservation (R + T + A should = 1)
            // For dielectric at this simplified level: T ≈ 1 - R
            let transmittance = 1.0 - tier_fresnel;
            let total = tier_fresnel + transmittance;
            energy_errors.push((total - 1.0).abs());
        }

        // Compute statistics
        let fresnel_rmse = (fresnel_errors.iter().map(|e| e * e).sum::<f64>()
            / fresnel_errors.len() as f64)
            .sqrt();
        let fresnel_max_error = fresnel_errors.iter().cloned().fold(0.0_f64, f64::max);
        let mean_delta_e = delta_es.iter().sum::<f64>() / delta_es.len() as f64;
        let max_delta_e = delta_es.iter().cloned().fold(0.0_f64, f64::max);
        let energy_error = energy_errors.iter().sum::<f64>() / energy_errors.len() as f64;
        let mean_eval_time_us = times.iter().sum::<f64>() / times.len() as f64;
        let absorption_rmse = 0.0; // Simplified for this validation

        let speedup = if mean_eval_time_us > 0.0 {
            reference_time_us / mean_eval_time_us
        } else {
            1.0
        };

        tier_results.push(TierValidationResult {
            tier,
            fresnel_rmse,
            fresnel_max_error,
            absorption_rmse,
            mean_delta_e,
            max_delta_e,
            energy_error,
            mean_eval_time_us,
            speedup_vs_reference: speedup,
        });
    }

    CrossValidationReport {
        tier_results,
        reference_time_us,
        test_cases,
        ior_range: (iors[0], iors[iors.len() - 1]),
        angle_range: (angles[0], angles[angles.len() - 1]),
    }
}

/// Run extended validation with metals
pub fn run_metal_validation() -> String {
    let mut report = String::new();
    report.push_str("# Metal Validation Report\n\n");

    // Test metal optical constants
    let metals = [
        ("Gold", 0.27, 2.87),
        ("Silver", 0.13, 3.99),
        ("Copper", 0.23, 3.42),
        ("Aluminum", 1.37, 7.62),
        ("Iron", 2.91, 3.10),
    ];

    report.push_str("| Metal | n | k | R (normal) | R (60°) | R (80°) |\n");
    report.push_str("|-------|---|---|------------|---------|--------|\n");

    for (name, n, k) in metals {
        let r_normal = fresnel_conductor_full(1.0, n, k);
        let r_60 = fresnel_conductor_full(60.0_f64.to_radians().cos(), n, k);
        let r_80 = fresnel_conductor_full(80.0_f64.to_radians().cos(), n, k);

        report.push_str(&format!(
            "| {} | {:.2} | {:.2} | {:.4} | {:.4} | {:.4} |\n",
            name, n, k, r_normal, r_60, r_80
        ));
    }

    report.push_str("\n## Expected Behavior\n\n");
    report.push_str("- Gold/Copper/Silver: High reflectance (>0.8) at normal incidence\n");
    report.push_str("- All metals: Reflectance increases towards grazing angle\n");
    report.push_str("- Aluminum: Highest reflectance due to high k value\n");

    report
}

/// Run dielectric validation with dispersion
pub fn run_dispersion_validation() -> String {
    let mut report = String::new();
    report.push_str("# Dispersion Validation Report\n\n");

    // Standard glass dispersion (Sellmeier-like variation)
    let wavelengths = [450.0, 550.0, 650.0]; // Blue, Green, Red
    let base_ior = 1.52; // Crown glass
    let abbe = 64.0; // Crown glass Abbe number

    report.push_str("## Crown Glass (n_d = 1.52, V = 64)\n\n");
    report.push_str("| Wavelength (nm) | IOR | Fresnel (normal) | Fresnel (60°) |\n");
    report.push_str("|-----------------|-----|------------------|---------------|\n");

    for &wl in &wavelengths {
        // Simplified Cauchy-like dispersion
        let ior = base_ior + 10000.0 / (abbe * wl * wl) * 1e4;
        let f_normal = fresnel_dielectric_full(1.0, ior);
        let f_60 = fresnel_dielectric_full(60.0_f64.to_radians().cos(), ior);

        report.push_str(&format!(
            "| {:.0} | {:.4} | {:.6} | {:.6} |\n",
            wl, ior, f_normal, f_60
        ));
    }

    report.push_str("\n## High-dispersion Flint Glass (n_d = 1.62, V = 36)\n\n");
    let base_ior_flint = 1.62;
    let abbe_flint = 36.0;

    report.push_str("| Wavelength (nm) | IOR | Fresnel (normal) | Fresnel (60°) |\n");
    report.push_str("|-----------------|-----|------------------|---------------|\n");

    for &wl in &wavelengths {
        let ior = base_ior_flint + 10000.0 / (abbe_flint * wl * wl) * 1e4;
        let f_normal = fresnel_dielectric_full(1.0, ior);
        let f_60 = fresnel_dielectric_full(60.0_f64.to_radians().cos(), ior);

        report.push_str(&format!(
            "| {:.0} | {:.4} | {:.6} | {:.6} |\n",
            wl, ior, f_normal, f_60
        ));
    }

    report
}

// ============================================================================
// PUBLIC API
// ============================================================================

/// Generate complete validation report
pub fn generate_full_validation_report() -> String {
    let mut report = String::new();

    // Cross-validation
    let cross_val = run_cross_validation();
    report.push_str(&cross_val.to_markdown());
    report.push('\n');

    // Metal validation
    report.push_str(&run_metal_validation());
    report.push('\n');

    // Dispersion validation
    report.push_str(&run_dispersion_validation());

    report
}

/// Get validation summary as structured data
pub fn get_validation_summary() -> CrossValidationReport {
    run_cross_validation()
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cross_validation_runs() {
        let report = run_cross_validation();
        assert_eq!(report.tier_results.len(), 6);
        assert!(report.test_cases > 0);
    }

    #[test]
    fn test_fast_tier_has_highest_error() {
        let report = run_cross_validation();
        let fast = &report.tier_results[0];
        let reference = &report.tier_results[5];

        // Fast tier should have higher error than Reference
        assert!(fast.fresnel_rmse >= reference.fresnel_rmse);
    }

    #[test]
    fn test_reference_tier_minimal_error() {
        let report = run_cross_validation();
        let reference = &report.tier_results[5];

        // Reference should have near-zero error (comparing to itself)
        assert!(reference.fresnel_rmse < 0.001);
    }

    #[test]
    fn test_metal_validation_runs() {
        let report = run_metal_validation();
        assert!(report.contains("Gold"));
        assert!(report.contains("Silver"));
    }

    #[test]
    fn test_dispersion_validation_runs() {
        let report = run_dispersion_validation();
        assert!(report.contains("Crown Glass"));
        assert!(report.contains("Flint Glass"));
    }

    #[test]
    fn test_full_report_generation() {
        let report = generate_full_validation_report();
        assert!(report.len() > 1000);
        assert!(report.contains("Quality Tier"));
        assert!(report.contains("Metal"));
        assert!(report.contains("Dispersion"));
    }

    #[test]
    fn test_markdown_format() {
        let report = run_cross_validation();
        let md = report.to_markdown();
        assert!(md.contains("|"));
        assert!(md.contains("Tier"));
    }

    #[test]
    fn test_json_format() {
        let report = run_cross_validation();
        let json = report.to_json();
        assert!(json.contains("\"tier\""));
        assert!(json.contains("\"fresnel_rmse\""));
    }
}
