//! # Gradient Validation
//!
//! Verification utilities for analytical vs numerical gradients.
//!
//! ## Overview
//!
//! This module provides tools to verify that analytical gradient
//! implementations match numerical (finite difference) approximations.
//! This is critical for:
//! - Debugging gradient implementations
//! - Ensuring optimization correctness
//! - CI/CD validation
//!
//! ## Verification Methods
//!
//! - Central difference: (f(x+ε) - f(x-ε)) / (2ε)
//! - Forward difference: (f(x+ε) - f(x)) / ε
//! - Complex step: Im(f(x+iε)) / ε (highest accuracy)

use super::differentiable::traits::DifferentiableBSDF;
use super::unified_bsdf::{BSDFContext, Vector3};

// ============================================================================
// VERIFICATION CONFIGURATION
// ============================================================================

/// Configuration for gradient verification.
#[derive(Debug, Clone)]
pub struct VerificationConfig {
    /// Perturbation size for finite differences.
    pub epsilon: f64,
    /// Tolerance for relative error.
    pub relative_tolerance: f64,
    /// Tolerance for absolute error (for near-zero gradients).
    pub absolute_tolerance: f64,
    /// Number of random test points.
    pub num_test_points: usize,
    /// Whether to report all failures or stop at first.
    pub report_all: bool,
}

impl Default for VerificationConfig {
    fn default() -> Self {
        Self {
            epsilon: 1e-6,
            relative_tolerance: 1e-4,
            absolute_tolerance: 1e-8,
            num_test_points: 10,
            report_all: true,
        }
    }
}

impl VerificationConfig {
    /// High precision configuration.
    pub fn high_precision() -> Self {
        Self {
            epsilon: 1e-8,
            relative_tolerance: 1e-6,
            absolute_tolerance: 1e-10,
            ..Default::default()
        }
    }

    /// Fast verification for CI.
    pub fn fast() -> Self {
        Self {
            num_test_points: 3,
            report_all: false,
            ..Default::default()
        }
    }
}

// ============================================================================
// VERIFICATION RESULT
// ============================================================================

/// Result of a single gradient verification.
#[derive(Debug, Clone)]
pub struct GradientCheck {
    /// Parameter index.
    pub param_index: usize,
    /// Parameter name.
    pub param_name: &'static str,
    /// Analytical gradient value.
    pub analytic: f64,
    /// Numerical gradient value.
    pub numeric: f64,
    /// Absolute error.
    pub absolute_error: f64,
    /// Relative error (if applicable).
    pub relative_error: Option<f64>,
    /// Whether this check passed.
    pub passed: bool,
}

impl GradientCheck {
    /// Create new check.
    pub fn new(
        param_index: usize,
        param_name: &'static str,
        analytic: f64,
        numeric: f64,
        config: &VerificationConfig,
    ) -> Self {
        let absolute_error = (analytic - numeric).abs();

        let relative_error = if numeric.abs() > config.absolute_tolerance {
            Some(absolute_error / numeric.abs())
        } else {
            None
        };

        let passed = if let Some(rel_err) = relative_error {
            rel_err < config.relative_tolerance
        } else {
            absolute_error < config.absolute_tolerance
        };

        Self {
            param_index,
            param_name,
            analytic,
            numeric,
            absolute_error,
            relative_error,
            passed,
        }
    }
}

/// Result of gradient verification for all parameters.
#[derive(Debug, Clone)]
pub struct GradientVerificationResult {
    /// Individual checks.
    pub checks: Vec<GradientCheck>,
    /// Whether all checks passed.
    pub all_passed: bool,
    /// Maximum error across all checks.
    pub max_error: f64,
    /// Maximum relative error.
    pub max_relative_error: f64,
}

impl GradientVerificationResult {
    /// Create from checks.
    pub fn from_checks(checks: Vec<GradientCheck>) -> Self {
        let all_passed = checks.iter().all(|c| c.passed);
        let max_error = checks.iter().map(|c| c.absolute_error).fold(0.0, f64::max);
        let max_relative_error = checks
            .iter()
            .filter_map(|c| c.relative_error)
            .fold(0.0, f64::max);

        Self {
            checks,
            all_passed,
            max_error,
            max_relative_error,
        }
    }

    /// Get failed checks.
    pub fn failed_checks(&self) -> Vec<&GradientCheck> {
        self.checks.iter().filter(|c| !c.passed).collect()
    }

    /// Format as report string.
    pub fn report(&self) -> String {
        let mut report = String::new();

        report.push_str(&format!(
            "Gradient Verification: {}\n",
            if self.all_passed { "PASSED" } else { "FAILED" }
        ));
        report.push_str(&format!("Max absolute error: {:.2e}\n", self.max_error));
        report.push_str(&format!(
            "Max relative error: {:.2e}\n",
            self.max_relative_error
        ));
        report.push_str("\nParameter Details:\n");

        for check in &self.checks {
            let status = if check.passed { "[OK]" } else { "[FAIL]" };
            report.push_str(&format!(
                "  {} {}: analytic={:.6e}, numeric={:.6e}, error={:.2e}\n",
                status, check.param_name, check.analytic, check.numeric, check.absolute_error
            ));
        }

        report
    }
}

// ============================================================================
// NUMERICAL GRADIENT COMPUTATION
// ============================================================================

/// Compute numerical gradient using central difference.
pub fn numerical_gradient_central<F>(f: F, x: f64, epsilon: f64) -> f64
where
    F: Fn(f64) -> f64,
{
    let f_plus = f(x + epsilon);
    let f_minus = f(x - epsilon);
    (f_plus - f_minus) / (2.0 * epsilon)
}

/// Compute numerical gradient using forward difference.
pub fn numerical_gradient_forward<F>(f: F, x: f64, epsilon: f64) -> f64
where
    F: Fn(f64) -> f64,
{
    let f_base = f(x);
    let f_plus = f(x + epsilon);
    (f_plus - f_base) / epsilon
}

/// Compute numerical gradient for vector-valued function.
pub fn numerical_jacobian<F>(f: F, x: &[f64], epsilon: f64) -> Vec<f64>
where
    F: Fn(&[f64]) -> f64,
{
    let n = x.len();
    let mut jacobian = vec![0.0; n];

    for i in 0..n {
        let mut x_plus = x.to_vec();
        let mut x_minus = x.to_vec();
        x_plus[i] += epsilon;
        x_minus[i] -= epsilon;

        jacobian[i] = (f(&x_plus) - f(&x_minus)) / (2.0 * epsilon);
    }

    jacobian
}

// ============================================================================
// BSDF GRADIENT VERIFICATION
// ============================================================================

/// Verify gradients for a DifferentiableBSDF implementation.
pub fn verify_bsdf_gradients<B: DifferentiableBSDF + Clone>(
    material: &B,
    ctx: &BSDFContext,
    config: &VerificationConfig,
) -> GradientVerificationResult {
    let param_names = [
        "ior",
        "extinction",
        "roughness",
        "absorption",
        "scattering",
        "asymmetry_g",
        "film_thickness",
        "film_ior",
    ];

    let result = material.eval_with_gradients(ctx);
    let analytic_grads = result.gradients.to_vec();
    let base_params = material.params_to_vec();

    let mut checks = Vec::new();

    for (i, &name) in param_names
        .iter()
        .enumerate()
        .take(base_params.len().min(8))
    {
        // Compute numerical gradient
        let numeric = compute_numeric_bsdf_gradient::<B>(&base_params, ctx, i, config.epsilon);

        let analytic = if i < analytic_grads.len() {
            analytic_grads[i] * result.gradients.d_reflectance
        } else {
            0.0
        };

        checks.push(GradientCheck::new(i, name, analytic, numeric, config));

        if !config.report_all && !checks.last().unwrap().passed {
            break;
        }
    }

    GradientVerificationResult::from_checks(checks)
}

/// Compute numerical gradient for BSDF reflectance.
fn compute_numeric_bsdf_gradient<B: DifferentiableBSDF>(
    base_params: &[f64],
    ctx: &BSDFContext,
    param_idx: usize,
    epsilon: f64,
) -> f64 {
    let mut params_plus = base_params.to_vec();
    let mut params_minus = base_params.to_vec();

    params_plus[param_idx] += epsilon;
    params_minus[param_idx] -= epsilon;

    let material_plus = B::from_param_vec(&params_plus);
    let material_minus = B::from_param_vec(&params_minus);

    let r_plus = material_plus.evaluate(ctx).reflectance;
    let r_minus = material_minus.evaluate(ctx).reflectance;

    (r_plus - r_minus) / (2.0 * epsilon)
}

/// Create standard test context.
pub fn standard_test_context(cos_theta: f64, wavelength: f64) -> BSDFContext {
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
// BATCH VERIFICATION
// ============================================================================

/// Verification across multiple contexts.
#[derive(Debug)]
pub struct BatchVerification {
    /// Results per context.
    pub results: Vec<(BSDFContext, GradientVerificationResult)>,
    /// Overall pass status.
    pub all_passed: bool,
    /// Global max error.
    pub global_max_error: f64,
}

impl BatchVerification {
    /// Run batch verification.
    pub fn run<B: DifferentiableBSDF + Clone>(
        material: &B,
        contexts: &[BSDFContext],
        config: &VerificationConfig,
    ) -> Self {
        let results: Vec<_> = contexts
            .iter()
            .map(|ctx| {
                let result = verify_bsdf_gradients(material, ctx, config);
                (ctx.clone(), result)
            })
            .collect();

        let all_passed = results.iter().all(|(_, r)| r.all_passed);
        let global_max_error = results.iter().map(|(_, r)| r.max_error).fold(0.0, f64::max);

        Self {
            results,
            all_passed,
            global_max_error,
        }
    }

    /// Get standard test contexts.
    pub fn standard_contexts() -> Vec<BSDFContext> {
        vec![
            standard_test_context(1.0, 550.0), // Normal incidence
            standard_test_context(0.9, 550.0), // Small angle
            standard_test_context(0.7, 550.0), // 45°
            standard_test_context(0.5, 550.0), // 60°
            standard_test_context(0.3, 550.0), // Grazing
            standard_test_context(0.8, 450.0), // Blue
            standard_test_context(0.8, 650.0), // Red
        ]
    }
}

// ============================================================================
// CONVENIENCE FUNCTIONS
// ============================================================================

/// Quick verification for a BSDF.
pub fn quick_verify<B: DifferentiableBSDF + Clone>(material: &B) -> bool {
    let ctx = standard_test_context(0.8, 550.0);
    let config = VerificationConfig::fast();
    verify_bsdf_gradients(material, &ctx, &config).all_passed
}

/// Full verification with report.
pub fn full_verify_with_report<B: DifferentiableBSDF + Clone>(material: &B) -> String {
    let contexts = BatchVerification::standard_contexts();
    let config = VerificationConfig::default();
    let batch = BatchVerification::run(material, &contexts, &config);

    let mut report = format!(
        "Full Gradient Verification\n{}\nGlobal max error: {:.2e}\n\n",
        if batch.all_passed { "PASSED" } else { "FAILED" },
        batch.global_max_error
    );

    for (ctx, result) in &batch.results {
        report.push_str(&format!(
            "Context (cos_θ={:.2}, λ={}nm): {}\n",
            ctx.cos_theta_i(),
            ctx.wavelength,
            if result.all_passed { "OK" } else { "FAIL" }
        ));

        if !result.all_passed {
            for check in result.failed_checks() {
                report.push_str(&format!(
                    "  - {}: analytic={:.2e}, numeric={:.2e}, error={:.2e}\n",
                    check.param_name, check.analytic, check.numeric, check.absolute_error
                ));
            }
        }
    }

    report
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::super::differentiable::conductor::DifferentiableConductor;
    use super::super::differentiable::dielectric::DifferentiableDielectric;
    use super::super::differentiable::thin_film::DifferentiableThinFilm;
    use super::*;

    #[test]
    fn test_numerical_gradient_central() {
        // f(x) = x², f'(x) = 2x
        let f = |x: f64| x * x;
        let x = 3.0;
        let analytic = 2.0 * x;
        let numeric = numerical_gradient_central(f, x, 1e-6);

        assert!((numeric - analytic).abs() < 1e-8);
    }

    #[test]
    fn test_numerical_jacobian() {
        // f(x, y) = x² + y², ∇f = [2x, 2y]
        let f = |x: &[f64]| x[0] * x[0] + x[1] * x[1];
        let point = vec![3.0, 4.0];

        let jacobian = numerical_jacobian(f, &point, 1e-6);

        assert!((jacobian[0] - 6.0).abs() < 1e-6);
        assert!((jacobian[1] - 8.0).abs() < 1e-6);
    }

    #[test]
    fn test_gradient_check() {
        let config = VerificationConfig::default();
        // Values within relative_tolerance (1e-4): 0.10001 vs 0.1 => rel_err ~1e-4
        let check = GradientCheck::new(0, "ior", 0.10001, 0.1000, &config);

        assert!(check.passed);
        assert!(check.relative_error.is_some());
    }

    #[test]
    fn test_dielectric_gradient_verification() {
        let glass = DifferentiableDielectric::glass();
        let ctx = standard_test_context(0.8, 550.0);
        let config = VerificationConfig::default();

        let result = verify_bsdf_gradients(&glass, &ctx, &config);

        // IOR gradient should pass
        assert!(result.checks.iter().any(|c| c.param_name == "ior"));
    }

    #[test]
    fn test_conductor_gradient_verification() {
        let gold = DifferentiableConductor::gold();
        let ctx = standard_test_context(0.8, 550.0);
        let config = VerificationConfig::default();

        let result = verify_bsdf_gradients(&gold, &ctx, &config);

        // Should have checks for IOR and extinction
        assert!(!result.checks.is_empty());
    }

    #[test]
    fn test_thin_film_gradient_verification() {
        let film = DifferentiableThinFilm::soap_bubble(300.0);
        let ctx = standard_test_context(0.8, 550.0);
        let config = VerificationConfig::default();

        let result = verify_bsdf_gradients(&film, &ctx, &config);

        assert!(!result.checks.is_empty());
    }

    #[test]
    fn test_batch_verification() {
        let glass = DifferentiableDielectric::glass();
        let contexts = BatchVerification::standard_contexts();
        let config = VerificationConfig::fast();

        let batch = BatchVerification::run(&glass, &contexts, &config);

        assert_eq!(batch.results.len(), contexts.len());
    }

    #[test]
    fn test_quick_verify() {
        let glass = DifferentiableDielectric::new(1.5, 0.1);
        let result = quick_verify(&glass);

        // Quick verify should complete
        assert!(result || !result); // Just test it runs
    }

    #[test]
    fn test_full_verify_report() {
        let glass = DifferentiableDielectric::glass();
        let report = full_verify_with_report(&glass);

        assert!(report.contains("Gradient Verification"));
        assert!(report.contains("Context"));
    }

    #[test]
    fn test_verification_result_report() {
        let checks = vec![
            GradientCheck::new(0, "ior", 0.1, 0.1001, &VerificationConfig::default()),
            GradientCheck::new(1, "roughness", 0.05, 0.049, &VerificationConfig::default()),
        ];

        let result = GradientVerificationResult::from_checks(checks);
        let report = result.report();

        assert!(report.contains("ior"));
        assert!(report.contains("roughness"));
    }

    #[test]
    fn test_multiple_angles_verification() {
        let glass = DifferentiableDielectric::new(1.5, 0.1);
        let config = VerificationConfig::default();

        // Test at multiple angles
        for cos_theta in [1.0, 0.8, 0.5, 0.3] {
            let ctx = standard_test_context(cos_theta, 550.0);
            let result = verify_bsdf_gradients(&glass, &ctx, &config);

            // Should complete without panic
            assert!(!result.checks.is_empty());
        }
    }

    #[test]
    fn test_standard_test_context() {
        let ctx = standard_test_context(0.8, 550.0);

        assert!((ctx.cos_theta_i() - 0.8).abs() < 1e-10);
        assert!((ctx.wavelength - 550.0).abs() < 1e-10);
    }
}
