//! # Phase 2 Validation and Benchmarks
//!
//! Comprehensive validation comparing Phase 2 features:
//!
//! - **DHG vs Single H-G**: Accuracy and performance comparison
//! - **Sellmeier vs Cauchy**: Dispersion model accuracy
//! - **Quality Tier Performance**: Throughput per tier
//! - **Memory Analysis**: LUT sizes and efficiency
//!
//! ## Running Benchmarks
//!
//! ```bash
//! cargo test --release phase2_validation -- --nocapture
//! ```

use std::time::Instant;

use super::dhg_lut::{dhg_fast, dhg_preset, CompactDHGLUT, DHGPreset, DoubleHGLUT};
use super::dispersion::{wavelengths, CauchyDispersion, Dispersion, SellmeierDispersion};
use super::enhanced_presets::QualityTier;
use super::quality_tiers::TierFeatures;
use super::scattering::{double_henyey_greenstein, henyey_greenstein, hg_fast};
use super::spectral_fresnel::{fresnel_rgb, fresnel_rgb_fast};

// ============================================================================
// COMPARISON METRICS
// ============================================================================

/// Comparison result between two methods
#[derive(Debug, Clone)]
pub struct ComparisonResult {
    /// Name of the comparison
    pub name: String,
    /// Method A name
    pub method_a: String,
    /// Method B name
    pub method_b: String,
    /// Method A average time (ns)
    pub time_a_ns: f64,
    /// Method B average time (ns)
    pub time_b_ns: f64,
    /// Speedup ratio (A/B, >1 means B is faster)
    pub speedup: f64,
    /// Maximum error between methods
    pub max_error: f64,
    /// Average error between methods
    pub avg_error: f64,
    /// Is the error within acceptable tolerance?
    pub accuracy_ok: bool,
}

impl ComparisonResult {
    /// Format as table row
    pub fn to_row(&self) -> String {
        format!(
            "| {} | {:.1}ns | {:.1}ns | {:.2}x | {:.4} | {} |",
            self.name,
            self.time_a_ns,
            self.time_b_ns,
            self.speedup,
            self.avg_error,
            if self.accuracy_ok { "PASS" } else { "FAIL" }
        )
    }
}

// ============================================================================
// DHG VS SINGLE H-G COMPARISON
// ============================================================================

/// Compare Double H-G to Single H-G for various material configurations
pub fn compare_dhg_vs_hg() -> Vec<ComparisonResult> {
    let mut results = Vec::new();

    // Test configurations representing different material types
    let configs = [
        ("Forward (g=0.5)", 0.5),
        ("Strong Forward (g=0.8)", 0.8),
        ("Isotropic (g=0.0)", 0.0),
        ("Slight Back (g=-0.2)", -0.2),
    ];

    let iterations = 100_000;

    for (name, g) in configs {
        // Single H-G timing
        let start = Instant::now();
        let mut sum_single = 0.0;
        for i in 0..iterations {
            let cos_theta = -1.0 + 2.0 * (i as f64 / iterations as f64);
            sum_single += hg_fast(cos_theta, g);
        }
        let time_single = start.elapsed().as_nanos() as f64 / iterations as f64;
        std::hint::black_box(sum_single);

        // DHG with equivalent parameters (forward lobe only)
        let start = Instant::now();
        let mut sum_dhg = 0.0;
        for i in 0..iterations {
            let cos_theta = -1.0 + 2.0 * (i as f64 / iterations as f64);
            sum_dhg += dhg_fast(cos_theta, g.max(0.0), g.min(0.0), 0.5);
        }
        let time_dhg = start.elapsed().as_nanos() as f64 / iterations as f64;
        std::hint::black_box(sum_dhg);

        // Calculate error (they won't match exactly due to different models)
        let mut max_error = 0.0f64;
        let mut total_error = 0.0;
        let n_samples = 100;
        for i in 0..n_samples {
            let cos_theta = -1.0 + 2.0 * (i as f64 / n_samples as f64);
            let single = henyey_greenstein(cos_theta, g);
            // Use single-lobe equivalent
            let dhg = if g >= 0.0 {
                double_henyey_greenstein(cos_theta, g, 0.0, 1.0)
            } else {
                double_henyey_greenstein(cos_theta, 0.0, g, 0.0)
            };
            let error = (single - dhg).abs() / single.max(0.001);
            max_error = max_error.max(error);
            total_error += error;
        }
        let avg_error = total_error / n_samples as f64;

        results.push(ComparisonResult {
            name: name.to_string(),
            method_a: "Single H-G (LUT)".to_string(),
            method_b: "DHG (LUT)".to_string(),
            time_a_ns: time_single,
            time_b_ns: time_dhg,
            speedup: time_dhg / time_single,
            max_error,
            avg_error,
            accuracy_ok: avg_error < 0.01, // When using single-lobe equivalent
        });
    }

    results
}

/// Compare DHG LUT to direct calculation
pub fn compare_dhg_lut_vs_direct() -> Vec<ComparisonResult> {
    let mut results = Vec::new();

    let presets = [
        ("Milk", DHGPreset::Milk),
        ("Opal", DHGPreset::Opal),
        ("Skin", DHGPreset::Skin),
        ("Marble", DHGPreset::Marble),
    ];

    let iterations = 100_000;

    for (name, preset) in presets {
        let lut = CompactDHGLUT::global();
        let (g_f, g_b, w) = lut.get_config(preset);

        // Direct calculation
        let start = Instant::now();
        let mut sum_direct = 0.0;
        for i in 0..iterations {
            let cos_theta = -1.0 + 2.0 * (i as f64 / iterations as f64);
            sum_direct += double_henyey_greenstein(cos_theta, g_f, g_b, w);
        }
        let time_direct = start.elapsed().as_nanos() as f64 / iterations as f64;
        std::hint::black_box(sum_direct);

        // LUT lookup
        let start = Instant::now();
        let mut sum_lut = 0.0;
        for i in 0..iterations {
            let cos_theta = -1.0 + 2.0 * (i as f64 / iterations as f64);
            sum_lut += dhg_preset(cos_theta, preset);
        }
        let time_lut = start.elapsed().as_nanos() as f64 / iterations as f64;
        std::hint::black_box(sum_lut);

        // Calculate accuracy
        let mut max_error = 0.0f64;
        let mut total_error = 0.0;
        let n_samples = 100;
        for i in 0..n_samples {
            let cos_theta = -1.0 + 2.0 * (i as f64 / n_samples as f64);
            let direct = double_henyey_greenstein(cos_theta, g_f, g_b, w);
            let from_lut = dhg_preset(cos_theta, preset);
            let error = (direct - from_lut).abs() / direct.max(0.001);
            max_error = max_error.max(error);
            total_error += error;
        }
        let avg_error = total_error / n_samples as f64;

        results.push(ComparisonResult {
            name: name.to_string(),
            method_a: "DHG Direct".to_string(),
            method_b: "DHG Preset LUT".to_string(),
            time_a_ns: time_direct,
            time_b_ns: time_lut,
            speedup: time_direct / time_lut,
            max_error,
            avg_error,
            accuracy_ok: avg_error < 0.02,
        });
    }

    results
}

// ============================================================================
// SELLMEIER VS CAUCHY COMPARISON
// ============================================================================

/// Compare Sellmeier to Cauchy for dispersion accuracy
pub fn compare_sellmeier_vs_cauchy() -> Vec<ComparisonResult> {
    let mut results = Vec::new();

    // BK7 glass - both models available
    let cauchy_bk7 = CauchyDispersion::crown_glass();
    let sellmeier_bk7 = SellmeierDispersion::bk7();

    let iterations = 100_000;

    // Timing comparison
    let start = Instant::now();
    let mut sum_cauchy = 0.0;
    for i in 0..iterations {
        let lambda = 400.0 + (i as f64 / iterations as f64) * 400.0;
        sum_cauchy += cauchy_bk7.n(lambda);
    }
    let time_cauchy = start.elapsed().as_nanos() as f64 / iterations as f64;
    std::hint::black_box(sum_cauchy);

    let start = Instant::now();
    let mut sum_sellmeier = 0.0;
    for i in 0..iterations {
        let lambda = 400.0 + (i as f64 / iterations as f64) * 400.0;
        sum_sellmeier += sellmeier_bk7.n(lambda);
    }
    let time_sellmeier = start.elapsed().as_nanos() as f64 / iterations as f64;
    std::hint::black_box(sum_sellmeier);

    // Accuracy comparison (Sellmeier is reference)
    let test_wavelengths = [
        wavelengths::BLUE,
        wavelengths::GREEN,
        wavelengths::RED,
        wavelengths::SODIUM_D,
        450.0,
        500.0,
        550.0,
        600.0,
        650.0,
        700.0,
    ];

    let mut max_error = 0.0f64;
    let mut total_error = 0.0;
    for lambda in test_wavelengths {
        let n_cauchy = cauchy_bk7.n(lambda);
        let n_sellmeier = sellmeier_bk7.n(lambda);
        let error = (n_cauchy - n_sellmeier).abs() / n_sellmeier;
        max_error = max_error.max(error);
        total_error += error;
    }
    let avg_error = total_error / test_wavelengths.len() as f64;

    results.push(ComparisonResult {
        name: "BK7 IOR".to_string(),
        method_a: "Cauchy".to_string(),
        method_b: "Sellmeier".to_string(),
        time_a_ns: time_cauchy,
        time_b_ns: time_sellmeier,
        speedup: time_sellmeier / time_cauchy,
        max_error,
        avg_error,
        accuracy_ok: avg_error < 0.005, // 0.5% tolerance
    });

    // RGB evaluation comparison
    let start = Instant::now();
    for _ in 0..iterations {
        let _ = cauchy_bk7.n_rgb();
    }
    let time_cauchy_rgb = start.elapsed().as_nanos() as f64 / iterations as f64;

    let start = Instant::now();
    for _ in 0..iterations {
        let _ = sellmeier_bk7.n_rgb();
    }
    let time_sellmeier_rgb = start.elapsed().as_nanos() as f64 / iterations as f64;

    let cauchy_rgb = cauchy_bk7.n_rgb();
    let sellmeier_rgb = sellmeier_bk7.n_rgb();
    let mut total_rgb_error = 0.0;
    for i in 0..3 {
        total_rgb_error += (cauchy_rgb[i] - sellmeier_rgb[i]).abs() / sellmeier_rgb[i];
    }
    let avg_rgb_error = total_rgb_error / 3.0;

    results.push(ComparisonResult {
        name: "BK7 RGB".to_string(),
        method_a: "Cauchy n_rgb".to_string(),
        method_b: "Sellmeier n_rgb".to_string(),
        time_a_ns: time_cauchy_rgb,
        time_b_ns: time_sellmeier_rgb,
        speedup: time_sellmeier_rgb / time_cauchy_rgb,
        max_error: (cauchy_rgb[2] - sellmeier_rgb[2]).abs() / sellmeier_rgb[2],
        avg_error: avg_rgb_error,
        accuracy_ok: avg_rgb_error < 0.005,
    });

    // Abbe number comparison
    let abbe_cauchy = cauchy_bk7.abbe_number();
    let abbe_sellmeier = sellmeier_bk7.abbe_number();
    let abbe_error = (abbe_cauchy - abbe_sellmeier).abs() / abbe_sellmeier;

    results.push(ComparisonResult {
        name: "BK7 Abbe".to_string(),
        method_a: format!("Cauchy ({:.1})", abbe_cauchy),
        method_b: format!("Sellmeier ({:.1})", abbe_sellmeier),
        time_a_ns: 0.0,
        time_b_ns: 0.0,
        speedup: 1.0,
        max_error: abbe_error,
        avg_error: abbe_error,
        accuracy_ok: abbe_error < 0.1, // 10% tolerance for Abbe
    });

    results
}

// ============================================================================
// QUALITY TIER BENCHMARKS
// ============================================================================

/// Benchmark each quality tier's rendering performance
pub fn benchmark_quality_tiers() -> Vec<(QualityTier, TierBenchmark)> {
    let tiers = [
        QualityTier::Fast,
        QualityTier::Standard,
        QualityTier::High,
        QualityTier::Reference,
    ];

    let mut results = Vec::new();

    for tier in tiers {
        let features = TierFeatures::for_tier(tier);
        let benchmark = benchmark_tier(&features);
        results.push((tier, benchmark));
    }

    results
}

/// Benchmark result for a quality tier
#[derive(Debug, Clone)]
pub struct TierBenchmark {
    /// Average time per material (ns)
    pub avg_ns: f64,
    /// Operations per second
    pub throughput: f64,
    /// Memory used (bytes)
    pub memory_bytes: usize,
    /// Features enabled
    pub features_enabled: Vec<String>,
}

fn benchmark_tier(features: &TierFeatures) -> TierBenchmark {
    let iterations = 50_000;
    let cauchy = CauchyDispersion::crown_glass();
    let sellmeier = SellmeierDispersion::bk7();

    let start = Instant::now();

    for i in 0..iterations {
        let cos_theta = 0.1 + 0.8 * (i as f64 / iterations as f64);

        // Fresnel evaluation
        if features.spectral_fresnel {
            if features.use_luts {
                let _ = fresnel_rgb_fast(&cauchy, cos_theta);
            } else {
                let _ = fresnel_rgb(&cauchy, cos_theta);
            }
        } else {
            let _ = super::fresnel::fresnel_schlick(1.0, 1.5, cos_theta);
        }

        // Dispersion evaluation
        if features.sellmeier_dispersion {
            let _ = sellmeier.n_rgb();
        } else {
            let _ = cauchy.n_rgb();
        }

        // Scattering evaluation
        if features.dhg_scattering {
            if features.use_luts {
                let _ = dhg_preset(cos_theta, DHGPreset::Milk);
            } else {
                let _ = double_henyey_greenstein(cos_theta, 0.7, -0.2, 0.8);
            }
        } else {
            if features.use_luts {
                let _ = hg_fast(cos_theta, 0.5);
            } else {
                let _ = henyey_greenstein(cos_theta, 0.5);
            }
        }
    }

    let elapsed = start.elapsed();
    let avg_ns = elapsed.as_nanos() as f64 / iterations as f64;
    let throughput = 1e9 / avg_ns;

    // Calculate memory based on LUTs used
    let memory_bytes = if features.use_luts {
        let mut mem = super::lut::total_lut_memory();
        if features.dhg_scattering {
            mem += CompactDHGLUT::global().memory_size();
        }
        mem
    } else {
        0
    };

    let mut features_enabled = Vec::new();
    if features.spectral_fresnel {
        features_enabled.push("Spectral Fresnel".to_string());
    }
    if features.sellmeier_dispersion {
        features_enabled.push("Sellmeier".to_string());
    }
    if features.dhg_scattering {
        features_enabled.push("DHG Scattering".to_string());
    }
    if features.use_luts {
        features_enabled.push("LUTs".to_string());
    }
    if features.chromatic_effects {
        features_enabled.push("Chromatic".to_string());
    }

    TierBenchmark {
        avg_ns,
        throughput,
        memory_bytes,
        features_enabled,
    }
}

// ============================================================================
// MEMORY ANALYSIS
// ============================================================================

/// Analyze total memory usage for Phase 2 features
pub fn memory_analysis() -> Vec<(String, usize)> {
    let mut report = Vec::new();

    // Phase 1 LUTs
    report.push((
        "Fresnel LUT".to_string(),
        super::lut::FresnelLUT::global().memory_size(),
    ));
    report.push((
        "Beer-Lambert LUT".to_string(),
        super::lut::BeerLambertLUT::global().memory_size(),
    ));
    report.push((
        "H-G LUT".to_string(),
        super::scattering::HenyeyGreensteinLUT::global().memory_size(),
    ));
    report.push((
        "Spectral Fresnel LUT".to_string(),
        super::spectral_fresnel::SpectralFresnelLUT::global().memory_size(),
    ));

    // Phase 2 LUTs
    report.push((
        "DHG Full LUT".to_string(),
        DoubleHGLUT::global().memory_size(),
    ));
    report.push((
        "DHG Compact LUT".to_string(),
        CompactDHGLUT::global().memory_size(),
    ));

    // Total
    let total: usize = report.iter().map(|(_, size)| size).sum();
    report.push(("TOTAL".to_string(), total));

    report
}

// ============================================================================
// COMPREHENSIVE REPORT
// ============================================================================

/// Generate full Phase 2 validation report
pub fn full_phase2_report() -> String {
    let mut report = String::new();

    report.push_str("# PBR Phase 2 Validation Report\n\n");

    // DHG vs H-G
    report.push_str("## 1. DHG vs Single H-G Comparison\n\n");
    report.push_str("| Config | Single H-G | DHG LUT | Overhead | Avg Error | Status |\n");
    report.push_str("|--------|------------|---------|----------|-----------|--------|\n");
    for result in compare_dhg_vs_hg() {
        report.push_str(&result.to_row());
        report.push('\n');
    }

    // DHG LUT vs Direct
    report.push_str("\n## 2. DHG LUT vs Direct Calculation\n\n");
    report.push_str("| Preset | Direct | LUT | Speedup | Avg Error | Status |\n");
    report.push_str("|--------|--------|-----|---------|-----------|--------|\n");
    for result in compare_dhg_lut_vs_direct() {
        report.push_str(&result.to_row());
        report.push('\n');
    }

    // Sellmeier vs Cauchy
    report.push_str("\n## 3. Sellmeier vs Cauchy Comparison\n\n");
    report.push_str("| Test | Cauchy | Sellmeier | Overhead | Avg Error | Status |\n");
    report.push_str("|------|--------|-----------|----------|-----------|--------|\n");
    for result in compare_sellmeier_vs_cauchy() {
        report.push_str(&result.to_row());
        report.push('\n');
    }

    // Quality Tier Benchmarks
    report.push_str("\n## 4. Quality Tier Performance\n\n");
    report.push_str("| Tier | Avg Time | Throughput | Memory | Features |\n");
    report.push_str("|------|----------|------------|--------|----------|\n");
    for (tier, benchmark) in benchmark_quality_tiers() {
        report.push_str(&format!(
            "| {:?} | {:.1}ns | {:.1}M/s | {:.1}KB | {} |\n",
            tier,
            benchmark.avg_ns,
            benchmark.throughput / 1e6,
            benchmark.memory_bytes as f64 / 1024.0,
            benchmark.features_enabled.join(", ")
        ));
    }

    // Memory Analysis
    report.push_str("\n## 5. Memory Usage\n\n");
    report.push_str("| Component | Size (KB) |\n");
    report.push_str("|-----------|----------|\n");
    for (name, size) in memory_analysis() {
        report.push_str(&format!("| {} | {:.1} |\n", name, size as f64 / 1024.0));
    }

    report
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dhg_comparison_runs() {
        let results = compare_dhg_vs_hg();
        assert!(!results.is_empty());

        for result in &results {
            assert!(result.time_a_ns > 0.0);
            assert!(result.time_b_ns > 0.0);
        }
    }

    #[test]
    fn test_dhg_lut_comparison() {
        let results = compare_dhg_lut_vs_direct();
        assert!(!results.is_empty());

        for result in &results {
            // In debug mode, LUT might not be faster due to bounds checking
            // In release mode, LUT should be ~3-4x faster
            // We only check accuracy here; speedup is best verified in release builds
            assert!(
                result.accuracy_ok,
                "LUT accuracy should be within tolerance for {:?}",
                result.name
            );
        }
    }

    #[test]
    fn test_sellmeier_cauchy_comparison() {
        let results = compare_sellmeier_vs_cauchy();
        assert!(!results.is_empty());

        // Cauchy should be faster than Sellmeier
        let ior_result = &results[0];
        assert!(ior_result.speedup > 1.0, "Cauchy should be faster");
    }

    #[test]
    fn test_quality_tier_benchmarks() {
        let results = benchmark_quality_tiers();
        assert_eq!(results.len(), 4);

        // Fast should be faster than High
        let fast_result = &results[0].1;
        let high_result = &results[2].1;
        assert!(
            fast_result.throughput > high_result.throughput,
            "Fast tier should have higher throughput"
        );
    }

    #[test]
    fn test_memory_analysis() {
        let analysis = memory_analysis();
        assert!(!analysis.is_empty());

        let total = analysis.iter().find(|(name, _)| name == "TOTAL");
        assert!(total.is_some());

        let (_, total_size) = total.unwrap();
        // Should be under 2MB total
        assert!(
            *total_size < 2_000_000,
            "Total LUT memory should be under 2MB"
        );
    }

    #[test]
    fn test_full_report_generation() {
        let report = full_phase2_report();
        assert!(!report.is_empty());
        assert!(report.contains("Phase 2 Validation"));
        assert!(report.contains("DHG"));
        assert!(report.contains("Sellmeier"));
        assert!(report.contains("Quality Tier"));
    }

    #[test]
    #[ignore] // Long-running benchmark
    fn test_print_full_report() {
        let report = full_phase2_report();
        println!("{}", report);
    }
}
