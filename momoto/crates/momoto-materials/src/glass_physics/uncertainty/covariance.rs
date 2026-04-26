//! # Parameter Covariance Matrix
//!
//! Covariance estimation and management for material parameters.

// ============================================================================
// PARAMETER COVARIANCE MATRIX
// ============================================================================

/// Parameter covariance matrix with efficient storage.
///
/// Uses lower triangular storage for symmetric matrix.
#[derive(Debug, Clone)]
pub struct ParameterCovarianceMatrix {
    /// Number of parameters.
    pub n: usize,
    /// Lower triangular elements (row-major).
    data: Vec<f64>,
    /// Parameter names (optional).
    pub param_names: Vec<String>,
}

impl ParameterCovarianceMatrix {
    /// Create zero matrix.
    pub fn zeros(n: usize) -> Self {
        let size = n * (n + 1) / 2;
        Self {
            n,
            data: vec![0.0; size],
            param_names: (0..n).map(|i| format!("p{}", i)).collect(),
        }
    }

    /// Create identity matrix (unit variance, no correlation).
    pub fn identity(n: usize) -> Self {
        let mut mat = Self::zeros(n);
        for i in 0..n {
            mat.set(i, i, 1.0);
        }
        mat
    }

    /// Create diagonal matrix.
    pub fn diagonal(variances: &[f64]) -> Self {
        let n = variances.len();
        let mut mat = Self::zeros(n);
        for (i, &v) in variances.iter().enumerate() {
            mat.set(i, i, v);
        }
        mat
    }

    /// Create from full matrix (takes lower triangle).
    pub fn from_full(full: &[Vec<f64>]) -> Self {
        let n = full.len();
        let mut mat = Self::zeros(n);
        for i in 0..n {
            for j in 0..=i {
                if j < full[i].len() {
                    mat.set(i, j, full[i][j]);
                }
            }
        }
        mat
    }

    /// Create from samples (columns are parameters, rows are samples).
    pub fn from_samples(samples: &[Vec<f64>]) -> Self {
        if samples.is_empty() || samples[0].is_empty() {
            return Self::zeros(0);
        }

        let n_samples = samples.len();
        let n_params = samples[0].len();

        // Compute means
        let means: Vec<f64> = (0..n_params)
            .map(|j| samples.iter().map(|s| s[j]).sum::<f64>() / n_samples as f64)
            .collect();

        // Compute covariance
        let mut mat = Self::zeros(n_params);

        for i in 0..n_params {
            for j in 0..=i {
                let cov: f64 = samples
                    .iter()
                    .map(|s| (s[i] - means[i]) * (s[j] - means[j]))
                    .sum::<f64>()
                    / (n_samples - 1) as f64;
                mat.set(i, j, cov);
            }
        }

        mat
    }

    /// Set parameter names.
    pub fn with_names(mut self, names: Vec<String>) -> Self {
        if names.len() == self.n {
            self.param_names = names;
        }
        self
    }

    /// Get index in lower triangular storage.
    fn idx(&self, row: usize, col: usize) -> usize {
        let (i, j) = if row >= col { (row, col) } else { (col, row) };
        i * (i + 1) / 2 + j
    }

    /// Get element.
    pub fn get(&self, row: usize, col: usize) -> f64 {
        if row >= self.n || col >= self.n {
            return 0.0;
        }
        self.data[self.idx(row, col)]
    }

    /// Set element.
    pub fn set(&mut self, row: usize, col: usize, value: f64) {
        if row >= self.n || col >= self.n {
            return;
        }
        let idx = self.idx(row, col);
        self.data[idx] = value;
    }

    /// Get variance (diagonal element).
    pub fn variance(&self, i: usize) -> f64 {
        self.get(i, i)
    }

    /// Get standard deviation.
    pub fn std_dev(&self, i: usize) -> f64 {
        self.variance(i).sqrt()
    }

    /// Get correlation between parameters.
    pub fn correlation(&self, i: usize, j: usize) -> f64 {
        let cov = self.get(i, j);
        let var_i = self.variance(i);
        let var_j = self.variance(j);

        if var_i < 1e-10 || var_j < 1e-10 {
            return 0.0;
        }

        cov / (var_i.sqrt() * var_j.sqrt())
    }

    /// Get all variances.
    pub fn variances(&self) -> Vec<f64> {
        (0..self.n).map(|i| self.variance(i)).collect()
    }

    /// Get all standard deviations.
    pub fn std_devs(&self) -> Vec<f64> {
        (0..self.n).map(|i| self.std_dev(i)).collect()
    }

    /// Get correlation matrix.
    pub fn correlation_matrix(&self) -> Vec<Vec<f64>> {
        let mut corr = vec![vec![0.0; self.n]; self.n];
        for i in 0..self.n {
            for j in 0..self.n {
                corr[i][j] = self.correlation(i, j);
            }
        }
        corr
    }

    /// Find highly correlated pairs (|r| > threshold).
    pub fn find_correlated_pairs(&self, threshold: f64) -> Vec<(usize, usize, f64)> {
        let mut pairs = Vec::new();
        for i in 0..self.n {
            for j in 0..i {
                let r = self.correlation(i, j);
                if r.abs() > threshold {
                    pairs.push((i, j, r));
                }
            }
        }
        pairs.sort_by(|a, b| b.2.abs().partial_cmp(&a.2.abs()).unwrap());
        pairs
    }

    /// Compute determinant (uses Cholesky).
    pub fn determinant(&self) -> f64 {
        let chol = self.cholesky();
        if chol.is_none() {
            return 0.0;
        }
        let l = chol.unwrap();
        let mut det = 1.0;
        for i in 0..self.n {
            det *= l.get(i, i);
        }
        det * det
    }

    /// Compute Cholesky decomposition (returns lower triangular L where Σ = LL').
    pub fn cholesky(&self) -> Option<ParameterCovarianceMatrix> {
        let mut l = ParameterCovarianceMatrix::zeros(self.n);

        for i in 0..self.n {
            for j in 0..=i {
                let mut sum = self.get(i, j);

                for k in 0..j {
                    sum -= l.get(i, k) * l.get(j, k);
                }

                if i == j {
                    if sum <= 0.0 {
                        return None; // Not positive definite
                    }
                    l.set(i, j, sum.sqrt());
                } else {
                    l.set(i, j, sum / l.get(j, j));
                }
            }
        }

        Some(l)
    }

    /// Add regularization to ensure positive definiteness.
    pub fn regularize(&mut self, lambda: f64) {
        for i in 0..self.n {
            let current = self.variance(i);
            self.set(i, i, current + lambda);
        }
    }

    /// Scale by a factor.
    pub fn scale(&mut self, factor: f64) {
        for v in &mut self.data {
            *v *= factor;
        }
    }

    /// Get trace (sum of variances).
    pub fn trace(&self) -> f64 {
        (0..self.n).map(|i| self.variance(i)).sum()
    }

    /// Convert to full matrix.
    pub fn to_full(&self) -> Vec<Vec<f64>> {
        let mut full = vec![vec![0.0; self.n]; self.n];
        for i in 0..self.n {
            for j in 0..self.n {
                full[i][j] = self.get(i, j);
            }
        }
        full
    }

    /// Memory size in bytes.
    pub fn memory_size(&self) -> usize {
        self.data.len() * 8 + self.param_names.iter().map(|s| s.len()).sum::<usize>() + 32
    }
}

// ============================================================================
// COVARIANCE ESTIMATOR
// ============================================================================

/// Estimator for incremental covariance computation.
#[derive(Debug, Clone)]
pub struct CovarianceEstimator {
    /// Running sum of samples.
    sum: Vec<f64>,
    /// Running sum of products.
    sum_sq: Vec<Vec<f64>>,
    /// Sample count.
    count: usize,
    /// Number of parameters.
    n: usize,
}

impl CovarianceEstimator {
    /// Create new estimator.
    pub fn new(n: usize) -> Self {
        Self {
            sum: vec![0.0; n],
            sum_sq: vec![vec![0.0; n]; n],
            count: 0,
            n,
        }
    }

    /// Add a sample.
    pub fn add(&mut self, sample: &[f64]) {
        if sample.len() != self.n {
            return;
        }

        for i in 0..self.n {
            self.sum[i] += sample[i];
            for j in 0..=i {
                self.sum_sq[i][j] += sample[i] * sample[j];
            }
        }
        self.count += 1;
    }

    /// Get current covariance estimate.
    pub fn estimate(&self) -> ParameterCovarianceMatrix {
        if self.count < 2 {
            return ParameterCovarianceMatrix::identity(self.n);
        }

        let n = self.count as f64;
        let mut mat = ParameterCovarianceMatrix::zeros(self.n);

        for i in 0..self.n {
            let mean_i = self.sum[i] / n;
            for j in 0..=i {
                let mean_j = self.sum[j] / n;
                let cov = self.sum_sq[i][j] / n - mean_i * mean_j;
                // Bessel's correction
                let cov_corrected = cov * n / (n - 1.0);
                mat.set(i, j, cov_corrected);
            }
        }

        mat
    }

    /// Get current means.
    pub fn means(&self) -> Vec<f64> {
        if self.count == 0 {
            return vec![0.0; self.n];
        }
        self.sum.iter().map(|s| s / self.count as f64).collect()
    }

    /// Get sample count.
    pub fn sample_count(&self) -> usize {
        self.count
    }

    /// Reset estimator.
    pub fn reset(&mut self) {
        self.sum = vec![0.0; self.n];
        self.sum_sq = vec![vec![0.0; self.n]; self.n];
        self.count = 0;
    }
}

// ============================================================================
// HELPER FUNCTIONS
// ============================================================================

/// Estimate covariance from samples with optional shrinkage.
pub fn estimate_covariance(
    samples: &[Vec<f64>],
    shrinkage: Option<f64>,
) -> ParameterCovarianceMatrix {
    let mut cov = ParameterCovarianceMatrix::from_samples(samples);

    if let Some(alpha) = shrinkage {
        cov = shrinkage_covariance(&cov, alpha);
    }

    cov
}

/// Apply Ledoit-Wolf shrinkage to covariance matrix.
///
/// Shrinks towards scaled identity matrix.
pub fn shrinkage_covariance(
    cov: &ParameterCovarianceMatrix,
    alpha: f64,
) -> ParameterCovarianceMatrix {
    let alpha = alpha.clamp(0.0, 1.0);
    let n = cov.n;

    // Target: scaled identity with average variance
    let avg_var = cov.trace() / n as f64;

    let mut shrunk = ParameterCovarianceMatrix::zeros(n);
    shrunk.param_names = cov.param_names.clone();

    for i in 0..n {
        for j in 0..=i {
            let original = cov.get(i, j);
            let target = if i == j { avg_var } else { 0.0 };
            let value = (1.0 - alpha) * original + alpha * target;
            shrunk.set(i, j, value);
        }
    }

    shrunk
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_covariance_zeros() {
        let cov = ParameterCovarianceMatrix::zeros(3);
        assert_eq!(cov.n, 3);
        assert!((cov.get(0, 0) - 0.0).abs() < 1e-10);
    }

    #[test]
    fn test_covariance_identity() {
        let cov = ParameterCovarianceMatrix::identity(3);
        assert!((cov.variance(0) - 1.0).abs() < 1e-10);
        assert!((cov.get(0, 1) - 0.0).abs() < 1e-10);
    }

    #[test]
    fn test_covariance_diagonal() {
        let cov = ParameterCovarianceMatrix::diagonal(&[1.0, 4.0, 9.0]);
        assert!((cov.std_dev(0) - 1.0).abs() < 1e-10);
        assert!((cov.std_dev(1) - 2.0).abs() < 1e-10);
        assert!((cov.std_dev(2) - 3.0).abs() < 1e-10);
    }

    #[test]
    fn test_covariance_from_samples() {
        // Samples with known covariance
        let samples = vec![vec![1.0, 2.0], vec![3.0, 4.0], vec![5.0, 6.0]];
        let cov = ParameterCovarianceMatrix::from_samples(&samples);

        // Variance should be positive
        assert!(cov.variance(0) > 0.0);
        assert!(cov.variance(1) > 0.0);

        // Perfect correlation expected
        let r = cov.correlation(0, 1);
        assert!((r - 1.0).abs() < 0.01);
    }

    #[test]
    fn test_covariance_correlation() {
        // Anti-correlated samples
        let samples = vec![vec![0.0, 1.0], vec![1.0, 0.0]];
        let cov = ParameterCovarianceMatrix::from_samples(&samples);
        let r = cov.correlation(0, 1);
        assert!(r < 0.0); // Negative correlation
    }

    #[test]
    fn test_covariance_symmetry() {
        let cov =
            ParameterCovarianceMatrix::from_samples(&[vec![1.0, 2.0, 3.0], vec![4.0, 5.0, 6.0]]);

        assert!((cov.get(1, 0) - cov.get(0, 1)).abs() < 1e-10);
        assert!((cov.get(2, 0) - cov.get(0, 2)).abs() < 1e-10);
    }

    #[test]
    fn test_covariance_find_correlated() {
        let samples: Vec<Vec<f64>> = (0..100)
            .map(|i| vec![i as f64, i as f64 * 2.0, i as f64 % 3.0])
            .collect();
        let cov = ParameterCovarianceMatrix::from_samples(&samples);

        let pairs = cov.find_correlated_pairs(0.9);
        // Parameters 0 and 1 should be highly correlated
        assert!(pairs
            .iter()
            .any(|(i, j, _)| (*i == 1 && *j == 0) || (*i == 0 && *j == 1)));
    }

    #[test]
    fn test_covariance_cholesky() {
        let cov = ParameterCovarianceMatrix::diagonal(&[4.0, 9.0, 16.0]);
        let chol = cov.cholesky();
        assert!(chol.is_some());

        let l = chol.unwrap();
        assert!((l.get(0, 0) - 2.0).abs() < 1e-10);
        assert!((l.get(1, 1) - 3.0).abs() < 1e-10);
        assert!((l.get(2, 2) - 4.0).abs() < 1e-10);
    }

    #[test]
    fn test_covariance_determinant() {
        let cov = ParameterCovarianceMatrix::diagonal(&[2.0, 3.0]);
        let det = cov.determinant();
        assert!((det - 6.0).abs() < 1e-10);
    }

    #[test]
    fn test_covariance_estimator() {
        let mut est = CovarianceEstimator::new(2);

        est.add(&[1.0, 2.0]);
        est.add(&[3.0, 4.0]);
        est.add(&[5.0, 6.0]);

        let cov = est.estimate();
        assert!(cov.variance(0) > 0.0);
        assert_eq!(est.sample_count(), 3);
    }

    #[test]
    fn test_shrinkage_covariance() {
        let cov = ParameterCovarianceMatrix::from_samples(&[
            vec![1.0, 2.0],
            vec![3.0, 4.0],
            vec![5.0, 6.0],
        ]);

        // Full shrinkage should give identity-like matrix
        let shrunk = shrinkage_covariance(&cov, 1.0);
        assert!((shrunk.correlation(0, 1) - 0.0).abs() < 0.01);

        // No shrinkage should preserve original
        let no_shrink = shrinkage_covariance(&cov, 0.0);
        assert!((no_shrink.correlation(0, 1) - cov.correlation(0, 1)).abs() < 0.01);
    }

    #[test]
    fn test_covariance_regularize() {
        let mut cov = ParameterCovarianceMatrix::diagonal(&[0.0, 0.0]);
        cov.regularize(1e-6);
        assert!(cov.variance(0) > 0.0);

        // Should be positive definite after regularization
        assert!(cov.cholesky().is_some());
    }
}
