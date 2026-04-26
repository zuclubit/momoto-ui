//! # Twin Validation and Drift Monitoring
//!
//! Validate material twins and monitor temporal drift.
//!
//! ## Overview
//!
//! This module provides:
//! - **TwinValidator**: Validates physical constraints (energy, spectral)
//! - **DriftMonitor**: Tracks parameter evolution over time
//!
//! ## Validation Checks
//!
//! - Energy conservation (BSDF integral ≤ 1)
//! - Spectral consistency (smooth, physical spectra)
//! - Parameter bounds (within physical limits)
//! - Temporal continuity (smooth evolution)

use crate::glass_physics::material_fingerprint::MaterialFingerprint;

// ============================================================================
// VALIDATION RESULT
// ============================================================================

/// Result of a validation check.
#[derive(Debug, Clone)]
pub struct ValidationResult {
    /// Whether validation passed.
    pub passed: bool,
    /// Severity of any issues (0 = none, 1 = critical).
    pub severity: f64,
    /// List of issues found.
    pub issues: Vec<ValidationIssue>,
    /// Validation score (0-1, 1 = perfect).
    pub score: f64,
}

impl ValidationResult {
    /// Create a passing result.
    pub fn pass() -> Self {
        Self {
            passed: true,
            severity: 0.0,
            issues: Vec::new(),
            score: 1.0,
        }
    }

    /// Create a failing result.
    pub fn fail(issues: Vec<ValidationIssue>) -> Self {
        let severity = issues.iter().map(|i| i.severity()).fold(0.0, f64::max);
        let score = 1.0 - severity;
        Self {
            passed: false,
            severity,
            issues,
            score: score.max(0.0),
        }
    }

    /// Merge with another result.
    pub fn merge(self, other: ValidationResult) -> Self {
        let mut issues = self.issues;
        issues.extend(other.issues);
        let passed = self.passed && other.passed;
        let severity = self.severity.max(other.severity);
        let score = (self.score + other.score) / 2.0;
        Self {
            passed,
            severity,
            issues,
            score,
        }
    }
}

/// A validation issue.
#[derive(Debug, Clone)]
pub struct ValidationIssue {
    /// Category of issue.
    pub category: IssueCategory,
    /// Description of the issue.
    pub description: String,
    /// Parameter index if applicable.
    pub param_index: Option<usize>,
    /// Actual value.
    pub actual: Option<f64>,
    /// Expected range or value.
    pub expected: Option<(f64, f64)>,
}

impl ValidationIssue {
    /// Get severity of this issue (0-1).
    pub fn severity(&self) -> f64 {
        match self.category {
            IssueCategory::EnergyViolation => 0.9,
            IssueCategory::SpectralAnomaly => 0.6,
            IssueCategory::ParameterOutOfBounds => 0.7,
            IssueCategory::TemporalDiscontinuity => 0.5,
            IssueCategory::IdentifiabilityWarning => 0.3,
            IssueCategory::CalibrationWarning => 0.4,
        }
    }
}

/// Categories of validation issues.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IssueCategory {
    /// Energy conservation violated.
    EnergyViolation,
    /// Spectral data anomaly.
    SpectralAnomaly,
    /// Parameter outside physical bounds.
    ParameterOutOfBounds,
    /// Temporal discontinuity detected.
    TemporalDiscontinuity,
    /// Identifiability warning.
    IdentifiabilityWarning,
    /// Calibration quality warning.
    CalibrationWarning,
}

// ============================================================================
// TWIN VALIDATOR
// ============================================================================

/// Configuration for twin validation.
#[derive(Debug, Clone)]
pub struct ValidationConfig {
    /// Maximum allowed energy (BSDF integral).
    pub max_energy: f64,
    /// Minimum allowed energy.
    pub min_energy: f64,
    /// Maximum spectral gradient (smoothness).
    pub max_spectral_gradient: f64,
    /// IOR bounds.
    pub ior_bounds: (f64, f64),
    /// Roughness bounds.
    pub roughness_bounds: (f64, f64),
    /// Absorption bounds.
    pub absorption_bounds: (f64, f64),
    /// Maximum temporal drift rate per frame.
    pub max_drift_rate: f64,
}

impl Default for ValidationConfig {
    fn default() -> Self {
        Self {
            max_energy: 1.0 + 1e-6, // Allow tiny numerical error
            min_energy: 0.0,
            max_spectral_gradient: 0.5, // Max change per nm
            ior_bounds: (1.0, 3.5),
            roughness_bounds: (0.0, 1.0),
            absorption_bounds: (0.0, 100.0),
            max_drift_rate: 0.01, // 1% per frame max
        }
    }
}

/// Validates material twins for physical consistency.
#[derive(Debug, Clone)]
pub struct TwinValidator {
    /// Validation configuration.
    config: ValidationConfig,
    /// Validation history.
    history: Vec<ValidationRecord>,
    /// Maximum history size.
    max_history: usize,
}

/// Record of a validation.
#[derive(Debug, Clone)]
pub struct ValidationRecord {
    /// Fingerprint of validated twin.
    pub fingerprint: MaterialFingerprint,
    /// Validation result.
    pub result: ValidationResult,
    /// Timestamp (frame number or epoch).
    pub timestamp: u64,
}

impl TwinValidator {
    /// Create a new validator with default config.
    pub fn new() -> Self {
        Self {
            config: ValidationConfig::default(),
            history: Vec::new(),
            max_history: 1000,
        }
    }

    /// Create with custom configuration.
    pub fn with_config(config: ValidationConfig) -> Self {
        Self {
            config,
            history: Vec::new(),
            max_history: 1000,
        }
    }

    /// Validate energy conservation.
    pub fn validate_energy(&self, energy: f64) -> ValidationResult {
        if energy > self.config.max_energy {
            ValidationResult::fail(vec![ValidationIssue {
                category: IssueCategory::EnergyViolation,
                description: format!(
                    "Energy {} exceeds maximum {} (non-physical BSDF)",
                    energy, self.config.max_energy
                ),
                param_index: None,
                actual: Some(energy),
                expected: Some((self.config.min_energy, self.config.max_energy)),
            }])
        } else if energy < self.config.min_energy {
            ValidationResult::fail(vec![ValidationIssue {
                category: IssueCategory::EnergyViolation,
                description: format!("Negative energy {} (invalid)", energy),
                param_index: None,
                actual: Some(energy),
                expected: Some((self.config.min_energy, self.config.max_energy)),
            }])
        } else {
            ValidationResult::pass()
        }
    }

    /// Validate IOR parameter.
    pub fn validate_ior(&self, ior: f64, param_index: usize) -> ValidationResult {
        let (min, max) = self.config.ior_bounds;
        if ior < min || ior > max {
            ValidationResult::fail(vec![ValidationIssue {
                category: IssueCategory::ParameterOutOfBounds,
                description: format!("IOR {} outside physical bounds [{}, {}]", ior, min, max),
                param_index: Some(param_index),
                actual: Some(ior),
                expected: Some((min, max)),
            }])
        } else {
            ValidationResult::pass()
        }
    }

    /// Validate roughness parameter.
    pub fn validate_roughness(&self, roughness: f64, param_index: usize) -> ValidationResult {
        let (min, max) = self.config.roughness_bounds;
        if roughness < min || roughness > max {
            ValidationResult::fail(vec![ValidationIssue {
                category: IssueCategory::ParameterOutOfBounds,
                description: format!("Roughness {} outside bounds [{}, {}]", roughness, min, max),
                param_index: Some(param_index),
                actual: Some(roughness),
                expected: Some((min, max)),
            }])
        } else {
            ValidationResult::pass()
        }
    }

    /// Validate spectral data smoothness.
    pub fn validate_spectral(&self, wavelengths: &[f64], values: &[f64]) -> ValidationResult {
        if wavelengths.len() != values.len() || wavelengths.is_empty() {
            return ValidationResult::fail(vec![ValidationIssue {
                category: IssueCategory::SpectralAnomaly,
                description: "Invalid spectral data dimensions".to_string(),
                param_index: None,
                actual: None,
                expected: None,
            }]);
        }

        let mut issues = Vec::new();

        // Check for negative values
        for (i, &v) in values.iter().enumerate() {
            if v < 0.0 {
                issues.push(ValidationIssue {
                    category: IssueCategory::SpectralAnomaly,
                    description: format!(
                        "Negative spectral value {} at wavelength {}nm",
                        v, wavelengths[i]
                    ),
                    param_index: None,
                    actual: Some(v),
                    expected: Some((0.0, f64::INFINITY)),
                });
            }
        }

        // Check for excessive gradients
        for i in 1..values.len() {
            let dv = (values[i] - values[i - 1]).abs();
            let dl = (wavelengths[i] - wavelengths[i - 1]).abs();
            if dl > 0.0 {
                let gradient = dv / dl;
                if gradient > self.config.max_spectral_gradient {
                    issues.push(ValidationIssue {
                        category: IssueCategory::SpectralAnomaly,
                        description: format!(
                            "Spectral discontinuity at {}nm (gradient={})",
                            wavelengths[i], gradient
                        ),
                        param_index: None,
                        actual: Some(gradient),
                        expected: Some((0.0, self.config.max_spectral_gradient)),
                    });
                }
            }
        }

        if issues.is_empty() {
            ValidationResult::pass()
        } else {
            ValidationResult::fail(issues)
        }
    }

    /// Validate a complete parameter set.
    pub fn validate_parameters(&self, params: &[f64], names: &[&str]) -> ValidationResult {
        let mut result = ValidationResult::pass();

        for (i, (&param, &name)) in params.iter().zip(names.iter()).enumerate() {
            let check = match name {
                n if n.contains("ior") || n.contains("IOR") => self.validate_ior(param, i),
                n if n.contains("roughness") || n.contains("alpha") => {
                    self.validate_roughness(param, i)
                }
                n if n.contains("absorption") || n.contains("sigma") => {
                    let (min, max) = self.config.absorption_bounds;
                    if param < min || param > max {
                        ValidationResult::fail(vec![ValidationIssue {
                            category: IssueCategory::ParameterOutOfBounds,
                            description: format!(
                                "{} = {} outside bounds [{}, {}]",
                                name, param, min, max
                            ),
                            param_index: Some(i),
                            actual: Some(param),
                            expected: Some((min, max)),
                        }])
                    } else {
                        ValidationResult::pass()
                    }
                }
                _ => ValidationResult::pass(),
            };
            result = result.merge(check);
        }

        result
    }

    /// Record a validation result.
    pub fn record(
        &mut self,
        fingerprint: MaterialFingerprint,
        result: ValidationResult,
        timestamp: u64,
    ) {
        self.history.push(ValidationRecord {
            fingerprint,
            result,
            timestamp,
        });

        // Trim history if too large
        if self.history.len() > self.max_history {
            self.history.remove(0);
        }
    }

    /// Get validation history for a fingerprint.
    pub fn history_for(&self, fingerprint: &MaterialFingerprint) -> Vec<&ValidationRecord> {
        self.history
            .iter()
            .filter(|r| &r.fingerprint == fingerprint)
            .collect()
    }

    /// Get overall validation pass rate.
    pub fn pass_rate(&self) -> f64 {
        if self.history.is_empty() {
            return 1.0;
        }
        let passed = self.history.iter().filter(|r| r.result.passed).count();
        passed as f64 / self.history.len() as f64
    }
}

impl Default for TwinValidator {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// DRIFT MONITOR
// ============================================================================

/// A single drift observation.
#[derive(Debug, Clone)]
pub struct DriftObservation {
    /// Parameter values at this time.
    pub parameters: Vec<f64>,
    /// Timestamp (frame number).
    pub timestamp: u64,
    /// Fingerprint at this time.
    pub fingerprint: MaterialFingerprint,
}

/// Drift statistics for a parameter.
#[derive(Debug, Clone)]
pub struct DriftStatistics {
    /// Parameter index.
    pub param_index: usize,
    /// Total drift (first to last).
    pub total_drift: f64,
    /// Average drift per frame.
    pub drift_rate: f64,
    /// Maximum single-frame drift.
    pub max_drift: f64,
    /// Drift variance.
    pub drift_variance: f64,
    /// Trend direction (-1, 0, +1).
    pub trend: i8,
    /// Is drift within acceptable limits?
    pub acceptable: bool,
}

/// Drift report for all parameters.
#[derive(Debug, Clone)]
pub struct DriftReport {
    /// Statistics per parameter.
    pub param_stats: Vec<DriftStatistics>,
    /// Number of observations.
    pub n_observations: usize,
    /// Time span (frames).
    pub time_span: u64,
    /// Overall drift score (0 = no drift, 1 = severe).
    pub overall_drift_score: f64,
    /// Parameters with concerning drift.
    pub concerning_params: Vec<usize>,
}

impl DriftReport {
    /// Check if any drift is concerning.
    pub fn has_concerning_drift(&self) -> bool {
        !self.concerning_params.is_empty()
    }

    /// Get formatted summary.
    pub fn summary(&self) -> String {
        if self.concerning_params.is_empty() {
            format!(
                "Drift OK: {} observations over {} frames, score={:.3}",
                self.n_observations, self.time_span, self.overall_drift_score
            )
        } else {
            format!(
                "DRIFT WARNING: {} parameters drifting, score={:.3}",
                self.concerning_params.len(),
                self.overall_drift_score
            )
        }
    }
}

/// Monitors temporal drift in material parameters.
#[derive(Debug, Clone)]
pub struct DriftMonitor {
    /// Observations over time.
    observations: Vec<DriftObservation>,
    /// Maximum observations to keep.
    max_observations: usize,
    /// Parameter names.
    param_names: Vec<String>,
    /// Maximum acceptable drift rate.
    max_drift_rate: f64,
    /// Window size for trend analysis.
    trend_window: usize,
}

impl DriftMonitor {
    /// Create a new drift monitor.
    pub fn new(n_params: usize) -> Self {
        Self {
            observations: Vec::new(),
            max_observations: 10000,
            param_names: (0..n_params).map(|i| format!("p{}", i)).collect(),
            max_drift_rate: 0.001, // 0.1% per frame
            trend_window: 100,
        }
    }

    /// Set parameter names.
    pub fn with_names(mut self, names: Vec<String>) -> Self {
        self.param_names = names;
        self
    }

    /// Set maximum drift rate.
    pub fn with_max_drift_rate(mut self, rate: f64) -> Self {
        self.max_drift_rate = rate;
        self
    }

    /// Record an observation.
    pub fn observe(&mut self, params: Vec<f64>, timestamp: u64, fingerprint: MaterialFingerprint) {
        self.observations.push(DriftObservation {
            parameters: params,
            timestamp,
            fingerprint,
        });

        // Trim if too large
        if self.observations.len() > self.max_observations {
            self.observations.remove(0);
        }
    }

    /// Analyze drift for all parameters.
    pub fn analyze(&self) -> DriftReport {
        let n = self.observations.len();
        if n < 2 {
            return DriftReport {
                param_stats: Vec::new(),
                n_observations: n,
                time_span: 0,
                overall_drift_score: 0.0,
                concerning_params: Vec::new(),
            };
        }

        let n_params = self.observations[0].parameters.len();
        let time_span = self.observations.last().unwrap().timestamp
            - self.observations.first().unwrap().timestamp;

        let mut param_stats = Vec::with_capacity(n_params);
        let mut concerning = Vec::new();
        let mut total_score = 0.0;

        for p in 0..n_params {
            let stats = self.analyze_param(p, time_span);
            if !stats.acceptable {
                concerning.push(p);
            }
            total_score += if stats.acceptable { 0.0 } else { 1.0 };
            param_stats.push(stats);
        }

        let overall_score = if n_params > 0 {
            total_score / n_params as f64
        } else {
            0.0
        };

        DriftReport {
            param_stats,
            n_observations: n,
            time_span,
            overall_drift_score: overall_score,
            concerning_params: concerning,
        }
    }

    /// Analyze drift for a single parameter.
    fn analyze_param(&self, param_index: usize, time_span: u64) -> DriftStatistics {
        let n = self.observations.len();
        if n < 2 || param_index >= self.observations[0].parameters.len() {
            return DriftStatistics {
                param_index,
                total_drift: 0.0,
                drift_rate: 0.0,
                max_drift: 0.0,
                drift_variance: 0.0,
                trend: 0,
                acceptable: true,
            };
        }

        // Collect values
        let values: Vec<f64> = self
            .observations
            .iter()
            .map(|o| o.parameters[param_index])
            .collect();

        // Total drift
        let first = values[0];
        let last = *values.last().unwrap();
        let total_drift = (last - first).abs();

        // Frame-to-frame drifts
        let mut drifts: Vec<f64> = Vec::with_capacity(n - 1);
        for i in 1..n {
            drifts.push((values[i] - values[i - 1]).abs());
        }

        // Statistics
        let max_drift = drifts.iter().cloned().fold(0.0, f64::max);
        let mean_drift = drifts.iter().sum::<f64>() / drifts.len() as f64;
        let drift_rate = if time_span > 0 {
            total_drift / time_span as f64
        } else {
            mean_drift
        };

        // Variance
        let variance = if drifts.len() > 1 {
            let sq_sum: f64 = drifts.iter().map(|d| (d - mean_drift).powi(2)).sum();
            sq_sum / (drifts.len() - 1) as f64
        } else {
            0.0
        };

        // Trend detection (linear regression slope sign)
        let trend = self.detect_trend(&values);

        // Check acceptability
        let acceptable = drift_rate <= self.max_drift_rate;

        DriftStatistics {
            param_index,
            total_drift,
            drift_rate,
            max_drift,
            drift_variance: variance,
            trend,
            acceptable,
        }
    }

    /// Detect trend direction using simple linear regression.
    fn detect_trend(&self, values: &[f64]) -> i8 {
        let n = values.len();
        if n < 3 {
            return 0;
        }

        // Use last `trend_window` values
        let start = if n > self.trend_window {
            n - self.trend_window
        } else {
            0
        };
        let window = &values[start..];
        let w_len = window.len() as f64;

        // Linear regression
        let x_mean = (w_len - 1.0) / 2.0;
        let y_mean: f64 = window.iter().sum::<f64>() / w_len;

        let mut num = 0.0;
        let mut den = 0.0;

        for (i, &y) in window.iter().enumerate() {
            let x = i as f64;
            num += (x - x_mean) * (y - y_mean);
            den += (x - x_mean).powi(2);
        }

        if den.abs() < 1e-15 {
            return 0;
        }

        let slope = num / den;

        // Threshold for significant trend
        let threshold = 1e-6 * y_mean.abs().max(1.0);

        if slope > threshold {
            1
        } else if slope < -threshold {
            -1
        } else {
            0
        }
    }

    /// Get current observation count.
    pub fn n_observations(&self) -> usize {
        self.observations.len()
    }

    /// Check if drift is within acceptable limits.
    pub fn is_stable(&self) -> bool {
        let report = self.analyze();
        !report.has_concerning_drift()
    }

    /// Get parameter value history.
    pub fn parameter_history(&self, param_index: usize) -> Vec<(u64, f64)> {
        self.observations
            .iter()
            .filter_map(|o| {
                if param_index < o.parameters.len() {
                    Some((o.timestamp, o.parameters[param_index]))
                } else {
                    None
                }
            })
            .collect()
    }

    /// Reset the monitor.
    pub fn reset(&mut self) {
        self.observations.clear();
    }
}

// ============================================================================
// MEMORY ESTIMATION
// ============================================================================

/// Estimate memory usage for twin validation module.
pub fn estimate_validation_memory() -> usize {
    // TwinValidator with history
    let validator = 1000 * 64; // 1000 records * ~64 bytes

    // DriftMonitor
    let monitor = 10000 * 56; // 10000 observations * ~56 bytes

    validator + monitor + 512 // overhead
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validation_result_pass() {
        let result = ValidationResult::pass();
        assert!(result.passed);
        assert_eq!(result.score, 1.0);
        assert!(result.issues.is_empty());
    }

    #[test]
    fn test_validation_result_fail() {
        let issue = ValidationIssue {
            category: IssueCategory::EnergyViolation,
            description: "Test".to_string(),
            param_index: None,
            actual: Some(1.5),
            expected: Some((0.0, 1.0)),
        };
        let result = ValidationResult::fail(vec![issue]);

        assert!(!result.passed);
        assert!(result.severity > 0.0);
        assert_eq!(result.issues.len(), 1);
    }

    #[test]
    fn test_validation_result_merge() {
        let r1 = ValidationResult::pass();
        let r2 = ValidationResult::fail(vec![ValidationIssue {
            category: IssueCategory::ParameterOutOfBounds,
            description: "Test".to_string(),
            param_index: Some(0),
            actual: Some(2.0),
            expected: Some((0.0, 1.0)),
        }]);

        let merged = r1.merge(r2);
        assert!(!merged.passed);
        assert_eq!(merged.issues.len(), 1);
    }

    #[test]
    fn test_validator_energy() {
        let validator = TwinValidator::new();

        assert!(validator.validate_energy(0.5).passed);
        assert!(validator.validate_energy(1.0).passed);
        assert!(!validator.validate_energy(1.1).passed);
        assert!(!validator.validate_energy(-0.1).passed);
    }

    #[test]
    fn test_validator_ior() {
        let validator = TwinValidator::new();

        assert!(validator.validate_ior(1.5, 0).passed);
        assert!(validator.validate_ior(2.5, 0).passed);
        assert!(!validator.validate_ior(0.5, 0).passed);
        assert!(!validator.validate_ior(5.0, 0).passed);
    }

    #[test]
    fn test_validator_roughness() {
        let validator = TwinValidator::new();

        assert!(validator.validate_roughness(0.0, 0).passed);
        assert!(validator.validate_roughness(0.5, 0).passed);
        assert!(validator.validate_roughness(1.0, 0).passed);
        assert!(!validator.validate_roughness(-0.1, 0).passed);
        assert!(!validator.validate_roughness(1.5, 0).passed);
    }

    #[test]
    fn test_validator_spectral() {
        let validator = TwinValidator::new();

        // Valid smooth spectrum
        let wavelengths = vec![400.0, 450.0, 500.0, 550.0, 600.0];
        let values = vec![0.1, 0.2, 0.3, 0.35, 0.4];
        assert!(validator.validate_spectral(&wavelengths, &values).passed);

        // Negative value
        let bad_values = vec![0.1, -0.2, 0.3, 0.35, 0.4];
        assert!(
            !validator
                .validate_spectral(&wavelengths, &bad_values)
                .passed
        );
    }

    #[test]
    fn test_drift_monitor_basic() {
        let mut monitor = DriftMonitor::new(2);
        let fp = MaterialFingerprint::from_bytes(&[0u8; 16]);

        monitor.observe(vec![1.0, 2.0], 0, fp.clone());
        monitor.observe(vec![1.0, 2.0], 1, fp.clone());
        monitor.observe(vec![1.0, 2.0], 2, fp.clone());

        let report = monitor.analyze();
        assert!(!report.has_concerning_drift());
        assert_eq!(report.n_observations, 3);
    }

    #[test]
    fn test_drift_monitor_detects_drift() {
        let mut monitor = DriftMonitor::new(1).with_max_drift_rate(0.001);
        let fp = MaterialFingerprint::from_bytes(&[0u8; 16]);

        // Large drift
        for i in 0..100 {
            monitor.observe(vec![1.0 + i as f64 * 0.1], i, fp.clone());
        }

        let report = monitor.analyze();
        assert!(report.has_concerning_drift());
    }

    #[test]
    fn test_drift_statistics() {
        let mut monitor = DriftMonitor::new(1);
        let fp = MaterialFingerprint::from_bytes(&[0u8; 16]);

        monitor.observe(vec![1.0], 0, fp.clone());
        monitor.observe(vec![1.1], 1, fp.clone());
        monitor.observe(vec![1.2], 2, fp.clone());

        let report = monitor.analyze();
        assert_eq!(report.param_stats.len(), 1);
        assert!((report.param_stats[0].total_drift - 0.2).abs() < 0.01);
    }

    #[test]
    fn test_trend_detection() {
        let mut monitor = DriftMonitor::new(1);
        let fp = MaterialFingerprint::from_bytes(&[0u8; 16]);

        // Increasing trend
        for i in 0..50 {
            monitor.observe(vec![1.0 + i as f64 * 0.01], i, fp.clone());
        }

        let report = monitor.analyze();
        assert_eq!(report.param_stats[0].trend, 1);
    }

    #[test]
    fn test_parameter_history() {
        let mut monitor = DriftMonitor::new(2);
        let fp = MaterialFingerprint::from_bytes(&[0u8; 16]);

        monitor.observe(vec![1.0, 2.0], 0, fp.clone());
        monitor.observe(vec![1.5, 2.5], 10, fp.clone());
        monitor.observe(vec![2.0, 3.0], 20, fp.clone());

        let history = monitor.parameter_history(0);
        assert_eq!(history.len(), 3);
        assert_eq!(history[0], (0, 1.0));
        assert_eq!(history[2], (20, 2.0));
    }

    #[test]
    fn test_validator_record_history() {
        let mut validator = TwinValidator::new();
        let fp = MaterialFingerprint::from_bytes(&[1u8; 16]);

        validator.record(fp.clone(), ValidationResult::pass(), 0);
        validator.record(fp.clone(), ValidationResult::pass(), 1);

        let history = validator.history_for(&fp);
        assert_eq!(history.len(), 2);
    }

    #[test]
    fn test_pass_rate() {
        let mut validator = TwinValidator::new();
        let fp = MaterialFingerprint::from_bytes(&[0u8; 16]);

        validator.record(fp.clone(), ValidationResult::pass(), 0);
        validator.record(fp.clone(), ValidationResult::pass(), 1);
        validator.record(fp.clone(), ValidationResult::fail(vec![]), 2);

        assert!((validator.pass_rate() - 0.666).abs() < 0.01);
    }

    #[test]
    fn test_drift_report_summary() {
        let report = DriftReport {
            param_stats: Vec::new(),
            n_observations: 100,
            time_span: 100,
            overall_drift_score: 0.05,
            concerning_params: Vec::new(),
        };

        let summary = report.summary();
        assert!(summary.contains("Drift OK"));
    }

    #[test]
    fn test_issue_severity() {
        let energy = ValidationIssue {
            category: IssueCategory::EnergyViolation,
            description: String::new(),
            param_index: None,
            actual: None,
            expected: None,
        };
        assert!(energy.severity() > 0.8);

        let warning = ValidationIssue {
            category: IssueCategory::IdentifiabilityWarning,
            description: String::new(),
            param_index: None,
            actual: None,
            expected: None,
        };
        assert!(warning.severity() < 0.5);
    }

    #[test]
    fn test_memory_estimate() {
        let mem = estimate_validation_memory();
        assert!(mem > 0);
        assert!(mem < 1_000_000); // Should be under 1MB
    }

    #[test]
    fn test_monitor_reset() {
        let mut monitor = DriftMonitor::new(1);
        let fp = MaterialFingerprint::from_bytes(&[0u8; 16]);

        monitor.observe(vec![1.0], 0, fp);
        assert_eq!(monitor.n_observations(), 1);

        monitor.reset();
        assert_eq!(monitor.n_observations(), 0);
    }

    #[test]
    fn test_stability_check() {
        let mut monitor = DriftMonitor::new(2);
        let fp = MaterialFingerprint::from_bytes(&[0u8; 16]);

        // Stable observations
        for i in 0..10 {
            monitor.observe(vec![1.0, 2.0], i, fp.clone());
        }

        assert!(monitor.is_stable());
    }
}
