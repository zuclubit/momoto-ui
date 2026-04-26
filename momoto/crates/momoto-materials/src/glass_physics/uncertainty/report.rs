//! # Confidence Report
//!
//! Human-readable uncertainty summaries for material twins.

use super::bootstrap::ConfidenceInterval;
use super::covariance::ParameterCovarianceMatrix;

// ============================================================================
// CONFIDENCE LEVEL
// ============================================================================

/// Standard confidence levels.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConfidenceLevel {
    /// 68% (1σ)
    P68,
    /// 90%
    P90,
    /// 95% (default)
    P95,
    /// 99%
    P99,
    /// 99.9%
    P999,
}

impl ConfidenceLevel {
    /// Get probability value.
    pub fn probability(&self) -> f64 {
        match self {
            ConfidenceLevel::P68 => 0.68,
            ConfidenceLevel::P90 => 0.90,
            ConfidenceLevel::P95 => 0.95,
            ConfidenceLevel::P99 => 0.99,
            ConfidenceLevel::P999 => 0.999,
        }
    }

    /// Get z-score multiplier.
    pub fn z_score(&self) -> f64 {
        match self {
            ConfidenceLevel::P68 => 1.0,
            ConfidenceLevel::P90 => 1.645,
            ConfidenceLevel::P95 => 1.96,
            ConfidenceLevel::P99 => 2.576,
            ConfidenceLevel::P999 => 3.291,
        }
    }

    /// Get display string.
    pub fn display(&self) -> &'static str {
        match self {
            ConfidenceLevel::P68 => "68%",
            ConfidenceLevel::P90 => "90%",
            ConfidenceLevel::P95 => "95%",
            ConfidenceLevel::P99 => "99%",
            ConfidenceLevel::P999 => "99.9%",
        }
    }
}

impl Default for ConfidenceLevel {
    fn default() -> Self {
        ConfidenceLevel::P95
    }
}

// ============================================================================
// CONFIDENCE WARNING
// ============================================================================

/// Warning about uncertainty estimation.
#[derive(Debug, Clone)]
pub enum ConfidenceWarning {
    /// Insufficient data for reliable estimation.
    InsufficientData {
        n_observations: usize,
        minimum: usize,
    },
    /// High parameter correlation.
    HighCorrelation {
        param_a: String,
        param_b: String,
        correlation: f64,
    },
    /// Wide confidence interval (high uncertainty).
    WideInterval { param: String, relative_width: f64 },
    /// Covariance matrix ill-conditioned.
    IllConditioned { condition_number: f64 },
    /// Parameter at boundary.
    AtBoundary { param: String, bound: String },
    /// Convergence issues.
    ConvergenceIssue { message: String },
    /// Non-identifiable parameter.
    NonIdentifiable { param: String },
}

impl ConfidenceWarning {
    /// Get warning severity (0-3).
    pub fn severity(&self) -> u8 {
        match self {
            ConfidenceWarning::InsufficientData { .. } => 3,
            ConfidenceWarning::NonIdentifiable { .. } => 3,
            ConfidenceWarning::HighCorrelation { correlation, .. } => {
                if correlation.abs() > 0.99 {
                    3
                } else {
                    2
                }
            }
            ConfidenceWarning::WideInterval { relative_width, .. } => {
                if *relative_width > 1.0 {
                    3
                } else {
                    2
                }
            }
            ConfidenceWarning::IllConditioned { .. } => 2,
            ConfidenceWarning::AtBoundary { .. } => 1,
            ConfidenceWarning::ConvergenceIssue { .. } => 2,
        }
    }

    /// Get warning message.
    pub fn message(&self) -> String {
        match self {
            ConfidenceWarning::InsufficientData {
                n_observations,
                minimum,
            } => {
                format!("Only {} observations (need {})", n_observations, minimum)
            }
            ConfidenceWarning::HighCorrelation {
                param_a,
                param_b,
                correlation,
            } => {
                format!(
                    "{} and {} are highly correlated (r={:.3})",
                    param_a, param_b, correlation
                )
            }
            ConfidenceWarning::WideInterval {
                param,
                relative_width,
            } => {
                format!(
                    "{} has wide uncertainty ({:.0}% of estimate)",
                    param,
                    relative_width * 100.0
                )
            }
            ConfidenceWarning::IllConditioned { condition_number } => {
                format!(
                    "Covariance matrix is ill-conditioned (κ={:.2e})",
                    condition_number
                )
            }
            ConfidenceWarning::AtBoundary { param, bound } => {
                format!("{} is at {} bound", param, bound)
            }
            ConfidenceWarning::ConvergenceIssue { message } => {
                format!("Convergence issue: {}", message)
            }
            ConfidenceWarning::NonIdentifiable { param } => {
                format!("{} is not identifiable from data", param)
            }
        }
    }
}

impl std::fmt::Display for ConfidenceWarning {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let severity = match self.severity() {
            3 => "[CRITICAL]",
            2 => "[WARNING]",
            _ => "[INFO]",
        };
        write!(f, "{} {}", severity, self.message())
    }
}

// ============================================================================
// PARAMETER UNCERTAINTY
// ============================================================================

/// Uncertainty information for a single parameter.
#[derive(Debug, Clone)]
pub struct ParameterUncertainty {
    /// Parameter name.
    pub name: String,
    /// Point estimate.
    pub estimate: f64,
    /// Standard error.
    pub std_error: f64,
    /// Confidence interval.
    pub ci: ConfidenceInterval,
    /// Coefficient of variation.
    pub cv: f64,
    /// Is parameter identifiable?
    pub identifiable: bool,
    /// Correlations with other parameters (name, correlation).
    pub correlations: Vec<(String, f64)>,
}

impl ParameterUncertainty {
    /// Create from estimate and standard error.
    pub fn new(name: &str, estimate: f64, std_error: f64, level: ConfidenceLevel) -> Self {
        let z = level.z_score();
        let ci = ConfidenceInterval::new(
            estimate - z * std_error,
            estimate + z * std_error,
            estimate,
            level.probability(),
        );
        let cv = if estimate.abs() > 1e-10 {
            std_error / estimate.abs()
        } else {
            f64::INFINITY
        };

        Self {
            name: name.to_string(),
            estimate,
            std_error,
            ci,
            cv,
            identifiable: std_error.is_finite() && std_error < estimate.abs() * 10.0,
            correlations: Vec::new(),
        }
    }

    /// Set correlations.
    pub fn with_correlations(mut self, correlations: Vec<(String, f64)>) -> Self {
        self.correlations = correlations;
        self
    }

    /// Check if parameter is well-determined.
    pub fn is_well_determined(&self) -> bool {
        self.identifiable && self.cv < 0.5
    }

    /// Format as string.
    pub fn format(&self) -> String {
        format!(
            "{}: {:.4} ± {:.4} ({})",
            self.name,
            self.estimate,
            self.std_error,
            if self.is_well_determined() {
                "OK"
            } else {
                "uncertain"
            }
        )
    }
}

// ============================================================================
// TWIN CONFIDENCE REPORT
// ============================================================================

/// Complete confidence report for a material twin.
#[derive(Debug, Clone)]
pub struct TwinConfidenceReport {
    /// Parameter names.
    pub param_names: Vec<String>,
    /// Point estimates.
    pub estimates: Vec<f64>,
    /// Standard errors.
    pub standard_errors: Vec<f64>,
    /// Confidence intervals (lower, upper).
    pub confidence_intervals: Vec<(f64, f64)>,
    /// Parameter uncertainties with details.
    pub parameters: Vec<ParameterUncertainty>,
    /// Highly correlated parameter pairs.
    pub correlations: Vec<(usize, usize, f64)>,
    /// Overall confidence score (0-1).
    pub overall_confidence: f64,
    /// Warnings.
    pub warnings: Vec<ConfidenceWarning>,
    /// Confidence level used.
    pub level: ConfidenceLevel,
    /// Number of observations.
    pub n_observations: usize,
}

impl TwinConfidenceReport {
    /// Create from covariance matrix.
    pub fn from_covariance(
        cov: &ParameterCovarianceMatrix,
        estimates: &[f64],
        level: ConfidenceLevel,
    ) -> Self {
        let n = cov.n;
        let z = level.z_score();

        let param_names = cov.param_names.clone();
        let standard_errors = cov.std_devs();

        let confidence_intervals: Vec<(f64, f64)> = estimates
            .iter()
            .zip(standard_errors.iter())
            .map(|(est, se)| (est - z * se, est + z * se))
            .collect();

        // Build parameter uncertainties
        let mut parameters = Vec::with_capacity(n);
        for i in 0..n {
            let mut param =
                ParameterUncertainty::new(&param_names[i], estimates[i], standard_errors[i], level);

            // Add correlations
            let corrs: Vec<(String, f64)> = (0..n)
                .filter(|&j| j != i)
                .map(|j| (param_names[j].clone(), cov.correlation(i, j)))
                .filter(|(_, r)| r.abs() > 0.3)
                .collect();
            param = param.with_correlations(corrs);

            parameters.push(param);
        }

        // Find high correlations
        let correlations = cov.find_correlated_pairs(0.7);

        // Generate warnings
        let mut warnings = Vec::new();

        // Check for high correlations
        for (i, j, r) in &correlations {
            if r.abs() > 0.9 {
                warnings.push(ConfidenceWarning::HighCorrelation {
                    param_a: param_names[*i].clone(),
                    param_b: param_names[*j].clone(),
                    correlation: *r,
                });
            }
        }

        // Check for wide intervals
        for param in &parameters {
            if param.cv > 0.5 {
                warnings.push(ConfidenceWarning::WideInterval {
                    param: param.name.clone(),
                    relative_width: param.cv,
                });
            }
            if !param.identifiable {
                warnings.push(ConfidenceWarning::NonIdentifiable {
                    param: param.name.clone(),
                });
            }
        }

        // Compute overall confidence
        let overall_confidence = Self::compute_overall_confidence(&parameters, &warnings);

        Self {
            param_names,
            estimates: estimates.to_vec(),
            standard_errors,
            confidence_intervals,
            parameters,
            correlations,
            overall_confidence,
            warnings,
            level,
            n_observations: 0,
        }
    }

    /// Compute overall confidence score.
    fn compute_overall_confidence(
        params: &[ParameterUncertainty],
        warnings: &[ConfidenceWarning],
    ) -> f64 {
        // Start with perfect confidence
        let mut confidence: f64 = 1.0;

        // Reduce for each non-identifiable parameter
        for param in params {
            if !param.is_well_determined() {
                confidence *= 0.8;
            }
        }

        // Reduce for warnings
        for warning in warnings {
            match warning.severity() {
                3 => confidence *= 0.5,
                2 => confidence *= 0.9,
                _ => confidence *= 0.95,
            }
        }

        confidence.clamp(0.0, 1.0)
    }

    /// Set number of observations.
    pub fn with_observations(mut self, n: usize) -> Self {
        self.n_observations = n;
        if n < 10 {
            self.warnings.push(ConfidenceWarning::InsufficientData {
                n_observations: n,
                minimum: 10,
            });
            self.overall_confidence *= 0.5;
        }
        self
    }

    /// Get parameter uncertainty by name.
    pub fn get_parameter(&self, name: &str) -> Option<&ParameterUncertainty> {
        self.parameters.iter().find(|p| p.name == name)
    }

    /// Get number of well-determined parameters.
    pub fn n_well_determined(&self) -> usize {
        self.parameters
            .iter()
            .filter(|p| p.is_well_determined())
            .count()
    }

    /// Get number of critical warnings.
    pub fn n_critical_warnings(&self) -> usize {
        self.warnings.iter().filter(|w| w.severity() == 3).count()
    }

    /// Check if report is acceptable (no critical warnings).
    pub fn is_acceptable(&self) -> bool {
        self.n_critical_warnings() == 0 && self.overall_confidence > 0.5
    }

    /// Format as detailed string.
    pub fn format_detailed(&self) -> String {
        let mut s = String::new();

        s.push_str("╔════════════════════════════════════════════════════════════╗\n");
        s.push_str("║              MATERIAL TWIN CONFIDENCE REPORT               ║\n");
        s.push_str("╠════════════════════════════════════════════════════════════╣\n");

        s.push_str(&format!(
            "║ Confidence Level: {:>40} ║\n",
            self.level.display()
        ));
        s.push_str(&format!(
            "║ Overall Confidence: {:>38.1}% ║\n",
            self.overall_confidence * 100.0
        ));
        s.push_str(&format!(
            "║ Parameters: {:>46} ║\n",
            format!(
                "{}/{} well-determined",
                self.n_well_determined(),
                self.parameters.len()
            )
        ));

        s.push_str("╠════════════════════════════════════════════════════════════╣\n");
        s.push_str("║ Parameter Estimates                                        ║\n");
        s.push_str("╟────────────────────────────────────────────────────────────╢\n");

        for param in &self.parameters {
            let status = if param.is_well_determined() {
                "✓"
            } else {
                "?"
            };
            s.push_str(&format!(
                "║ {} {}: {:>10.4} ± {:>8.4} {:>18} ║\n",
                status,
                format!("{:12}", param.name),
                param.estimate,
                param.std_error,
                format!("[{:.3}, {:.3}]", param.ci.lower, param.ci.upper)
            ));
        }

        if !self.warnings.is_empty() {
            s.push_str("╠════════════════════════════════════════════════════════════╣\n");
            s.push_str("║ Warnings                                                   ║\n");
            s.push_str("╟────────────────────────────────────────────────────────────╢\n");

            for warning in &self.warnings {
                let severity = match warning.severity() {
                    3 => "⚠",
                    2 => "!",
                    _ => "i",
                };
                let msg = warning.message();
                let truncated = if msg.len() > 55 {
                    format!("{}...", &msg[..52])
                } else {
                    msg
                };
                s.push_str(&format!("║ {} {:56} ║\n", severity, truncated));
            }
        }

        s.push_str("╚════════════════════════════════════════════════════════════╝\n");

        s
    }
}

impl std::fmt::Display for TwinConfidenceReport {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.format_detailed())
    }
}

// ============================================================================
// HELPER FUNCTIONS
// ============================================================================

/// Format uncertainty as string.
pub fn format_uncertainty(estimate: f64, std_error: f64, decimals: usize) -> String {
    format!(
        "{:.prec$} ± {:.prec$}",
        estimate,
        std_error,
        prec = decimals
    )
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_confidence_level() {
        let level = ConfidenceLevel::P95;
        assert!((level.probability() - 0.95).abs() < 0.01);
        assert!((level.z_score() - 1.96).abs() < 0.01);
    }

    #[test]
    fn test_confidence_warning_severity() {
        let critical = ConfidenceWarning::InsufficientData {
            n_observations: 5,
            minimum: 10,
        };
        assert_eq!(critical.severity(), 3);

        let info = ConfidenceWarning::AtBoundary {
            param: "ior".to_string(),
            bound: "lower".to_string(),
        };
        assert_eq!(info.severity(), 1);
    }

    #[test]
    fn test_parameter_uncertainty() {
        let param = ParameterUncertainty::new("ior", 1.5, 0.01, ConfidenceLevel::P95);

        assert!(param.is_well_determined());
        assert!((param.cv - 0.01 / 1.5).abs() < 0.001);
        assert!(param.ci.contains(1.5));
    }

    #[test]
    fn test_parameter_uncertainty_poor() {
        let param = ParameterUncertainty::new("roughness", 0.1, 0.5, ConfidenceLevel::P95);

        assert!(!param.is_well_determined());
        assert!(param.cv > 1.0);
    }

    #[test]
    fn test_twin_confidence_report() {
        let cov = ParameterCovarianceMatrix::diagonal(&[0.01, 0.04, 0.09]).with_names(vec![
            "ior".to_string(),
            "roughness".to_string(),
            "k".to_string(),
        ]);

        let estimates = vec![1.5, 0.1, 0.0];
        let report = TwinConfidenceReport::from_covariance(&cov, &estimates, ConfidenceLevel::P95);

        assert_eq!(report.parameters.len(), 3);
        assert!((report.standard_errors[0] - 0.1).abs() < 0.01);
    }

    #[test]
    fn test_report_with_high_correlation() {
        // Create a covariance matrix with high correlation
        let mut cov = ParameterCovarianceMatrix::zeros(2);
        cov.set(0, 0, 1.0);
        cov.set(1, 1, 1.0);
        cov.set(0, 1, 0.95);
        cov = cov.with_names(vec!["a".to_string(), "b".to_string()]);

        let report = TwinConfidenceReport::from_covariance(&cov, &[1.0, 1.0], ConfidenceLevel::P95);

        assert!(report
            .warnings
            .iter()
            .any(|w| matches!(w, ConfidenceWarning::HighCorrelation { .. })));
    }

    #[test]
    fn test_report_is_acceptable() {
        let cov = ParameterCovarianceMatrix::diagonal(&[0.01, 0.01]);
        let report = TwinConfidenceReport::from_covariance(&cov, &[1.0, 1.0], ConfidenceLevel::P95);

        assert!(report.is_acceptable());
    }

    #[test]
    fn test_report_with_insufficient_observations() {
        let cov = ParameterCovarianceMatrix::diagonal(&[0.01]);
        let report = TwinConfidenceReport::from_covariance(&cov, &[1.0], ConfidenceLevel::P95)
            .with_observations(5);

        assert!(report
            .warnings
            .iter()
            .any(|w| matches!(w, ConfidenceWarning::InsufficientData { .. })));
        assert!(!report.is_acceptable());
    }

    #[test]
    fn test_format_uncertainty() {
        let s = format_uncertainty(1.5, 0.1, 3);
        assert!(s.contains("1.500"));
        assert!(s.contains("0.100"));
    }

    #[test]
    fn test_report_display() {
        let cov = ParameterCovarianceMatrix::diagonal(&[0.01, 0.04])
            .with_names(vec!["ior".to_string(), "roughness".to_string()]);
        let report = TwinConfidenceReport::from_covariance(&cov, &[1.5, 0.1], ConfidenceLevel::P95);

        let display = format!("{}", report);
        assert!(display.contains("CONFIDENCE REPORT"));
        assert!(display.contains("ior"));
    }
}
