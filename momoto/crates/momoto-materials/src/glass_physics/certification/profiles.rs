//! # Certified Twin Profiles
//!
//! Complete certification profiles for material twins including
//! all metadata, test results, and traceability information.

use crate::glass_physics::metrology::{ToleranceBudget, TraceabilityChain};

use super::levels::CertificationLevel;
use super::requirements::TestResult;

// ============================================================================
// TWIN IDENTIFICATION
// ============================================================================

/// Unique identifier for a material twin.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TwinId(pub u64);

impl TwinId {
    /// Generate new unique ID.
    pub fn new() -> Self {
        use std::sync::atomic::{AtomicU64, Ordering};
        static COUNTER: AtomicU64 = AtomicU64::new(1);
        Self(COUNTER.fetch_add(1, Ordering::Relaxed))
    }

    /// Create from existing value.
    pub fn from_value(value: u64) -> Self {
        Self(value)
    }
}

impl Default for TwinId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for TwinId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "TWIN-{:08X}", self.0)
    }
}

// ============================================================================
// NEURAL CORRECTION STATS
// ============================================================================

/// Statistics for neural correction usage.
#[derive(Debug, Clone, Default)]
pub struct NeuralCorrectionStats {
    /// Total number of evaluations.
    pub total_evaluations: u64,
    /// Number of evaluations where correction was applied.
    pub corrections_applied: u64,
    /// Mean correction magnitude (absolute).
    pub mean_correction_magnitude: f64,
    /// Maximum correction magnitude.
    pub max_correction_magnitude: f64,
    /// Fraction of output attributed to neural correction.
    pub correction_share: f64,
    /// Violations of correction bounds.
    pub violations: Vec<NeuralViolation>,
}

/// Record of a neural correction bound violation.
#[derive(Debug, Clone)]
pub struct NeuralViolation {
    /// When violation occurred.
    pub timestamp: u64,
    /// Actual correction magnitude.
    pub correction_magnitude: f64,
    /// Threshold that was violated.
    pub threshold: f64,
    /// Context/description.
    pub context: String,
}

impl NeuralCorrectionStats {
    /// Create new stats instance.
    pub fn new() -> Self {
        Self::default()
    }

    /// Record a correction.
    pub fn record(&mut self, magnitude: f64, threshold: f64) {
        self.total_evaluations += 1;

        if magnitude.abs() > 1e-10 {
            self.corrections_applied += 1;

            // Update mean
            let n = self.corrections_applied as f64;
            self.mean_correction_magnitude =
                self.mean_correction_magnitude * (n - 1.0) / n + magnitude.abs() / n;

            // Update max
            self.max_correction_magnitude = self.max_correction_magnitude.max(magnitude.abs());

            // Check for violation
            if magnitude.abs() > threshold {
                self.violations.push(NeuralViolation {
                    timestamp: std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_secs(),
                    correction_magnitude: magnitude.abs(),
                    threshold,
                    context: String::new(),
                });
            }
        }
    }

    /// Get correction rate.
    pub fn correction_rate(&self) -> f64 {
        if self.total_evaluations > 0 {
            self.corrections_applied as f64 / self.total_evaluations as f64
        } else {
            0.0
        }
    }

    /// Check if stats are within bounds.
    pub fn within_bounds(&self, max_share: f64) -> bool {
        self.correction_share <= max_share && self.violations.is_empty()
    }

    /// Generate stats report.
    pub fn report(&self) -> String {
        format!(
            "Neural Correction Statistics:\n\
             ├── Total Evaluations:    {}\n\
             ├── Corrections Applied:  {} ({:.1}%)\n\
             ├── Mean Magnitude:       {:.6}\n\
             ├── Max Magnitude:        {:.6}\n\
             ├── Correction Share:     {:.2}%\n\
             └── Violations:           {}",
            self.total_evaluations,
            self.corrections_applied,
            self.correction_rate() * 100.0,
            self.mean_correction_magnitude,
            self.max_correction_magnitude,
            self.correction_share * 100.0,
            self.violations.len()
        )
    }
}

// ============================================================================
// CERTIFICATION METADATA
// ============================================================================

/// Metadata for certification.
#[derive(Debug, Clone)]
pub struct CertificationMetadata {
    /// Certification authority/organization.
    pub authority: String,
    /// Certifying engineer/operator.
    pub operator: Option<String>,
    /// Certification software version.
    pub software_version: String,
    /// Reference standards used.
    pub reference_standards: Vec<String>,
    /// Additional notes.
    pub notes: Vec<String>,
    /// Certification location.
    pub location: Option<String>,
    /// Environmental conditions hash.
    pub environment_hash: Option<String>,
}

impl Default for CertificationMetadata {
    fn default() -> Self {
        Self {
            authority: "Momoto Materials Engine".to_string(),
            operator: None,
            software_version: env!("CARGO_PKG_VERSION").to_string(),
            reference_standards: vec!["ISO 10110-12:2019".to_string(), "CIE 15:2018".to_string()],
            notes: Vec::new(),
            location: None,
            environment_hash: None,
        }
    }
}

impl CertificationMetadata {
    /// Create new metadata.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set authority.
    pub fn with_authority(mut self, authority: impl Into<String>) -> Self {
        self.authority = authority.into();
        self
    }

    /// Set operator.
    pub fn with_operator(mut self, operator: impl Into<String>) -> Self {
        self.operator = Some(operator.into());
        self
    }

    /// Add reference standard.
    pub fn with_standard(mut self, standard: impl Into<String>) -> Self {
        self.reference_standards.push(standard.into());
        self
    }

    /// Add note.
    pub fn with_note(mut self, note: impl Into<String>) -> Self {
        self.notes.push(note.into());
        self
    }
}

// ============================================================================
// CERTIFIED TWIN PROFILE
// ============================================================================

/// Complete certified profile for a material twin.
#[derive(Debug, Clone)]
pub struct CertifiedTwinProfile {
    /// Unique twin identifier.
    pub twin_id: TwinId,
    /// Material name.
    pub name: String,
    /// Certification level achieved.
    pub level: CertificationLevel,
    /// Certification timestamp.
    pub certified_at: u64,
    /// Expiration timestamp (if applicable).
    pub valid_until: Option<u64>,
    /// Individual test results.
    pub test_results: Vec<TestResult>,
    /// Traceability chain.
    pub traceability: TraceabilityChain,
    /// Tolerance budget used.
    pub tolerance_budget: ToleranceBudget,
    /// Neural correction statistics.
    pub neural_correction_stats: NeuralCorrectionStats,
    /// Certification metadata.
    pub metadata: CertificationMetadata,
}

impl CertifiedTwinProfile {
    /// Create new certified profile.
    pub fn new(
        name: impl Into<String>,
        level: CertificationLevel,
        test_results: Vec<TestResult>,
    ) -> Self {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        let valid_until = level
            .validity_days()
            .map(|days| now + (days as u64) * 24 * 60 * 60);

        Self {
            twin_id: TwinId::new(),
            name: name.into(),
            level,
            certified_at: now,
            valid_until,
            test_results,
            traceability: TraceabilityChain::new(),
            tolerance_budget: ToleranceBudget::for_certification_level(
                crate::glass_physics::metrology::CertificationTolerance::Industrial,
            ),
            neural_correction_stats: NeuralCorrectionStats::new(),
            metadata: CertificationMetadata::default(),
        }
    }

    /// Set traceability chain.
    pub fn with_traceability(mut self, chain: TraceabilityChain) -> Self {
        self.traceability = chain;
        self
    }

    /// Set tolerance budget.
    pub fn with_tolerance_budget(mut self, budget: ToleranceBudget) -> Self {
        self.tolerance_budget = budget;
        self
    }

    /// Set neural correction stats.
    pub fn with_neural_stats(mut self, stats: NeuralCorrectionStats) -> Self {
        self.neural_correction_stats = stats;
        self
    }

    /// Set metadata.
    pub fn with_metadata(mut self, metadata: CertificationMetadata) -> Self {
        self.metadata = metadata;
        self
    }

    /// Check if certification is still valid.
    pub fn is_valid(&self) -> bool {
        match self.valid_until {
            None => true,
            Some(expiry) => {
                let now = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs();
                now < expiry
            }
        }
    }

    /// Get days until expiration.
    pub fn days_until_expiry(&self) -> Option<i64> {
        self.valid_until.map(|expiry| {
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs();
            ((expiry as i64) - (now as i64)) / (24 * 60 * 60)
        })
    }

    /// Check if all tests passed.
    pub fn all_tests_passed(&self) -> bool {
        self.test_results.iter().all(|r| r.passed)
    }

    /// Get number of passed tests.
    pub fn passed_test_count(&self) -> usize {
        self.test_results.iter().filter(|r| r.passed).count()
    }

    /// Get number of failed tests.
    pub fn failed_test_count(&self) -> usize {
        self.test_results.iter().filter(|r| !r.passed).count()
    }

    /// Get certificate summary.
    pub fn summary(&self) -> CertificateSummary {
        CertificateSummary {
            twin_id: self.twin_id.clone(),
            name: self.name.clone(),
            level: self.level,
            is_valid: self.is_valid(),
            all_tests_passed: self.all_tests_passed(),
            tests_passed: self.passed_test_count(),
            tests_total: self.test_results.len(),
            neural_share: self.neural_correction_stats.correction_share,
            tolerance_utilization: self.tolerance_budget.utilization(),
        }
    }

    /// Generate full certification report.
    pub fn report(&self) -> String {
        let mut report = String::new();

        // Header
        report.push_str(&format!(
            "╔════════════════════════════════════════════════════════════╗\n"
        ));
        report.push_str(&format!(
            "║         MATERIAL TWIN CERTIFICATION REPORT                ║\n"
        ));
        report.push_str(&format!(
            "╚════════════════════════════════════════════════════════════╝\n\n"
        ));

        // Twin Info
        report.push_str(&format!("Twin ID:      {}\n", self.twin_id));
        report.push_str(&format!("Name:         {}\n", self.name));
        report.push_str(&format!(
            "Level:        {} ({})\n",
            self.level,
            self.level.code()
        ));
        report.push_str(&format!(
            "Status:       {}\n",
            if self.is_valid() { "VALID" } else { "EXPIRED" }
        ));

        // Validity
        if let Some(days) = self.days_until_expiry() {
            if days > 0 {
                report.push_str(&format!("Expires in:   {} days\n", days));
            } else {
                report.push_str(&format!("Expired:      {} days ago\n", -days));
            }
        }

        report.push_str("\n");

        // Test Results Summary
        report.push_str(&format!(
            "Test Results: {}/{} passed\n",
            self.passed_test_count(),
            self.test_results.len()
        ));

        if self.failed_test_count() > 0 {
            report.push_str("Failed Tests:\n");
            for result in &self.test_results {
                if !result.passed {
                    report.push_str(&format!(
                        "  - {}: {:.6} (max: {:.6})\n",
                        result.test.name(),
                        result.actual_value,
                        result.threshold
                    ));
                }
            }
        }

        report.push_str("\n");

        // Tolerance Budget
        report.push_str(&format!("Tolerance Budget:\n"));
        report.push_str(&format!(
            "  Target:      {:.4}\n",
            self.tolerance_budget.target
        ));
        report.push_str(&format!(
            "  Used:        {:.4}\n",
            self.tolerance_budget.total_used
        ));
        report.push_str(&format!(
            "  Utilization: {:.1}%\n",
            self.tolerance_budget.utilization()
        ));

        report.push_str("\n");

        // Neural Correction
        report.push_str(&format!("Neural Correction:\n"));
        report.push_str(&format!(
            "  Share:       {:.2}% (max: {:.0}%)\n",
            self.neural_correction_stats.correction_share * 100.0,
            self.level.max_neural_share() * 100.0
        ));
        report.push_str(&format!(
            "  Violations:  {}\n",
            self.neural_correction_stats.violations.len()
        ));

        report.push_str("\n");

        // Traceability
        report.push_str(&format!(
            "Traceability:  {} entries\n",
            self.traceability.entries.len()
        ));

        // Metadata
        report.push_str(&format!(
            "\nCertification Authority: {}\n",
            self.metadata.authority
        ));
        if let Some(ref op) = self.metadata.operator {
            report.push_str(&format!("Certified by: {}\n", op));
        }

        report
    }
}

// ============================================================================
// CERTIFICATE SUMMARY
// ============================================================================

/// Compact summary of certification.
#[derive(Debug, Clone)]
pub struct CertificateSummary {
    /// Twin identifier.
    pub twin_id: TwinId,
    /// Material name.
    pub name: String,
    /// Certification level.
    pub level: CertificationLevel,
    /// Whether certificate is valid.
    pub is_valid: bool,
    /// Whether all tests passed.
    pub all_tests_passed: bool,
    /// Number of tests passed.
    pub tests_passed: usize,
    /// Total number of tests.
    pub tests_total: usize,
    /// Neural correction share.
    pub neural_share: f64,
    /// Tolerance budget utilization.
    pub tolerance_utilization: f64,
}

impl CertificateSummary {
    /// Get pass rate percentage.
    pub fn pass_rate(&self) -> f64 {
        if self.tests_total > 0 {
            (self.tests_passed as f64 / self.tests_total as f64) * 100.0
        } else {
            0.0
        }
    }

    /// Generate one-line summary.
    pub fn one_line(&self) -> String {
        format!(
            "{} | {} | {} | {}/{} tests | Neural: {:.1}%",
            self.twin_id,
            self.name,
            self.level.code(),
            self.tests_passed,
            self.tests_total,
            self.neural_share * 100.0
        )
    }
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::glass_physics::certification::requirements::MandatoryTest;

    #[test]
    fn test_twin_id_generation() {
        let id1 = TwinId::new();
        let id2 = TwinId::new();
        assert_ne!(id1, id2);
    }

    #[test]
    fn test_twin_id_display() {
        let id = TwinId::from_value(255);
        let s = format!("{}", id);
        assert!(s.contains("TWIN-"));
        assert!(s.contains("000000FF"));
    }

    #[test]
    fn test_neural_correction_stats() {
        let mut stats = NeuralCorrectionStats::new();

        stats.record(0.01, 0.05);
        stats.record(0.02, 0.05);
        stats.record(0.03, 0.05);

        assert_eq!(stats.corrections_applied, 3);
        assert!(stats.mean_correction_magnitude > 0.0);
        assert!((stats.max_correction_magnitude - 0.03).abs() < 1e-10);
        assert!(stats.violations.is_empty());
    }

    #[test]
    fn test_neural_violation() {
        let mut stats = NeuralCorrectionStats::new();

        stats.record(0.1, 0.05); // Violation!

        assert_eq!(stats.violations.len(), 1);
        assert!(!stats.within_bounds(0.05));
    }

    #[test]
    fn test_certified_profile_creation() {
        let results = vec![TestResult::pass(
            MandatoryTest::EnergyConservation { max_error: 0.05 },
            0.02,
        )];

        let profile =
            CertifiedTwinProfile::new("Test Material", CertificationLevel::Research, results);

        assert!(!profile.name.is_empty());
        assert!(profile.is_valid());
        assert!(profile.all_tests_passed());
    }

    #[test]
    fn test_profile_validity() {
        let results = vec![];
        let mut profile =
            CertifiedTwinProfile::new("Test", CertificationLevel::Industrial, results);

        // Set expiry to past
        profile.valid_until = Some(0);

        assert!(!profile.is_valid());
        assert!(profile.days_until_expiry().unwrap() < 0);
    }

    #[test]
    fn test_profile_with_failed_tests() {
        let results = vec![
            TestResult::pass(MandatoryTest::EnergyConservation { max_error: 0.05 }, 0.02),
            TestResult::fail(
                MandatoryTest::NeuralCorrectionBound { max_share: 0.05 },
                0.08,
                "Exceeded",
            ),
        ];

        let profile = CertifiedTwinProfile::new("Test", CertificationLevel::Industrial, results);

        assert!(!profile.all_tests_passed());
        assert_eq!(profile.passed_test_count(), 1);
        assert_eq!(profile.failed_test_count(), 1);
    }

    #[test]
    fn test_profile_summary() {
        let results = vec![TestResult::pass(
            MandatoryTest::EnergyConservation { max_error: 0.05 },
            0.02,
        )];

        let profile = CertifiedTwinProfile::new("Gold", CertificationLevel::Reference, results);
        let summary = profile.summary();

        assert_eq!(summary.name, "Gold");
        assert_eq!(summary.level, CertificationLevel::Reference);
        assert!(summary.is_valid);
        assert!((summary.pass_rate() - 100.0).abs() < 1e-10);
    }

    #[test]
    fn test_profile_report() {
        let results = vec![TestResult::pass(
            MandatoryTest::EnergyConservation { max_error: 0.05 },
            0.02,
        )];

        let profile =
            CertifiedTwinProfile::new("Test Material", CertificationLevel::Industrial, results);
        let report = profile.report();

        assert!(report.contains("CERTIFICATION REPORT"));
        assert!(report.contains("Test Material"));
        assert!(report.contains("Industrial"));
    }

    #[test]
    fn test_metadata() {
        let metadata = CertificationMetadata::new()
            .with_authority("Test Lab")
            .with_operator("Engineer A")
            .with_standard("ISO 12345")
            .with_note("Special conditions");

        assert_eq!(metadata.authority, "Test Lab");
        assert_eq!(metadata.operator, Some("Engineer A".to_string()));
        assert!(metadata
            .reference_standards
            .iter()
            .any(|s| s.contains("12345")));
    }

    #[test]
    fn test_neural_stats_report() {
        let mut stats = NeuralCorrectionStats::new();
        stats.total_evaluations = 1000;
        stats.corrections_applied = 100;
        stats.correction_share = 0.03;

        let report = stats.report();
        assert!(report.contains("1000"));
        assert!(report.contains("100"));
        assert!(report.contains("3"));
    }
}
