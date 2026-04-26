//! # Fisher Information Matrix
//!
//! Fisher Information estimation for Cramer-Rao bounds.

use super::covariance::ParameterCovarianceMatrix;

// ============================================================================
// FISHER INFORMATION MATRIX
// ============================================================================

/// Fisher Information matrix for parameter uncertainty.
///
/// The Fisher Information provides a lower bound on the variance
/// of any unbiased estimator (Cramer-Rao bound).
#[derive(Debug, Clone)]
pub struct FisherInformationMatrix {
    /// Matrix data (symmetric, stored as full for simplicity).
    data: Vec<Vec<f64>>,
    /// Number of parameters.
    pub n: usize,
    /// Parameter names.
    pub param_names: Vec<String>,
    /// Observation count used for estimation.
    pub n_observations: usize,
}

impl FisherInformationMatrix {
    /// Create zero matrix.
    pub fn zeros(n: usize) -> Self {
        Self {
            data: vec![vec![0.0; n]; n],
            n,
            param_names: (0..n).map(|i| format!("p{}", i)).collect(),
            n_observations: 0,
        }
    }

    /// Create from gradient outer products.
    ///
    /// Fisher Information ≈ E[∇L ∇L^T] where L is log-likelihood.
    /// For least squares: I ≈ J^T J / σ² where J is Jacobian.
    pub fn from_gradients(gradients: &[Vec<f64>], noise_variance: f64) -> Self {
        if gradients.is_empty() {
            return Self::zeros(0);
        }

        let n = gradients[0].len();
        let n_obs = gradients.len();
        let mut fisher = Self::zeros(n);
        fisher.n_observations = n_obs;

        // I = Σ (g_i g_i^T) / σ²
        for g in gradients {
            for i in 0..n {
                for j in 0..n {
                    fisher.data[i][j] += g[i] * g[j];
                }
            }
        }

        // Scale by inverse noise variance
        let scale = 1.0 / noise_variance.max(1e-10);
        for row in &mut fisher.data {
            for val in row.iter_mut() {
                *val *= scale;
            }
        }

        fisher
    }

    /// Create from Jacobian matrix.
    ///
    /// J is (n_obs x n_params), Fisher = J^T J / σ²
    pub fn from_jacobian(jacobian: &[Vec<f64>], noise_variance: f64) -> Self {
        if jacobian.is_empty() {
            return Self::zeros(0);
        }

        let n_obs = jacobian.len();
        let n_params = jacobian[0].len();
        let mut fisher = Self::zeros(n_params);
        fisher.n_observations = n_obs;

        // I = J^T J / σ²
        for i in 0..n_params {
            for j in 0..n_params {
                let mut sum = 0.0;
                for row in jacobian {
                    sum += row[i] * row[j];
                }
                fisher.data[i][j] = sum / noise_variance.max(1e-10);
            }
        }

        fisher
    }

    /// Set parameter names.
    pub fn with_names(mut self, names: Vec<String>) -> Self {
        if names.len() == self.n {
            self.param_names = names;
        }
        self
    }

    /// Get element.
    pub fn get(&self, i: usize, j: usize) -> f64 {
        if i >= self.n || j >= self.n {
            return 0.0;
        }
        self.data[i][j]
    }

    /// Set element (maintains symmetry).
    pub fn set(&mut self, i: usize, j: usize, value: f64) {
        if i >= self.n || j >= self.n {
            return;
        }
        self.data[i][j] = value;
        self.data[j][i] = value;
    }

    /// Get diagonal element (expected information for parameter i).
    pub fn diagonal(&self, i: usize) -> f64 {
        self.get(i, i)
    }

    /// Get all diagonal elements.
    pub fn diagonals(&self) -> Vec<f64> {
        (0..self.n).map(|i| self.diagonal(i)).collect()
    }

    /// Compute trace (total information).
    pub fn trace(&self) -> f64 {
        (0..self.n).map(|i| self.diagonal(i)).sum()
    }

    /// Compute determinant.
    pub fn determinant(&self) -> f64 {
        // For small matrices, use direct computation
        if self.n == 1 {
            return self.data[0][0];
        }
        if self.n == 2 {
            return self.data[0][0] * self.data[1][1] - self.data[0][1] * self.data[1][0];
        }

        // LU decomposition for larger matrices
        let lu = self.lu_decomposition();
        if lu.is_none() {
            return 0.0;
        }
        let (_l, u, _) = lu.unwrap();
        let mut det = 1.0;
        for i in 0..self.n {
            det *= u[i][i];
        }
        det
    }

    /// LU decomposition (returns L, U, permutation).
    fn lu_decomposition(&self) -> Option<(Vec<Vec<f64>>, Vec<Vec<f64>>, Vec<usize>)> {
        let n = self.n;
        let mut l = vec![vec![0.0; n]; n];
        let mut u = self.data.clone();
        let mut perm: Vec<usize> = (0..n).collect();

        for i in 0..n {
            l[i][i] = 1.0;

            // Find pivot
            let mut max_val = u[i][i].abs();
            let mut max_row = i;
            for k in (i + 1)..n {
                if u[k][i].abs() > max_val {
                    max_val = u[k][i].abs();
                    max_row = k;
                }
            }

            if max_val < 1e-15 {
                return None; // Singular
            }

            // Swap rows
            if max_row != i {
                u.swap(i, max_row);
                perm.swap(i, max_row);
            }

            // Eliminate
            for k in (i + 1)..n {
                let factor = u[k][i] / u[i][i];
                l[k][i] = factor;
                for j in i..n {
                    u[k][j] -= factor * u[i][j];
                }
            }
        }

        Some((l, u, perm))
    }

    /// Invert the Fisher Information to get covariance matrix.
    ///
    /// This gives the Cramer-Rao lower bound on parameter variance.
    pub fn invert(&self) -> Option<ParameterCovarianceMatrix> {
        if self.n == 0 {
            return None;
        }

        // Use Cholesky for symmetric positive definite matrix
        let chol = self.cholesky();
        if chol.is_none() {
            // Fallback to regularized pseudo-inverse
            return self.regularized_inverse(1e-6);
        }

        let l = chol.unwrap();
        let l_inv = Self::invert_lower_triangular(&l);

        // Σ = L^(-T) L^(-1)
        let mut cov = ParameterCovarianceMatrix::zeros(self.n);
        cov.param_names = self.param_names.clone();

        for i in 0..self.n {
            for j in 0..=i {
                let mut sum = 0.0;
                for k in 0..self.n {
                    sum += l_inv[k][i] * l_inv[k][j];
                }
                cov.set(i, j, sum);
            }
        }

        Some(cov)
    }

    /// Cholesky decomposition.
    fn cholesky(&self) -> Option<Vec<Vec<f64>>> {
        let n = self.n;
        let mut l = vec![vec![0.0; n]; n];

        for i in 0..n {
            for j in 0..=i {
                let mut sum = self.data[i][j];

                for k in 0..j {
                    sum -= l[i][k] * l[j][k];
                }

                if i == j {
                    if sum <= 0.0 {
                        return None;
                    }
                    l[i][j] = sum.sqrt();
                } else {
                    if l[j][j].abs() < 1e-15 {
                        return None;
                    }
                    l[i][j] = sum / l[j][j];
                }
            }
        }

        Some(l)
    }

    /// Invert lower triangular matrix.
    fn invert_lower_triangular(l: &[Vec<f64>]) -> Vec<Vec<f64>> {
        let n = l.len();
        let mut inv = vec![vec![0.0; n]; n];

        for i in 0..n {
            inv[i][i] = 1.0 / l[i][i];
            for j in (i + 1)..n {
                let mut sum = 0.0;
                for k in i..j {
                    sum -= l[j][k] * inv[k][i];
                }
                inv[j][i] = sum / l[j][j];
            }
        }

        inv
    }

    /// Regularized pseudo-inverse for ill-conditioned matrices.
    fn regularized_inverse(&self, lambda: f64) -> Option<ParameterCovarianceMatrix> {
        let mut regularized = self.clone();
        for i in 0..self.n {
            regularized.data[i][i] += lambda;
        }
        regularized.invert()
    }

    /// Add regularization for numerical stability.
    pub fn regularize(&mut self, lambda: f64) {
        for i in 0..self.n {
            self.data[i][i] += lambda;
        }
    }

    /// Scale by a factor.
    pub fn scale(&mut self, factor: f64) {
        for row in &mut self.data {
            for val in row.iter_mut() {
                *val *= factor;
            }
        }
    }

    /// Add another Fisher matrix.
    pub fn add(&mut self, other: &FisherInformationMatrix) {
        if other.n != self.n {
            return;
        }
        for i in 0..self.n {
            for j in 0..self.n {
                self.data[i][j] += other.data[i][j];
            }
        }
        self.n_observations += other.n_observations;
    }
}

// ============================================================================
// FISHER INFORMATION ESTIMATOR
// ============================================================================

/// Incremental estimator for Fisher Information.
#[derive(Debug, Clone)]
pub struct FisherInformationEstimator {
    /// Accumulated Fisher matrix.
    fisher: FisherInformationMatrix,
    /// Assumed noise variance.
    noise_variance: f64,
    /// Running gradient mean for bias correction.
    grad_mean: Vec<f64>,
    /// Sample count.
    count: usize,
}

impl FisherInformationEstimator {
    /// Create new estimator.
    pub fn new(n_params: usize, noise_variance: f64) -> Self {
        Self {
            fisher: FisherInformationMatrix::zeros(n_params),
            noise_variance,
            grad_mean: vec![0.0; n_params],
            count: 0,
        }
    }

    /// Add a gradient observation.
    pub fn add_gradient(&mut self, gradient: &[f64]) {
        let n = self.fisher.n;
        if gradient.len() != n {
            return;
        }

        // Update running mean
        self.count += 1;
        for i in 0..n {
            self.grad_mean[i] += (gradient[i] - self.grad_mean[i]) / self.count as f64;
        }

        // Add outer product
        for i in 0..n {
            for j in 0..n {
                let current = self.fisher.get(i, j);
                self.fisher.set(
                    i,
                    j,
                    current + gradient[i] * gradient[j] / self.noise_variance,
                );
            }
        }
        self.fisher.n_observations += 1;
    }

    /// Get current Fisher estimate.
    pub fn estimate(&self) -> FisherInformationMatrix {
        self.fisher.clone()
    }

    /// Get Cramer-Rao bounds.
    pub fn cramer_rao_bounds(&self) -> Option<Vec<f64>> {
        let cov = self.fisher.invert()?;
        Some(cov.std_devs())
    }

    /// Get sample count.
    pub fn sample_count(&self) -> usize {
        self.count
    }

    /// Reset estimator.
    pub fn reset(&mut self) {
        let n = self.fisher.n;
        self.fisher = FisherInformationMatrix::zeros(n);
        self.grad_mean = vec![0.0; n];
        self.count = 0;
    }
}

// ============================================================================
// HELPER FUNCTIONS
// ============================================================================

/// Compute Cramer-Rao lower bounds from Fisher Information.
pub fn cramer_rao_bounds(fisher: &FisherInformationMatrix) -> Option<Vec<f64>> {
    let cov = fisher.invert()?;
    Some(cov.std_devs())
}

/// Compute expected Fisher diagonal elements.
///
/// For linear model: I_ii = Σ (∂f/∂θ_i)² / σ²
pub fn expected_fisher_diagonal(gradients: &[Vec<f64>], noise_variance: f64) -> Vec<f64> {
    if gradients.is_empty() {
        return Vec::new();
    }

    let n = gradients[0].len();
    let mut diag = vec![0.0; n];

    for g in gradients {
        for (i, d) in diag.iter_mut().enumerate() {
            *d += g[i] * g[i];
        }
    }

    for d in &mut diag {
        *d /= noise_variance.max(1e-10);
    }

    diag
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fisher_zeros() {
        let fisher = FisherInformationMatrix::zeros(3);
        assert_eq!(fisher.n, 3);
        assert!((fisher.get(0, 0) - 0.0).abs() < 1e-10);
    }

    #[test]
    fn test_fisher_from_gradients() {
        let gradients = vec![vec![1.0, 0.0], vec![0.0, 1.0], vec![1.0, 1.0]];
        let fisher = FisherInformationMatrix::from_gradients(&gradients, 1.0);

        assert!((fisher.get(0, 0) - 2.0).abs() < 1e-10); // 1² + 0² + 1²
        assert!((fisher.get(1, 1) - 2.0).abs() < 1e-10); // 0² + 1² + 1²
    }

    #[test]
    fn test_fisher_symmetry() {
        let gradients = vec![vec![1.0, 2.0, 3.0], vec![4.0, 5.0, 6.0]];
        let fisher = FisherInformationMatrix::from_gradients(&gradients, 1.0);

        assert!((fisher.get(0, 1) - fisher.get(1, 0)).abs() < 1e-10);
        assert!((fisher.get(0, 2) - fisher.get(2, 0)).abs() < 1e-10);
    }

    #[test]
    fn test_fisher_invert_diagonal() {
        let mut fisher = FisherInformationMatrix::zeros(2);
        fisher.set(0, 0, 4.0);
        fisher.set(1, 1, 9.0);

        let cov = fisher.invert();
        assert!(cov.is_some());

        let c = cov.unwrap();
        assert!((c.variance(0) - 0.25).abs() < 1e-10); // 1/4
        assert!((c.variance(1) - 1.0 / 9.0).abs() < 1e-10);
    }

    #[test]
    fn test_cramer_rao_bounds() {
        let mut fisher = FisherInformationMatrix::zeros(2);
        fisher.set(0, 0, 100.0);
        fisher.set(1, 1, 25.0);

        let bounds = cramer_rao_bounds(&fisher);
        assert!(bounds.is_some());

        let b = bounds.unwrap();
        assert!((b[0] - 0.1).abs() < 1e-10); // sqrt(1/100)
        assert!((b[1] - 0.2).abs() < 1e-10); // sqrt(1/25)
    }

    #[test]
    fn test_fisher_estimator() {
        let mut est = FisherInformationEstimator::new(2, 1.0);

        est.add_gradient(&[1.0, 0.0]);
        est.add_gradient(&[0.0, 1.0]);
        est.add_gradient(&[1.0, 1.0]);

        let fisher = est.estimate();
        assert!((fisher.diagonal(0) - 2.0).abs() < 1e-10);
        assert_eq!(est.sample_count(), 3);
    }

    #[test]
    fn test_fisher_determinant() {
        let mut fisher = FisherInformationMatrix::zeros(2);
        fisher.set(0, 0, 2.0);
        fisher.set(1, 1, 3.0);

        let det = fisher.determinant();
        assert!((det - 6.0).abs() < 1e-10);
    }

    #[test]
    fn test_fisher_regularize() {
        let mut fisher = FisherInformationMatrix::zeros(2);
        fisher.regularize(1.0);

        assert!((fisher.diagonal(0) - 1.0).abs() < 1e-10);
        assert!(fisher.invert().is_some());
    }

    #[test]
    fn test_fisher_add() {
        let mut f1 = FisherInformationMatrix::zeros(2);
        f1.set(0, 0, 1.0);

        let mut f2 = FisherInformationMatrix::zeros(2);
        f2.set(0, 0, 2.0);

        f1.add(&f2);
        assert!((f1.diagonal(0) - 3.0).abs() < 1e-10);
    }

    #[test]
    fn test_expected_fisher_diagonal() {
        let gradients = vec![vec![2.0, 3.0], vec![2.0, 3.0]];
        let diag = expected_fisher_diagonal(&gradients, 1.0);

        assert!((diag[0] - 8.0).abs() < 1e-10); // 2² + 2²
        assert!((diag[1] - 18.0).abs() < 1e-10); // 3² + 3²
    }
}
