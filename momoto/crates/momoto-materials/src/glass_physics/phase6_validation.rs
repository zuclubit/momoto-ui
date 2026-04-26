//! # Phase 6 Validation and Benchmarks
//!
//! Comprehensive testing and validation for Phase 6 features:
//! - SIMD batch evaluation performance
//! - Combined effects accuracy
//! - Perceptual loss functions
//! - Material datasets coverage
//!
//! ## Validation Categories
//!
//! 1. Performance benchmarks (SIMD speedup, throughput)
//! 2. Accuracy tests (color space, Delta E)
//! 3. Integration tests (combined effects)
//! 4. Memory analysis

use std::time::Instant;

use super::combined_effects::presets as combined_presets;
use super::material_datasets::{MaterialCategory, MaterialDatabase};
use super::perceptual_loss::{
    delta_e_2000, delta_e_76, delta_e_94, perceptual_loss, rgb_to_lab, Illuminant, LabColor,
    PerceptualLossConfig,
};
use super::simd_batch::{
    beer_lambert_batch, beer_lambert_scalar, fresnel_batch, fresnel_schlick_scalar,
    henyey_greenstein_batch, henyey_greenstein_scalar, SimdBatchEvaluator, SimdBatchInput,
    SimdConfig,
};

// ============================================================================
// VALIDATION STRUCTURES
// ============================================================================

/// Single validation check result
#[derive(Debug, Clone)]
pub struct ValidationCheck {
    pub name: String,
    pub passed: bool,
    pub details: String,
}

/// Validation result for a category
#[derive(Debug, Clone)]
pub struct ValidationResult {
    pub category: String,
    pub checks: Vec<ValidationCheck>,
}

impl ValidationResult {
    pub fn new(category: &str) -> Self {
        Self {
            category: category.to_string(),
            checks: Vec::new(),
        }
    }

    pub fn add_check(&mut self, name: &str, passed: bool, details: String) {
        self.checks.push(ValidationCheck {
            name: name.to_string(),
            passed,
            details,
        });
    }

    pub fn all_passed(&self) -> bool {
        self.checks.iter().all(|c| c.passed)
    }

    pub fn summary(&self) -> String {
        let passed = self.checks.iter().filter(|c| c.passed).count();
        let total = self.checks.len();
        format!("{}: {}/{} passed", self.category, passed, total)
    }

    pub fn to_markdown(&self) -> String {
        let mut md = format!("## {}\n\n", self.category);
        md.push_str("| Check | Status | Details |\n");
        md.push_str("|-------|--------|--------|\n");

        for check in &self.checks {
            let status = if check.passed { "PASS" } else { "FAIL" };
            md.push_str(&format!(
                "| {} | {} | {} |\n",
                check.name, status, check.details
            ));
        }

        md
    }
}

// ============================================================================
// BENCHMARK STRUCTURES
// ============================================================================

/// SIMD benchmark results
#[derive(Debug, Clone)]
pub struct SimdBenchmarks {
    pub scalar_throughput: f64,
    pub batch_throughput: f64,
    pub speedup: f64,
    pub n_materials: usize,
    pub total_time_ms: f64,
}

/// Performance benchmark entry
#[derive(Debug, Clone)]
pub struct BenchmarkEntry {
    pub name: String,
    pub iterations: usize,
    pub total_time_us: u64,
    pub per_iteration_us: f64,
}

/// Phase 6 benchmark results
#[derive(Debug, Clone)]
pub struct Phase6BenchmarkResults {
    pub simd_benchmarks: SimdBenchmarks,
    pub combined_effects_benchmarks: Vec<BenchmarkEntry>,
    pub perceptual_benchmarks: Vec<BenchmarkEntry>,
}

impl Phase6BenchmarkResults {
    pub fn to_markdown(&self) -> String {
        let mut md = String::new();
        md.push_str("## Phase 6 Performance Benchmarks\n\n");

        md.push_str("### SIMD Batch Evaluation\n\n");
        md.push_str(&format!(
            "- Materials: {}\n",
            self.simd_benchmarks.n_materials
        ));
        md.push_str(&format!(
            "- Scalar throughput: {:.0} ops/sec\n",
            self.simd_benchmarks.scalar_throughput
        ));
        md.push_str(&format!(
            "- Batch throughput: {:.0} ops/sec\n",
            self.simd_benchmarks.batch_throughput
        ));
        md.push_str(&format!(
            "- Speedup: {:.2}x\n\n",
            self.simd_benchmarks.speedup
        ));

        md.push_str("### Combined Effects\n\n");
        md.push_str("| Operation | Iterations | Total (µs) | Per Iter (µs) |\n");
        md.push_str("|-----------|------------|------------|---------------|\n");
        for b in &self.combined_effects_benchmarks {
            md.push_str(&format!(
                "| {} | {} | {} | {:.2} |\n",
                b.name, b.iterations, b.total_time_us, b.per_iteration_us
            ));
        }

        md.push_str("\n### Perceptual Functions\n\n");
        md.push_str("| Operation | Iterations | Total (µs) | Per Iter (µs) |\n");
        md.push_str("|-----------|------------|------------|---------------|\n");
        for b in &self.perceptual_benchmarks {
            md.push_str(&format!(
                "| {} | {} | {} | {:.2} |\n",
                b.name, b.iterations, b.total_time_us, b.per_iteration_us
            ));
        }

        md
    }
}

// ============================================================================
// MEMORY ANALYSIS
// ============================================================================

/// Phase 6 memory analysis
#[derive(Debug, Clone)]
pub struct Phase6MemoryAnalysis {
    pub perceptual_luts: usize,
    pub material_datasets: usize,
    pub simd_buffers: usize,
    pub combined_effects: usize,
    pub total_phase6: usize,
    pub total_all_phases: usize,
}

impl Phase6MemoryAnalysis {
    pub fn analyze() -> Self {
        let perceptual = super::perceptual_loss::total_perceptual_memory();
        let datasets = super::material_datasets::total_datasets_memory();
        let simd = super::simd_batch::total_simd_memory();
        let combined = super::combined_effects::total_combined_memory();

        let total_phase6 = perceptual + datasets + simd + combined;

        // Estimate total from all phases
        let total_all = total_phase6 + 500_000; // ~500KB from Phases 1-5

        Self {
            perceptual_luts: perceptual,
            material_datasets: datasets,
            simd_buffers: simd,
            combined_effects: combined,
            total_phase6,
            total_all_phases: total_all,
        }
    }

    pub fn to_markdown(&self) -> String {
        let mut md = String::new();
        md.push_str("## Phase 6 Memory Analysis\n\n");
        md.push_str("| Component | Size (bytes) | Size (KB) |\n");
        md.push_str("|-----------|--------------|----------|\n");

        md.push_str(&format!(
            "| Perceptual LUTs | {} | {:.2} |\n",
            self.perceptual_luts,
            self.perceptual_luts as f64 / 1024.0
        ));
        md.push_str(&format!(
            "| Material Datasets | {} | {:.2} |\n",
            self.material_datasets,
            self.material_datasets as f64 / 1024.0
        ));
        md.push_str(&format!(
            "| SIMD Buffers | {} | {:.2} |\n",
            self.simd_buffers,
            self.simd_buffers as f64 / 1024.0
        ));
        md.push_str(&format!(
            "| Combined Effects | {} | {:.2} |\n",
            self.combined_effects,
            self.combined_effects as f64 / 1024.0
        ));
        md.push_str(&format!(
            "| **Total Phase 6** | **{}** | **{:.2}** |\n",
            self.total_phase6,
            self.total_phase6 as f64 / 1024.0
        ));
        md.push_str(&format!(
            "| Total All Phases | {} | {:.2} |\n",
            self.total_all_phases,
            self.total_all_phases as f64 / 1024.0
        ));

        md
    }
}

// ============================================================================
// PERCEPTUAL LOSS VALIDATION
// ============================================================================

/// Validate perceptual loss functions
pub fn validate_perceptual_loss() -> ValidationResult {
    let mut result = ValidationResult::new("Perceptual Loss");

    // Test 1: Color space round-trip
    {
        let rgb = [0.5, 0.3, 0.8];
        let lab = rgb_to_lab(rgb, Illuminant::D65);
        let back = super::perceptual_loss::lab_to_rgb(lab, Illuminant::D65);

        let error = (rgb[0] - back[0]).abs() + (rgb[1] - back[1]).abs() + (rgb[2] - back[2]).abs();
        result.add_check(
            "RGB-LAB round-trip",
            error < 0.001,
            format!("error = {:.6}", error),
        );
    }

    // Test 2: Delta E same color
    {
        let lab = LabColor::new(50.0, 25.0, -30.0);
        let de = delta_e_2000(lab, lab);
        result.add_check("Delta E same color", de < 1e-10, format!("ΔE = {:.10}", de));
    }

    // Test 3: Delta E formulas consistency
    {
        let lab1 = LabColor::new(50.0, 25.0, -30.0);
        let lab2 = LabColor::new(52.0, 27.0, -28.0);

        let de76 = delta_e_76(lab1, lab2);
        let de94 = delta_e_94(lab1, lab2);
        let de2000 = delta_e_2000(lab1, lab2);

        // All should be positive and in similar range
        let consistent =
            de76 > 0.0 && de94 > 0.0 && de2000 > 0.0 && de76 < 20.0 && de94 < 20.0 && de2000 < 20.0;

        result.add_check(
            "Delta E formulas consistent",
            consistent,
            format!("ΔE76={:.2}, ΔE94={:.2}, ΔE2000={:.2}", de76, de94, de2000),
        );
    }

    // Test 4: White point
    {
        let white_lab = rgb_to_lab([1.0, 1.0, 1.0], Illuminant::D65);
        let is_white =
            (white_lab.l - 100.0).abs() < 0.5 && white_lab.a.abs() < 0.5 && white_lab.b.abs() < 0.5;

        result.add_check(
            "White point accuracy",
            is_white,
            format!(
                "L*={:.2}, a*={:.2}, b*={:.2}",
                white_lab.l, white_lab.a, white_lab.b
            ),
        );
    }

    // Test 5: Perceptual loss gradient
    {
        let rendered = vec![[0.5, 0.3, 0.8]];
        let reference = vec![[0.52, 0.32, 0.78]];
        let config = PerceptualLossConfig::default();

        let loss = perceptual_loss(&rendered, &reference, &config);
        result.add_check(
            "Perceptual loss computation",
            loss > 0.0 && loss < 10.0,
            format!("loss = {:.4}", loss),
        );
    }

    result
}

// ============================================================================
// MATERIAL DATASETS VALIDATION
// ============================================================================

/// Validate material datasets
pub fn validate_material_datasets() -> ValidationResult {
    let mut result = ValidationResult::new("Material Datasets");

    let db = MaterialDatabase::builtin();

    // Test 1: Database population
    {
        result.add_check(
            "Database populated",
            db.len() == 10,
            format!("{} materials", db.len()),
        );
    }

    // Test 2: Material lookup
    {
        let bk7 = db.get("BK7 Glass");
        result.add_check(
            "Material lookup",
            bk7.is_some(),
            format!("BK7 found: {}", bk7.is_some()),
        );
    }

    // Test 3: Wavelength coverage
    {
        let bk7 = db.get("BK7 Glass").unwrap();
        let (min_w, max_w) = bk7.wavelength_range();
        let covers_visible = min_w <= 400.0 && max_w >= 700.0;

        result.add_check(
            "Visible spectrum coverage",
            covers_visible,
            format!("{:.0}-{:.0} nm", min_w, max_w),
        );
    }

    // Test 4: Category filtering
    {
        let metals = db.by_category(MaterialCategory::Metal);
        result.add_check(
            "Metal category",
            metals.len() >= 4,
            format!("{} metals", metals.len()),
        );
    }

    // Test 5: Similar material search
    {
        let bk7 = db.get("BK7 Glass").unwrap();
        let similar = db.find_similar(bk7, 3);

        // First should be BK7 itself
        result.add_check(
            "Similar search",
            similar.len() == 3 && similar[0].1 < 0.01,
            format!("{} results, best error {:.4}", similar.len(), similar[0].1),
        );
    }

    // Test 6: Reflectance interpolation
    {
        let gold = db.get("Gold").unwrap();
        let r_550 = gold.reflectance_at(550.0);

        result.add_check(
            "Reflectance interpolation",
            r_550 > 0.0 && r_550 < 1.0,
            format!("R(550nm) = {:.3}", r_550),
        );
    }

    result
}

// ============================================================================
// SIMD BATCH VALIDATION
// ============================================================================

/// Validate SIMD batch evaluation
pub fn validate_simd_batch() -> ValidationResult {
    let mut result = ValidationResult::new("SIMD Batch Evaluation");

    // Test 1: Scalar vs batch consistency
    {
        let n = 100;
        let cos_theta: Vec<f64> = (0..n).map(|i| (i as f64) / (n as f64 - 1.0)).collect();
        let ior = vec![1.5; n];
        let mut batch_out = vec![0.0; n];

        fresnel_batch(&cos_theta, &ior, &mut batch_out);

        let mut max_error: f64 = 0.0;
        for i in 0..n {
            let scalar = fresnel_schlick_scalar(cos_theta[i], ior[i]);
            let error = (batch_out[i] - scalar).abs();
            max_error = max_error.max(error);
        }

        result.add_check(
            "Fresnel batch consistency",
            max_error < 1e-10,
            format!("max error = {:.2e}", max_error),
        );
    }

    // Test 2: Beer-Lambert batch
    {
        let n = 100;
        let absorption = vec![0.1; n];
        let thickness = vec![10.0; n];
        let mut out = vec![0.0; n];

        beer_lambert_batch(&absorption, &thickness, &mut out);

        let expected = beer_lambert_scalar(0.1, 10.0);
        let consistent = out.iter().all(|&v| (v - expected).abs() < 1e-10);

        result.add_check(
            "Beer-Lambert batch",
            consistent,
            format!("expected {:.4}, got {:.4}", expected, out[0]),
        );
    }

    // Test 3: HG batch
    {
        let n = 100;
        let cos_theta = vec![0.5; n];
        let g = vec![0.7; n];
        let mut out = vec![0.0; n];

        henyey_greenstein_batch(&cos_theta, &g, &mut out);

        let expected = henyey_greenstein_scalar(0.5, 0.7);
        let consistent = out.iter().all(|&v| (v - expected).abs() < 1e-10);

        result.add_check(
            "HG phase batch",
            consistent,
            format!("expected {:.6}, got {:.6}", expected, out[0]),
        );
    }

    // Test 4: Full evaluator
    {
        let input = SimdBatchInput::uniform(1000, 1.5, 0.7, 0.01, 10.0);
        let evaluator = SimdBatchEvaluator::default();
        let output = evaluator.evaluate(&input);

        let valid = output.len() == 1000
            && output.fresnel.iter().all(|&f| f >= 0.0 && f <= 1.0)
            && output.transmittance.iter().all(|&t| t >= 0.0 && t <= 1.0);

        result.add_check(
            "Full batch evaluator",
            valid,
            format!("{} results", output.len()),
        );
    }

    result
}

// ============================================================================
// COMBINED EFFECTS VALIDATION
// ============================================================================

/// Validate combined effects
pub fn validate_combined_effects() -> ValidationResult {
    let mut result = ValidationResult::new("Combined Effects");

    // Test 1: Glass preset
    {
        let glass = combined_presets::glass();
        let r = glass.evaluate(550.0, 1.0);

        // Glass at normal incidence: ~4%
        result.add_check(
            "Glass reflectance",
            (r - 0.04).abs() < 0.02,
            format!("R = {:.3}", r),
        );
    }

    // Test 2: Soap bubble iridescence
    {
        let bubble = combined_presets::soap_bubble();
        let r_blue = bubble.evaluate(450.0, 0.8);
        let r_red = bubble.evaluate(650.0, 0.8);

        // Should show wavelength dependence
        result.add_check(
            "Soap bubble iridescence",
            (r_blue - r_red).abs() > 0.01,
            format!("R(450)={:.3}, R(650)={:.3}", r_blue, r_red),
        );
    }

    // Test 3: Metal reflectance
    {
        let patina = combined_presets::copper_patina();
        let r = patina.evaluate(550.0, 1.0);

        // Oxidized copper still has high reflectance (copper base ~95%)
        // With oxidation layers, expect moderate to high reflectance
        result.add_check(
            "Copper patina",
            r > 0.1 && r < 0.98,
            format!("R = {:.3}", r),
        );
    }

    // Test 4: RGB output
    {
        let opal = combined_presets::opal_glass();
        let rgb = opal.evaluate_rgb(0.7);

        let valid_rgb = rgb.iter().all(|&c| c >= 0.0 && c <= 1.0);
        result.add_check(
            "RGB output valid",
            valid_rgb,
            format!("RGB = [{:.3}, {:.3}, {:.3}]", rgb[0], rgb[1], rgb[2]),
        );
    }

    // Test 5: Spectral output
    {
        let morpho = combined_presets::morpho_wing();
        let spectrum = morpho.evaluate_spectral(0.8);

        result.add_check(
            "Spectral output",
            spectrum.len() == 31,
            format!("{} wavelengths", spectrum.len()),
        );
    }

    // Test 6: CSS output
    {
        let pearl = combined_presets::pearl();
        let css = pearl.to_css(30.0);

        result.add_check(
            "CSS generation",
            css.contains("gradient") && css.contains("rgb"),
            format!("{} chars", css.len()),
        );
    }

    result
}

// ============================================================================
// BENCHMARKS
// ============================================================================

/// Benchmark SIMD performance
pub fn benchmark_simd() -> SimdBenchmarks {
    let n_materials = 10_000;

    // Prepare input
    let input = SimdBatchInput::uniform(n_materials, 1.5, 0.7, 0.01, 10.0);
    let evaluator = SimdBatchEvaluator::new(SimdConfig::vectorized());

    // Benchmark scalar
    let start = Instant::now();
    let iterations = 100;
    for _ in 0..iterations {
        for i in 0..n_materials {
            let _f = fresnel_schlick_scalar(input.cos_theta[i], input.ior[i]);
            let _t = beer_lambert_scalar(input.absorption[i], input.thickness[i]);
            let _p = henyey_greenstein_scalar(input.cos_theta[i], input.g[i]);
        }
    }
    let scalar_time = start.elapsed();
    let scalar_throughput = (n_materials * iterations) as f64 / scalar_time.as_secs_f64();

    // Benchmark batch
    let start = Instant::now();
    for _ in 0..iterations {
        let _result = evaluator.evaluate(&input);
    }
    let batch_time = start.elapsed();
    let batch_throughput = (n_materials * iterations) as f64 / batch_time.as_secs_f64();

    SimdBenchmarks {
        scalar_throughput,
        batch_throughput,
        speedup: batch_throughput / scalar_throughput,
        n_materials,
        total_time_ms: batch_time.as_secs_f64() * 1000.0,
    }
}

/// Benchmark Phase 6 features
pub fn benchmark_phase6() -> Phase6BenchmarkResults {
    let simd_benchmarks = benchmark_simd();
    let mut combined_effects_benchmarks = Vec::new();
    let mut perceptual_benchmarks = Vec::new();

    // Benchmark combined effects
    {
        let materials = vec![
            ("Glass", combined_presets::glass()),
            ("Soap Bubble", combined_presets::soap_bubble()),
            ("Copper Patina", combined_presets::copper_patina()),
            ("Opal Glass", combined_presets::opal_glass()),
        ];

        for (name, material) in materials {
            let start = Instant::now();
            let iterations = 10_000;

            for _ in 0..iterations {
                let _ = material.evaluate_rgb(0.7);
            }

            let elapsed = start.elapsed();
            combined_effects_benchmarks.push(BenchmarkEntry {
                name: name.to_string(),
                iterations,
                total_time_us: elapsed.as_micros() as u64,
                per_iteration_us: elapsed.as_micros() as f64 / iterations as f64,
            });
        }
    }

    // Benchmark perceptual functions
    {
        let rgb = [0.5, 0.3, 0.8];
        let lab = rgb_to_lab(rgb, Illuminant::D65);
        let lab2 = LabColor::new(52.0, 27.0, -28.0);

        // RGB to LAB
        let start = Instant::now();
        let iterations = 100_000;
        for _ in 0..iterations {
            let _ = rgb_to_lab(rgb, Illuminant::D65);
        }
        let elapsed = start.elapsed();
        perceptual_benchmarks.push(BenchmarkEntry {
            name: "RGB to LAB".to_string(),
            iterations,
            total_time_us: elapsed.as_micros() as u64,
            per_iteration_us: elapsed.as_micros() as f64 / iterations as f64,
        });

        // Delta E 2000
        let start = Instant::now();
        for _ in 0..iterations {
            let _ = delta_e_2000(lab, lab2);
        }
        let elapsed = start.elapsed();
        perceptual_benchmarks.push(BenchmarkEntry {
            name: "Delta E 2000".to_string(),
            iterations,
            total_time_us: elapsed.as_micros() as u64,
            per_iteration_us: elapsed.as_micros() as f64 / iterations as f64,
        });
    }

    Phase6BenchmarkResults {
        simd_benchmarks,
        combined_effects_benchmarks,
        perceptual_benchmarks,
    }
}

// ============================================================================
// FULL VALIDATION
// ============================================================================

/// Run all Phase 6 validations
pub fn run_all_validations() -> Vec<ValidationResult> {
    vec![
        validate_perceptual_loss(),
        validate_material_datasets(),
        validate_simd_batch(),
        validate_combined_effects(),
    ]
}

/// Generate full Phase 6 validation report
pub fn generate_validation_report() -> String {
    let mut report = String::new();
    report.push_str("# Phase 6 Validation Report\n\n");

    let validations = run_all_validations();
    let all_passed = validations.iter().all(|v| v.all_passed());

    report.push_str(&format!(
        "**Overall Status:** {}\n\n",
        if all_passed {
            "ALL PASSED"
        } else {
            "SOME FAILURES"
        }
    ));

    report.push_str("## Summary\n\n");
    for v in &validations {
        let status = if v.all_passed() { "OK" } else { "FAIL" };
        report.push_str(&format!("- {} {}\n", status, v.summary()));
    }
    report.push('\n');

    for v in &validations {
        report.push_str(&v.to_markdown());
        report.push('\n');
    }

    let benchmarks = benchmark_phase6();
    report.push_str(&benchmarks.to_markdown());
    report.push('\n');

    let memory = Phase6MemoryAnalysis::analyze();
    report.push_str(&memory.to_markdown());

    report
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_perceptual_validation() {
        let result = validate_perceptual_loss();
        assert!(
            result.all_passed(),
            "Perceptual validation failed: {}",
            result.summary()
        );
    }

    #[test]
    fn test_material_datasets_validation() {
        let result = validate_material_datasets();
        assert!(
            result.all_passed(),
            "Material datasets validation failed: {}",
            result.summary()
        );
    }

    #[test]
    fn test_simd_batch_validation() {
        let result = validate_simd_batch();
        assert!(
            result.all_passed(),
            "SIMD batch validation failed: {}",
            result.summary()
        );
    }

    #[test]
    fn test_combined_effects_validation() {
        let result = validate_combined_effects();
        if !result.all_passed() {
            eprintln!("{}", result.to_markdown());
        }
        assert!(
            result.all_passed(),
            "Combined effects validation failed: {}",
            result.summary()
        );
    }

    #[test]
    fn test_benchmarks_run() {
        let benchmarks = benchmark_phase6();

        // SIMD speedup varies by system load and CPU. In test environments,
        // timing can be inconsistent. We just verify it runs without being
        // catastrophically slow (>10x slower than scalar would indicate a bug).
        assert!(
            benchmarks.simd_benchmarks.speedup > 0.1,
            "SIMD speedup catastrophically low: {}",
            benchmarks.simd_benchmarks.speedup
        );

        // Combined effects should complete
        assert!(!benchmarks.combined_effects_benchmarks.is_empty());

        // Perceptual should be fast
        for b in &benchmarks.perceptual_benchmarks {
            assert!(
                b.per_iteration_us < 100.0,
                "{} too slow: {} µs",
                b.name,
                b.per_iteration_us
            );
        }
    }

    #[test]
    fn test_memory_analysis() {
        let memory = Phase6MemoryAnalysis::analyze();

        // Phase 6 should be under 500KB
        assert!(
            memory.total_phase6 < 500_000,
            "Phase 6 memory too high: {} bytes",
            memory.total_phase6
        );
    }

    #[test]
    fn test_full_report_generation() {
        let report = generate_validation_report();

        assert!(report.contains("Phase 6 Validation Report"));
        assert!(report.contains("Summary"));
        assert!(report.contains("Memory Analysis"));
    }

    #[test]
    fn test_all_validations_pass() {
        let validations = run_all_validations();
        for v in &validations {
            assert!(v.all_passed(), "Validation '{}' failed", v.category);
        }
    }
}
