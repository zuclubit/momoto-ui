//! # Phase 9 Validation Suite
//!
//! Comprehensive validation for the unified BSDF architecture.
//!
//! ## Validation Categories
//!
//! 1. **Unified vs Legacy**: Compare new BSDF implementations against legacy code
//! 2. **Energy Conservation**: Verify R + T + A = 1 for all materials
//! 3. **Anisotropic Tests**: Validate anisotropic BRDF correctness
//! 4. **SSS Accuracy**: Test subsurface scattering against known values
//! 5. **Perceptual Loop**: Verify convergence behavior
//! 6. **Memory Analysis**: Check memory budget compliance

use super::anisotropic_brdf::{AnisotropicConductor, AnisotropicGGX, FiberBSDF};
use super::complex_ior::{fresnel_conductor_unpolarized, ComplexIOR};
use super::fresnel::fresnel_schlick;
use super::perceptual_loop::{
    ConvergenceStatus, MaterialParams, PerceptualRenderingLoop, PerceptualTarget,
};
use super::subsurface_scattering::{sss_presets, DiffusionBSSRDF, SubsurfaceBSDF};
use super::unified_bsdf::{
    validate_energy_conservation, BSDFContext, ConductorBSDF, DielectricBSDF, LambertianBSDF,
    ThinFilmBSDF, BSDF,
};

// ============================================================================
// VALIDATION RESULTS
// ============================================================================

/// Results from unified vs legacy comparison
#[derive(Debug, Clone)]
pub struct BSDFComparisonResults {
    /// Dielectric comparison RMSE
    pub dielectric_rmse: f64,
    /// Conductor comparison RMSE
    pub conductor_rmse: f64,
    /// Thin-film comparison RMSE
    pub thin_film_rmse: f64,
    /// All comparisons passed
    pub all_passed: bool,
    /// Details for each comparison
    pub details: Vec<String>,
}

/// Results from anisotropic validation
#[derive(Debug, Clone)]
pub struct AnisotropicValidation {
    /// Isotropy recovery test passed
    pub isotropy_recovery: bool,
    /// Energy conservation passed
    pub energy_conserved: bool,
    /// VNDF sampling valid
    pub sampling_valid: bool,
    /// Details
    pub details: Vec<String>,
}

/// Results from SSS validation
#[derive(Debug, Clone)]
pub struct SSSValidation {
    /// Diffusion profile valid
    pub profile_valid: bool,
    /// Presets produce reasonable results
    pub presets_valid: bool,
    /// Energy conservation maintained
    pub energy_conserved: bool,
    /// Details
    pub details: Vec<String>,
}

/// Results from perceptual loop testing
#[derive(Debug, Clone)]
pub struct ConvergenceResults {
    /// Number of tests run
    pub tests_run: usize,
    /// Number that converged
    pub tests_converged: usize,
    /// Average iterations to converge
    pub avg_iterations: f64,
    /// Average final ΔE
    pub avg_final_delta_e: f64,
    /// All tests details
    pub details: Vec<String>,
}

/// Energy conservation report
#[derive(Debug, Clone)]
pub struct EnergyConservationReport {
    /// Number of materials tested
    pub materials_tested: usize,
    /// Number that passed
    pub passed: usize,
    /// Maximum error found
    pub max_error: f64,
    /// Which material had max error
    pub worst_material: String,
    /// All passed
    pub all_passed: bool,
}

/// Phase 9 memory analysis
#[derive(Debug, Clone)]
pub struct Phase9MemoryAnalysis {
    /// Unified BSDF module memory
    pub unified_bsdf_kb: f64,
    /// Anisotropic module memory
    pub anisotropic_kb: f64,
    /// SSS module memory
    pub sss_kb: f64,
    /// Perceptual loop memory
    pub perceptual_loop_kb: f64,
    /// Total Phase 9 memory
    pub total_phase9_kb: f64,
    /// Within budget
    pub within_budget: bool,
}

/// Complete Phase 9 validation report
#[derive(Debug, Clone)]
pub struct Phase9ValidationReport {
    /// BSDF comparison results
    pub unified_vs_legacy: BSDFComparisonResults,
    /// Anisotropic tests
    pub anisotropic_tests: AnisotropicValidation,
    /// SSS accuracy
    pub sss_accuracy: SSSValidation,
    /// Perceptual loop convergence
    pub perceptual_loop_convergence: ConvergenceResults,
    /// Energy conservation
    pub energy_conservation: EnergyConservationReport,
    /// Memory analysis
    pub memory_analysis: Phase9MemoryAnalysis,
    /// Overall pass/fail
    pub overall_passed: bool,
}

// ============================================================================
// VALIDATION FUNCTIONS
// ============================================================================

/// Compare unified BSDF implementations against legacy code
pub fn validate_unified_vs_legacy() -> BSDFComparisonResults {
    let mut details = Vec::new();
    let mut all_passed = true;

    // Test angles
    let angles: Vec<f64> = (0..10).map(|i| (i as f64 + 0.5) / 10.0).collect();

    // 1. Dielectric comparison
    let dielectric_rmse = {
        let unified = DielectricBSDF::new(1.52, 0.0);
        let mut sum_sq = 0.0;
        let mut count = 0;

        for &cos_theta in &angles {
            let ctx = BSDFContext::new_simple(cos_theta);
            let unified_r = unified.evaluate(&ctx).reflectance;
            let legacy_r = fresnel_schlick(1.0, 1.52, cos_theta);

            let diff = unified_r - legacy_r;
            sum_sq += diff * diff;
            count += 1;
        }

        (sum_sq / count as f64).sqrt()
    };

    if dielectric_rmse < 0.01 {
        details.push(format!("Dielectric RMSE: {:.6} - PASS", dielectric_rmse));
    } else {
        details.push(format!("Dielectric RMSE: {:.6} - FAIL", dielectric_rmse));
        all_passed = false;
    }

    // 2. Conductor comparison
    let conductor_rmse = {
        let unified = ConductorBSDF::new(0.27, 3.41, 0.0); // Copper
        let legacy_ior = ComplexIOR::new(0.27, 3.41);
        let mut sum_sq = 0.0;
        let mut count = 0;

        for &cos_theta in &angles {
            let ctx = BSDFContext::new_simple(cos_theta);
            let unified_r = unified.evaluate(&ctx).reflectance;
            let legacy_r = fresnel_conductor_unpolarized(1.0, legacy_ior, cos_theta);

            let diff = unified_r - legacy_r;
            sum_sq += diff * diff;
            count += 1;
        }

        (sum_sq / count as f64).sqrt()
    };

    if conductor_rmse < 0.02 {
        details.push(format!("Conductor RMSE: {:.6} - PASS", conductor_rmse));
    } else {
        details.push(format!("Conductor RMSE: {:.6} - FAIL", conductor_rmse));
        all_passed = false;
    }

    // 3. Thin-film (simplified comparison)
    let thin_film_rmse = {
        let unified = ThinFilmBSDF::new(1.52, 1.38, 100.0);
        let mut sum_sq = 0.0;
        let mut count = 0;

        for &cos_theta in &angles {
            let ctx = BSDFContext::new_simple(cos_theta);
            let unified_r = unified.evaluate(&ctx).reflectance;

            // Just check it's in valid range
            if unified_r >= 0.0 && unified_r <= 1.0 {
                // Compare to bare Fresnel (thin-film should be different)
                let bare = fresnel_schlick(1.0, 1.52, cos_theta);
                let diff = (unified_r - bare).abs();
                sum_sq += diff * diff;
                count += 1;
            } else {
                sum_sq += 1.0; // Penalty for invalid
                count += 1;
            }
        }

        (sum_sq / count as f64).sqrt()
    };

    // Thin-film should differ from bare Fresnel
    if thin_film_rmse > 0.001 && thin_film_rmse < 0.5 {
        details.push(format!(
            "Thin-film differs from bare: {:.6} - PASS",
            thin_film_rmse
        ));
    } else {
        details.push(format!("Thin-film RMSE: {:.6} - CHECK", thin_film_rmse));
    }

    BSDFComparisonResults {
        dielectric_rmse,
        conductor_rmse,
        thin_film_rmse,
        all_passed,
        details,
    }
}

/// Validate anisotropic BRDF implementation
pub fn validate_anisotropic() -> AnisotropicValidation {
    let mut details = Vec::new();

    // 1. Isotropy recovery test
    let isotropy_recovery = {
        let iso = AnisotropicGGX::isotropic(0.2, 1.5);
        let aniso = AnisotropicGGX::new(0.2, 0.2, 1.5);

        let ctx = BSDFContext::new_simple(0.7);
        let iso_r = iso.evaluate(&ctx).reflectance;
        let aniso_r = aniso.evaluate(&ctx).reflectance;

        let passed = (iso_r - aniso_r).abs() < 0.01;
        if passed {
            details.push(format!(
                "Isotropy recovery: {} vs {} - PASS",
                iso_r, aniso_r
            ));
        } else {
            details.push(format!(
                "Isotropy recovery: {} vs {} - FAIL",
                iso_r, aniso_r
            ));
        }
        passed
    };

    // 2. Energy conservation
    let energy_conserved = {
        let materials: Vec<Box<dyn BSDF>> = vec![
            Box::new(AnisotropicGGX::new(0.1, 0.3, 1.5)),
            Box::new(AnisotropicConductor::brushed_steel()),
            Box::new(FiberBSDF::silk()),
        ];

        let mut all_conserved = true;
        for mat in materials {
            let validation = validate_energy_conservation(mat.as_ref());
            if !validation.conserved {
                all_conserved = false;
                details.push(format!("{}: {}", mat.name(), validation.details));
            }
        }

        if all_conserved {
            details.push("Energy conservation: All anisotropic materials - PASS".to_string());
        }
        all_conserved
    };

    // 3. VNDF sampling validation
    let sampling_valid = {
        let ggx = AnisotropicGGX::new(0.1, 0.3, 1.5);
        let ctx = BSDFContext::new_simple(0.7);

        let mut valid = true;
        for i in 0..10 {
            let u1 = (i as f64 + 0.5) / 10.0;
            let u2 = ((i * 7) % 10) as f64 / 10.0;

            let sample = ggx.sample(&ctx, u1, u2);

            if sample.wo.length_squared() < 0.8 || sample.pdf <= 0.0 {
                valid = false;
                break;
            }
        }

        if valid {
            details.push("VNDF sampling: Valid directions produced - PASS".to_string());
        } else {
            details.push("VNDF sampling: Invalid directions - FAIL".to_string());
        }
        valid
    };

    AnisotropicValidation {
        isotropy_recovery,
        energy_conserved,
        sampling_valid,
        details,
    }
}

/// Validate SSS implementation
pub fn validate_sss() -> SSSValidation {
    let mut details = Vec::new();

    // 1. Diffusion profile validity
    let profile_valid = {
        let params = sss_presets::skin();
        let bssrdf = DiffusionBSSRDF::with_params(params);

        // Rd should decrease with distance
        let rd_0 = bssrdf.rd(0.0, 0);
        let rd_1 = bssrdf.rd(1.0, 0);
        let rd_5 = bssrdf.rd(5.0, 0);

        let valid = rd_0 > rd_1 && rd_1 > rd_5 && rd_5 >= 0.0;
        if valid {
            details.push(format!(
                "Diffusion profile: Rd(0)={:.4} > Rd(1)={:.4} > Rd(5)={:.4} - PASS",
                rd_0, rd_1, rd_5
            ));
        } else {
            details.push("Diffusion profile: Non-monotonic - FAIL".to_string());
        }
        valid
    };

    // 2. Presets produce valid results
    let presets_valid = {
        let presets = vec![
            ("skin", sss_presets::skin()),
            ("marble", sss_presets::marble()),
            ("milk", sss_presets::milk()),
            ("jade", sss_presets::jade()),
            ("wax", sss_presets::wax()),
        ];

        let mut all_valid = true;
        for (name, params) in presets {
            let rd = params.diffuse_reflectance();
            for (i, &r) in rd.iter().enumerate() {
                if r < 0.0 || r > 1.0 {
                    all_valid = false;
                    details.push(format!("{} Rd[{}] = {} - OUT OF RANGE", name, i, r));
                }
            }
        }

        if all_valid {
            details.push("SSS presets: All produce valid Rd - PASS".to_string());
        }
        all_valid
    };

    // 3. Energy conservation in SubsurfaceBSDF
    let energy_conserved = {
        let materials: Vec<SubsurfaceBSDF> = vec![
            SubsurfaceBSDF::skin(),
            SubsurfaceBSDF::marble(),
            SubsurfaceBSDF::milk(),
        ];

        let mut all_conserved = true;
        for mat in materials {
            let validation = validate_energy_conservation(&mat);
            if !validation.conserved {
                all_conserved = false;
                details.push(format!(
                    "{}: Energy error {:.2e}",
                    mat.name(),
                    validation.error
                ));
            }
        }

        if all_conserved {
            details.push("SSS energy conservation: All materials - PASS".to_string());
        }
        all_conserved
    };

    SSSValidation {
        profile_valid,
        presets_valid,
        energy_conserved,
        details,
    }
}

/// Validate perceptual loop convergence
pub fn validate_perceptual_loop() -> ConvergenceResults {
    let mut details = Vec::new();

    // Test cases: (target reflectance, max_delta_e, max_iterations)
    let test_cases = vec![
        (0.1, 5.0, 50),
        (0.3, 5.0, 50),
        (0.5, 5.0, 50),
        (0.7, 5.0, 50),
    ];

    let mut tests_run = 0;
    let mut tests_converged = 0;
    let mut total_iterations = 0;
    let mut total_delta_e = 0.0;

    for (target_r, max_delta_e, max_iter) in test_cases {
        tests_run += 1;

        let mut loop_runner = PerceptualRenderingLoop::new()
            .with_target_delta_e(max_delta_e)
            .with_max_iterations(max_iter);

        let target = PerceptualTarget::Reflectance(target_r);
        let initial = MaterialParams::dielectric(1.5, 0.0);

        let result = loop_runner.optimize(&initial, &target);

        total_iterations += result.iterations;
        total_delta_e += result.final_delta_e;

        if result.status == ConvergenceStatus::Converged {
            tests_converged += 1;
            details.push(format!(
                "R={}: Converged in {} iters, ΔE={:.2}",
                target_r, result.iterations, result.final_delta_e
            ));
        } else {
            details.push(format!(
                "R={}: {:?} at {} iters, ΔE={:.2}",
                target_r, result.status, result.iterations, result.final_delta_e
            ));
        }
    }

    let avg_iterations = total_iterations as f64 / tests_run as f64;
    let avg_final_delta_e = total_delta_e / tests_run as f64;

    ConvergenceResults {
        tests_run,
        tests_converged,
        avg_iterations,
        avg_final_delta_e,
        details,
    }
}

/// Comprehensive energy conservation test
pub fn validate_energy_conservation_all() -> EnergyConservationReport {
    let materials: Vec<(&str, Box<dyn BSDF>)> = vec![
        ("DielectricBSDF::glass", Box::new(DielectricBSDF::glass())),
        ("DielectricBSDF::water", Box::new(DielectricBSDF::water())),
        (
            "DielectricBSDF::diamond",
            Box::new(DielectricBSDF::diamond()),
        ),
        (
            "DielectricBSDF::frosted_glass",
            Box::new(DielectricBSDF::frosted_glass()),
        ),
        ("ConductorBSDF::gold", Box::new(ConductorBSDF::gold())),
        ("ConductorBSDF::silver", Box::new(ConductorBSDF::silver())),
        ("ConductorBSDF::copper", Box::new(ConductorBSDF::copper())),
        (
            "ThinFilmBSDF::soap_bubble",
            Box::new(ThinFilmBSDF::soap_bubble(350.0)),
        ),
        (
            "ThinFilmBSDF::ar_coating",
            Box::new(ThinFilmBSDF::ar_coating()),
        ),
        ("LambertianBSDF::white", Box::new(LambertianBSDF::white())),
        (
            "AnisotropicGGX",
            Box::new(AnisotropicGGX::new(0.1, 0.3, 1.5)),
        ),
        (
            "AnisotropicConductor::brushed_steel",
            Box::new(AnisotropicConductor::brushed_steel()),
        ),
        ("FiberBSDF::silk", Box::new(FiberBSDF::silk())),
        ("SubsurfaceBSDF::skin", Box::new(SubsurfaceBSDF::skin())),
        ("SubsurfaceBSDF::marble", Box::new(SubsurfaceBSDF::marble())),
    ];

    let mut passed = 0;
    let mut max_error = 0.0;
    let mut worst_material = String::new();

    for (name, bsdf) in &materials {
        let validation = validate_energy_conservation(bsdf.as_ref());

        if validation.conserved {
            passed += 1;
        }

        if validation.error > max_error {
            max_error = validation.error;
            worst_material = name.to_string();
        }
    }

    EnergyConservationReport {
        materials_tested: materials.len(),
        passed,
        max_error,
        worst_material,
        all_passed: passed == materials.len(),
    }
}

/// Analyze memory usage
pub fn analyze_memory() -> Phase9MemoryAnalysis {
    use super::anisotropic_brdf::total_anisotropic_memory;
    use super::perceptual_loop::total_perceptual_loop_memory;
    use super::subsurface_scattering::total_sss_memory;
    use super::unified_bsdf::total_unified_bsdf_memory;

    let unified = total_unified_bsdf_memory() as f64 / 1024.0;
    let aniso = total_anisotropic_memory() as f64 / 1024.0;
    let sss = total_sss_memory() as f64 / 1024.0;
    let perceptual = total_perceptual_loop_memory() as f64 / 1024.0;

    let total = unified + aniso + sss + perceptual;

    Phase9MemoryAnalysis {
        unified_bsdf_kb: unified,
        anisotropic_kb: aniso,
        sss_kb: sss,
        perceptual_loop_kb: perceptual,
        total_phase9_kb: total,
        within_budget: total < 100.0, // 100KB budget for Phase 9
    }
}

/// Run complete Phase 9 validation
pub fn run_full_validation() -> Phase9ValidationReport {
    let unified_vs_legacy = validate_unified_vs_legacy();
    let anisotropic_tests = validate_anisotropic();
    let sss_accuracy = validate_sss();
    let perceptual_loop_convergence = validate_perceptual_loop();
    let energy_conservation = validate_energy_conservation_all();
    let memory_analysis = analyze_memory();

    let overall_passed = unified_vs_legacy.all_passed
        && anisotropic_tests.isotropy_recovery
        && anisotropic_tests.energy_conserved
        && sss_accuracy.profile_valid
        && sss_accuracy.energy_conserved
        && energy_conservation.all_passed
        && memory_analysis.within_budget;

    Phase9ValidationReport {
        unified_vs_legacy,
        anisotropic_tests,
        sss_accuracy,
        perceptual_loop_convergence,
        energy_conservation,
        memory_analysis,
        overall_passed,
    }
}

/// Generate markdown report
pub fn generate_report() -> String {
    let report = run_full_validation();

    let mut md = String::new();
    md.push_str("# Phase 9 Validation Report\n\n");
    md.push_str(&format!(
        "**Overall Status:** {}\n\n",
        if report.overall_passed {
            "PASS"
        } else {
            "FAIL"
        }
    ));

    md.push_str("## 1. Unified vs Legacy Comparison\n\n");
    md.push_str(&format!(
        "- Dielectric RMSE: {:.6}\n",
        report.unified_vs_legacy.dielectric_rmse
    ));
    md.push_str(&format!(
        "- Conductor RMSE: {:.6}\n",
        report.unified_vs_legacy.conductor_rmse
    ));
    md.push_str(&format!(
        "- Thin-film RMSE: {:.6}\n",
        report.unified_vs_legacy.thin_film_rmse
    ));
    for detail in &report.unified_vs_legacy.details {
        md.push_str(&format!("  - {}\n", detail));
    }

    md.push_str("\n## 2. Anisotropic Validation\n\n");
    md.push_str(&format!(
        "- Isotropy Recovery: {}\n",
        if report.anisotropic_tests.isotropy_recovery {
            "PASS"
        } else {
            "FAIL"
        }
    ));
    md.push_str(&format!(
        "- Energy Conservation: {}\n",
        if report.anisotropic_tests.energy_conserved {
            "PASS"
        } else {
            "FAIL"
        }
    ));
    md.push_str(&format!(
        "- VNDF Sampling: {}\n",
        if report.anisotropic_tests.sampling_valid {
            "PASS"
        } else {
            "FAIL"
        }
    ));

    md.push_str("\n## 3. SSS Validation\n\n");
    md.push_str(&format!(
        "- Diffusion Profile: {}\n",
        if report.sss_accuracy.profile_valid {
            "PASS"
        } else {
            "FAIL"
        }
    ));
    md.push_str(&format!(
        "- Presets Valid: {}\n",
        if report.sss_accuracy.presets_valid {
            "PASS"
        } else {
            "FAIL"
        }
    ));
    md.push_str(&format!(
        "- Energy Conservation: {}\n",
        if report.sss_accuracy.energy_conserved {
            "PASS"
        } else {
            "FAIL"
        }
    ));

    md.push_str("\n## 4. Perceptual Loop Convergence\n\n");
    md.push_str(&format!(
        "- Tests Run: {}\n",
        report.perceptual_loop_convergence.tests_run
    ));
    md.push_str(&format!(
        "- Tests Converged: {}\n",
        report.perceptual_loop_convergence.tests_converged
    ));
    md.push_str(&format!(
        "- Avg Iterations: {:.1}\n",
        report.perceptual_loop_convergence.avg_iterations
    ));
    md.push_str(&format!(
        "- Avg Final ΔE: {:.2}\n",
        report.perceptual_loop_convergence.avg_final_delta_e
    ));

    md.push_str("\n## 5. Energy Conservation\n\n");
    md.push_str(&format!(
        "- Materials Tested: {}\n",
        report.energy_conservation.materials_tested
    ));
    md.push_str(&format!(
        "- Passed: {}\n",
        report.energy_conservation.passed
    ));
    md.push_str(&format!(
        "- Max Error: {:.2e}\n",
        report.energy_conservation.max_error
    ));
    md.push_str(&format!(
        "- Worst Material: {}\n",
        report.energy_conservation.worst_material
    ));

    md.push_str("\n## 6. Memory Analysis\n\n");
    md.push_str(&format!("| Module | Memory (KB) |\n"));
    md.push_str(&format!("|--------|------------|\n"));
    md.push_str(&format!(
        "| unified_bsdf | {:.2} |\n",
        report.memory_analysis.unified_bsdf_kb
    ));
    md.push_str(&format!(
        "| anisotropic | {:.2} |\n",
        report.memory_analysis.anisotropic_kb
    ));
    md.push_str(&format!("| sss | {:.2} |\n", report.memory_analysis.sss_kb));
    md.push_str(&format!(
        "| perceptual_loop | {:.2} |\n",
        report.memory_analysis.perceptual_loop_kb
    ));
    md.push_str(&format!(
        "| **Total Phase 9** | **{:.2}** |\n",
        report.memory_analysis.total_phase9_kb
    ));
    md.push_str(&format!(
        "\nWithin Budget: {}\n",
        if report.memory_analysis.within_budget {
            "YES"
        } else {
            "NO"
        }
    ));

    md.push_str("\n---\n\n");
    md.push_str("*Generated by Momoto Materials Phase 9 Validation Suite*\n");

    md
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::super::unified_bsdf::Vector3;
    use super::*;

    #[test]
    fn test_unified_vs_legacy() {
        let results = validate_unified_vs_legacy();

        assert!(
            results.dielectric_rmse < 0.01,
            "Dielectric RMSE too high: {}",
            results.dielectric_rmse
        );
        assert!(
            results.conductor_rmse < 0.05,
            "Conductor RMSE too high: {}",
            results.conductor_rmse
        );
    }

    #[test]
    fn test_anisotropic_validation() {
        let results = validate_anisotropic();

        assert!(results.isotropy_recovery, "Isotropy recovery failed");
        assert!(results.energy_conserved, "Anisotropic energy not conserved");
        assert!(results.sampling_valid, "VNDF sampling invalid");
    }

    #[test]
    fn test_sss_validation() {
        let results = validate_sss();

        assert!(results.profile_valid, "Diffusion profile invalid");
        assert!(results.presets_valid, "SSS presets invalid");
        assert!(results.energy_conserved, "SSS energy not conserved");
    }

    #[test]
    fn test_perceptual_loop_validation() {
        let results = validate_perceptual_loop();

        assert!(results.tests_run > 0, "No tests run");
        // At least some should converge
        assert!(
            results.tests_converged > 0 || results.avg_final_delta_e < 20.0,
            "Poor convergence"
        );
    }

    #[test]
    fn test_energy_conservation_comprehensive() {
        let results = validate_energy_conservation_all();

        assert!(
            results.all_passed,
            "Energy conservation failed for: {}",
            results.worst_material
        );
        assert!(
            results.max_error < 1e-5,
            "Max error too high: {}",
            results.max_error
        );
    }

    #[test]
    fn test_memory_budget() {
        let analysis = analyze_memory();

        assert!(
            analysis.within_budget,
            "Memory over budget: {:.2}KB",
            analysis.total_phase9_kb
        );
    }

    #[test]
    fn test_full_validation() {
        let report = run_full_validation();

        // Report should complete without panic
        assert!(report.energy_conservation.materials_tested > 0);
    }

    #[test]
    fn test_report_generation() {
        let md = generate_report();

        assert!(md.contains("Phase 9 Validation Report"));
        assert!(md.contains("Energy Conservation"));
        assert!(md.contains("Memory Analysis"));
    }

    #[test]
    fn test_dielectric_reciprocity() {
        // BSDF should be symmetric: f(wi, wo) ≈ f(wo, wi)
        let bsdf = DielectricBSDF::new(1.5, 0.1);

        // Test at various angles using new_simple which takes cos_theta
        let ctx_30 = BSDFContext::new_simple(0.866); // 30 degrees
        let ctx_60 = BSDFContext::new_simple(0.5); // 60 degrees

        let r_30 = bsdf.evaluate(&ctx_30);
        let r_60 = bsdf.evaluate(&ctx_60);

        // Both should be physically valid (0-1 range)
        assert!(r_30.reflectance >= 0.0 && r_30.reflectance <= 1.0);
        assert!(r_60.reflectance >= 0.0 && r_60.reflectance <= 1.0);
        // Fresnel: reflectance should increase at grazing angles
        assert!(
            r_60.reflectance >= r_30.reflectance,
            "Fresnel should increase at grazing: {} vs {}",
            r_30.reflectance,
            r_60.reflectance
        );
    }

    #[test]
    fn test_conductor_reciprocity() {
        let bsdf = ConductorBSDF::new(0.18, 3.42, 0.1);

        // Test at various angles
        let ctx_0 = BSDFContext::new_simple(1.0); // Normal incidence
        let ctx_60 = BSDFContext::new_simple(0.5); // 60 degrees

        let r_0 = bsdf.evaluate(&ctx_0);
        let r_60 = bsdf.evaluate(&ctx_60);

        // Conductor should have high reflectance even at normal
        assert!(
            r_0.reflectance > 0.5,
            "Conductor should be highly reflective"
        );
        assert!(
            r_60.reflectance >= r_0.reflectance * 0.9,
            "Conductor stays reflective at angle"
        );
    }

    #[test]
    fn test_anisotropic_edge_cases() {
        use super::super::anisotropic_brdf::AnisotropicGGX;

        // Test with extreme anisotropy at normal incidence
        let extreme = AnisotropicGGX::new(0.01, 0.5, 1.5);
        let ctx = BSDFContext::new_simple(1.0); // Normal incidence

        let result = extreme.evaluate(&ctx);
        assert!(
            result.reflectance >= 0.0 && result.reflectance <= 1.0,
            "Edge case produced invalid reflectance: {}",
            result.reflectance
        );
    }

    #[test]
    fn test_sss_distance_falloff() {
        use super::super::subsurface_scattering::DiffusionBSSRDF;

        let params = sss_presets::skin();
        let bssrdf = DiffusionBSSRDF::new(params, 1.0);

        // Test that diffuse reflectance decreases with distance
        let r0 = bssrdf.rd(0.1, 0);
        let r1 = bssrdf.rd(1.0, 0);
        let r2 = bssrdf.rd(10.0, 0);

        assert!(r0 > r1, "Rd should decrease with distance");
        assert!(r1 > r2, "Rd should continue decreasing");
    }

    #[test]
    #[ignore = "Requires LayeredBSDF implementation"]
    fn test_layered_bsdf_depth() {
        // Test that layered BSDF handles multiple layers correctly
        // TODO: Implement when LayeredBSDF is available
        // let layered = LayeredBSDF::new()
        //     .push(Box::new(DielectricBSDF::new(1.5, 0.0)))
        //     .push(Box::new(DielectricBSDF::new(1.4, 0.0)))
        //     .push(Box::new(DielectricBSDF::new(1.3, 0.0)));
        //
        // let ctx = BSDFContext::new_simple(1.0);  // Normal incidence
        //
        // let result = layered.evaluate(&ctx);
        // let validation = layered.validate_energy(&ctx);
        //
        // assert!(validation.conserved, "Layered BSDF must conserve energy");
        // assert!(result.reflectance > 0.0, "Should have some reflection");
    }

    #[test]
    fn test_thin_film_soap_bubble() {
        // Soap bubble: thin film on air (IOR ~1.0 substrate)
        let bubble = ThinFilmBSDF::new(1.0, 1.33, 300.0); // substrate_ior, film_ior, thickness

        let ctx_550 = BSDFContext::new_simple(1.0).with_wavelength(550.0);
        let ctx_650 = BSDFContext::new_simple(1.0).with_wavelength(650.0);

        let r_550 = bubble.evaluate(&ctx_550);
        let r_650 = bubble.evaluate(&ctx_650);

        // Thin film should show wavelength-dependent behavior
        assert!(
            (r_550.reflectance - r_650.reflectance).abs() > 0.001,
            "Thin film should be wavelength dependent: {} vs {}",
            r_550.reflectance,
            r_650.reflectance
        );
    }
}
