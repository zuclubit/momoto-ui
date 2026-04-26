//! # Certification Levels
//!
//! Defines the hierarchy of certification levels for material twins.
//! Each level has specific requirements for accuracy, reproducibility,
//! and metrological traceability.

use std::fmt;

// ============================================================================
// CERTIFICATION LEVEL ENUM
// ============================================================================

/// Certification levels for material twins.
///
/// Levels are ordered from least to most stringent:
/// Experimental < Research < Industrial < Reference
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum CertificationLevel {
    /// Experimental: Research prototypes, no formal guarantees.
    /// Suitable for early development and proof-of-concept.
    Experimental,
    /// Research: Published methods, reproducible results.
    /// Suitable for academic research and publications.
    Research,
    /// Industrial: Production-ready, validated against standards.
    /// Suitable for manufacturing and quality control.
    Industrial,
    /// Reference: Metrological standard, fully traceable.
    /// Suitable for calibration laboratories and standards bodies.
    Reference,
}

impl CertificationLevel {
    /// Get all levels in ascending order of stringency.
    pub fn all() -> &'static [CertificationLevel] {
        &[
            CertificationLevel::Experimental,
            CertificationLevel::Research,
            CertificationLevel::Industrial,
            CertificationLevel::Reference,
        ]
    }

    /// Get maximum allowed color difference (ΔE2000).
    pub fn max_delta_e(&self) -> f64 {
        match self {
            CertificationLevel::Experimental => 5.0,
            CertificationLevel::Research => 2.0,
            CertificationLevel::Industrial => 1.0,
            CertificationLevel::Reference => 0.5,
        }
    }

    /// Get minimum required number of validation observations.
    pub fn min_observations(&self) -> usize {
        match self {
            CertificationLevel::Experimental => 10,
            CertificationLevel::Research => 100,
            CertificationLevel::Industrial => 1000,
            CertificationLevel::Reference => 10000,
        }
    }

    /// Get maximum allowed neural correction share (fraction of output).
    pub fn max_neural_share(&self) -> f64 {
        match self {
            CertificationLevel::Experimental => 0.20, // 20%
            CertificationLevel::Research => 0.10,     // 10%
            CertificationLevel::Industrial => 0.05,   // 5%
            CertificationLevel::Reference => 0.02,    // 2%
        }
    }

    /// Get minimum required reproducibility (1 - max_variance).
    pub fn min_reproducibility(&self) -> f64 {
        match self {
            CertificationLevel::Experimental => 0.90, // 90%
            CertificationLevel::Research => 0.95,     // 95%
            CertificationLevel::Industrial => 0.99,   // 99%
            CertificationLevel::Reference => 0.999,   // 99.9%
        }
    }

    /// Get maximum allowed energy conservation violation.
    pub fn max_energy_violation(&self) -> f64 {
        match self {
            CertificationLevel::Experimental => 0.10, // 10%
            CertificationLevel::Research => 0.05,     // 5%
            CertificationLevel::Industrial => 0.02,   // 2%
            CertificationLevel::Reference => 0.01,    // 1%
        }
    }

    /// Get maximum spectral RMSE allowed.
    pub fn max_spectral_rmse(&self) -> f64 {
        match self {
            CertificationLevel::Experimental => 0.10,
            CertificationLevel::Research => 0.05,
            CertificationLevel::Industrial => 0.02,
            CertificationLevel::Reference => 0.01,
        }
    }

    /// Get certification validity period in days.
    pub fn validity_days(&self) -> Option<u32> {
        match self {
            CertificationLevel::Experimental => None,    // No expiry
            CertificationLevel::Research => Some(365),   // 1 year
            CertificationLevel::Industrial => Some(180), // 6 months
            CertificationLevel::Reference => Some(90),   // 3 months
        }
    }

    /// Check if calibration is required.
    pub fn requires_calibration(&self) -> bool {
        match self {
            CertificationLevel::Experimental => false,
            CertificationLevel::Research => false,
            CertificationLevel::Industrial => true,
            CertificationLevel::Reference => true,
        }
    }

    /// Check if traceability chain is required.
    pub fn requires_traceability(&self) -> bool {
        match self {
            CertificationLevel::Experimental => false,
            CertificationLevel::Research => true,
            CertificationLevel::Industrial => true,
            CertificationLevel::Reference => true,
        }
    }

    /// Check if ground truth validation is required.
    pub fn requires_ground_truth(&self) -> bool {
        match self {
            CertificationLevel::Experimental => false,
            CertificationLevel::Research => true,
            CertificationLevel::Industrial => true,
            CertificationLevel::Reference => true,
        }
    }

    /// Get human-readable level name.
    pub fn name(&self) -> &'static str {
        match self {
            CertificationLevel::Experimental => "Experimental",
            CertificationLevel::Research => "Research",
            CertificationLevel::Industrial => "Industrial",
            CertificationLevel::Reference => "Reference",
        }
    }

    /// Get short code for level.
    pub fn code(&self) -> &'static str {
        match self {
            CertificationLevel::Experimental => "EXP",
            CertificationLevel::Research => "RES",
            CertificationLevel::Industrial => "IND",
            CertificationLevel::Reference => "REF",
        }
    }

    /// Get level description.
    pub fn description(&self) -> &'static str {
        match self {
            CertificationLevel::Experimental => {
                "Research prototype with no formal accuracy guarantees"
            }
            CertificationLevel::Research => {
                "Published methods with reproducible results for academic use"
            }
            CertificationLevel::Industrial => {
                "Production-ready with validated accuracy for manufacturing"
            }
            CertificationLevel::Reference => {
                "Metrological standard with full traceability for calibration"
            }
        }
    }

    /// Check if level can be achieved given metrics.
    pub fn can_achieve(&self, metrics: &CertificationMetrics) -> bool {
        metrics.delta_e <= self.max_delta_e()
            && metrics.observations >= self.min_observations()
            && metrics.neural_share <= self.max_neural_share()
            && metrics.reproducibility >= self.min_reproducibility()
            && metrics.energy_violation <= self.max_energy_violation()
            && (!self.requires_calibration() || metrics.is_calibrated)
            && (!self.requires_traceability() || metrics.has_traceability)
    }

    /// Get highest achievable level for given metrics.
    pub fn highest_achievable(metrics: &CertificationMetrics) -> Option<CertificationLevel> {
        for level in Self::all().iter().rev() {
            if level.can_achieve(metrics) {
                return Some(*level);
            }
        }
        None
    }
}

impl fmt::Display for CertificationLevel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.name())
    }
}

// ============================================================================
// CERTIFICATION METRICS
// ============================================================================

/// Metrics used to determine certification level.
#[derive(Debug, Clone, Default)]
pub struct CertificationMetrics {
    /// Color difference (ΔE2000) against reference.
    pub delta_e: f64,
    /// Number of validation observations.
    pub observations: usize,
    /// Fraction of output from neural correction.
    pub neural_share: f64,
    /// Reproducibility score (1 - variance).
    pub reproducibility: f64,
    /// Energy conservation violation (max).
    pub energy_violation: f64,
    /// Spectral RMSE.
    pub spectral_rmse: f64,
    /// Whether instrument is calibrated.
    pub is_calibrated: bool,
    /// Whether traceability chain exists.
    pub has_traceability: bool,
    /// Whether ground truth validation passed.
    pub ground_truth_passed: bool,
}

impl CertificationMetrics {
    /// Create metrics with default (worst-case) values.
    pub fn new() -> Self {
        Self::default()
    }

    /// Create exemplary metrics for testing.
    pub fn exemplary() -> Self {
        Self {
            delta_e: 0.3,
            observations: 50000,
            neural_share: 0.01,
            reproducibility: 0.9999,
            energy_violation: 0.005,
            spectral_rmse: 0.005,
            is_calibrated: true,
            has_traceability: true,
            ground_truth_passed: true,
        }
    }

    /// Create research-grade metrics.
    pub fn research_grade() -> Self {
        Self {
            delta_e: 1.5,
            observations: 500,
            neural_share: 0.08,
            reproducibility: 0.97,
            energy_violation: 0.03,
            spectral_rmse: 0.03,
            is_calibrated: false,
            has_traceability: true,
            ground_truth_passed: true,
        }
    }

    /// Set delta E metric.
    pub fn with_delta_e(mut self, delta_e: f64) -> Self {
        self.delta_e = delta_e;
        self
    }

    /// Set observations count.
    pub fn with_observations(mut self, n: usize) -> Self {
        self.observations = n;
        self
    }

    /// Set neural share.
    pub fn with_neural_share(mut self, share: f64) -> Self {
        self.neural_share = share;
        self
    }

    /// Set reproducibility.
    pub fn with_reproducibility(mut self, r: f64) -> Self {
        self.reproducibility = r;
        self
    }

    /// Check against specific level requirements.
    pub fn check_level(&self, level: CertificationLevel) -> LevelCheck {
        let mut failures = Vec::new();

        if self.delta_e > level.max_delta_e() {
            failures.push(format!(
                "ΔE2000 {:.3} exceeds max {:.1}",
                self.delta_e,
                level.max_delta_e()
            ));
        }

        if self.observations < level.min_observations() {
            failures.push(format!(
                "Observations {} below min {}",
                self.observations,
                level.min_observations()
            ));
        }

        if self.neural_share > level.max_neural_share() {
            failures.push(format!(
                "Neural share {:.1}% exceeds max {:.0}%",
                self.neural_share * 100.0,
                level.max_neural_share() * 100.0
            ));
        }

        if self.reproducibility < level.min_reproducibility() {
            failures.push(format!(
                "Reproducibility {:.3} below min {:.3}",
                self.reproducibility,
                level.min_reproducibility()
            ));
        }

        if self.energy_violation > level.max_energy_violation() {
            failures.push(format!(
                "Energy violation {:.1}% exceeds max {:.0}%",
                self.energy_violation * 100.0,
                level.max_energy_violation() * 100.0
            ));
        }

        if level.requires_calibration() && !self.is_calibrated {
            failures.push("Calibration required but not present".to_string());
        }

        if level.requires_traceability() && !self.has_traceability {
            failures.push("Traceability chain required but not present".to_string());
        }

        LevelCheck {
            level,
            passed: failures.is_empty(),
            failures,
        }
    }
}

/// Result of checking metrics against a certification level.
#[derive(Debug, Clone)]
pub struct LevelCheck {
    /// Level being checked.
    pub level: CertificationLevel,
    /// Whether all requirements passed.
    pub passed: bool,
    /// List of failures (if any).
    pub failures: Vec<String>,
}

impl LevelCheck {
    /// Generate check report.
    pub fn report(&self) -> String {
        let mut report = format!(
            "{} Level Check: {}\n",
            self.level,
            if self.passed { "PASSED" } else { "FAILED" }
        );

        if !self.passed {
            report.push_str("Failures:\n");
            for failure in &self.failures {
                report.push_str(&format!("  - {}\n", failure));
            }
        }

        report
    }
}

// ============================================================================
// LEVEL REQUIREMENTS SUMMARY
// ============================================================================

/// Summary of requirements for all certification levels.
#[derive(Debug, Clone)]
pub struct LevelRequirements {
    /// Level being described.
    pub level: CertificationLevel,
    /// Maximum ΔE2000.
    pub max_delta_e: f64,
    /// Minimum observations.
    pub min_observations: usize,
    /// Maximum neural share.
    pub max_neural_share: f64,
    /// Minimum reproducibility.
    pub min_reproducibility: f64,
    /// Maximum energy violation.
    pub max_energy_violation: f64,
    /// Validity period in days.
    pub validity_days: Option<u32>,
    /// Requires calibration.
    pub requires_calibration: bool,
    /// Requires traceability.
    pub requires_traceability: bool,
}

impl LevelRequirements {
    /// Get requirements for a level.
    pub fn for_level(level: CertificationLevel) -> Self {
        Self {
            level,
            max_delta_e: level.max_delta_e(),
            min_observations: level.min_observations(),
            max_neural_share: level.max_neural_share(),
            min_reproducibility: level.min_reproducibility(),
            max_energy_violation: level.max_energy_violation(),
            validity_days: level.validity_days(),
            requires_calibration: level.requires_calibration(),
            requires_traceability: level.requires_traceability(),
        }
    }

    /// Generate requirements report.
    pub fn report(&self) -> String {
        format!(
            "{} Level Requirements:\n\
             ├── Max ΔE2000:          {:.1}\n\
             ├── Min Observations:    {}\n\
             ├── Max Neural Share:    {:.0}%\n\
             ├── Min Reproducibility: {:.1}%\n\
             ├── Max Energy Violation:{:.0}%\n\
             ├── Validity:            {}\n\
             ├── Calibration Required:{}\n\
             └── Traceability Required:{}",
            self.level,
            self.max_delta_e,
            self.min_observations,
            self.max_neural_share * 100.0,
            self.min_reproducibility * 100.0,
            self.max_energy_violation * 100.0,
            self.validity_days
                .map(|d| format!("{} days", d))
                .unwrap_or_else(|| "No expiry".to_string()),
            if self.requires_calibration {
                "Yes"
            } else {
                "No"
            },
            if self.requires_traceability {
                "Yes"
            } else {
                "No"
            },
        )
    }
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_level_ordering() {
        assert!(CertificationLevel::Experimental < CertificationLevel::Research);
        assert!(CertificationLevel::Research < CertificationLevel::Industrial);
        assert!(CertificationLevel::Industrial < CertificationLevel::Reference);
    }

    #[test]
    fn test_delta_e_limits() {
        assert_eq!(CertificationLevel::Experimental.max_delta_e(), 5.0);
        assert_eq!(CertificationLevel::Research.max_delta_e(), 2.0);
        assert_eq!(CertificationLevel::Industrial.max_delta_e(), 1.0);
        assert_eq!(CertificationLevel::Reference.max_delta_e(), 0.5);
    }

    #[test]
    fn test_observations_requirements() {
        assert_eq!(CertificationLevel::Experimental.min_observations(), 10);
        assert_eq!(CertificationLevel::Reference.min_observations(), 10000);
    }

    #[test]
    fn test_neural_share_limits() {
        // Reference should have strictest limit
        assert!(CertificationLevel::Reference.max_neural_share() < 0.05);
        // Experimental most lenient
        assert!(CertificationLevel::Experimental.max_neural_share() >= 0.20);
    }

    #[test]
    fn test_can_achieve_experimental() {
        let metrics = CertificationMetrics {
            delta_e: 4.0,
            observations: 15,
            neural_share: 0.15,
            reproducibility: 0.92,
            energy_violation: 0.08,
            ..Default::default()
        };

        assert!(CertificationLevel::Experimental.can_achieve(&metrics));
        assert!(!CertificationLevel::Research.can_achieve(&metrics));
    }

    #[test]
    fn test_can_achieve_reference() {
        let metrics = CertificationMetrics::exemplary();
        assert!(CertificationLevel::Reference.can_achieve(&metrics));
    }

    #[test]
    fn test_highest_achievable() {
        let metrics = CertificationMetrics::research_grade();
        let highest = CertificationLevel::highest_achievable(&metrics);
        assert!(highest.is_some());
        assert!(highest.unwrap() >= CertificationLevel::Research);
    }

    #[test]
    fn test_highest_achievable_none() {
        let metrics = CertificationMetrics {
            delta_e: 10.0, // Way too high
            ..Default::default()
        };
        assert!(CertificationLevel::highest_achievable(&metrics).is_none());
    }

    #[test]
    fn test_level_check() {
        let metrics = CertificationMetrics::research_grade();
        let check = metrics.check_level(CertificationLevel::Research);
        assert!(check.passed);
    }

    #[test]
    fn test_level_check_failure() {
        let metrics = CertificationMetrics {
            delta_e: 3.0, // Too high for Industrial
            observations: 5000,
            neural_share: 0.03,
            reproducibility: 0.995,
            energy_violation: 0.01,
            is_calibrated: true,
            has_traceability: true,
            ground_truth_passed: true,
            spectral_rmse: 0.01,
        };

        let check = metrics.check_level(CertificationLevel::Industrial);
        assert!(!check.passed);
        assert!(!check.failures.is_empty());
    }

    #[test]
    fn test_requirements_report() {
        let req = LevelRequirements::for_level(CertificationLevel::Industrial);
        let report = req.report();

        assert!(report.contains("Industrial"));
        assert!(report.contains("1.0")); // Delta E
        assert!(report.contains("1000")); // Observations
    }

    #[test]
    fn test_all_levels() {
        let levels = CertificationLevel::all();
        assert_eq!(levels.len(), 4);
        assert_eq!(levels[0], CertificationLevel::Experimental);
        assert_eq!(levels[3], CertificationLevel::Reference);
    }

    #[test]
    fn test_level_codes() {
        assert_eq!(CertificationLevel::Experimental.code(), "EXP");
        assert_eq!(CertificationLevel::Research.code(), "RES");
        assert_eq!(CertificationLevel::Industrial.code(), "IND");
        assert_eq!(CertificationLevel::Reference.code(), "REF");
    }

    #[test]
    fn test_validity_periods() {
        assert!(CertificationLevel::Experimental.validity_days().is_none());
        assert!(CertificationLevel::Research.validity_days().is_some());
        assert!(
            CertificationLevel::Reference.validity_days()
                < CertificationLevel::Research.validity_days()
        );
    }

    #[test]
    fn test_calibration_requirements() {
        assert!(!CertificationLevel::Experimental.requires_calibration());
        assert!(CertificationLevel::Industrial.requires_calibration());
        assert!(CertificationLevel::Reference.requires_calibration());
    }
}
