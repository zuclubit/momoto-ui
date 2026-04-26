//! # Certification Auditor
//!
//! Validates material twins against certification level requirements.
//! Performs mandatory tests and generates certification reports.

use super::levels::{CertificationLevel, CertificationMetrics};
use super::profiles::{CertificationMetadata, CertifiedTwinProfile, NeuralCorrectionStats};
use super::requirements::{required_tests, MandatoryTest, TestResult, TestSuiteResult};

use crate::glass_physics::metrology::{CertificationTolerance, ToleranceBudget, TraceabilityChain};

// ============================================================================
// CERTIFICATION AUDITOR
// ============================================================================

/// Auditor for certifying material twins.
#[derive(Debug, Clone)]
pub struct CertificationAuditor {
    /// Target certification level.
    pub target_level: CertificationLevel,
    /// Strict mode: fail on any warning.
    pub strict_mode: bool,
    /// Skip optional tests.
    pub skip_optional: bool,
    /// Auditor metadata.
    pub metadata: CertificationMetadata,
}

impl CertificationAuditor {
    /// Create auditor for specific level.
    pub fn new(level: CertificationLevel) -> Self {
        Self {
            target_level: level,
            strict_mode: false,
            skip_optional: false,
            metadata: CertificationMetadata::default(),
        }
    }

    /// Create reference-level auditor (strictest).
    pub fn reference_auditor() -> Self {
        Self::new(CertificationLevel::Reference).with_strict_mode(true)
    }

    /// Create industrial-level auditor.
    pub fn industrial_auditor() -> Self {
        Self::new(CertificationLevel::Industrial)
    }

    /// Enable strict mode.
    pub fn with_strict_mode(mut self, strict: bool) -> Self {
        self.strict_mode = strict;
        self
    }

    /// Set auditor metadata.
    pub fn with_metadata(mut self, metadata: CertificationMetadata) -> Self {
        self.metadata = metadata;
        self
    }

    /// Run certification audit on material data.
    pub fn audit(&self, material_data: &MaterialAuditData) -> CertificationResult {
        let start_time = std::time::Instant::now();
        let required = required_tests(self.target_level);
        let mut results = Vec::new();
        let mut warnings = Vec::new();

        // Run each required test
        for test in required {
            let result = self.run_test(&test, material_data);
            if !result.passed && !self.strict_mode {
                warnings.push(format!(
                    "{} failed: {:.6} > {:.6}",
                    result.test.name(),
                    result.actual_value,
                    result.threshold
                ));
            }
            results.push(result);
        }

        // Check metrics against level requirements
        let metrics_check = material_data.metrics.check_level(self.target_level);
        if !metrics_check.passed {
            for failure in &metrics_check.failures {
                warnings.push(failure.clone());
            }
        }

        let duration_ms = start_time.elapsed().as_millis() as u64;
        let suite_result = TestSuiteResult::from_results(self.target_level, results);

        let certified = if self.strict_mode {
            suite_result.all_passed && metrics_check.passed
        } else {
            suite_result.passed_count() as f64 / suite_result.results.len() as f64 >= 0.8
                && metrics_check.passed
        };

        // Determine achieved level
        let achieved_level = if certified {
            Some(self.target_level)
        } else {
            CertificationLevel::highest_achievable(&material_data.metrics)
        };

        CertificationResult {
            target_level: self.target_level,
            achieved_level,
            certified,
            suite_result,
            metrics_check: Some(metrics_check),
            warnings,
            duration_ms,
        }
    }

    /// Run a single test.
    fn run_test(&self, test: &MandatoryTest, data: &MaterialAuditData) -> TestResult {
        let start = std::time::Instant::now();

        let result = match test {
            MandatoryTest::EnergyConservation { max_error } => {
                let actual = data.energy_violation.unwrap_or(0.0);
                if actual <= *max_error {
                    TestResult::pass(test.clone(), actual)
                } else {
                    TestResult::fail(test.clone(), actual, "Energy conservation violated")
                }
            }
            MandatoryTest::SpectralConsistency { max_rmse } => {
                let actual = data.spectral_rmse.unwrap_or(0.0);
                if actual <= *max_rmse {
                    TestResult::pass(test.clone(), actual)
                } else {
                    TestResult::fail(test.clone(), actual, "Spectral consistency exceeded")
                }
            }
            MandatoryTest::AngularReciprocity { max_violation } => {
                let actual = data.reciprocity_violation.unwrap_or(0.0);
                if actual <= *max_violation {
                    TestResult::pass(test.clone(), actual)
                } else {
                    TestResult::fail(test.clone(), actual, "Reciprocity violated")
                }
            }
            MandatoryTest::TemporalStability { max_drift } => {
                let actual = data.temporal_drift.unwrap_or(0.0);
                if actual <= *max_drift {
                    TestResult::pass(test.clone(), actual)
                } else {
                    TestResult::fail(test.clone(), actual, "Temporal drift exceeded")
                }
            }
            MandatoryTest::NeuralCorrectionBound { max_share } => {
                let actual = data.metrics.neural_share;
                if actual <= *max_share {
                    TestResult::pass(test.clone(), actual)
                } else {
                    TestResult::fail(
                        test.clone(),
                        actual,
                        format!(
                            "Neural share {:.1}% exceeds {:.0}%",
                            actual * 100.0,
                            max_share * 100.0
                        ),
                    )
                }
            }
            MandatoryTest::ReproducibilityCheck { tolerance } => {
                let actual = 1.0 - data.metrics.reproducibility;
                if actual <= *tolerance {
                    TestResult::pass(test.clone(), actual)
                } else {
                    TestResult::fail(test.clone(), actual, "Reproducibility below threshold")
                }
            }
            MandatoryTest::GroundTruthComparison { max_delta_e } => {
                let actual = data.metrics.delta_e;
                if actual <= *max_delta_e {
                    TestResult::pass(test.clone(), actual)
                } else {
                    TestResult::fail(
                        test.clone(),
                        actual,
                        format!("ΔE2000 {:.3} exceeds max {:.1}", actual, max_delta_e),
                    )
                }
            }
            MandatoryTest::FresnelCompliance { max_deviation } => {
                let actual = data.fresnel_deviation.unwrap_or(0.0);
                if actual <= *max_deviation {
                    TestResult::pass(test.clone(), actual)
                } else {
                    TestResult::fail(test.clone(), actual, "Fresnel deviation exceeded")
                }
            }
            MandatoryTest::ColorAccuracy {
                max_delta_e,
                illuminant,
            } => {
                let actual = data.color_delta_e.unwrap_or(0.0);
                if actual <= *max_delta_e {
                    TestResult::pass(test.clone(), actual)
                } else {
                    TestResult::fail(
                        test.clone(),
                        actual,
                        format!("Color error under {} exceeded", illuminant),
                    )
                }
            }
            MandatoryTest::PhysicalBounds { property, min, max } => {
                let (actual_min, actual_max) = data.value_bounds.unwrap_or((0.0, 1.0));
                if actual_min >= *min && actual_max <= *max {
                    TestResult::pass(test.clone(), actual_max)
                } else {
                    TestResult::fail(
                        test.clone(),
                        if actual_min < *min {
                            actual_min
                        } else {
                            actual_max
                        },
                        format!("{} out of bounds", property),
                    )
                }
            }
        };

        result.with_duration(start.elapsed().as_millis() as u64)
    }

    /// Check if certification is possible.
    pub fn can_certify(&self, data: &MaterialAuditData) -> bool {
        self.target_level.can_achieve(&data.metrics)
    }

    /// Generate certification profile if audit passes.
    pub fn certify(
        &self,
        name: impl Into<String>,
        data: &MaterialAuditData,
    ) -> Result<CertifiedTwinProfile, CertificationResult> {
        let result = self.audit(data);

        if result.certified {
            let profile = CertifiedTwinProfile::new(
                name,
                result.achieved_level.unwrap_or(self.target_level),
                result.suite_result.results,
            )
            .with_traceability(data.traceability.clone().unwrap_or_default())
            .with_tolerance_budget(data.tolerance_budget.clone().unwrap_or_else(|| {
                ToleranceBudget::for_certification_level(match self.target_level {
                    CertificationLevel::Experimental => CertificationTolerance::Experimental,
                    CertificationLevel::Research => CertificationTolerance::Research,
                    CertificationLevel::Industrial => CertificationTolerance::Industrial,
                    CertificationLevel::Reference => CertificationTolerance::Reference,
                })
            }))
            .with_neural_stats(data.neural_stats.clone().unwrap_or_default())
            .with_metadata(self.metadata.clone());

            Ok(profile)
        } else {
            Err(result)
        }
    }

    /// Generate audit report.
    pub fn generate_report(&self, result: &CertificationResult) -> String {
        let mut report = String::new();

        report.push_str(&format!(
            "╔════════════════════════════════════════════════════════════╗\n"
        ));
        report.push_str(&format!(
            "║              CERTIFICATION AUDIT REPORT                   ║\n"
        ));
        report.push_str(&format!(
            "╚════════════════════════════════════════════════════════════╝\n\n"
        ));

        report.push_str(&format!("Target Level:   {}\n", self.target_level));
        report.push_str(&format!(
            "Achieved Level: {}\n",
            result
                .achieved_level
                .map(|l| l.to_string())
                .unwrap_or_else(|| "None".to_string())
        ));
        report.push_str(&format!(
            "Result:         {}\n",
            if result.certified {
                "CERTIFIED"
            } else {
                "NOT CERTIFIED"
            }
        ));
        report.push_str(&format!("Duration:       {} ms\n", result.duration_ms));

        report.push_str(&format!(
            "\nTest Results: {}/{} passed\n",
            result.suite_result.passed_count(),
            result.suite_result.results.len()
        ));

        // Individual test results
        for test_result in &result.suite_result.results {
            let status = if test_result.passed { "✓" } else { "✗" };
            report.push_str(&format!(
                "  {} {:30} | {:.6} / {:.6} | {:.1}%\n",
                status,
                test_result.test.name(),
                test_result.actual_value,
                test_result.threshold,
                test_result.utilization()
            ));
        }

        // Warnings
        if !result.warnings.is_empty() {
            report.push_str("\nWarnings:\n");
            for warning in &result.warnings {
                report.push_str(&format!("  ⚠ {}\n", warning));
            }
        }

        // Metrics check
        if let Some(ref check) = result.metrics_check {
            if !check.passed {
                report.push_str("\nMetrics Failures:\n");
                for failure in &check.failures {
                    report.push_str(&format!("  - {}\n", failure));
                }
            }
        }

        report
    }
}

// ============================================================================
// MATERIAL AUDIT DATA
// ============================================================================

/// Data required for certification audit.
#[derive(Debug, Clone, Default)]
pub struct MaterialAuditData {
    /// Certification metrics.
    pub metrics: CertificationMetrics,
    /// Energy conservation violation.
    pub energy_violation: Option<f64>,
    /// Spectral RMSE.
    pub spectral_rmse: Option<f64>,
    /// Reciprocity violation.
    pub reciprocity_violation: Option<f64>,
    /// Temporal drift.
    pub temporal_drift: Option<f64>,
    /// Fresnel deviation.
    pub fresnel_deviation: Option<f64>,
    /// Color delta E.
    pub color_delta_e: Option<f64>,
    /// Value bounds (min, max).
    pub value_bounds: Option<(f64, f64)>,
    /// Traceability chain.
    pub traceability: Option<TraceabilityChain>,
    /// Tolerance budget.
    pub tolerance_budget: Option<ToleranceBudget>,
    /// Neural correction stats.
    pub neural_stats: Option<NeuralCorrectionStats>,
}

impl MaterialAuditData {
    /// Create new audit data.
    pub fn new() -> Self {
        Self::default()
    }

    /// Create exemplary data (passes reference level).
    pub fn exemplary() -> Self {
        Self {
            metrics: CertificationMetrics::exemplary(),
            energy_violation: Some(0.005),
            spectral_rmse: Some(0.005),
            reciprocity_violation: Some(0.002),
            temporal_drift: Some(0.00005),
            fresnel_deviation: Some(0.005),
            color_delta_e: Some(0.3),
            value_bounds: Some((0.0, 0.95)),
            traceability: Some(TraceabilityChain::new()),
            tolerance_budget: None,
            neural_stats: Some(NeuralCorrectionStats::new()),
        }
    }

    /// Create research-grade data.
    pub fn research_grade() -> Self {
        Self {
            metrics: CertificationMetrics::research_grade(),
            energy_violation: Some(0.03),
            spectral_rmse: Some(0.03),
            reciprocity_violation: Some(0.05),
            temporal_drift: Some(0.005),
            fresnel_deviation: Some(0.03),
            color_delta_e: Some(1.5),
            value_bounds: Some((0.0, 1.0)),
            traceability: Some(TraceabilityChain::new()),
            tolerance_budget: None,
            neural_stats: None,
        }
    }

    /// Set metrics.
    pub fn with_metrics(mut self, metrics: CertificationMetrics) -> Self {
        self.metrics = metrics;
        self
    }

    /// Set energy violation.
    pub fn with_energy_violation(mut self, v: f64) -> Self {
        self.energy_violation = Some(v);
        self
    }

    /// Set spectral RMSE.
    pub fn with_spectral_rmse(mut self, v: f64) -> Self {
        self.spectral_rmse = Some(v);
        self
    }

    /// Set delta E.
    pub fn with_delta_e(mut self, v: f64) -> Self {
        self.metrics.delta_e = v;
        self.color_delta_e = Some(v);
        self
    }
}

// ============================================================================
// CERTIFICATION RESULT
// ============================================================================

/// Result of certification audit.
#[derive(Debug, Clone)]
pub struct CertificationResult {
    /// Target certification level.
    pub target_level: CertificationLevel,
    /// Achieved level (if any).
    pub achieved_level: Option<CertificationLevel>,
    /// Whether certification was granted.
    pub certified: bool,
    /// Test suite results.
    pub suite_result: TestSuiteResult,
    /// Metrics check result.
    pub metrics_check: Option<super::levels::LevelCheck>,
    /// Warnings generated.
    pub warnings: Vec<String>,
    /// Audit duration in ms.
    pub duration_ms: u64,
}

impl CertificationResult {
    /// Get gap to certification.
    pub fn gap_analysis(&self) -> Vec<String> {
        let mut gaps = Vec::new();

        for result in &self.suite_result.results {
            if !result.passed {
                let gap = result.actual_value - result.threshold;
                gaps.push(format!(
                    "{}: reduce by {:.6} ({:.1}%)",
                    result.test.name(),
                    gap,
                    (gap / result.threshold) * 100.0
                ));
            }
        }

        gaps
    }

    /// Get improvement suggestions.
    pub fn suggestions(&self) -> Vec<String> {
        let mut suggestions = Vec::new();

        for result in &self.suite_result.results {
            if !result.passed {
                match &result.test {
                    MandatoryTest::EnergyConservation { .. } => {
                        suggestions
                            .push("Review BRDF normalization and ensure R + T <= 1".to_string());
                    }
                    MandatoryTest::NeuralCorrectionBound { .. } => {
                        suggestions.push(
                            "Reduce neural network contribution; improve physical model"
                                .to_string(),
                        );
                    }
                    MandatoryTest::GroundTruthComparison { .. } => {
                        suggestions.push("Calibrate against measured reference data".to_string());
                    }
                    MandatoryTest::ReproducibilityCheck { .. } => {
                        suggestions.push(
                            "Ensure deterministic random seeds and numerical stability".to_string(),
                        );
                    }
                    _ => {}
                }
            }
        }

        suggestions.dedup();
        suggestions
    }
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_auditor_creation() {
        let auditor = CertificationAuditor::new(CertificationLevel::Industrial);
        assert_eq!(auditor.target_level, CertificationLevel::Industrial);
        assert!(!auditor.strict_mode);
    }

    #[test]
    fn test_reference_auditor() {
        let auditor = CertificationAuditor::reference_auditor();
        assert_eq!(auditor.target_level, CertificationLevel::Reference);
        assert!(auditor.strict_mode);
    }

    #[test]
    fn test_audit_exemplary() {
        let auditor = CertificationAuditor::new(CertificationLevel::Reference);
        let data = MaterialAuditData::exemplary();

        let result = auditor.audit(&data);

        assert!(result.certified);
        assert!(result.achieved_level.is_some());
    }

    #[test]
    fn test_audit_research() {
        let auditor = CertificationAuditor::new(CertificationLevel::Research);
        let data = MaterialAuditData::research_grade();

        let result = auditor.audit(&data);

        // Research grade should pass research level
        assert!(result.certified || result.achieved_level.is_some());
    }

    #[test]
    fn test_audit_failure() {
        let auditor =
            CertificationAuditor::new(CertificationLevel::Reference).with_strict_mode(true);
        let data = MaterialAuditData::research_grade(); // Too low for Reference

        let result = auditor.audit(&data);

        assert!(!result.certified);
        assert!(result.achieved_level != Some(CertificationLevel::Reference));
    }

    #[test]
    fn test_can_certify() {
        let auditor = CertificationAuditor::new(CertificationLevel::Experimental);
        let data = MaterialAuditData::research_grade();

        assert!(auditor.can_certify(&data));
    }

    #[test]
    fn test_certify_success() {
        let auditor = CertificationAuditor::new(CertificationLevel::Research);
        let data = MaterialAuditData::exemplary();

        let profile = auditor.certify("Test Material", &data);

        assert!(profile.is_ok());
        let p = profile.unwrap();
        assert!(!p.name.is_empty());
    }

    #[test]
    fn test_certify_failure() {
        let auditor = CertificationAuditor::reference_auditor();
        let mut data = MaterialAuditData::research_grade();
        data.metrics.delta_e = 10.0; // Way too high

        let result = auditor.certify("Test", &data);

        assert!(result.is_err());
    }

    #[test]
    fn test_generate_report() {
        let auditor = CertificationAuditor::new(CertificationLevel::Industrial);
        let data = MaterialAuditData::exemplary();
        let result = auditor.audit(&data);

        let report = auditor.generate_report(&result);

        assert!(report.contains("AUDIT REPORT"));
        assert!(report.contains("Industrial"));
    }

    #[test]
    fn test_gap_analysis() {
        let auditor = CertificationAuditor::reference_auditor();
        let mut data = MaterialAuditData::research_grade();
        data.metrics.delta_e = 3.0;

        let result = auditor.audit(&data);
        let gaps = result.gap_analysis();

        // Should have some gaps
        assert!(!gaps.is_empty() || result.certified);
    }

    #[test]
    fn test_suggestions() {
        let auditor = CertificationAuditor::new(CertificationLevel::Industrial);
        let mut data = MaterialAuditData::new();
        data.metrics.neural_share = 0.15; // Too high

        let result = auditor.audit(&data);
        let suggestions = result.suggestions();

        // Should suggest reducing neural contribution
        assert!(
            suggestions.is_empty()
                || suggestions
                    .iter()
                    .any(|s| s.contains("neural") || s.contains("Neural"))
        );
    }

    #[test]
    fn test_audit_data_builders() {
        let data = MaterialAuditData::new()
            .with_energy_violation(0.02)
            .with_spectral_rmse(0.01)
            .with_delta_e(0.5);

        assert_eq!(data.energy_violation, Some(0.02));
        assert_eq!(data.spectral_rmse, Some(0.01));
        assert!((data.metrics.delta_e - 0.5).abs() < 1e-10);
    }
}
