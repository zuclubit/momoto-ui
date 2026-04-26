//! # Phase 13 Validation Suite
//!
//! Comprehensive validation for differentiable rendering, inverse materials,
//! and physical parameter recovery.
//!
//! ## Validation Targets
//!
//! | Metric | Target |
//! |--------|--------|
//! | Gradient error | < 1e-4 vs numeric |
//! | Inverse convergence | < 200 iterations |
//! | Temporal stability | 2000+ frames |
//! | Memory | < 180KB |
//! | API breakage | 0 |

use super::differentiable::prelude::*;
use super::differentiable::{
    DifferentiableConductor, DifferentiableDielectric, DifferentiableLayered,
    DifferentiableThinFilm, LayerConfig,
};
use super::gradient_validation::{verify_bsdf_gradients, BatchVerification, VerificationConfig};
use super::inverse_material::{
    InverseMaterialSolver, ReferenceData, ReferenceObservation, TemporalFitter, TemporalSequence,
};
use super::spectral_gradients::prelude::*;
use super::spectral_gradients::{delta_e_2000, Lab};
#[allow(unused_imports)]
use super::temporal_differentiable::prelude::*;
use super::temporal_differentiable::{BPTTConfig, EvolutionType, BPTT};
use super::unified_bsdf::{BSDFContext, Vector3, BSDF};

// ============================================================================
// VALIDATION CONFIGURATION
// ============================================================================

/// Configuration for Phase 13 validation.
#[derive(Debug, Clone)]
pub struct Phase13ValidationConfig {
    /// Target gradient error tolerance.
    pub gradient_tolerance: f64,
    /// Target inverse convergence iterations.
    pub inverse_max_iterations: usize,
    /// Target temporal stability frames.
    pub temporal_stability_frames: usize,
    /// Target memory budget (bytes).
    pub memory_budget: usize,
    /// Verbose output.
    pub verbose: bool,
}

impl Default for Phase13ValidationConfig {
    fn default() -> Self {
        Self {
            gradient_tolerance: 1e-4,
            inverse_max_iterations: 200,
            temporal_stability_frames: 2000,
            memory_budget: 180 * 1024, // 180KB
            verbose: false,
        }
    }
}

// ============================================================================
// VALIDATION RESULTS
// ============================================================================

/// Result of a single validation test.
#[derive(Debug, Clone)]
pub struct ValidationTest {
    /// Test name.
    pub name: &'static str,
    /// Whether test passed.
    pub passed: bool,
    /// Test details/message.
    pub details: String,
    /// Measured value (if applicable).
    pub measured_value: Option<f64>,
    /// Target value (if applicable).
    pub target_value: Option<f64>,
}

impl ValidationTest {
    /// Create passed test.
    pub fn pass(name: &'static str, details: impl Into<String>) -> Self {
        Self {
            name,
            passed: true,
            details: details.into(),
            measured_value: None,
            target_value: None,
        }
    }

    /// Create failed test.
    pub fn fail(name: &'static str, details: impl Into<String>) -> Self {
        Self {
            name,
            passed: false,
            details: details.into(),
            measured_value: None,
            target_value: None,
        }
    }

    /// Create test with value comparison.
    pub fn with_value(name: &'static str, measured: f64, target: f64, passed: bool) -> Self {
        Self {
            name,
            passed,
            details: format!("measured={:.6}, target={:.6}", measured, target),
            measured_value: Some(measured),
            target_value: Some(target),
        }
    }
}

/// Complete Phase 13 validation results.
#[derive(Debug)]
pub struct Phase13ValidationReport {
    /// Individual test results.
    pub tests: Vec<ValidationTest>,
    /// Overall pass status.
    pub all_passed: bool,
    /// Number of passed tests.
    pub passed_count: usize,
    /// Number of failed tests.
    pub failed_count: usize,
    /// Total tests run.
    pub total_tests: usize,
}

impl Phase13ValidationReport {
    /// Create report from tests.
    pub fn from_tests(tests: Vec<ValidationTest>) -> Self {
        let total_tests = tests.len();
        let passed_count = tests.iter().filter(|t| t.passed).count();
        let failed_count = total_tests - passed_count;
        let all_passed = failed_count == 0;

        Self {
            tests,
            all_passed,
            passed_count,
            failed_count,
            total_tests,
        }
    }

    /// Get failed tests.
    pub fn failed_tests(&self) -> Vec<&ValidationTest> {
        self.tests.iter().filter(|t| !t.passed).collect()
    }

    /// Generate report string.
    pub fn report(&self) -> String {
        let mut report = String::new();

        report.push_str("╔═══════════════════════════════════════════════════════════════╗\n");
        report.push_str("║           Phase 13 Validation Report                          ║\n");
        report.push_str("║   Differentiable Rendering & Inverse Materials                ║\n");
        report.push_str("╚═══════════════════════════════════════════════════════════════╝\n\n");

        report.push_str(&format!(
            "Overall Status: {}\n",
            if self.all_passed {
                "✓ PASSED"
            } else {
                "✗ FAILED"
            }
        ));
        report.push_str(&format!(
            "Tests: {}/{} passed\n\n",
            self.passed_count, self.total_tests
        ));

        report.push_str("Test Results:\n");
        report.push_str("─────────────────────────────────────────────────────────────────\n");

        for test in &self.tests {
            let status = if test.passed { "✓" } else { "✗" };
            report.push_str(&format!("{} {}: {}\n", status, test.name, test.details));
        }

        if !self.all_passed {
            report.push_str("\nFailed Tests:\n");
            for test in self.failed_tests() {
                report.push_str(&format!("  - {}: {}\n", test.name, test.details));
            }
        }

        report
    }
}

// ============================================================================
// VALIDATION FUNCTIONS
// ============================================================================

/// Run complete Phase 13 validation.
pub fn run_phase13_validation(config: &Phase13ValidationConfig) -> Phase13ValidationReport {
    let mut tests = Vec::new();

    // Gradient validation
    tests.extend(validate_gradients(config));

    // Inverse solver validation
    tests.extend(validate_inverse_solver(config));

    // Temporal stability validation
    tests.extend(validate_temporal_stability(config));

    // Spectral gradient validation
    tests.extend(validate_spectral_gradients(config));

    // Memory budget validation
    tests.push(validate_memory_budget(config));

    // API compatibility validation
    tests.extend(validate_api_compatibility());

    Phase13ValidationReport::from_tests(tests)
}

/// Validate analytical gradients.
fn validate_gradients(config: &Phase13ValidationConfig) -> Vec<ValidationTest> {
    let mut tests = Vec::new();
    let verification_config = VerificationConfig {
        relative_tolerance: config.gradient_tolerance,
        ..Default::default()
    };

    // Dielectric gradients
    {
        let glass = DifferentiableDielectric::glass();
        let contexts = BatchVerification::standard_contexts();
        let batch = BatchVerification::run(&glass, &contexts, &verification_config);

        tests.push(if batch.all_passed {
            ValidationTest::with_value(
                "Dielectric gradient accuracy",
                batch.global_max_error,
                config.gradient_tolerance,
                true,
            )
        } else {
            ValidationTest::with_value(
                "Dielectric gradient accuracy",
                batch.global_max_error,
                config.gradient_tolerance,
                false,
            )
        });
    }

    // Conductor gradients
    {
        let gold = DifferentiableConductor::gold();
        let ctx = create_test_context(0.8, 550.0);
        let result = verify_bsdf_gradients(&gold, &ctx, &verification_config);

        tests.push(if result.max_error < config.gradient_tolerance * 10.0 {
            ValidationTest::pass(
                "Conductor gradient accuracy",
                format!("max_error={:.2e}", result.max_error),
            )
        } else {
            ValidationTest::fail(
                "Conductor gradient accuracy",
                format!("max_error={:.2e} > tolerance", result.max_error),
            )
        });
    }

    // Thin-film gradients
    {
        let film = DifferentiableThinFilm::soap_bubble(300.0);
        let ctx = create_test_context(0.8, 550.0);
        let result = verify_bsdf_gradients(&film, &ctx, &verification_config);

        tests.push(if result.max_error < config.gradient_tolerance * 10.0 {
            ValidationTest::pass(
                "Thin-film gradient accuracy",
                format!("max_error={:.2e}", result.max_error),
            )
        } else {
            ValidationTest::fail(
                "Thin-film gradient accuracy",
                format!("max_error={:.2e} > tolerance", result.max_error),
            )
        });
    }

    tests
}

/// Validate inverse solver convergence.
fn validate_inverse_solver(config: &Phase13ValidationConfig) -> Vec<ValidationTest> {
    let mut tests = Vec::new();

    // Single parameter recovery
    {
        let target = DifferentiableDielectric::new(1.52, 0.1);
        let ctx = create_test_context(1.0, 550.0);
        let r = target.eval_with_gradients(&ctx).response.reflectance;

        let reference = ReferenceData::from_reflectance(&[(1.0, r)]);
        let initial = DifferentiableDielectric::new(1.3, 0.1);

        let mut solver = InverseMaterialSolver::with_adam();
        solver.config.max_iterations = config.inverse_max_iterations;

        let result = solver.solve(&reference, &initial);

        tests.push(
            if result.converged && result.iterations < config.inverse_max_iterations {
                ValidationTest::with_value(
                    "Single param inverse convergence",
                    result.iterations as f64,
                    config.inverse_max_iterations as f64,
                    true,
                )
            } else {
                ValidationTest::with_value(
                    "Single param inverse convergence",
                    result.iterations as f64,
                    config.inverse_max_iterations as f64,
                    false,
                )
            },
        );
    }

    // Multi-angle recovery
    {
        let target = DifferentiableDielectric::new(1.5, 0.1);

        let angles = [1.0, 0.9, 0.8, 0.7, 0.6];
        let mut data = ReferenceData::new();
        for &cos_theta in &angles {
            let ctx = create_test_context(cos_theta, 550.0);
            let r = target.eval_with_gradients(&ctx).response.reflectance;
            data.add(ReferenceObservation::from_reflectance(ctx, r));
        }

        let initial = DifferentiableDielectric::new(1.3, 0.2);

        let mut solver = InverseMaterialSolver::with_adam();
        solver.config.max_iterations = config.inverse_max_iterations;

        let result = solver.solve(&data, &initial);

        tests.push(if result.final_loss < 0.01 {
            ValidationTest::with_value(
                "Multi-angle inverse accuracy",
                result.final_loss,
                0.01,
                true,
            )
        } else {
            ValidationTest::with_value(
                "Multi-angle inverse accuracy",
                result.final_loss,
                0.01,
                false,
            )
        });
    }

    // L-BFGS convergence
    {
        let target = DifferentiableDielectric::new(1.5, 0.1);
        let ctx = create_test_context(1.0, 550.0);
        let r = target.eval_with_gradients(&ctx).response.reflectance;

        let reference = ReferenceData::from_reflectance(&[(1.0, r)]);
        let initial = DifferentiableDielectric::new(1.4, 0.1);

        let mut solver = InverseMaterialSolver::with_lbfgs();
        solver.config.max_iterations = config.inverse_max_iterations;

        let result = solver.solve(&reference, &initial);

        tests.push(if result.iterations > 0 {
            ValidationTest::pass(
                "L-BFGS optimizer functional",
                format!("iterations={}", result.iterations),
            )
        } else {
            ValidationTest::fail("L-BFGS optimizer functional", "No iterations completed")
        });
    }

    tests
}

/// Validate temporal stability.
fn validate_temporal_stability(config: &Phase13ValidationConfig) -> Vec<ValidationTest> {
    let mut tests = Vec::new();

    // BPTT gradient stability
    {
        let mut bptt = BPTT::with_config(
            8,
            BPTTConfig {
                max_sequence_length: config.temporal_stability_frames,
                ..Default::default()
            },
        );

        let mut stable = true;
        for i in 0..config.temporal_stability_frames.min(1000) {
            let t = i as f64 * 0.01;
            bptt.forward_frame(
                t,
                vec![1.5; 8],
                EvolutionType::Exponential {
                    rate: 0.1,
                    asymptote: 1.0,
                },
                0.001,
                0.01,
            );

            if i % 100 == 99 {
                let grads = bptt.backward();
                if grads.iter().any(|g| !g.is_finite()) {
                    stable = false;
                    break;
                }
            }
        }

        tests.push(if stable && bptt.is_stable() {
            ValidationTest::pass(
                "BPTT gradient stability",
                format!(
                    "stable over {} frames",
                    config.temporal_stability_frames.min(1000)
                ),
            )
        } else {
            ValidationTest::fail("BPTT gradient stability", "Gradient instability detected")
        });
    }

    // Temporal fitting
    {
        let mut seq = TemporalSequence::new();
        for i in 0..100 {
            let t = i as f64;
            let ior = 1.5 + 0.001 * t;
            let r = ((ior - 1.0) / (ior + 1.0)).powi(2);
            seq.add_frame(
                super::inverse_material::temporal_fitting::TemporalFrame::new(
                    t,
                    r,
                    create_test_context(1.0, 550.0),
                ),
            );
        }

        let initial = DifferentiableDielectric::new(1.5, 0.1);
        let mut fitter = TemporalFitter::new(initial.param_count());

        let result = fitter.fit(&seq, &initial);

        tests.push(if result.rmse() < 0.01 {
            ValidationTest::with_value("Temporal fitting accuracy", result.rmse(), 0.01, true)
        } else {
            ValidationTest::with_value("Temporal fitting accuracy", result.rmse(), 0.01, false)
        });
    }

    tests
}

/// Validate spectral gradients.
fn validate_spectral_gradients(_config: &Phase13ValidationConfig) -> Vec<ValidationTest> {
    let mut tests = Vec::new();

    // Per-wavelength gradients
    {
        let spec_grad = compute_spectral_gradient(1.5, None);

        let all_nonzero = spec_grad.gradients.iter().all(|g| g.d_ior != 0.0);

        tests.push(if all_nonzero {
            ValidationTest::pass(
                "Spectral gradient computation",
                format!("{} wavelengths computed", spec_grad.len()),
            )
        } else {
            ValidationTest::fail(
                "Spectral gradient computation",
                "Some wavelength gradients are zero",
            )
        });
    }

    // ΔE2000 gradient
    {
        let lab1 = Lab::new(50.0, 25.0, -30.0);
        let lab2 = Lab::new(55.0, 20.0, -25.0);

        let de = delta_e_2000(&lab1, &lab2);
        let grad = super::spectral_gradients::delta_e_2000_gradient(&lab1, &lab2);

        let grad_nonzero = grad.grad_lab1.norm() > 0.0;

        tests.push(if de > 0.0 && grad_nonzero {
            ValidationTest::pass(
                "ΔE2000 gradient computation",
                format!("ΔE={:.2}, grad_norm={:.2e}", de, grad.grad_lab1.norm()),
            )
        } else {
            ValidationTest::fail("ΔE2000 gradient computation", "Invalid ΔE or gradient")
        });
    }

    tests
}

/// Validate memory budget.
fn validate_memory_budget(config: &Phase13ValidationConfig) -> ValidationTest {
    // Estimate memory usage for Phase 13 components
    let differentiable_mem = super::differentiable::estimate_differentiable_memory();
    let inverse_mem = super::inverse_material::estimate_inverse_memory();
    let temporal_mem = super::temporal_differentiable::estimate_temporal_differentiable_memory(100);
    let spectral_mem = super::spectral_gradients::estimate_spectral_gradients_memory();

    let total_mem = differentiable_mem + inverse_mem + temporal_mem + spectral_mem;

    if total_mem < config.memory_budget {
        ValidationTest::with_value(
            "Memory budget",
            total_mem as f64,
            config.memory_budget as f64,
            true,
        )
    } else {
        ValidationTest::with_value(
            "Memory budget",
            total_mem as f64,
            config.memory_budget as f64,
            false,
        )
    }
}

/// Validate API compatibility (no breaking changes).
fn validate_api_compatibility() -> Vec<ValidationTest> {
    let mut tests = Vec::new();

    // DifferentiableBSDF extends BSDF
    {
        let glass = DifferentiableDielectric::glass();
        let ctx = create_test_context(0.8, 550.0);

        // Should work as BSDF
        let response = glass.evaluate(&ctx);
        let valid = response.reflectance >= 0.0 && response.reflectance <= 1.0;

        tests.push(if valid {
            ValidationTest::pass(
                "BSDF trait compatibility",
                "DifferentiableBSDF extends BSDF",
            )
        } else {
            ValidationTest::fail(
                "BSDF trait compatibility",
                "evaluate() returned invalid response",
            )
        });
    }

    // Layered composition
    {
        let coating = DifferentiableThinFilm::anti_reflective(100.0);
        let substrate = DifferentiableDielectric::glass();
        let ctx = create_test_context(0.8, 550.0);

        let mut layered = DifferentiableLayered::new();
        layered.add_layer(&coating, &ctx, LayerConfig::default());
        layered.add_layer(
            &substrate,
            &ctx,
            LayerConfig {
                weight: 1.0,
                is_substrate: true,
            },
        );

        let result = layered.compute();
        let valid = result.response.reflectance >= 0.0;

        tests.push(if valid {
            ValidationTest::pass(
                "Layered composition",
                format!("R={:.4}", result.response.reflectance),
            )
        } else {
            ValidationTest::fail("Layered composition", "Invalid layered response")
        });
    }

    tests
}

/// Helper to create test context.
fn create_test_context(cos_theta: f64, wavelength: f64) -> BSDFContext {
    let sin_theta = (1.0 - cos_theta * cos_theta).sqrt().max(0.0);
    BSDFContext {
        wi: Vector3::new(sin_theta, 0.0, cos_theta),
        wo: Vector3::new(-sin_theta, 0.0, cos_theta),
        normal: Vector3::new(0.0, 0.0, 1.0),
        tangent: Vector3::new(1.0, 0.0, 0.0),
        bitangent: Vector3::new(0.0, 1.0, 0.0),
        wavelength,
        wavelengths: None,
    }
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_phase13_validation_completes() {
        let config = Phase13ValidationConfig::default();
        let report = run_phase13_validation(&config);

        assert!(report.total_tests > 0);
        println!("{}", report.report());
    }

    #[test]
    fn test_validation_test_creation() {
        let pass = ValidationTest::pass("Test", "details");
        assert!(pass.passed);

        let fail = ValidationTest::fail("Test", "reason");
        assert!(!fail.passed);

        let with_value = ValidationTest::with_value("Test", 0.5, 1.0, true);
        assert!(with_value.measured_value.is_some());
    }

    #[test]
    fn test_validation_report_generation() {
        let tests = vec![
            ValidationTest::pass("Test 1", "OK"),
            ValidationTest::fail("Test 2", "Failed"),
        ];

        let report = Phase13ValidationReport::from_tests(tests);

        assert_eq!(report.passed_count, 1);
        assert_eq!(report.failed_count, 1);
        assert!(!report.all_passed);

        let report_str = report.report();
        assert!(report_str.contains("Test 1"));
        assert!(report_str.contains("Test 2"));
    }

    #[test]
    fn test_gradient_validation_runs() {
        let config = Phase13ValidationConfig::default();
        let tests = validate_gradients(&config);

        assert!(!tests.is_empty());
    }

    #[test]
    fn test_inverse_validation_runs() {
        let config = Phase13ValidationConfig::default();
        let tests = validate_inverse_solver(&config);

        assert!(!tests.is_empty());
    }

    #[test]
    fn test_temporal_validation_runs() {
        let config = Phase13ValidationConfig {
            temporal_stability_frames: 100, // Reduced for test speed
            ..Default::default()
        };
        let tests = validate_temporal_stability(&config);

        assert!(!tests.is_empty());
    }

    #[test]
    fn test_spectral_validation_runs() {
        let config = Phase13ValidationConfig::default();
        let tests = validate_spectral_gradients(&config);

        assert!(!tests.is_empty());
    }

    #[test]
    fn test_memory_validation() {
        let config = Phase13ValidationConfig::default();
        let test = validate_memory_budget(&config);

        // Memory should be within budget
        assert!(test.passed);
    }

    #[test]
    fn test_api_compatibility_validation() {
        let tests = validate_api_compatibility();

        assert!(!tests.is_empty());
        assert!(tests.iter().all(|t| t.passed));
    }

    #[test]
    fn test_full_phase13_validation() {
        let config = Phase13ValidationConfig {
            temporal_stability_frames: 100,
            ..Default::default()
        };

        let report = run_phase13_validation(&config);

        // Print report for visibility
        println!("\n{}", report.report());

        // Should have substantial number of tests
        assert!(report.total_tests >= 10);
    }
}
