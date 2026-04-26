//! # PBR Phase 1 Validation and Benchmarks
//!
//! Comprehensive validation tests and performance benchmarks for
//! the new physically-based rendering components.
//!
//! ## Validation Areas
//!
//! 1. **Physical Accuracy**: Compare against reference values from literature
//! 2. **LUT Accuracy**: Verify lookup tables match direct calculation
//! 3. **Spectral Consistency**: RGB channels follow expected ordering
//! 4. **Performance**: Measure cycles and throughput for each tier
//!
//! ## Running Benchmarks
//!
//! ```bash
//! cargo test --release pbr_validation -- --nocapture
//! ```

use std::time::Instant;

use super::dispersion::{CauchyDispersion, Dispersion, SellmeierDispersion};
use super::fresnel::{fresnel_full, fresnel_schlick};
use super::lut::{BeerLambertLUT, FresnelLUT};
use super::scattering::{henyey_greenstein, HenyeyGreensteinLUT};
use super::spectral_fresnel::{fresnel_rgb, fresnel_rgb_fast, SpectralFresnelLUT};

// ============================================================================
// PHYSICAL ACCURACY VALIDATION
// ============================================================================

/// Measured refractive index data for validation
/// Source: RefractiveIndex.INFO, Schott Glass Catalog
pub struct ReferenceData {
    pub material: &'static str,
    pub wavelength_nm: f64,
    pub n_measured: f64,
    pub tolerance: f64,
}

/// Reference data for validation
pub fn reference_data() -> Vec<ReferenceData> {
    vec![
        // Fused Silica (Malitson 1965)
        ReferenceData {
            material: "Fused Silica",
            wavelength_nm: 486.1,
            n_measured: 1.4631,
            tolerance: 0.001,
        },
        ReferenceData {
            material: "Fused Silica",
            wavelength_nm: 587.6,
            n_measured: 1.4585,
            tolerance: 0.001,
        },
        ReferenceData {
            material: "Fused Silica",
            wavelength_nm: 656.3,
            n_measured: 1.4564,
            tolerance: 0.001,
        },
        // BK7 Glass (Schott Catalog)
        ReferenceData {
            material: "BK7",
            wavelength_nm: 486.1,
            n_measured: 1.5224,
            tolerance: 0.002,
        },
        ReferenceData {
            material: "BK7",
            wavelength_nm: 587.6,
            n_measured: 1.5168,
            tolerance: 0.002,
        },
        ReferenceData {
            material: "BK7",
            wavelength_nm: 656.3,
            n_measured: 1.5143,
            tolerance: 0.002,
        },
        // Water at 20C
        ReferenceData {
            material: "Water",
            wavelength_nm: 589.3,
            n_measured: 1.333,
            tolerance: 0.002,
        },
    ]
}

/// Validate dispersion models against measured data
pub fn validate_dispersion_accuracy() -> Vec<ValidationResult> {
    let mut results = Vec::new();

    // Test Sellmeier fused silica
    let silica = SellmeierDispersion::fused_silica();
    for data in reference_data()
        .iter()
        .filter(|d| d.material == "Fused Silica")
    {
        let calculated = silica.n(data.wavelength_nm);
        let error = (calculated - data.n_measured).abs();
        results.push(ValidationResult {
            test: format!("Sellmeier {} @ {}nm", data.material, data.wavelength_nm),
            expected: data.n_measured,
            actual: calculated,
            error,
            passed: error < data.tolerance,
        });
    }

    // Test Sellmeier BK7
    let bk7 = SellmeierDispersion::bk7();
    for data in reference_data().iter().filter(|d| d.material == "BK7") {
        let calculated = bk7.n(data.wavelength_nm);
        let error = (calculated - data.n_measured).abs();
        results.push(ValidationResult {
            test: format!("Sellmeier {} @ {}nm", data.material, data.wavelength_nm),
            expected: data.n_measured,
            actual: calculated,
            error,
            passed: error < data.tolerance,
        });
    }

    // Test Cauchy water
    let water = CauchyDispersion::water();
    for data in reference_data().iter().filter(|d| d.material == "Water") {
        let calculated = water.n(data.wavelength_nm);
        let error = (calculated - data.n_measured).abs();
        results.push(ValidationResult {
            test: format!("Cauchy {} @ {}nm", data.material, data.wavelength_nm),
            expected: data.n_measured,
            actual: calculated,
            error,
            passed: error < data.tolerance,
        });
    }

    results
}

/// Validate Fresnel against exact equations
pub fn validate_fresnel_accuracy() -> Vec<ValidationResult> {
    let mut results = Vec::new();

    // Test Schlick vs Full Fresnel at various angles
    // Note: Schlick is an approximation that diverges at grazing angles.
    // We use angle-dependent tolerance: tight at normal, loose at grazing.
    let ior = 1.5;
    let test_angles = [1.0, 0.9, 0.7, 0.5, 0.3, 0.1, 0.05];

    for &cos_theta in &test_angles {
        let schlick = fresnel_schlick(1.0, ior, cos_theta);
        let (rs, rp) = fresnel_full(1.0, ior, cos_theta);
        let full_avg = (rs + rp) / 2.0;

        // Angle-dependent tolerance: Schlick is less accurate at grazing angles
        // cos_theta < 0.3 => grazing angle region where Schlick deviates more
        let tolerance = if cos_theta < 0.2 {
            0.10 // 10% at extreme grazing (expected Schlick limitation)
        } else if cos_theta < 0.5 {
            0.05 // 5% at moderate angles
        } else {
            0.02 // 2% at normal-ish angles
        };

        let error = (schlick - full_avg).abs();
        results.push(ValidationResult {
            test: format!("Schlick vs Full @ cos={:.2}", cos_theta),
            expected: full_avg,
            actual: schlick,
            error,
            passed: error < tolerance,
        });
    }

    // Test normal incidence reflectance
    // R0 = ((n-1)/(n+1))^2 = ((1.5-1)/(1.5+1))^2 = 0.04
    let r0_expected = 0.04;
    let r0_actual = fresnel_schlick(1.0, 1.5, 1.0);
    results.push(ValidationResult {
        test: "R0 at normal incidence (n=1.5)".to_string(),
        expected: r0_expected,
        actual: r0_actual,
        error: (r0_actual - r0_expected).abs(),
        passed: (r0_actual - r0_expected).abs() < 0.001,
    });

    results
}

/// Validate Henyey-Greenstein normalization
pub fn validate_hg_normalization() -> Vec<ValidationResult> {
    let mut results = Vec::new();

    // Phase function should integrate to 1 over sphere
    for g in [-0.8, -0.3, 0.0, 0.3, 0.8] {
        let n_samples = 10000;
        let mut integral = 0.0;

        for i in 0..n_samples {
            let cos_theta = -1.0 + 2.0 * (i as f64 / n_samples as f64);
            let p = henyey_greenstein(cos_theta, g);
            integral += p * 2.0 * std::f64::consts::PI * (2.0 / n_samples as f64);
        }

        results.push(ValidationResult {
            test: format!("H-G normalization @ g={:.1}", g),
            expected: 1.0,
            actual: integral,
            error: (integral - 1.0).abs(),
            passed: (integral - 1.0).abs() < 0.02,
        });
    }

    results
}

// ============================================================================
// LUT ACCURACY VALIDATION
// ============================================================================

/// Validate LUT accuracy against direct calculation
pub fn validate_lut_accuracy() -> Vec<ValidationResult> {
    let mut results = Vec::new();

    // Fresnel LUT
    let fresnel_lut = FresnelLUT::global();
    for ior in [1.2, 1.5, 1.8, 2.2] {
        for cos_theta in [0.1, 0.3, 0.5, 0.7, 0.9] {
            let direct = fresnel_schlick(1.0, ior, cos_theta);
            let lut = fresnel_lut.lookup(ior, cos_theta);
            let error = (direct - lut).abs();

            results.push(ValidationResult {
                test: format!("Fresnel LUT ior={:.1} cos={:.1}", ior, cos_theta),
                expected: direct,
                actual: lut,
                error,
                passed: error < 0.01, // 1% tolerance
            });
        }
    }

    // H-G LUT
    let hg_lut = HenyeyGreensteinLUT::global();
    for g in [-0.5, 0.0, 0.5] {
        for cos_theta in [-0.8, 0.0, 0.8] {
            let direct = henyey_greenstein(cos_theta, g);
            let lut = hg_lut.lookup(cos_theta, g);
            let error = (direct - lut).abs() / direct.max(0.001);

            results.push(ValidationResult {
                test: format!("H-G LUT g={:.1} cos={:.1}", g, cos_theta),
                expected: direct,
                actual: lut,
                error,
                passed: error < 0.02, // 2% relative tolerance
            });
        }
    }

    results
}

// ============================================================================
// SPECTRAL VALIDATION
// ============================================================================

/// Validate RGB spectral ordering
pub fn validate_spectral_ordering() -> Vec<ValidationResult> {
    let mut results = Vec::new();

    // For dispersive materials, n_blue > n_green > n_red
    let crown = CauchyDispersion::crown_glass();
    let n_rgb = crown.n_rgb();

    results.push(ValidationResult {
        test: "Spectral ordering: n_red < n_green".to_string(),
        expected: 1.0, // Just checking ordering
        actual: if n_rgb[0] < n_rgb[1] { 1.0 } else { 0.0 },
        error: 0.0,
        passed: n_rgb[0] < n_rgb[1],
    });

    results.push(ValidationResult {
        test: "Spectral ordering: n_green < n_blue".to_string(),
        expected: 1.0,
        actual: if n_rgb[1] < n_rgb[2] { 1.0 } else { 0.0 },
        error: 0.0,
        passed: n_rgb[1] < n_rgb[2],
    });

    // Fresnel RGB should also follow ordering (higher n = higher F0)
    let f_rgb = fresnel_rgb(&crown, 0.5);
    results.push(ValidationResult {
        test: "Fresnel RGB ordering: r < g < b".to_string(),
        expected: 1.0,
        actual: if f_rgb[0] < f_rgb[1] && f_rgb[1] < f_rgb[2] {
            1.0
        } else {
            0.0
        },
        error: 0.0,
        passed: f_rgb[0] < f_rgb[1] && f_rgb[1] < f_rgb[2],
    });

    results
}

// ============================================================================
// PERFORMANCE BENCHMARKS
// ============================================================================

/// Benchmark result
#[derive(Debug)]
pub struct BenchmarkResult {
    pub name: String,
    pub iterations: u64,
    pub total_time_ns: u64,
    pub ns_per_op: f64,
    pub throughput: f64, // ops per second
}

/// Run performance benchmarks
pub fn run_benchmarks() -> Vec<BenchmarkResult> {
    let mut results = Vec::new();

    // Fresnel Schlick (baseline)
    results.push(benchmark_fresnel_schlick());

    // Fresnel LUT
    results.push(benchmark_fresnel_lut());

    // H-G direct
    results.push(benchmark_hg_direct());

    // H-G LUT
    results.push(benchmark_hg_lut());

    // RGB Fresnel direct
    results.push(benchmark_fresnel_rgb_direct());

    // RGB Fresnel fast
    results.push(benchmark_fresnel_rgb_fast());

    // Cauchy dispersion
    results.push(benchmark_cauchy());

    // Sellmeier dispersion
    results.push(benchmark_sellmeier());

    results
}

fn benchmark_fresnel_schlick() -> BenchmarkResult {
    let iterations = 1_000_000;
    let start = Instant::now();

    let mut sum = 0.0;
    for i in 0..iterations {
        let cos_theta = i as f64 / iterations as f64;
        sum += fresnel_schlick(1.0, 1.5, cos_theta);
    }

    let elapsed = start.elapsed().as_nanos() as u64;
    // Prevent optimization
    std::hint::black_box(sum);

    BenchmarkResult {
        name: "Fresnel Schlick".to_string(),
        iterations,
        total_time_ns: elapsed,
        ns_per_op: elapsed as f64 / iterations as f64,
        throughput: iterations as f64 / (elapsed as f64 / 1e9),
    }
}

fn benchmark_fresnel_lut() -> BenchmarkResult {
    let lut = FresnelLUT::global();
    let iterations = 1_000_000;
    let start = Instant::now();

    let mut sum = 0.0;
    for i in 0..iterations {
        let cos_theta = i as f64 / iterations as f64;
        sum += lut.lookup(1.5, cos_theta);
    }

    let elapsed = start.elapsed().as_nanos() as u64;
    std::hint::black_box(sum);

    BenchmarkResult {
        name: "Fresnel LUT".to_string(),
        iterations,
        total_time_ns: elapsed,
        ns_per_op: elapsed as f64 / iterations as f64,
        throughput: iterations as f64 / (elapsed as f64 / 1e9),
    }
}

fn benchmark_hg_direct() -> BenchmarkResult {
    let iterations = 1_000_000;
    let start = Instant::now();

    let mut sum = 0.0;
    for i in 0..iterations {
        let cos_theta = -1.0 + 2.0 * (i as f64 / iterations as f64);
        sum += henyey_greenstein(cos_theta, 0.5);
    }

    let elapsed = start.elapsed().as_nanos() as u64;
    std::hint::black_box(sum);

    BenchmarkResult {
        name: "H-G Direct".to_string(),
        iterations,
        total_time_ns: elapsed,
        ns_per_op: elapsed as f64 / iterations as f64,
        throughput: iterations as f64 / (elapsed as f64 / 1e9),
    }
}

fn benchmark_hg_lut() -> BenchmarkResult {
    let lut = HenyeyGreensteinLUT::global();
    let iterations = 1_000_000;
    let start = Instant::now();

    let mut sum = 0.0;
    for i in 0..iterations {
        let cos_theta = -1.0 + 2.0 * (i as f64 / iterations as f64);
        sum += lut.lookup(cos_theta, 0.5);
    }

    let elapsed = start.elapsed().as_nanos() as u64;
    std::hint::black_box(sum);

    BenchmarkResult {
        name: "H-G LUT".to_string(),
        iterations,
        total_time_ns: elapsed,
        ns_per_op: elapsed as f64 / iterations as f64,
        throughput: iterations as f64 / (elapsed as f64 / 1e9),
    }
}

fn benchmark_fresnel_rgb_direct() -> BenchmarkResult {
    let crown = CauchyDispersion::crown_glass();
    let iterations = 1_000_000;
    let start = Instant::now();

    let mut sum = [0.0, 0.0, 0.0];
    for i in 0..iterations {
        let cos_theta = i as f64 / iterations as f64;
        let rgb = fresnel_rgb(&crown, cos_theta);
        sum[0] += rgb[0];
        sum[1] += rgb[1];
        sum[2] += rgb[2];
    }

    let elapsed = start.elapsed().as_nanos() as u64;
    std::hint::black_box(sum);

    BenchmarkResult {
        name: "Fresnel RGB Direct".to_string(),
        iterations,
        total_time_ns: elapsed,
        ns_per_op: elapsed as f64 / iterations as f64,
        throughput: iterations as f64 / (elapsed as f64 / 1e9),
    }
}

fn benchmark_fresnel_rgb_fast() -> BenchmarkResult {
    let crown = CauchyDispersion::crown_glass();
    let iterations = 1_000_000;
    let start = Instant::now();

    let mut sum = [0.0, 0.0, 0.0];
    for i in 0..iterations {
        let cos_theta = i as f64 / iterations as f64;
        let rgb = fresnel_rgb_fast(&crown, cos_theta);
        sum[0] += rgb[0];
        sum[1] += rgb[1];
        sum[2] += rgb[2];
    }

    let elapsed = start.elapsed().as_nanos() as u64;
    std::hint::black_box(sum);

    BenchmarkResult {
        name: "Fresnel RGB Fast (LUT)".to_string(),
        iterations,
        total_time_ns: elapsed,
        ns_per_op: elapsed as f64 / iterations as f64,
        throughput: iterations as f64 / (elapsed as f64 / 1e9),
    }
}

fn benchmark_cauchy() -> BenchmarkResult {
    let crown = CauchyDispersion::crown_glass();
    let iterations = 1_000_000;
    let start = Instant::now();

    let mut sum = 0.0;
    for i in 0..iterations {
        let wavelength = 400.0 + 350.0 * (i as f64 / iterations as f64);
        sum += crown.n(wavelength);
    }

    let elapsed = start.elapsed().as_nanos() as u64;
    std::hint::black_box(sum);

    BenchmarkResult {
        name: "Cauchy Dispersion".to_string(),
        iterations,
        total_time_ns: elapsed,
        ns_per_op: elapsed as f64 / iterations as f64,
        throughput: iterations as f64 / (elapsed as f64 / 1e9),
    }
}

fn benchmark_sellmeier() -> BenchmarkResult {
    let silica = SellmeierDispersion::fused_silica();
    let iterations = 1_000_000;
    let start = Instant::now();

    let mut sum = 0.0;
    for i in 0..iterations {
        let wavelength = 400.0 + 350.0 * (i as f64 / iterations as f64);
        sum += silica.n(wavelength);
    }

    let elapsed = start.elapsed().as_nanos() as u64;
    std::hint::black_box(sum);

    BenchmarkResult {
        name: "Sellmeier Dispersion".to_string(),
        iterations,
        total_time_ns: elapsed,
        ns_per_op: elapsed as f64 / iterations as f64,
        throughput: iterations as f64 / (elapsed as f64 / 1e9),
    }
}

// ============================================================================
// RESULT STRUCTURES
// ============================================================================

/// Validation test result
#[derive(Debug)]
pub struct ValidationResult {
    pub test: String,
    pub expected: f64,
    pub actual: f64,
    pub error: f64,
    pub passed: bool,
}

/// Memory usage report
pub fn memory_report() -> Vec<(String, usize)> {
    vec![
        (
            "Fresnel LUT".to_string(),
            FresnelLUT::global().memory_size(),
        ),
        (
            "Beer-Lambert LUT".to_string(),
            BeerLambertLUT::global().memory_size(),
        ),
        (
            "H-G LUT".to_string(),
            HenyeyGreensteinLUT::global().memory_size(),
        ),
        (
            "Spectral Fresnel LUT".to_string(),
            SpectralFresnelLUT::global().memory_size(),
        ),
    ]
}

/// Generate full validation report
pub fn full_validation_report() -> String {
    let mut report = String::new();

    report.push_str("# PBR Phase 1 Validation Report\n\n");

    // Physical accuracy
    report.push_str("## Physical Accuracy\n\n");
    report.push_str("| Test | Expected | Actual | Error | Status |\n");
    report.push_str("|------|----------|--------|-------|--------|\n");
    for result in validate_dispersion_accuracy() {
        report.push_str(&format!(
            "| {} | {:.4} | {:.4} | {:.4} | {} |\n",
            result.test,
            result.expected,
            result.actual,
            result.error,
            if result.passed { "PASS" } else { "FAIL" }
        ));
    }

    // Fresnel accuracy
    report.push_str("\n## Fresnel Accuracy\n\n");
    report.push_str("| Test | Expected | Actual | Error | Status |\n");
    report.push_str("|------|----------|--------|-------|--------|\n");
    for result in validate_fresnel_accuracy() {
        report.push_str(&format!(
            "| {} | {:.4} | {:.4} | {:.4} | {} |\n",
            result.test,
            result.expected,
            result.actual,
            result.error,
            if result.passed { "PASS" } else { "FAIL" }
        ));
    }

    // LUT accuracy
    report.push_str("\n## LUT Accuracy\n\n");
    report.push_str("| Test | Expected | Actual | Error | Status |\n");
    report.push_str("|------|----------|--------|-------|--------|\n");
    for result in validate_lut_accuracy() {
        report.push_str(&format!(
            "| {} | {:.4} | {:.4} | {:.4} | {} |\n",
            result.test,
            result.expected,
            result.actual,
            result.error,
            if result.passed { "PASS" } else { "FAIL" }
        ));
    }

    // Memory usage
    report.push_str("\n## Memory Usage\n\n");
    report.push_str("| Component | Size (KB) |\n");
    report.push_str("|-----------|----------|\n");
    let mut total = 0;
    for (name, size) in memory_report() {
        report.push_str(&format!("| {} | {:.1} |\n", name, size as f64 / 1024.0));
        total += size;
    }
    report.push_str(&format!(
        "| **Total** | **{:.1}** |\n",
        total as f64 / 1024.0
    ));

    report
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dispersion_validation() {
        let results = validate_dispersion_accuracy();
        let passed = results.iter().filter(|r| r.passed).count();
        let total = results.len();

        println!("\nDispersion Validation: {}/{} passed", passed, total);
        for result in &results {
            if !result.passed {
                println!("  FAIL: {} (error: {:.4})", result.test, result.error);
            }
        }

        assert!(
            passed >= total - 1,
            "At least {} tests should pass",
            total - 1
        );
    }

    #[test]
    fn test_fresnel_validation() {
        let results = validate_fresnel_accuracy();
        let passed = results.iter().filter(|r| r.passed).count();
        let total = results.len();

        println!("\nFresnel Validation: {}/{} passed", passed, total);
        assert_eq!(passed, total, "All Fresnel tests should pass");
    }

    #[test]
    fn test_hg_normalization() {
        let results = validate_hg_normalization();
        let passed = results.iter().filter(|r| r.passed).count();
        let total = results.len();

        println!("\nH-G Normalization: {}/{} passed", passed, total);
        assert_eq!(passed, total, "All H-G normalization tests should pass");
    }

    #[test]
    fn test_lut_validation() {
        let results = validate_lut_accuracy();
        let passed = results.iter().filter(|r| r.passed).count();
        let total = results.len();

        println!("\nLUT Validation: {}/{} passed", passed, total);
        assert!(passed >= total - 2, "Most LUT tests should pass");
    }

    #[test]
    fn test_spectral_validation() {
        let results = validate_spectral_ordering();
        let passed = results.iter().filter(|r| r.passed).count();
        let total = results.len();

        println!("\nSpectral Validation: {}/{} passed", passed, total);
        assert_eq!(passed, total, "All spectral ordering tests should pass");
    }

    #[test]
    fn test_memory_budget() {
        let mut total = 0;
        for (_, size) in memory_report() {
            total += size;
        }

        // Total should be under 1MB
        assert!(
            total < 1024 * 1024,
            "Total memory {} should be < 1MB",
            total
        );

        println!("\nTotal LUT memory: {:.1} KB", total as f64 / 1024.0);
    }

    #[test]
    #[ignore] // Run with: cargo test --release benchmark_performance -- --ignored --nocapture
    fn benchmark_performance() {
        println!("\n=== PBR Phase 1 Performance Benchmarks ===\n");
        println!(
            "{:<25} {:>12} {:>12} {:>15}",
            "Operation", "ns/op", "ops/sec", "Speedup"
        );
        println!("{:-<65}", "");

        let results = run_benchmarks();
        let baseline_ns = results
            .iter()
            .find(|r| r.name == "Fresnel Schlick")
            .map(|r| r.ns_per_op)
            .unwrap_or(1.0);

        for result in &results {
            let speedup = baseline_ns / result.ns_per_op;
            println!(
                "{:<25} {:>10.1}ns {:>10.0}M {:>12.2}x",
                result.name,
                result.ns_per_op,
                result.throughput / 1_000_000.0,
                speedup
            );
        }
    }

    #[test]
    #[ignore = "Requires enhanced_presets module integration"]
    fn test_preset_materials() {
        // Validate all presets have reasonable parameters
        // TODO: Import all_presets from enhanced_presets
        // for preset in all_presets() {
        //     assert!(preset.ior > 1.0 && preset.ior < 3.0);
        //     assert!(preset.roughness >= 0.0 && preset.roughness <= 1.0);
        //     assert!(preset.thickness > 0.0);
        // }
    }
}
