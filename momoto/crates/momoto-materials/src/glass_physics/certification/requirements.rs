//! # Mandatory Test Requirements
//!
//! Defines the mandatory tests required for each certification level.
//! Each test has specific pass/fail criteria and validation procedures.

use super::levels::CertificationLevel;

// ============================================================================
// MANDATORY TEST DEFINITIONS
// ============================================================================

/// Mandatory tests for certification.
#[derive(Debug, Clone)]
pub enum MandatoryTest {
    /// Energy conservation check.
    EnergyConservation {
        /// Maximum allowed violation (fraction).
        max_error: f64,
    },
    /// Spectral consistency across wavelengths.
    SpectralConsistency {
        /// Maximum allowed RMSE.
        max_rmse: f64,
    },
    /// Angular reciprocity (Helmholtz).
    AngularReciprocity {
        /// Maximum violation in sr⁻¹.
        max_violation: f64,
    },
    /// Temporal stability over time.
    TemporalStability {
        /// Maximum drift allowed.
        max_drift: f64,
    },
    /// Neural correction bound.
    NeuralCorrectionBound {
        /// Maximum share of output from neural.
        max_share: f64,
    },
    /// Reproducibility across runs.
    ReproducibilityCheck {
        /// Maximum variance allowed.
        tolerance: f64,
    },
    /// Ground truth comparison.
    GroundTruthComparison {
        /// Maximum ΔE2000 allowed.
        max_delta_e: f64,
    },
    /// Fresnel law compliance.
    FresnelCompliance {
        /// Maximum deviation from Fresnel.
        max_deviation: f64,
    },
    /// Color accuracy under standard illuminant.
    ColorAccuracy {
        /// Maximum ΔE2000 for color.
        max_delta_e: f64,
        /// Illuminant to use.
        illuminant: String,
    },
    /// Physical bounds check.
    PhysicalBounds {
        /// Property being checked.
        property: String,
        /// Minimum value.
        min: f64,
        /// Maximum value.
        max: f64,
    },
}

impl MandatoryTest {
    /// Get test name.
    pub fn name(&self) -> &'static str {
        match self {
            MandatoryTest::EnergyConservation { .. } => "Energy Conservation",
            MandatoryTest::SpectralConsistency { .. } => "Spectral Consistency",
            MandatoryTest::AngularReciprocity { .. } => "Angular Reciprocity",
            MandatoryTest::TemporalStability { .. } => "Temporal Stability",
            MandatoryTest::NeuralCorrectionBound { .. } => "Neural Correction Bound",
            MandatoryTest::ReproducibilityCheck { .. } => "Reproducibility",
            MandatoryTest::GroundTruthComparison { .. } => "Ground Truth Comparison",
            MandatoryTest::FresnelCompliance { .. } => "Fresnel Compliance",
            MandatoryTest::ColorAccuracy { .. } => "Color Accuracy",
            MandatoryTest::PhysicalBounds { .. } => "Physical Bounds",
        }
    }

    /// Get test description.
    pub fn description(&self) -> String {
        match self {
            MandatoryTest::EnergyConservation { max_error } => {
                format!(
                    "Verify total reflected + transmitted energy ≤ 1 (max error: {:.1}%)",
                    max_error * 100.0
                )
            }
            MandatoryTest::SpectralConsistency { max_rmse } => {
                format!(
                    "Verify spectral data is smooth and consistent (max RMSE: {:.4})",
                    max_rmse
                )
            }
            MandatoryTest::AngularReciprocity { max_violation } => {
                format!(
                    "Verify f(θi, θo) = f(θo, θi) (max violation: {:.4} sr⁻¹)",
                    max_violation
                )
            }
            MandatoryTest::TemporalStability { max_drift } => {
                format!(
                    "Verify results stable over repeated measurements (max drift: {:.4})",
                    max_drift
                )
            }
            MandatoryTest::NeuralCorrectionBound { max_share } => {
                format!(
                    "Verify neural contribution ≤ {:.0}% of output",
                    max_share * 100.0
                )
            }
            MandatoryTest::ReproducibilityCheck { tolerance } => {
                format!(
                    "Verify deterministic results across runs (tolerance: {:.6})",
                    tolerance
                )
            }
            MandatoryTest::GroundTruthComparison { max_delta_e } => {
                format!(
                    "Compare against measured reference data (max ΔE2000: {:.1})",
                    max_delta_e
                )
            }
            MandatoryTest::FresnelCompliance { max_deviation } => {
                format!(
                    "Verify Fresnel reflection at normal incidence (max deviation: {:.4})",
                    max_deviation
                )
            }
            MandatoryTest::ColorAccuracy {
                max_delta_e,
                illuminant,
            } => {
                format!(
                    "Color accuracy under {} illuminant (max ΔE2000: {:.1})",
                    illuminant, max_delta_e
                )
            }
            MandatoryTest::PhysicalBounds { property, min, max } => {
                format!("Verify {} in range [{:.4}, {:.4}]", property, min, max)
            }
        }
    }

    /// Get threshold value for the test.
    pub fn threshold(&self) -> f64 {
        match self {
            MandatoryTest::EnergyConservation { max_error } => *max_error,
            MandatoryTest::SpectralConsistency { max_rmse } => *max_rmse,
            MandatoryTest::AngularReciprocity { max_violation } => *max_violation,
            MandatoryTest::TemporalStability { max_drift } => *max_drift,
            MandatoryTest::NeuralCorrectionBound { max_share } => *max_share,
            MandatoryTest::ReproducibilityCheck { tolerance } => *tolerance,
            MandatoryTest::GroundTruthComparison { max_delta_e } => *max_delta_e,
            MandatoryTest::FresnelCompliance { max_deviation } => *max_deviation,
            MandatoryTest::ColorAccuracy { max_delta_e, .. } => *max_delta_e,
            MandatoryTest::PhysicalBounds { max, .. } => *max,
        }
    }
}

// ============================================================================
// TEST RESULT
// ============================================================================

/// Result of a mandatory test.
#[derive(Debug, Clone)]
pub struct TestResult {
    /// Test that was run.
    pub test: MandatoryTest,
    /// Whether test passed.
    pub passed: bool,
    /// Actual measured value.
    pub actual_value: f64,
    /// Threshold for passing.
    pub threshold: f64,
    /// Additional details.
    pub details: String,
    /// Test duration in milliseconds.
    pub duration_ms: u64,
}

impl TestResult {
    /// Create a passing result.
    pub fn pass(test: MandatoryTest, actual_value: f64) -> Self {
        let threshold = test.threshold();
        Self {
            test,
            passed: true,
            actual_value,
            threshold,
            details: String::new(),
            duration_ms: 0,
        }
    }

    /// Create a failing result.
    pub fn fail(test: MandatoryTest, actual_value: f64, details: impl Into<String>) -> Self {
        let threshold = test.threshold();
        Self {
            test,
            passed: false,
            actual_value,
            threshold,
            details: details.into(),
            duration_ms: 0,
        }
    }

    /// Set test duration.
    pub fn with_duration(mut self, ms: u64) -> Self {
        self.duration_ms = ms;
        self
    }

    /// Get margin (positive = passed with room).
    pub fn margin(&self) -> f64 {
        self.threshold - self.actual_value
    }

    /// Get utilization percentage.
    pub fn utilization(&self) -> f64 {
        if self.threshold > 0.0 {
            (self.actual_value / self.threshold) * 100.0
        } else {
            0.0
        }
    }

    /// Generate test result report.
    pub fn report(&self) -> String {
        let status = if self.passed { "PASS" } else { "FAIL" };
        let mut report = format!(
            "[{:^4}] {} | Actual: {:.6} | Threshold: {:.6} | Util: {:.1}%",
            status,
            self.test.name(),
            self.actual_value,
            self.threshold,
            self.utilization()
        );

        if !self.details.is_empty() {
            report.push_str(&format!("\n       Details: {}", self.details));
        }

        if self.duration_ms > 0 {
            report.push_str(&format!(" ({} ms)", self.duration_ms));
        }

        report
    }
}

// ============================================================================
// REQUIRED TESTS BY LEVEL
// ============================================================================

/// Get mandatory tests for a certification level.
pub fn required_tests(level: CertificationLevel) -> Vec<MandatoryTest> {
    match level {
        CertificationLevel::Experimental => vec![
            MandatoryTest::EnergyConservation { max_error: 0.10 },
            MandatoryTest::PhysicalBounds {
                property: "Reflectance".to_string(),
                min: 0.0,
                max: 1.0,
            },
        ],
        CertificationLevel::Research => vec![
            MandatoryTest::EnergyConservation { max_error: 0.05 },
            MandatoryTest::SpectralConsistency { max_rmse: 0.05 },
            MandatoryTest::NeuralCorrectionBound { max_share: 0.10 },
            MandatoryTest::ReproducibilityCheck { tolerance: 1e-4 },
            MandatoryTest::GroundTruthComparison { max_delta_e: 2.0 },
            MandatoryTest::PhysicalBounds {
                property: "Reflectance".to_string(),
                min: 0.0,
                max: 1.0,
            },
        ],
        CertificationLevel::Industrial => vec![
            MandatoryTest::EnergyConservation { max_error: 0.02 },
            MandatoryTest::SpectralConsistency { max_rmse: 0.02 },
            MandatoryTest::AngularReciprocity {
                max_violation: 0.01,
            },
            MandatoryTest::TemporalStability { max_drift: 0.001 },
            MandatoryTest::NeuralCorrectionBound { max_share: 0.05 },
            MandatoryTest::ReproducibilityCheck { tolerance: 1e-6 },
            MandatoryTest::GroundTruthComparison { max_delta_e: 1.0 },
            MandatoryTest::FresnelCompliance {
                max_deviation: 0.02,
            },
            MandatoryTest::ColorAccuracy {
                max_delta_e: 1.0,
                illuminant: "D65".to_string(),
            },
            MandatoryTest::PhysicalBounds {
                property: "Reflectance".to_string(),
                min: 0.0,
                max: 1.0,
            },
        ],
        CertificationLevel::Reference => vec![
            MandatoryTest::EnergyConservation { max_error: 0.01 },
            MandatoryTest::SpectralConsistency { max_rmse: 0.01 },
            MandatoryTest::AngularReciprocity {
                max_violation: 0.005,
            },
            MandatoryTest::TemporalStability { max_drift: 0.0001 },
            MandatoryTest::NeuralCorrectionBound { max_share: 0.02 },
            MandatoryTest::ReproducibilityCheck { tolerance: 1e-10 },
            MandatoryTest::GroundTruthComparison { max_delta_e: 0.5 },
            MandatoryTest::FresnelCompliance {
                max_deviation: 0.01,
            },
            MandatoryTest::ColorAccuracy {
                max_delta_e: 0.5,
                illuminant: "D65".to_string(),
            },
            MandatoryTest::ColorAccuracy {
                max_delta_e: 0.5,
                illuminant: "A".to_string(),
            },
            MandatoryTest::PhysicalBounds {
                property: "Reflectance".to_string(),
                min: 0.0,
                max: 1.0,
            },
            MandatoryTest::PhysicalBounds {
                property: "Transmittance".to_string(),
                min: 0.0,
                max: 1.0,
            },
        ],
    }
}

/// Get number of required tests for a level.
pub fn required_test_count(level: CertificationLevel) -> usize {
    required_tests(level).len()
}

// ============================================================================
// TEST SUITE RESULT
// ============================================================================

/// Result of running all required tests.
#[derive(Debug, Clone)]
pub struct TestSuiteResult {
    /// Certification level tested.
    pub level: CertificationLevel,
    /// Individual test results.
    pub results: Vec<TestResult>,
    /// Overall pass/fail.
    pub all_passed: bool,
    /// Total duration in milliseconds.
    pub total_duration_ms: u64,
}

impl TestSuiteResult {
    /// Create from test results.
    pub fn from_results(level: CertificationLevel, results: Vec<TestResult>) -> Self {
        let all_passed = results.iter().all(|r| r.passed);
        let total_duration_ms = results.iter().map(|r| r.duration_ms).sum();

        Self {
            level,
            results,
            all_passed,
            total_duration_ms,
        }
    }

    /// Get number of passed tests.
    pub fn passed_count(&self) -> usize {
        self.results.iter().filter(|r| r.passed).count()
    }

    /// Get number of failed tests.
    pub fn failed_count(&self) -> usize {
        self.results.iter().filter(|r| !r.passed).count()
    }

    /// Get failed tests.
    pub fn failed_tests(&self) -> Vec<&TestResult> {
        self.results.iter().filter(|r| !r.passed).collect()
    }

    /// Generate suite report.
    pub fn report(&self) -> String {
        let mut report = format!("{} Certification Test Suite\n", self.level);
        report.push_str(&format!(
            "Overall: {} ({}/{})\n",
            if self.all_passed { "PASSED" } else { "FAILED" },
            self.passed_count(),
            self.results.len()
        ));
        report.push_str(&format!(
            "Total Duration: {} ms\n\n",
            self.total_duration_ms
        ));

        report.push_str("Test Results:\n");
        for result in &self.results {
            report.push_str(&format!("  {}\n", result.report()));
        }

        if !self.all_passed {
            report.push_str("\nFailed Tests:\n");
            for result in self.failed_tests() {
                report.push_str(&format!(
                    "  - {}: {} (threshold: {})\n",
                    result.test.name(),
                    result.actual_value,
                    result.threshold
                ));
            }
        }

        report
    }
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mandatory_test_names() {
        let test = MandatoryTest::EnergyConservation { max_error: 0.05 };
        assert_eq!(test.name(), "Energy Conservation");
    }

    #[test]
    fn test_mandatory_test_description() {
        let test = MandatoryTest::NeuralCorrectionBound { max_share: 0.05 };
        let desc = test.description();
        assert!(desc.contains("neural"));
        assert!(desc.contains("5%"));
    }

    #[test]
    fn test_result_pass() {
        let test = MandatoryTest::EnergyConservation { max_error: 0.05 };
        let result = TestResult::pass(test, 0.02);

        assert!(result.passed);
        assert!(result.margin() > 0.0);
        assert!(result.utilization() < 100.0);
    }

    #[test]
    fn test_result_fail() {
        let test = MandatoryTest::EnergyConservation { max_error: 0.05 };
        let result = TestResult::fail(test, 0.08, "Exceeded threshold");

        assert!(!result.passed);
        assert!(result.margin() < 0.0);
        assert!(result.utilization() > 100.0);
    }

    #[test]
    fn test_required_tests_experimental() {
        let tests = required_tests(CertificationLevel::Experimental);
        assert!(tests.len() >= 2);
    }

    #[test]
    fn test_required_tests_reference() {
        let tests = required_tests(CertificationLevel::Reference);
        assert!(tests.len() >= 10); // Reference has most tests
    }

    #[test]
    fn test_increasing_requirements() {
        let exp_count = required_test_count(CertificationLevel::Experimental);
        let res_count = required_test_count(CertificationLevel::Research);
        let ind_count = required_test_count(CertificationLevel::Industrial);
        let ref_count = required_test_count(CertificationLevel::Reference);

        assert!(exp_count <= res_count);
        assert!(res_count <= ind_count);
        assert!(ind_count <= ref_count);
    }

    #[test]
    fn test_suite_result() {
        let results = vec![
            TestResult::pass(MandatoryTest::EnergyConservation { max_error: 0.05 }, 0.02),
            TestResult::pass(
                MandatoryTest::NeuralCorrectionBound { max_share: 0.10 },
                0.05,
            ),
            TestResult::fail(
                MandatoryTest::GroundTruthComparison { max_delta_e: 2.0 },
                2.5,
                "Exceeded",
            ),
        ];

        let suite = TestSuiteResult::from_results(CertificationLevel::Research, results);

        assert!(!suite.all_passed);
        assert_eq!(suite.passed_count(), 2);
        assert_eq!(suite.failed_count(), 1);
    }

    #[test]
    fn test_suite_report() {
        let results = vec![TestResult::pass(
            MandatoryTest::EnergyConservation { max_error: 0.05 },
            0.02,
        )];

        let suite = TestSuiteResult::from_results(CertificationLevel::Experimental, results);
        let report = suite.report();

        assert!(report.contains("Experimental"));
        assert!(report.contains("PASSED"));
        assert!(report.contains("Energy Conservation"));
    }

    #[test]
    fn test_fresnel_compliance() {
        let test = MandatoryTest::FresnelCompliance {
            max_deviation: 0.01,
        };
        assert_eq!(test.name(), "Fresnel Compliance");
        assert!((test.threshold() - 0.01).abs() < 1e-10);
    }

    #[test]
    fn test_color_accuracy() {
        let test = MandatoryTest::ColorAccuracy {
            max_delta_e: 0.5,
            illuminant: "D65".to_string(),
        };
        let desc = test.description();
        assert!(desc.contains("D65"));
        assert!(desc.contains("0.5"));
    }
}
