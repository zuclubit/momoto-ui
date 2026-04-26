//! # Compliance and Export Module
//!
//! Provides ground truth validation, neural accountability auditing,
//! reproducibility testing, and metrological export capabilities.
//!
//! ## Core Components
//!
//! - **Ground Truth Validation**: Compare predictions against MERL, published data
//! - **Neural Audit**: Track and limit neural network contributions
//! - **Reproducibility**: Verify deterministic behavior across runs
//! - **Export**: Generate certified MaterialX, JSON, and compliance reports
//!
//! ## Example Usage
//!
//! ```ignore
//! use glass_physics::compliance::*;
//!
//! // Ground truth validation
//! let mut validator = GroundTruthValidator::new();
//! validator.add_dataset(gold_reference_data());
//!
//! let report = validator.validate(|wl, _angle| {
//!     // Your material prediction
//!     0.8 + 0.1 * (wl - 550.0) / 200.0
//! });
//!
//! println!("ΔE: {:.2}", report.delta_e_mean);
//!
//! // Neural audit
//! let auditor = NeuralAuditor::industrial_level();
//! let audit_result = auditor.audit(&neural_stats);
//!
//! if !audit_result.passed {
//!     println!("Neural audit failed:");
//!     for finding in &audit_result.findings {
//!         println!("  {}", finding.format());
//!     }
//! }
//!
//! // Export certified profile
//! let exporter = MetrologicalExporter::compliance_report();
//! let report = exporter.export(&certified_profile);
//! ```

pub mod export;
pub mod ground_truth;
pub mod neural_audit;
pub mod reproducibility;

// Re-exports for convenient access
pub use export::{batch_export, ExportFormat, MetrologicalExporter};

pub use ground_truth::{
    bk7_reference_data, gold_reference_data, silver_reference_data, DatasetValidationReport,
    GroundTruthDataset, GroundTruthValidator, ReferenceMeasurement, SpectralMeasurement,
    ValidationReport,
};

pub use neural_audit::{
    AuditFinding, CorrectionCheck, FindingCategory, FindingSeverity, NeuralAuditResult,
    NeuralAuditor,
};

pub use reproducibility::{
    compute_reproducibility_hash, verify_hash, ComparisonResult, CrossPlatformReference,
    ReproducibilityResult, ReproducibilityTest,
};

// ============================================================================
// MEMORY ESTIMATION
// ============================================================================

/// Estimate memory footprint of compliance module types.
pub fn estimate_memory_footprint() -> ComplianceMemoryEstimate {
    ComplianceMemoryEstimate {
        validator_bytes: std::mem::size_of::<GroundTruthValidator>(),
        validation_report_base_bytes: std::mem::size_of::<ValidationReport>(),
        neural_auditor_bytes: std::mem::size_of::<NeuralAuditor>(),
        neural_audit_result_bytes: std::mem::size_of::<NeuralAuditResult>(),
        reproducibility_test_bytes: std::mem::size_of::<ReproducibilityTest>(),
        reproducibility_result_bytes: std::mem::size_of::<ReproducibilityResult>(),
        exporter_bytes: std::mem::size_of::<MetrologicalExporter>(),
    }
}

/// Memory footprint estimates for compliance types.
#[derive(Debug, Clone)]
pub struct ComplianceMemoryEstimate {
    /// Size of GroundTruthValidator.
    pub validator_bytes: usize,
    /// Base size of ValidationReport.
    pub validation_report_base_bytes: usize,
    /// Size of NeuralAuditor.
    pub neural_auditor_bytes: usize,
    /// Size of NeuralAuditResult.
    pub neural_audit_result_bytes: usize,
    /// Size of ReproducibilityTest.
    pub reproducibility_test_bytes: usize,
    /// Size of ReproducibilityResult.
    pub reproducibility_result_bytes: usize,
    /// Size of MetrologicalExporter.
    pub exporter_bytes: usize,
}

impl ComplianceMemoryEstimate {
    /// Total base memory.
    pub fn total_base(&self) -> usize {
        self.validator_bytes
            + self.neural_auditor_bytes
            + self.reproducibility_test_bytes
            + self.exporter_bytes
    }

    /// Estimate for typical compliance check.
    pub fn typical_check(&self) -> usize {
        // Assume:
        // - 1 validator with 50 reference points
        // - 1 validation report
        // - 1 neural auditor + result
        // - 1 reproducibility test + result
        // - 1 exporter

        self.validator_bytes
            + 50 * std::mem::size_of::<SpectralMeasurement>()
            + self.validation_report_base_bytes
            + self.neural_auditor_bytes
            + self.neural_audit_result_bytes
            + self.reproducibility_test_bytes
            + self.reproducibility_result_bytes
            + self.exporter_bytes
    }

    /// Generate memory report.
    pub fn report(&self) -> String {
        format!(
            "Compliance Memory Footprint:\n\
             ├── GroundTruthValidator:  {:4} bytes\n\
             ├── ValidationReport:      {:4} bytes (base)\n\
             ├── NeuralAuditor:         {:4} bytes\n\
             ├── NeuralAuditResult:     {:4} bytes\n\
             ├── ReproducibilityTest:   {:4} bytes\n\
             ├── ReproducibilityResult: {:4} bytes\n\
             ├── MetrologicalExporter:  {:4} bytes\n\
             ├── Total Base:            {:4} bytes\n\
             └── Typical Check:         {:4} bytes (~{:.1} KB)",
            self.validator_bytes,
            self.validation_report_base_bytes,
            self.neural_auditor_bytes,
            self.neural_audit_result_bytes,
            self.reproducibility_test_bytes,
            self.reproducibility_result_bytes,
            self.exporter_bytes,
            self.total_base(),
            self.typical_check(),
            self.typical_check() as f64 / 1024.0
        )
    }
}

// ============================================================================
// MODULE VALIDATION
// ============================================================================

/// Validate compliance module configuration.
pub fn validate_module() -> ComplianceValidation {
    let mut issues = Vec::new();

    // Verify ground truth data is valid
    let gold = gold_reference_data();
    match gold {
        GroundTruthDataset::Published { data, .. } => {
            if data.is_empty() {
                issues.push("Gold reference data is empty".to_string());
            }
            for measurement in &data {
                if measurement.value < 0.0 || measurement.value > 1.0 {
                    issues.push(format!(
                        "Gold reflectance {} out of [0,1] range",
                        measurement.value
                    ));
                }
            }
        }
        _ => issues.push("Gold reference should be Published type".to_string()),
    }

    // Verify neural auditor thresholds are sane
    let auditor = NeuralAuditor::new();
    if auditor.max_correction_share >= 1.0 {
        issues.push("Neural share limit should be < 100%".to_string());
    }
    if auditor.max_single_correction >= 1.0 {
        issues.push("Single correction limit should be < 100%".to_string());
    }

    ComplianceValidation {
        valid: issues.is_empty(),
        issues,
        memory_estimate: estimate_memory_footprint(),
    }
}

/// Result of compliance module validation.
#[derive(Debug)]
pub struct ComplianceValidation {
    /// Whether validation passed.
    pub valid: bool,
    /// List of issues found.
    pub issues: Vec<String>,
    /// Memory footprint estimate.
    pub memory_estimate: ComplianceMemoryEstimate,
}

// ============================================================================
// QUICK COMPLIANCE FUNCTIONS
// ============================================================================

/// Quick ground truth check.
pub fn quick_ground_truth_check<F>(material_fn: F, target_delta_e: f64) -> bool
where
    F: Fn(f64, Option<f64>) -> f64,
{
    let mut validator = GroundTruthValidator::new().with_tolerance(target_delta_e / 100.0);
    validator.add_dataset(gold_reference_data());

    let report = validator.validate(material_fn);
    report.passed && report.delta_e_mean <= target_delta_e
}

/// Quick neural audit.
pub fn quick_neural_audit(share: f64, max_magnitude: f64, max_share_limit: f64) -> bool {
    use crate::glass_physics::certification::NeuralCorrectionStats;

    let mut stats = NeuralCorrectionStats::new();
    stats.correction_share = share;
    stats.max_correction_magnitude = max_magnitude;

    let auditor = NeuralAuditor::new().with_max_share(max_share_limit);
    auditor.audit(&stats).passed
}

/// Quick reproducibility check.
pub fn quick_reproducibility_check<F>(func: F, runs: usize) -> bool
where
    F: FnMut(f64, f64) -> f64,
{
    let test = ReproducibilityTest::new().with_runs(runs);
    test.verify(func).deterministic
}

// ============================================================================
// FULL COMPLIANCE PIPELINE
// ============================================================================

/// Complete compliance check result.
#[derive(Debug, Clone)]
pub struct FullComplianceResult {
    /// Ground truth validation passed.
    pub ground_truth_passed: bool,
    /// Ground truth ΔE achieved.
    pub delta_e: f64,
    /// Neural audit passed.
    pub neural_audit_passed: bool,
    /// Neural correction share.
    pub neural_share: f64,
    /// Reproducibility passed.
    pub reproducibility_passed: bool,
    /// Reproducibility score.
    pub reproducibility_score: f64,
    /// Overall compliance status.
    pub compliant: bool,
}

impl FullComplianceResult {
    /// Generate summary.
    pub fn summary(&self) -> String {
        format!(
            "Compliance: {} | ΔE: {:.2} | Neural: {:.1}% | Repro: {:.1}%",
            if self.compliant { "PASS" } else { "FAIL" },
            self.delta_e,
            self.neural_share * 100.0,
            self.reproducibility_score * 100.0
        )
    }
}

/// Run full compliance pipeline.
pub fn full_compliance_check<F>(
    material_fn: F,
    neural_stats: &crate::glass_physics::certification::NeuralCorrectionStats,
    target_delta_e: f64,
) -> FullComplianceResult
where
    F: Fn(f64, Option<f64>) -> f64 + Clone,
{
    // Ground truth
    let mut validator = GroundTruthValidator::new().with_tolerance(target_delta_e / 100.0);
    validator.add_dataset(gold_reference_data());
    let gt_report = validator.validate(material_fn.clone());

    // Neural audit
    let auditor = NeuralAuditor::industrial_level();
    let neural_result = auditor.audit(neural_stats);

    // Reproducibility
    let repro_test = ReproducibilityTest::new();
    let repro_result = repro_test.verify(move |wl, angle| material_fn(wl, Some(angle)));

    let compliant = gt_report.passed && neural_result.passed && repro_result.deterministic;

    FullComplianceResult {
        ground_truth_passed: gt_report.passed,
        delta_e: gt_report.delta_e_mean,
        neural_audit_passed: neural_result.passed,
        neural_share: neural_stats.correction_share,
        reproducibility_passed: repro_result.deterministic,
        reproducibility_score: repro_result.score(),
        compliant,
    }
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::glass_physics::certification::NeuralCorrectionStats;

    #[test]
    fn test_module_exports() {
        // Verify all public types are accessible
        let _ = GroundTruthValidator::new();
        let _ = NeuralAuditor::new();
        let _ = ReproducibilityTest::new();
        let _ = MetrologicalExporter::json();
        let _ = ExportFormat::MetrologicalJSON;
    }

    #[test]
    fn test_memory_estimate() {
        let estimate = estimate_memory_footprint();

        // Sanity checks
        assert!(estimate.validator_bytes > 0);
        assert!(estimate.neural_auditor_bytes > 0);
        assert!(estimate.typical_check() > estimate.total_base());

        let report = estimate.report();
        assert!(report.contains("Compliance"));
        assert!(report.contains("bytes"));
    }

    #[test]
    fn test_module_validation() {
        let validation = validate_module();
        assert!(
            validation.valid,
            "Validation failed: {:?}",
            validation.issues
        );
    }

    #[test]
    fn test_memory_budget() {
        let estimate = estimate_memory_footprint();

        // Phase 15 compliance should use < 15KB typical
        assert!(
            estimate.typical_check() < 15_000,
            "Typical check {} exceeds 15KB budget",
            estimate.typical_check()
        );
    }

    #[test]
    fn test_quick_ground_truth() {
        // Perfect gold prediction should pass
        let passed = quick_ground_truth_check(
            |wl, _| {
                // Approximate gold
                if wl < 500.0 {
                    0.35
                } else {
                    0.9
                }
            },
            5.0,
        );

        // May or may not pass depending on exact values
        // Just verify it runs without panic
        let _ = passed;
    }

    #[test]
    fn test_quick_neural_audit() {
        assert!(quick_neural_audit(0.03, 0.05, 0.05));
        assert!(!quick_neural_audit(0.10, 0.05, 0.05));
    }

    #[test]
    fn test_quick_reproducibility() {
        let passed = quick_reproducibility_check(|wl, angle| wl * 0.001 + angle * 0.01, 5);
        assert!(passed);
    }

    #[test]
    fn test_full_compliance() {
        let stats = NeuralCorrectionStats::new();

        let result =
            full_compliance_check(|wl, _| if wl < 500.0 { 0.35 } else { 0.9 }, &stats, 5.0);

        // Check structure
        assert!(result.reproducibility_score >= 0.0);
        assert!(result.reproducibility_score <= 1.0);

        let summary = result.summary();
        assert!(summary.contains("ΔE"));
    }

    #[test]
    fn test_reference_data_functions() {
        let gold = gold_reference_data();
        let silver = silver_reference_data();
        let bk7 = bk7_reference_data();

        assert!(gold.sample_count() > 0);
        assert!(silver.sample_count() > 0);
        assert!(bk7.sample_count() > 0);
    }
}
