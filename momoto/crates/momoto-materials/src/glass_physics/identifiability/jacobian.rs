//! # Jacobian Rank Analysis
//!
//! Detect non-identifiable parameters via Jacobian rank analysis.

// ============================================================================
// RANK DEFICIENCY
// ============================================================================

/// Information about rank deficiency.
#[derive(Debug, Clone)]
pub struct RankDeficiency {
    /// Index of the problematic parameter.
    pub param_index: usize,
    /// Singular value (should be near zero for non-identifiable).
    pub singular_value: f64,
    /// Which other parameters are confounded with this one.
    pub confounded_with: Vec<usize>,
    /// Explanation.
    pub explanation: String,
}

// ============================================================================
// IDENTIFIABILITY RESULT
// ============================================================================

/// Result of identifiability analysis.
#[derive(Debug, Clone)]
pub struct IdentifiabilityResult {
    /// Number of parameters.
    pub n_params: usize,
    /// Effective rank of Jacobian.
    pub rank: usize,
    /// Condition number.
    pub condition_number: f64,
    /// Indices of non-identifiable parameters.
    pub non_identifiable: Vec<usize>,
    /// Identifiability ratio (rank / n_params).
    pub identifiability_ratio: f64,
    /// Singular values (sorted descending).
    pub singular_values: Vec<f64>,
    /// Rank deficiency details.
    pub deficiencies: Vec<RankDeficiency>,
    /// Overall identifiability score (0-1).
    pub score: f64,
}

impl IdentifiabilityResult {
    /// Check if all parameters are identifiable.
    pub fn all_identifiable(&self) -> bool {
        self.non_identifiable.is_empty()
    }

    /// Check if parameter is identifiable.
    pub fn is_identifiable(&self, idx: usize) -> bool {
        !self.non_identifiable.contains(&idx)
    }

    /// Get number of non-identifiable parameters.
    pub fn n_non_identifiable(&self) -> usize {
        self.non_identifiable.len()
    }

    /// Check if analysis is reliable (condition number not too high).
    pub fn is_reliable(&self) -> bool {
        self.condition_number < 1e10
    }

    /// Get a summary message.
    pub fn summary(&self) -> String {
        if self.all_identifiable() {
            format!(
                "All {} parameters identifiable (κ={:.2e})",
                self.n_params, self.condition_number
            )
        } else {
            format!(
                "{}/{} parameters non-identifiable (κ={:.2e})",
                self.n_non_identifiable(),
                self.n_params,
                self.condition_number
            )
        }
    }
}

// ============================================================================
// JACOBIAN RANK ANALYZER
// ============================================================================

/// Analyzer for Jacobian matrix rank.
#[derive(Debug, Clone)]
pub struct JacobianRankAnalyzer {
    /// Jacobian matrix (rows = observations, cols = parameters).
    jacobian: Vec<Vec<f64>>,
    /// Number of observations.
    n_obs: usize,
    /// Number of parameters.
    n_params: usize,
    /// Threshold for singular value cutoff.
    sv_threshold: f64,
    /// Parameter names (optional).
    param_names: Vec<String>,
}

impl JacobianRankAnalyzer {
    /// Create from Jacobian matrix.
    pub fn new(jacobian: Vec<Vec<f64>>) -> Self {
        let n_obs = jacobian.len();
        let n_params = if n_obs > 0 { jacobian[0].len() } else { 0 };

        Self {
            jacobian,
            n_obs,
            n_params,
            sv_threshold: 1e-10,
            param_names: (0..n_params).map(|i| format!("p{}", i)).collect(),
        }
    }

    /// Create from gradient samples.
    pub fn from_gradients(gradients: &[Vec<f64>]) -> Self {
        Self::new(gradients.to_vec())
    }

    /// Set singular value threshold.
    pub fn with_threshold(mut self, threshold: f64) -> Self {
        self.sv_threshold = threshold;
        self
    }

    /// Set parameter names.
    pub fn with_names(mut self, names: Vec<String>) -> Self {
        if names.len() == self.n_params {
            self.param_names = names;
        }
        self
    }

    /// Analyze identifiability.
    pub fn analyze(&self) -> IdentifiabilityResult {
        if self.n_obs == 0 || self.n_params == 0 {
            return IdentifiabilityResult {
                n_params: self.n_params,
                rank: 0,
                condition_number: f64::INFINITY,
                non_identifiable: (0..self.n_params).collect(),
                identifiability_ratio: 0.0,
                singular_values: Vec::new(),
                deficiencies: Vec::new(),
                score: 0.0,
            };
        }

        // Compute J^T J
        let jtj = self.compute_jtj();

        // Compute eigenvalues (approximation for singular values squared)
        let eigenvalues = self.power_iteration_all(&jtj, 50);

        // Singular values are sqrt of eigenvalues
        let mut singular_values: Vec<f64> = eigenvalues.iter().map(|e| e.sqrt()).collect();
        singular_values.sort_by(|a, b| b.partial_cmp(a).unwrap());

        // Compute condition number
        let max_sv = singular_values.first().copied().unwrap_or(1.0);
        let min_sv = singular_values.last().copied().unwrap_or(0.0);
        let condition_number = if min_sv > 1e-15 {
            max_sv / min_sv
        } else {
            f64::INFINITY
        };

        // Compute effective rank
        let rank = compute_effective_rank(&singular_values, self.sv_threshold);

        // Find non-identifiable parameters
        let relative_threshold = self.sv_threshold * max_sv;
        let non_identifiable: Vec<usize> = singular_values
            .iter()
            .enumerate()
            .filter(|(_, &sv)| sv < relative_threshold)
            .map(|(i, _)| i)
            .collect();

        // Identifiability ratio
        let identifiability_ratio = rank as f64 / self.n_params as f64;

        // Build deficiency info
        let deficiencies = non_identifiable
            .iter()
            .map(|&idx| RankDeficiency {
                param_index: idx,
                singular_value: singular_values.get(idx).copied().unwrap_or(0.0),
                confounded_with: Vec::new(),
                explanation: format!(
                    "Parameter {} has near-zero singular value",
                    self.param_names[idx]
                ),
            })
            .collect();

        // Compute overall score
        let score = if identifiability_ratio < 1.0 {
            identifiability_ratio * 0.5
        } else if condition_number > 1e6 {
            0.5 + 0.2 / (1.0 + (condition_number / 1e6).log10())
        } else {
            0.7 + 0.3 * (1.0 - (condition_number / 1e6).min(1.0))
        };

        IdentifiabilityResult {
            n_params: self.n_params,
            rank,
            condition_number,
            non_identifiable,
            identifiability_ratio,
            singular_values,
            deficiencies,
            score: score.clamp(0.0, 1.0),
        }
    }

    /// Compute J^T J matrix.
    fn compute_jtj(&self) -> Vec<Vec<f64>> {
        let n = self.n_params;
        let mut jtj = vec![vec![0.0; n]; n];

        for i in 0..n {
            for j in 0..=i {
                let mut sum = 0.0;
                for row in &self.jacobian {
                    if i < row.len() && j < row.len() {
                        sum += row[i] * row[j];
                    }
                }
                jtj[i][j] = sum;
                jtj[j][i] = sum;
            }
        }

        jtj
    }

    /// Power iteration to find all eigenvalues (approximate).
    fn power_iteration_all(&self, matrix: &[Vec<f64>], max_iter: usize) -> Vec<f64> {
        let n = matrix.len();
        let mut eigenvalues = Vec::with_capacity(n);
        let mut working_matrix = matrix.to_vec();

        for _ in 0..n {
            // Find largest eigenvalue of current matrix
            let (eigenvalue, eigenvector) = self.power_iteration_single(&working_matrix, max_iter);
            eigenvalues.push(eigenvalue);

            // Deflate matrix: A = A - λ * v * v^T
            for i in 0..n {
                for j in 0..n {
                    working_matrix[i][j] -= eigenvalue * eigenvector[i] * eigenvector[j];
                }
            }
        }

        eigenvalues
    }

    /// Single power iteration.
    fn power_iteration_single(&self, matrix: &[Vec<f64>], max_iter: usize) -> (f64, Vec<f64>) {
        let n = matrix.len();
        if n == 0 {
            return (0.0, Vec::new());
        }

        // Initial random vector
        let mut v: Vec<f64> = (0..n).map(|i| (i as f64 + 1.0).sin()).collect();

        // Normalize
        let norm: f64 = v.iter().map(|x| x * x).sum::<f64>().sqrt();
        if norm > 1e-15 {
            for x in &mut v {
                *x /= norm;
            }
        }

        let mut eigenvalue = 0.0;

        for _ in 0..max_iter {
            // w = A * v
            let mut w = vec![0.0; n];
            for i in 0..n {
                for j in 0..n {
                    w[i] += matrix[i][j] * v[j];
                }
            }

            // Compute eigenvalue estimate (Rayleigh quotient)
            eigenvalue = 0.0;
            for i in 0..n {
                eigenvalue += v[i] * w[i];
            }

            // Normalize w
            let norm: f64 = w.iter().map(|x| x * x).sum::<f64>().sqrt();
            if norm < 1e-15 {
                break;
            }
            for x in &mut w {
                *x /= norm;
            }

            v = w;
        }

        (eigenvalue.max(0.0), v)
    }

    /// Get Jacobian dimensions.
    pub fn dimensions(&self) -> (usize, usize) {
        (self.n_obs, self.n_params)
    }

    /// Check if Jacobian is overdetermined.
    pub fn is_overdetermined(&self) -> bool {
        self.n_obs >= self.n_params
    }
}

// ============================================================================
// HELPER FUNCTIONS
// ============================================================================

/// Compute effective rank from singular values.
pub fn compute_effective_rank(singular_values: &[f64], threshold: f64) -> usize {
    if singular_values.is_empty() {
        return 0;
    }

    let max_sv = singular_values.iter().cloned().fold(0.0, f64::max);
    let relative_threshold = threshold * max_sv;

    singular_values
        .iter()
        .filter(|&&sv| sv > relative_threshold)
        .count()
}

/// Compute condition number from singular values.
pub fn compute_condition_number(singular_values: &[f64]) -> f64 {
    if singular_values.is_empty() {
        return f64::INFINITY;
    }

    let max_sv = singular_values.iter().cloned().fold(0.0, f64::max);
    let min_sv = singular_values
        .iter()
        .cloned()
        .fold(f64::INFINITY, f64::min);

    if min_sv > 1e-15 {
        max_sv / min_sv
    } else {
        f64::INFINITY
    }
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_full_rank_jacobian() {
        // Full rank: each observation depends on different parameter
        let jacobian = vec![
            vec![1.0, 0.0, 0.0],
            vec![0.0, 1.0, 0.0],
            vec![0.0, 0.0, 1.0],
            vec![1.0, 1.0, 0.0],
            vec![0.0, 1.0, 1.0],
        ];

        let analyzer = JacobianRankAnalyzer::new(jacobian);
        let result = analyzer.analyze();

        assert_eq!(result.n_params, 3);
        assert!(result.rank >= 2); // Should be close to full rank
        assert!(result.condition_number < 1e10);
    }

    #[test]
    fn test_rank_deficient_jacobian() {
        // Rank deficient: columns 0 and 1 are proportional
        let jacobian = vec![
            vec![1.0, 2.0, 0.0],
            vec![2.0, 4.0, 1.0],
            vec![3.0, 6.0, 2.0],
        ];

        let analyzer = JacobianRankAnalyzer::new(jacobian).with_threshold(1e-6);
        let result = analyzer.analyze();

        assert!(result.rank < 3); // Should be rank deficient
        assert!(!result.all_identifiable());
    }

    #[test]
    fn test_empty_jacobian() {
        let jacobian: Vec<Vec<f64>> = Vec::new();
        let analyzer = JacobianRankAnalyzer::new(jacobian);
        let result = analyzer.analyze();

        assert_eq!(result.rank, 0);
        assert!(result.condition_number.is_infinite());
    }

    #[test]
    fn test_identifiability_result_methods() {
        let result = IdentifiabilityResult {
            n_params: 3,
            rank: 2,
            condition_number: 100.0,
            non_identifiable: vec![2],
            identifiability_ratio: 2.0 / 3.0,
            singular_values: vec![10.0, 5.0, 0.001],
            deficiencies: Vec::new(),
            score: 0.5,
        };

        assert!(!result.all_identifiable());
        assert!(result.is_identifiable(0));
        assert!(result.is_identifiable(1));
        assert!(!result.is_identifiable(2));
        assert_eq!(result.n_non_identifiable(), 1);
    }

    #[test]
    fn test_compute_effective_rank() {
        let svs = vec![10.0, 5.0, 1.0, 0.001, 0.0001];
        let rank = compute_effective_rank(&svs, 1e-3);
        assert_eq!(rank, 3); // Only first 3 are above threshold
    }

    #[test]
    fn test_compute_condition_number() {
        let svs = vec![10.0, 5.0, 1.0];
        let kappa = compute_condition_number(&svs);
        assert!((kappa - 10.0).abs() < 0.01);
    }

    #[test]
    fn test_analyzer_with_names() {
        let jacobian = vec![vec![1.0, 0.0], vec![0.0, 1.0]];
        let analyzer = JacobianRankAnalyzer::new(jacobian)
            .with_names(vec!["ior".to_string(), "roughness".to_string()]);

        assert_eq!(analyzer.param_names[0], "ior");
    }

    #[test]
    fn test_overdetermined() {
        let jacobian = vec![vec![1.0, 0.0], vec![0.0, 1.0], vec![1.0, 1.0]];
        let analyzer = JacobianRankAnalyzer::new(jacobian);

        assert!(analyzer.is_overdetermined());
        assert_eq!(analyzer.dimensions(), (3, 2));
    }
}
