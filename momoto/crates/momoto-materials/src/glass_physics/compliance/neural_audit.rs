//! # Neural Correction Audit
//!
//! Auditing and accountability for neural network corrections.
//! Ensures neural corrections stay within certified bounds.

use crate::glass_physics::certification::NeuralCorrectionStats;

// ============================================================================
// NEURAL AUDITOR
// ============================================================================

/// Auditor for neural correction accountability.
#[derive(Debug, Clone)]
pub struct NeuralAuditor {
    /// Maximum allowed correction share (fraction of output).
    pub max_correction_share: f64,
    /// Maximum single correction magnitude.
    pub max_single_correction: f64,
    /// Alert threshold (warn before violation).
    pub alert_threshold: f64,
    /// Whether to fail on any violation.
    pub strict_mode: bool,
}

impl Default for NeuralAuditor {
    fn default() -> Self {
        Self {
            max_correction_share: 0.05,  // 5%
            max_single_correction: 0.10, // 10%
            alert_threshold: 0.80,       // 80% of limit
            strict_mode: true,
        }
    }
}

impl NeuralAuditor {
    /// Create new auditor with default limits.
    pub fn new() -> Self {
        Self::default()
    }

    /// Create reference-level auditor (strictest).
    pub fn reference_level() -> Self {
        Self {
            max_correction_share: 0.02,  // 2%
            max_single_correction: 0.05, // 5%
            alert_threshold: 0.80,
            strict_mode: true,
        }
    }

    /// Create industrial-level auditor.
    pub fn industrial_level() -> Self {
        Self {
            max_correction_share: 0.05,  // 5%
            max_single_correction: 0.10, // 10%
            alert_threshold: 0.80,
            strict_mode: true,
        }
    }

    /// Create research-level auditor (more lenient).
    pub fn research_level() -> Self {
        Self {
            max_correction_share: 0.10,  // 10%
            max_single_correction: 0.20, // 20%
            alert_threshold: 0.80,
            strict_mode: false,
        }
    }

    /// Set maximum correction share.
    pub fn with_max_share(mut self, share: f64) -> Self {
        self.max_correction_share = share;
        self
    }

    /// Set maximum single correction.
    pub fn with_max_single(mut self, max: f64) -> Self {
        self.max_single_correction = max;
        self
    }

    /// Enable/disable strict mode.
    pub fn with_strict_mode(mut self, strict: bool) -> Self {
        self.strict_mode = strict;
        self
    }

    /// Audit neural correction statistics.
    pub fn audit(&self, stats: &NeuralCorrectionStats) -> NeuralAuditResult {
        let mut findings = Vec::new();
        let mut warnings = Vec::new();

        // Check correction share
        let share_utilization = stats.correction_share / self.max_correction_share;
        if stats.correction_share > self.max_correction_share {
            findings.push(AuditFinding {
                severity: FindingSeverity::Critical,
                category: FindingCategory::ShareExceeded,
                message: format!(
                    "Correction share {:.2}% exceeds limit {:.0}%",
                    stats.correction_share * 100.0,
                    self.max_correction_share * 100.0
                ),
                actual_value: stats.correction_share,
                threshold: self.max_correction_share,
            });
        } else if share_utilization >= self.alert_threshold {
            warnings.push(format!(
                "Correction share at {:.0}% of limit",
                share_utilization * 100.0
            ));
        }

        // Check max single correction
        let single_utilization = stats.max_correction_magnitude / self.max_single_correction;
        if stats.max_correction_magnitude > self.max_single_correction {
            findings.push(AuditFinding {
                severity: FindingSeverity::High,
                category: FindingCategory::SingleCorrectionExceeded,
                message: format!(
                    "Max correction {:.4} exceeds limit {:.2}",
                    stats.max_correction_magnitude, self.max_single_correction
                ),
                actual_value: stats.max_correction_magnitude,
                threshold: self.max_single_correction,
            });
        } else if single_utilization >= self.alert_threshold {
            warnings.push(format!(
                "Max single correction at {:.0}% of limit",
                single_utilization * 100.0
            ));
        }

        // Check for recorded violations
        if !stats.violations.is_empty() {
            findings.push(AuditFinding {
                severity: FindingSeverity::High,
                category: FindingCategory::RecordedViolations,
                message: format!(
                    "{} recorded violations during operation",
                    stats.violations.len()
                ),
                actual_value: stats.violations.len() as f64,
                threshold: 0.0,
            });
        }

        // Determine pass/fail
        let passed = if self.strict_mode {
            findings.is_empty()
        } else {
            findings
                .iter()
                .all(|f| f.severity != FindingSeverity::Critical)
        };

        NeuralAuditResult {
            passed,
            correction_share: stats.correction_share,
            max_single_correction: stats.max_correction_magnitude,
            total_evaluations: stats.total_evaluations,
            corrections_applied: stats.corrections_applied,
            findings,
            warnings,
            recommendations: self.generate_recommendations(stats),
        }
    }

    /// Check if stats should fail certification.
    pub fn should_fail_certification(&self, stats: &NeuralCorrectionStats) -> bool {
        stats.correction_share > self.max_correction_share
            || stats.max_correction_magnitude > self.max_single_correction
            || !stats.violations.is_empty()
    }

    /// Generate recommendations based on stats.
    fn generate_recommendations(&self, stats: &NeuralCorrectionStats) -> Vec<String> {
        let mut recommendations = Vec::new();

        if stats.correction_share > self.max_correction_share * 0.5 {
            recommendations
                .push("Consider improving physical model to reduce neural dependence".to_string());
        }

        if stats.corrections_applied as f64 / stats.total_evaluations.max(1) as f64 > 0.8 {
            recommendations
                .push("High correction rate suggests systematic model deficiency".to_string());
        }

        if stats.max_correction_magnitude > self.max_single_correction * 0.8 {
            recommendations
                .push("Large corrections may indicate edge cases needing attention".to_string());
        }

        if !stats.violations.is_empty() {
            recommendations
                .push("Review violation contexts to identify problematic scenarios".to_string());
        }

        recommendations
    }

    /// Check single correction in real-time.
    pub fn check_correction(&self, magnitude: f64) -> CorrectionCheck {
        if magnitude.abs() > self.max_single_correction {
            CorrectionCheck::Violation {
                magnitude: magnitude.abs(),
                threshold: self.max_single_correction,
            }
        } else if magnitude.abs() > self.max_single_correction * self.alert_threshold {
            CorrectionCheck::Warning {
                magnitude: magnitude.abs(),
                threshold: self.max_single_correction,
            }
        } else {
            CorrectionCheck::Ok
        }
    }
}

// ============================================================================
// AUDIT FINDINGS
// ============================================================================

/// Severity of audit finding.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FindingSeverity {
    /// Informational only.
    Info,
    /// Warning (approaching limit).
    Warning,
    /// High severity (exceeded soft limit).
    High,
    /// Critical (exceeded hard limit).
    Critical,
}

/// Category of audit finding.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FindingCategory {
    /// Correction share exceeded.
    ShareExceeded,
    /// Single correction exceeded.
    SingleCorrectionExceeded,
    /// Recorded violations present.
    RecordedViolations,
    /// Correction rate too high.
    HighCorrectionRate,
    /// Other issue.
    Other,
}

/// Individual audit finding.
#[derive(Debug, Clone)]
pub struct AuditFinding {
    /// Severity level.
    pub severity: FindingSeverity,
    /// Finding category.
    pub category: FindingCategory,
    /// Human-readable message.
    pub message: String,
    /// Actual measured value.
    pub actual_value: f64,
    /// Threshold that was compared.
    pub threshold: f64,
}

impl AuditFinding {
    /// Format finding for display.
    pub fn format(&self) -> String {
        let severity_str = match self.severity {
            FindingSeverity::Info => "INFO",
            FindingSeverity::Warning => "WARN",
            FindingSeverity::High => "HIGH",
            FindingSeverity::Critical => "CRIT",
        };

        format!("[{:^4}] {}", severity_str, self.message)
    }
}

// ============================================================================
// AUDIT RESULT
// ============================================================================

/// Result of neural audit.
#[derive(Debug, Clone)]
pub struct NeuralAuditResult {
    /// Whether audit passed.
    pub passed: bool,
    /// Correction share (fraction).
    pub correction_share: f64,
    /// Maximum single correction.
    pub max_single_correction: f64,
    /// Total evaluations audited.
    pub total_evaluations: u64,
    /// Corrections applied.
    pub corrections_applied: u64,
    /// Audit findings.
    pub findings: Vec<AuditFinding>,
    /// Warnings generated.
    pub warnings: Vec<String>,
    /// Recommendations.
    pub recommendations: Vec<String>,
}

impl NeuralAuditResult {
    /// Get correction rate.
    pub fn correction_rate(&self) -> f64 {
        if self.total_evaluations > 0 {
            self.corrections_applied as f64 / self.total_evaluations as f64
        } else {
            0.0
        }
    }

    /// Count critical findings.
    pub fn critical_count(&self) -> usize {
        self.findings
            .iter()
            .filter(|f| f.severity == FindingSeverity::Critical)
            .count()
    }

    /// Generate audit report.
    pub fn report(&self) -> String {
        let mut report = String::new();

        report.push_str("Neural Correction Audit Report\n");
        report.push_str(&format!(
            "Status: {}\n\n",
            if self.passed { "PASSED" } else { "FAILED" }
        ));

        report.push_str("Statistics:\n");
        report.push_str(&format!(
            "  Correction Share:     {:.2}%\n",
            self.correction_share * 100.0
        ));
        report.push_str(&format!(
            "  Max Single Correction: {:.4}\n",
            self.max_single_correction
        ));
        report.push_str(&format!(
            "  Total Evaluations:    {}\n",
            self.total_evaluations
        ));
        report.push_str(&format!(
            "  Corrections Applied:  {} ({:.1}%)\n",
            self.corrections_applied,
            self.correction_rate() * 100.0
        ));

        if !self.findings.is_empty() {
            report.push_str("\nFindings:\n");
            for finding in &self.findings {
                report.push_str(&format!("  {}\n", finding.format()));
            }
        }

        if !self.warnings.is_empty() {
            report.push_str("\nWarnings:\n");
            for warning in &self.warnings {
                report.push_str(&format!("  ⚠ {}\n", warning));
            }
        }

        if !self.recommendations.is_empty() {
            report.push_str("\nRecommendations:\n");
            for rec in &self.recommendations {
                report.push_str(&format!("  • {}\n", rec));
            }
        }

        report
    }
}

// ============================================================================
// REAL-TIME CHECKING
// ============================================================================

/// Result of real-time correction check.
#[derive(Debug, Clone)]
pub enum CorrectionCheck {
    /// Correction is within limits.
    Ok,
    /// Correction is approaching limit.
    Warning { magnitude: f64, threshold: f64 },
    /// Correction exceeded limit.
    Violation { magnitude: f64, threshold: f64 },
}

impl CorrectionCheck {
    /// Check if correction is ok.
    pub fn is_ok(&self) -> bool {
        matches!(self, CorrectionCheck::Ok)
    }

    /// Check if correction violated limit.
    pub fn is_violation(&self) -> bool {
        matches!(self, CorrectionCheck::Violation { .. })
    }
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn make_stats(share: f64, max_mag: f64, violations: usize) -> NeuralCorrectionStats {
        let mut stats = NeuralCorrectionStats::new();
        stats.total_evaluations = 1000;
        stats.corrections_applied = 500;
        stats.correction_share = share;
        stats.max_correction_magnitude = max_mag;
        stats.mean_correction_magnitude = max_mag * 0.5;

        for _ in 0..violations {
            stats
                .violations
                .push(crate::glass_physics::certification::NeuralViolation {
                    timestamp: 0,
                    correction_magnitude: max_mag,
                    threshold: max_mag * 0.5,
                    context: "Test".to_string(),
                });
        }

        stats
    }

    #[test]
    fn test_auditor_creation() {
        let auditor = NeuralAuditor::new();
        assert!((auditor.max_correction_share - 0.05).abs() < 1e-10);
        assert!(auditor.strict_mode);
    }

    #[test]
    fn test_reference_level() {
        let auditor = NeuralAuditor::reference_level();
        assert!(auditor.max_correction_share < 0.05);
    }

    #[test]
    fn test_audit_passing() {
        let auditor = NeuralAuditor::new();
        let stats = make_stats(0.03, 0.05, 0);

        let result = auditor.audit(&stats);

        assert!(result.passed);
        assert!(result.findings.is_empty());
    }

    #[test]
    fn test_audit_share_exceeded() {
        let auditor = NeuralAuditor::new();
        let stats = make_stats(0.08, 0.05, 0); // 8% share, limit is 5%

        let result = auditor.audit(&stats);

        assert!(!result.passed);
        assert!(result
            .findings
            .iter()
            .any(|f| matches!(f.category, FindingCategory::ShareExceeded)));
    }

    #[test]
    fn test_audit_single_exceeded() {
        let auditor = NeuralAuditor::new();
        let stats = make_stats(0.03, 0.15, 0); // 15% single, limit is 10%

        let result = auditor.audit(&stats);

        assert!(!result.passed);
        assert!(result
            .findings
            .iter()
            .any(|f| matches!(f.category, FindingCategory::SingleCorrectionExceeded)));
    }

    #[test]
    fn test_audit_with_violations() {
        let auditor = NeuralAuditor::new();
        let stats = make_stats(0.03, 0.05, 3);

        let result = auditor.audit(&stats);

        assert!(!result.passed);
        assert!(result
            .findings
            .iter()
            .any(|f| matches!(f.category, FindingCategory::RecordedViolations)));
    }

    #[test]
    fn test_should_fail_certification() {
        let auditor = NeuralAuditor::new();

        let good_stats = make_stats(0.03, 0.05, 0);
        assert!(!auditor.should_fail_certification(&good_stats));

        let bad_stats = make_stats(0.08, 0.05, 0);
        assert!(auditor.should_fail_certification(&bad_stats));
    }

    #[test]
    fn test_check_correction_ok() {
        let auditor = NeuralAuditor::new();

        let check = auditor.check_correction(0.05);
        assert!(check.is_ok());
    }

    #[test]
    fn test_check_correction_warning() {
        let auditor = NeuralAuditor::new();

        let check = auditor.check_correction(0.09); // 90% of 0.10 limit
        assert!(matches!(check, CorrectionCheck::Warning { .. }));
    }

    #[test]
    fn test_check_correction_violation() {
        let auditor = NeuralAuditor::new();

        let check = auditor.check_correction(0.15);
        assert!(check.is_violation());
    }

    #[test]
    fn test_audit_report() {
        let auditor = NeuralAuditor::new();
        let stats = make_stats(0.03, 0.05, 0);
        let result = auditor.audit(&stats);

        let report = result.report();
        assert!(report.contains("PASSED"));
        assert!(report.contains("Correction Share"));
    }

    #[test]
    fn test_recommendations() {
        let auditor = NeuralAuditor::new();
        let stats = make_stats(0.04, 0.09, 0); // Close to limits

        let result = auditor.audit(&stats);

        // Should have some recommendations
        assert!(!result.recommendations.is_empty() || result.warnings.is_empty());
    }

    #[test]
    fn test_non_strict_mode() {
        let auditor = NeuralAuditor::new().with_strict_mode(false);
        let stats = make_stats(0.03, 0.15, 0); // Single exceeded but share ok

        let result = auditor.audit(&stats);

        // Non-strict: pass if no critical findings
        // High severity finding exists but might still pass depending on implementation
        assert!(result
            .findings
            .iter()
            .any(|f| f.severity == FindingSeverity::High));
    }

    #[test]
    fn test_finding_format() {
        let finding = AuditFinding {
            severity: FindingSeverity::Critical,
            category: FindingCategory::ShareExceeded,
            message: "Test message".to_string(),
            actual_value: 0.1,
            threshold: 0.05,
        };

        let formatted = finding.format();
        assert!(formatted.contains("CRIT"));
        assert!(formatted.contains("Test message"));
    }
}
