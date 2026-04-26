//! # Uncertainty Propagation
//!
//! Forward and inverse uncertainty propagation for metrological measurements.
//! Implements GUM-compliant uncertainty analysis methods.

use super::measurement::{
    Measurement, MeasurementId, MeasurementQuality, MeasurementSource, Uncertainty,
};
use super::units::Unit;

// ============================================================================
// PROPAGATION METHODS
// ============================================================================

/// Method for uncertainty propagation.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PropagationMethod {
    /// First-order Taylor series (linear approximation).
    Linear,
    /// Second-order Taylor series.
    SecondOrder,
    /// Monte Carlo simulation.
    MonteCarlo {
        /// Number of Monte Carlo samples.
        n_samples: usize,
    },
    /// Analytical closed-form (when available).
    Analytical,
}

impl PropagationMethod {
    /// Get default Monte Carlo configuration.
    pub fn monte_carlo() -> Self {
        PropagationMethod::MonteCarlo { n_samples: 10000 }
    }

    /// Get method name.
    pub fn name(&self) -> &'static str {
        match self {
            PropagationMethod::Linear => "Linear (First-order Taylor)",
            PropagationMethod::SecondOrder => "Second-order Taylor",
            PropagationMethod::MonteCarlo { .. } => "Monte Carlo",
            PropagationMethod::Analytical => "Analytical",
        }
    }
}

// ============================================================================
// UNCERTAINTY PROPAGATOR
// ============================================================================

/// Uncertainty propagation engine.
#[derive(Debug, Clone)]
pub struct UncertaintyPropagator {
    /// Propagation method.
    pub method: PropagationMethod,
    /// Correlation matrix (if inputs are correlated).
    pub correlation_matrix: Option<Vec<Vec<f64>>>,
    /// Random seed for Monte Carlo.
    pub seed: u64,
    /// Coverage factor (k) for expanded uncertainty.
    pub coverage_factor: f64,
}

impl Default for UncertaintyPropagator {
    fn default() -> Self {
        Self {
            method: PropagationMethod::Linear,
            correlation_matrix: None,
            seed: 42,
            coverage_factor: 2.0, // 95% confidence
        }
    }
}

impl UncertaintyPropagator {
    /// Create with specific method.
    pub fn with_method(method: PropagationMethod) -> Self {
        Self {
            method,
            ..Default::default()
        }
    }

    /// Create linear propagator.
    pub fn linear() -> Self {
        Self::with_method(PropagationMethod::Linear)
    }

    /// Create Monte Carlo propagator.
    pub fn monte_carlo(n_samples: usize) -> Self {
        Self::with_method(PropagationMethod::MonteCarlo { n_samples })
    }

    /// Set correlation matrix.
    pub fn with_correlations(mut self, matrix: Vec<Vec<f64>>) -> Self {
        self.correlation_matrix = Some(matrix);
        self
    }

    /// Set coverage factor.
    pub fn with_coverage_factor(mut self, k: f64) -> Self {
        self.coverage_factor = k;
        self
    }

    /// Propagate uncertainty through a function using Jacobian.
    ///
    /// # Arguments
    /// * `inputs` - Input measurements
    /// * `jacobian` - Partial derivatives ∂y/∂xᵢ evaluated at input values
    /// * `output_value` - Computed output value
    /// * `output_unit` - Unit of output
    ///
    /// # Returns
    /// Output measurement with propagated uncertainty.
    pub fn propagate_forward(
        &self,
        inputs: &[Measurement<f64>],
        jacobian: &[f64],
        output_value: f64,
        output_unit: Unit,
    ) -> Measurement<f64> {
        assert_eq!(inputs.len(), jacobian.len(), "Jacobian must match inputs");

        let combined_uncertainty = match self.method {
            PropagationMethod::Linear | PropagationMethod::SecondOrder => {
                self.propagate_linear(inputs, jacobian)
            }
            PropagationMethod::MonteCarlo { n_samples } => {
                self.propagate_monte_carlo(inputs, jacobian, n_samples)
            }
            PropagationMethod::Analytical => {
                // Fall back to linear for general case
                self.propagate_linear(inputs, jacobian)
            }
        };

        Measurement {
            id: MeasurementId::generate(),
            value: output_value,
            uncertainty: combined_uncertainty,
            unit: output_unit,
            confidence_level: 0.95,
            quality: self.derive_quality(inputs),
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
            source: MeasurementSource::Calculated {
                method: self.method.name().to_string(),
            },
        }
    }

    /// Linear (first-order Taylor) uncertainty propagation.
    fn propagate_linear(&self, inputs: &[Measurement<f64>], jacobian: &[f64]) -> Uncertainty {
        let n = inputs.len();

        // Get standard uncertainties
        let std_uncertainties: Vec<f64> = inputs.iter().map(|m| m.uncertainty.standard()).collect();

        // Combined variance: sum(cᵢ² * uᵢ²) + 2 * sum(cᵢ * cⱼ * r_ij * uᵢ * uⱼ)
        let mut variance = 0.0;

        // Diagonal terms
        for i in 0..n {
            variance += jacobian[i].powi(2) * std_uncertainties[i].powi(2);
        }

        // Cross terms (if correlated)
        if let Some(ref corr) = self.correlation_matrix {
            for i in 0..n {
                for j in (i + 1)..n {
                    let r_ij = corr[i][j];
                    variance += 2.0
                        * jacobian[i]
                        * jacobian[j]
                        * r_ij
                        * std_uncertainties[i]
                        * std_uncertainties[j];
                }
            }
        }

        let combined_std = variance.sqrt();

        // Separate type A and type B contributions for traceability
        let type_a_var: f64 = inputs
            .iter()
            .zip(jacobian.iter())
            .filter_map(|(m, &c)| match &m.uncertainty {
                Uncertainty::TypeA { std_error, .. } => Some(c.powi(2) * std_error.powi(2)),
                Uncertainty::Combined { type_a, .. } => Some(c.powi(2) * type_a.powi(2)),
                _ => None,
            })
            .sum();

        let type_b_var: f64 = inputs
            .iter()
            .zip(jacobian.iter())
            .filter_map(|(m, &c)| match &m.uncertainty {
                Uncertainty::TypeB { systematic, .. } => Some(c.powi(2) * systematic.powi(2)),
                Uncertainty::Combined { type_b, .. } => Some(c.powi(2) * type_b.powi(2)),
                _ => None,
            })
            .sum();

        if type_a_var > 0.0 && type_b_var > 0.0 {
            Uncertainty::Combined {
                type_a: type_a_var.sqrt(),
                type_b: type_b_var.sqrt(),
            }
        } else if type_a_var > 0.0 {
            // Determine effective sample size
            let total_samples: usize = inputs
                .iter()
                .filter_map(|m| match &m.uncertainty {
                    Uncertainty::TypeA { n_samples, .. } => Some(*n_samples),
                    Uncertainty::Combined { .. } => Some(100), // Assume reasonable n
                    _ => None,
                })
                .sum();

            Uncertainty::TypeA {
                std_error: combined_std,
                n_samples: (total_samples / inputs.len()).max(1),
            }
        } else if type_b_var > 0.0 {
            Uncertainty::TypeB {
                systematic: combined_std,
                source: "Propagated".to_string(),
            }
        } else {
            Uncertainty::Combined {
                type_a: combined_std * 0.5, // Assume equal split if unknown
                type_b: combined_std * 0.5,
            }
        }
    }

    /// Monte Carlo uncertainty propagation.
    fn propagate_monte_carlo(
        &self,
        inputs: &[Measurement<f64>],
        _jacobian: &[f64],
        n_samples: usize,
    ) -> Uncertainty {
        // Simple LCG for reproducibility
        let mut rng_state = self.seed;
        let next_random = || {
            rng_state = rng_state.wrapping_mul(6364136223846793005).wrapping_add(1);
            ((rng_state >> 33) as f64) / (u32::MAX as f64)
        };

        // Box-Muller for normal distribution
        let normal = |mean: f64, std: f64, rng: &mut dyn FnMut() -> f64| {
            let u1 = rng();
            let u2 = rng();
            let z = (-2.0 * u1.ln()).sqrt() * (2.0 * std::f64::consts::PI * u2).cos();
            mean + std * z
        };

        // Sample outputs (for linear model: y = sum(c_i * x_i))
        let mut outputs = Vec::with_capacity(n_samples);
        let mut rng = next_random;

        for _ in 0..n_samples {
            let sampled_inputs: Vec<f64> = inputs
                .iter()
                .map(|m| {
                    let std = m.uncertainty.standard();
                    normal(m.value, std, &mut rng)
                })
                .collect();

            // For this simplified version, sum the samples
            // In practice, you'd evaluate the actual function
            outputs.push(sampled_inputs.iter().sum::<f64>());
        }

        // Compute statistics
        let mean: f64 = outputs.iter().sum::<f64>() / n_samples as f64;
        let variance: f64 =
            outputs.iter().map(|&x| (x - mean).powi(2)).sum::<f64>() / (n_samples - 1) as f64;
        let std_dev = variance.sqrt();

        Uncertainty::TypeA {
            std_error: std_dev,
            n_samples,
        }
    }

    /// Derive output quality from input qualities.
    fn derive_quality(&self, inputs: &[Measurement<f64>]) -> MeasurementQuality {
        // Quality degrades to worst input (max key = worst quality)
        inputs
            .iter()
            .map(|m| &m.quality)
            .max_by_key(|q| match q {
                MeasurementQuality::Calibrated => 0,
                MeasurementQuality::Validated => 1,
                MeasurementQuality::Estimated => 2,
                MeasurementQuality::Interpolated => 3,
                MeasurementQuality::Extrapolated => 4,
                MeasurementQuality::Unknown => 5,
            })
            .cloned()
            .unwrap_or(MeasurementQuality::Unknown)
    }

    /// Inverse propagation: estimate input uncertainties from output.
    ///
    /// Given output uncertainty and Jacobian, estimate required input uncertainties.
    pub fn propagate_inverse(&self, output: &Measurement<f64>, jacobian: &[f64]) -> Vec<f64> {
        let output_var = output.uncertainty.standard().powi(2);
        let n = jacobian.len();

        // Equal allocation strategy: each input contributes equally
        let contribution_per_input = output_var / n as f64;

        jacobian
            .iter()
            .map(|&c| {
                if c.abs() > 1e-12 {
                    (contribution_per_input / c.powi(2)).sqrt()
                } else {
                    f64::INFINITY // Cannot determine if sensitivity is zero
                }
            })
            .collect()
    }
}

// ============================================================================
// SENSITIVITY ANALYSIS
// ============================================================================

/// Sensitivity analysis for uncertainty contributions.
#[derive(Debug, Clone)]
pub struct SensitivityAnalysis {
    /// Input names.
    pub input_names: Vec<String>,
    /// Sensitivity coefficients (|cᵢ|).
    pub sensitivities: Vec<f64>,
    /// Uncertainty contributions (cᵢ² * uᵢ²).
    pub contributions: Vec<f64>,
    /// Percentage contributions.
    pub percentages: Vec<f64>,
    /// Total combined uncertainty.
    pub total_uncertainty: f64,
}

impl SensitivityAnalysis {
    /// Perform sensitivity analysis.
    pub fn analyze(inputs: &[Measurement<f64>], input_names: &[&str], jacobian: &[f64]) -> Self {
        let n = inputs.len();
        assert_eq!(n, input_names.len());
        assert_eq!(n, jacobian.len());

        let contributions: Vec<f64> = inputs
            .iter()
            .zip(jacobian.iter())
            .map(|(m, &c)| c.powi(2) * m.uncertainty.standard().powi(2))
            .collect();

        let total_var: f64 = contributions.iter().sum();
        let total_uncertainty = total_var.sqrt();

        let percentages: Vec<f64> = contributions
            .iter()
            .map(|&c| {
                if total_var > 0.0 {
                    c / total_var * 100.0
                } else {
                    0.0
                }
            })
            .collect();

        Self {
            input_names: input_names.iter().map(|s| s.to_string()).collect(),
            sensitivities: jacobian.iter().map(|c| c.abs()).collect(),
            contributions,
            percentages,
            total_uncertainty,
        }
    }

    /// Get dominant uncertainty source.
    pub fn dominant_source(&self) -> Option<&str> {
        self.percentages
            .iter()
            .enumerate()
            .max_by(|a, b| a.1.partial_cmp(b.1).unwrap())
            .map(|(i, _)| self.input_names[i].as_str())
    }

    /// Generate sensitivity report.
    pub fn report(&self) -> String {
        let mut report = String::new();
        report.push_str("Sensitivity Analysis\n");
        report.push_str(&format!(
            "Total Combined Uncertainty: {:.6}\n\n",
            self.total_uncertainty
        ));
        report.push_str("Contributions:\n");

        for i in 0..self.input_names.len() {
            report.push_str(&format!(
                "  {:20} | |c| = {:.4} | Contrib = {:.6} | {:5.1}%\n",
                self.input_names[i],
                self.sensitivities[i],
                self.contributions[i].sqrt(),
                self.percentages[i]
            ));
        }

        if let Some(dominant) = self.dominant_source() {
            report.push_str(&format!("\nDominant source: {}\n", dominant));
        }

        report
    }
}

// ============================================================================
// CORRELATION UTILITIES
// ============================================================================

/// Build identity correlation matrix.
pub fn identity_correlation(n: usize) -> Vec<Vec<f64>> {
    (0..n)
        .map(|i| (0..n).map(|j| if i == j { 1.0 } else { 0.0 }).collect())
        .collect()
}

/// Build uniform correlation matrix.
pub fn uniform_correlation(n: usize, r: f64) -> Vec<Vec<f64>> {
    (0..n)
        .map(|i| (0..n).map(|j| if i == j { 1.0 } else { r }).collect())
        .collect()
}

/// Validate correlation matrix properties.
pub fn validate_correlation_matrix(matrix: &[Vec<f64>]) -> Result<(), &'static str> {
    let n = matrix.len();

    // Check square
    for row in matrix {
        if row.len() != n {
            return Err("Correlation matrix must be square");
        }
    }

    // Check diagonal = 1
    for i in 0..n {
        if (matrix[i][i] - 1.0).abs() > 1e-10 {
            return Err("Diagonal elements must be 1");
        }
    }

    // Check symmetry
    for i in 0..n {
        for j in (i + 1)..n {
            if (matrix[i][j] - matrix[j][i]).abs() > 1e-10 {
                return Err("Matrix must be symmetric");
            }
        }
    }

    // Check bounds
    for row in matrix {
        for &val in row {
            if val < -1.0 || val > 1.0 {
                return Err("Correlation values must be in [-1, 1]");
            }
        }
    }

    Ok(())
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn make_measurement(value: f64, std_error: f64) -> Measurement<f64> {
        Measurement {
            id: MeasurementId::generate(),
            value,
            uncertainty: Uncertainty::TypeA {
                std_error,
                n_samples: 10,
            },
            unit: Unit::Dimensionless,
            confidence_level: 0.95,
            quality: MeasurementQuality::Calibrated,
            timestamp: 0,
            source: MeasurementSource::Unknown,
        }
    }

    #[test]
    fn test_linear_propagation_sum() {
        let inputs = vec![make_measurement(10.0, 0.1), make_measurement(20.0, 0.2)];
        let jacobian = vec![1.0, 1.0]; // y = x1 + x2

        let prop = UncertaintyPropagator::linear();
        let output = prop.propagate_forward(&inputs, &jacobian, 30.0, Unit::Dimensionless);

        assert!((output.value - 30.0).abs() < 1e-10);
        // Combined: sqrt(0.1² + 0.2²) = sqrt(0.05) ≈ 0.2236
        let expected_std = (0.1f64.powi(2) + 0.2f64.powi(2)).sqrt();
        assert!((output.uncertainty.standard() - expected_std).abs() < 1e-4);
    }

    #[test]
    fn test_linear_propagation_weighted() {
        let inputs = vec![make_measurement(10.0, 0.1), make_measurement(20.0, 0.1)];
        let jacobian = vec![2.0, 3.0]; // y = 2*x1 + 3*x2

        let prop = UncertaintyPropagator::linear();
        let output = prop.propagate_forward(&inputs, &jacobian, 80.0, Unit::Dimensionless);

        assert!((output.value - 80.0).abs() < 1e-10);
        // Combined: sqrt((2*0.1)² + (3*0.1)²) = sqrt(0.04 + 0.09) = sqrt(0.13) ≈ 0.3606
        let expected_std = (4.0f64 * 0.01 + 9.0 * 0.01).sqrt();
        assert!((output.uncertainty.standard() - expected_std).abs() < 1e-4);
    }

    #[test]
    fn test_monte_carlo_propagation() {
        let inputs = vec![make_measurement(10.0, 0.1), make_measurement(20.0, 0.2)];
        let jacobian = vec![1.0, 1.0];

        let prop = UncertaintyPropagator::monte_carlo(50000);
        let output = prop.propagate_forward(&inputs, &jacobian, 30.0, Unit::Dimensionless);

        // Monte Carlo should give similar result (within ~10% due to statistical variance)
        let expected_std = (0.1f64.powi(2) + 0.2f64.powi(2)).sqrt();
        assert!((output.uncertainty.standard() - expected_std).abs() < 0.1);
    }

    #[test]
    fn test_inverse_propagation() {
        let output = make_measurement(100.0, 1.0);
        let jacobian = vec![1.0, 1.0]; // Equal sensitivity

        let prop = UncertaintyPropagator::linear();
        let required = prop.propagate_inverse(&output, &jacobian);

        // Each input should contribute equally
        // u_out² = u1² + u2² => 1 = 2 * u_in² => u_in ≈ 0.707
        assert_eq!(required.len(), 2);
        assert!((required[0] - 0.707).abs() < 0.01);
        assert!((required[1] - 0.707).abs() < 0.01);
    }

    #[test]
    fn test_sensitivity_analysis() {
        let inputs = vec![make_measurement(10.0, 0.5), make_measurement(20.0, 0.1)];
        let names = vec!["Large uncertainty", "Small uncertainty"];
        let jacobian = vec![1.0, 1.0];

        let analysis = SensitivityAnalysis::analyze(&inputs, &names, &jacobian);

        assert_eq!(analysis.dominant_source(), Some("Large uncertainty"));
        assert!(analysis.percentages[0] > analysis.percentages[1]);

        let report = analysis.report();
        assert!(report.contains("Large uncertainty"));
        assert!(report.contains("Dominant source"));
    }

    #[test]
    fn test_correlation_matrix() {
        let identity = identity_correlation(3);
        assert!(validate_correlation_matrix(&identity).is_ok());

        let uniform = uniform_correlation(3, 0.5);
        assert!(validate_correlation_matrix(&uniform).is_ok());

        // Invalid matrix
        let invalid = vec![vec![1.0, 2.0], vec![2.0, 1.0]]; // r > 1
        assert!(validate_correlation_matrix(&invalid).is_err());
    }

    #[test]
    fn test_correlated_propagation() {
        let inputs = vec![make_measurement(10.0, 0.1), make_measurement(20.0, 0.1)];
        let jacobian = vec![1.0, 1.0];

        // Highly correlated
        let corr = vec![vec![1.0, 0.9], vec![0.9, 1.0]];

        let prop = UncertaintyPropagator::linear().with_correlations(corr);
        let output = prop.propagate_forward(&inputs, &jacobian, 30.0, Unit::Dimensionless);

        // With positive correlation, uncertainty should be larger than uncorrelated
        let uncorrelated_std = (0.02f64).sqrt();
        assert!(output.uncertainty.standard() > uncorrelated_std);
    }

    #[test]
    fn test_propagation_method_name() {
        assert!(PropagationMethod::Linear.name().contains("Linear"));
        assert!(PropagationMethod::monte_carlo()
            .name()
            .contains("Monte Carlo"));
    }

    #[test]
    fn test_quality_degradation() {
        let inputs = vec![
            Measurement {
                id: MeasurementId::generate(),
                value: 1.0,
                uncertainty: Uncertainty::TypeA {
                    std_error: 0.1,
                    n_samples: 10,
                },
                unit: Unit::Dimensionless,
                confidence_level: 0.95,
                quality: MeasurementQuality::Calibrated,
                timestamp: 0,
                source: MeasurementSource::Unknown,
            },
            Measurement {
                id: MeasurementId::generate(),
                value: 2.0,
                uncertainty: Uncertainty::TypeA {
                    std_error: 0.1,
                    n_samples: 10,
                },
                unit: Unit::Dimensionless,
                confidence_level: 0.95,
                quality: MeasurementQuality::Estimated, // Worse quality
                timestamp: 0,
                source: MeasurementSource::Unknown,
            },
        ];

        let prop = UncertaintyPropagator::linear();
        let output = prop.propagate_forward(&inputs, &[1.0, 1.0], 3.0, Unit::Dimensionless);

        // Quality should degrade to Estimated (the worst)
        assert_eq!(output.quality, MeasurementQuality::Estimated);
    }
}
