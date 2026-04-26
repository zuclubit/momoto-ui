//! # Certification System
//!
//! Provides certification levels, requirements, and auditing for material twins.
//!
//! ## Certification Levels
//!
//! Material twins can be certified at four levels, each with increasing requirements:
//!
//! | Level | Max ΔE2000 | Neural Share | Use Case |
//! |-------|------------|--------------|----------|
//! | Experimental | 5.0 | 20% | Research prototypes |
//! | Research | 2.0 | 10% | Academic publications |
//! | Industrial | 1.0 | 5% | Manufacturing QC |
//! | Reference | 0.5 | 2% | Calibration labs |
//!
//! ## Core Types
//!
//! - [`CertificationLevel`] - The four certification tiers
//! - [`MandatoryTest`] - Required tests for each level
//! - [`CertifiedTwinProfile`] - Complete certification record
//! - [`CertificationAuditor`] - Performs certification audits
//!
//! ## Example Usage
//!
//! ```ignore
//! use glass_physics::certification::*;
//!
//! // Create an auditor for industrial certification
//! let auditor = CertificationAuditor::industrial_auditor();
//!
//! // Prepare audit data
//! let data = MaterialAuditData::new()
//!     .with_delta_e(0.8)
//!     .with_energy_violation(0.01)
//!     .with_spectral_rmse(0.015);
//!
//! // Run certification
//! match auditor.certify("Gold Twin", &data) {
//!     Ok(profile) => {
//!         println!("Certified at {} level!", profile.level);
//!         println!("{}", profile.report());
//!     }
//!     Err(result) => {
//!         println!("Certification failed:");
//!         for gap in result.gap_analysis() {
//!             println!("  - {}", gap);
//!         }
//!     }
//! }
//! ```

pub mod auditor;
pub mod levels;
pub mod profiles;
pub mod requirements;

// Re-exports for convenient access
pub use auditor::{CertificationAuditor, CertificationResult, MaterialAuditData};
pub use levels::{CertificationLevel, CertificationMetrics, LevelCheck, LevelRequirements};
pub use profiles::{
    CertificateSummary, CertificationMetadata, CertifiedTwinProfile, NeuralCorrectionStats,
    NeuralViolation, TwinId,
};
pub use requirements::{
    required_test_count, required_tests, MandatoryTest, TestResult, TestSuiteResult,
};

// ============================================================================
// MEMORY ESTIMATION
// ============================================================================

/// Estimate memory footprint of certification module types.
pub fn estimate_memory_footprint() -> CertificationMemoryEstimate {
    CertificationMemoryEstimate {
        certified_profile_base_bytes: std::mem::size_of::<CertifiedTwinProfile>(),
        neural_stats_bytes: std::mem::size_of::<NeuralCorrectionStats>(),
        test_result_bytes: std::mem::size_of::<TestResult>(),
        auditor_bytes: std::mem::size_of::<CertificationAuditor>(),
        audit_data_bytes: std::mem::size_of::<MaterialAuditData>(),
        certification_result_bytes: std::mem::size_of::<CertificationResult>(),
    }
}

/// Memory footprint estimates for certification types.
#[derive(Debug, Clone)]
pub struct CertificationMemoryEstimate {
    /// Base size of CertifiedTwinProfile.
    pub certified_profile_base_bytes: usize,
    /// Size of NeuralCorrectionStats.
    pub neural_stats_bytes: usize,
    /// Size of TestResult.
    pub test_result_bytes: usize,
    /// Size of CertificationAuditor.
    pub auditor_bytes: usize,
    /// Size of MaterialAuditData.
    pub audit_data_bytes: usize,
    /// Size of CertificationResult.
    pub certification_result_bytes: usize,
}

impl CertificationMemoryEstimate {
    /// Total base memory.
    pub fn total_base(&self) -> usize {
        self.certified_profile_base_bytes
            + self.neural_stats_bytes
            + self.auditor_bytes
            + self.audit_data_bytes
    }

    /// Estimate for typical certification.
    pub fn typical_certification(&self) -> usize {
        // Assume:
        // - 1 profile with 12 test results
        // - 1 auditor
        // - 1 audit data
        // - 1 certification result

        self.certified_profile_base_bytes
            + self.test_result_bytes * 12
            + self.neural_stats_bytes
            + self.auditor_bytes
            + self.audit_data_bytes
            + self.certification_result_bytes
    }

    /// Generate memory report.
    pub fn report(&self) -> String {
        format!(
            "Certification Memory Footprint:\n\
             ├── CertifiedTwinProfile: {:4} bytes (base)\n\
             ├── NeuralCorrectionStats:{:4} bytes\n\
             ├── TestResult:           {:4} bytes\n\
             ├── CertificationAuditor: {:4} bytes\n\
             ├── MaterialAuditData:    {:4} bytes\n\
             ├── CertificationResult:  {:4} bytes\n\
             ├── Total Base:           {:4} bytes\n\
             └── Typical Certification:{:4} bytes (~{:.1} KB)",
            self.certified_profile_base_bytes,
            self.neural_stats_bytes,
            self.test_result_bytes,
            self.auditor_bytes,
            self.audit_data_bytes,
            self.certification_result_bytes,
            self.total_base(),
            self.typical_certification(),
            self.typical_certification() as f64 / 1024.0
        )
    }
}

// ============================================================================
// MODULE VALIDATION
// ============================================================================

/// Validate certification module configuration.
pub fn validate_module() -> CertificationValidation {
    let mut issues = Vec::new();

    // Verify level ordering
    if CertificationLevel::Experimental >= CertificationLevel::Research {
        issues.push("Level ordering incorrect".to_string());
    }

    // Verify thresholds decrease with level
    let exp_de = CertificationLevel::Experimental.max_delta_e();
    let ref_de = CertificationLevel::Reference.max_delta_e();
    if exp_de <= ref_de {
        issues.push("Delta E thresholds should decrease with level".to_string());
    }

    // Verify test counts increase with level
    let exp_tests = required_test_count(CertificationLevel::Experimental);
    let ref_tests = required_test_count(CertificationLevel::Reference);
    if exp_tests >= ref_tests {
        issues.push("Test count should increase with level".to_string());
    }

    CertificationValidation {
        valid: issues.is_empty(),
        issues,
        memory_estimate: estimate_memory_footprint(),
    }
}

/// Result of certification module validation.
#[derive(Debug)]
pub struct CertificationValidation {
    /// Whether validation passed.
    pub valid: bool,
    /// List of issues found.
    pub issues: Vec<String>,
    /// Memory footprint estimate.
    pub memory_estimate: CertificationMemoryEstimate,
}

// ============================================================================
// QUICK CERTIFICATION FUNCTIONS
// ============================================================================

/// Quick check if data can achieve a level.
pub fn can_achieve_level(metrics: &CertificationMetrics, level: CertificationLevel) -> bool {
    level.can_achieve(metrics)
}

/// Get highest achievable level for metrics.
pub fn highest_level(metrics: &CertificationMetrics) -> Option<CertificationLevel> {
    CertificationLevel::highest_achievable(metrics)
}

/// Quick certification at experimental level.
pub fn quick_certify_experimental(
    name: impl Into<String>,
    delta_e: f64,
) -> Result<CertifiedTwinProfile, String> {
    if delta_e > CertificationLevel::Experimental.max_delta_e() {
        return Err(format!(
            "ΔE2000 {:.2} exceeds experimental limit {:.1}",
            delta_e,
            CertificationLevel::Experimental.max_delta_e()
        ));
    }

    let results = vec![TestResult::pass(
        MandatoryTest::GroundTruthComparison {
            max_delta_e: CertificationLevel::Experimental.max_delta_e(),
        },
        delta_e,
    )];

    Ok(CertifiedTwinProfile::new(
        name,
        CertificationLevel::Experimental,
        results,
    ))
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_module_exports() {
        // Verify all public types are accessible
        let _ = CertificationLevel::Industrial;
        let _ = CertificationMetrics::new();
        let _ = MandatoryTest::EnergyConservation { max_error: 0.05 };
        let _ = TwinId::new();
        let _ = CertificationAuditor::new(CertificationLevel::Research);
    }

    #[test]
    fn test_memory_estimate() {
        let estimate = estimate_memory_footprint();

        // Sanity checks
        assert!(estimate.certified_profile_base_bytes > 0);
        assert!(estimate.auditor_bytes > 0);
        assert!(estimate.typical_certification() > estimate.total_base());

        let report = estimate.report();
        assert!(report.contains("Certification"));
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

        // Phase 15 certification should use < 12KB typical
        assert!(
            estimate.typical_certification() < 12_000,
            "Typical certification {} exceeds 12KB budget",
            estimate.typical_certification()
        );
    }

    #[test]
    fn test_can_achieve_level() {
        let metrics = CertificationMetrics::exemplary();
        assert!(can_achieve_level(&metrics, CertificationLevel::Reference));
        assert!(can_achieve_level(
            &metrics,
            CertificationLevel::Experimental
        ));
    }

    #[test]
    fn test_highest_level() {
        let metrics = CertificationMetrics::research_grade();
        let highest = highest_level(&metrics);

        assert!(highest.is_some());
        assert!(highest.unwrap() >= CertificationLevel::Research);
    }

    #[test]
    fn test_quick_certify_experimental() {
        let result = quick_certify_experimental("Test Material", 3.0);
        assert!(result.is_ok());

        let profile = result.unwrap();
        assert_eq!(profile.level, CertificationLevel::Experimental);
    }

    #[test]
    fn test_quick_certify_failure() {
        let result = quick_certify_experimental("Test Material", 10.0);
        assert!(result.is_err());
    }

    #[test]
    fn test_required_tests() {
        let exp_tests = required_tests(CertificationLevel::Experimental);
        let ref_tests = required_tests(CertificationLevel::Reference);

        assert!(exp_tests.len() < ref_tests.len());
    }

    #[test]
    fn test_full_certification_workflow() {
        // Create auditor
        let auditor = CertificationAuditor::new(CertificationLevel::Industrial);

        // Create audit data
        let data = MaterialAuditData::exemplary();

        // Verify can certify
        assert!(auditor.can_certify(&data));

        // Run certification
        let result = auditor.certify("Test Gold", &data);
        assert!(result.is_ok());

        // Check profile
        let profile = result.unwrap();
        assert_eq!(profile.name, "Test Gold");
        assert!(profile.level >= CertificationLevel::Industrial);
        assert!(profile.is_valid());

        // Generate report
        let report = profile.report();
        assert!(report.contains("Test Gold"));
    }
}
