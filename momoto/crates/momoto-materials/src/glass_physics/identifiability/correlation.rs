//! # Parameter Correlation Analysis
//!
//! Identify correlated and redundant parameters.

// ============================================================================
// CORRELATION CLUSTER
// ============================================================================

/// A cluster of correlated parameters.
#[derive(Debug, Clone)]
pub struct CorrelationCluster {
    /// Indices of parameters in this cluster.
    pub members: Vec<usize>,
    /// Average pairwise correlation within cluster.
    pub avg_correlation: f64,
    /// Strongest correlation in cluster.
    pub max_correlation: f64,
    /// Representative parameter (lowest index).
    pub representative: usize,
}

impl CorrelationCluster {
    /// Get cluster size.
    pub fn size(&self) -> usize {
        self.members.len()
    }

    /// Check if cluster is trivial (single element).
    pub fn is_trivial(&self) -> bool {
        self.members.len() <= 1
    }

    /// Get non-representative members (candidates for freezing).
    pub fn non_representatives(&self) -> Vec<usize> {
        self.members
            .iter()
            .filter(|&&m| m != self.representative)
            .copied()
            .collect()
    }
}

// ============================================================================
// PARAMETER CORRELATION MATRIX
// ============================================================================

/// Correlation matrix for parameters.
#[derive(Debug, Clone)]
pub struct ParameterCorrelationMatrix {
    /// Full correlation matrix.
    data: Vec<Vec<f64>>,
    /// Number of parameters.
    pub n: usize,
    /// Parameter names.
    pub param_names: Vec<String>,
}

impl ParameterCorrelationMatrix {
    /// Create from covariance matrix.
    pub fn from_covariance(covariance: &[Vec<f64>]) -> Self {
        let n = covariance.len();
        let mut data = vec![vec![0.0; n]; n];

        // Compute correlation: r_ij = cov_ij / (σ_i * σ_j)
        for i in 0..n {
            let sigma_i = covariance[i][i].sqrt();
            for j in 0..n {
                let sigma_j = covariance[j][j].sqrt();
                if sigma_i > 1e-15 && sigma_j > 1e-15 {
                    data[i][j] = covariance[i][j] / (sigma_i * sigma_j);
                } else if i == j {
                    data[i][j] = 1.0;
                }
            }
        }

        Self {
            data,
            n,
            param_names: (0..n).map(|i| format!("p{}", i)).collect(),
        }
    }

    /// Create from raw correlation values.
    pub fn from_raw(data: Vec<Vec<f64>>) -> Self {
        let n = data.len();
        Self {
            data,
            n,
            param_names: (0..n).map(|i| format!("p{}", i)).collect(),
        }
    }

    /// Set parameter names.
    pub fn with_names(mut self, names: Vec<String>) -> Self {
        if names.len() == self.n {
            self.param_names = names;
        }
        self
    }

    /// Get correlation between parameters.
    pub fn get(&self, i: usize, j: usize) -> f64 {
        if i < self.n && j < self.n {
            self.data[i][j]
        } else {
            0.0
        }
    }

    /// Find pairs above correlation threshold.
    pub fn find_high_correlations(&self, threshold: f64) -> Vec<(usize, usize, f64)> {
        let mut pairs = Vec::new();
        for i in 0..self.n {
            for j in (i + 1)..self.n {
                let r = self.data[i][j].abs();
                if r > threshold {
                    pairs.push((i, j, self.data[i][j]));
                }
            }
        }
        pairs.sort_by(|a, b| b.2.abs().partial_cmp(&a.2.abs()).unwrap());
        pairs
    }

    /// Get row as vector.
    pub fn row(&self, i: usize) -> &[f64] {
        &self.data[i]
    }

    /// Get maximum off-diagonal correlation.
    pub fn max_off_diagonal(&self) -> f64 {
        let mut max: f64 = 0.0;
        for i in 0..self.n {
            for j in (i + 1)..self.n {
                max = max.max(self.data[i][j].abs());
            }
        }
        max
    }

    /// Get average off-diagonal correlation.
    pub fn avg_off_diagonal(&self) -> f64 {
        if self.n < 2 {
            return 0.0;
        }
        let mut sum = 0.0;
        let mut count = 0;
        for i in 0..self.n {
            for j in (i + 1)..self.n {
                sum += self.data[i][j].abs();
                count += 1;
            }
        }
        sum / count as f64
    }
}

// ============================================================================
// CORRELATION ANALYSIS
// ============================================================================

/// Result of correlation analysis.
#[derive(Debug, Clone)]
pub struct CorrelationAnalysis {
    /// Parameter correlation matrix.
    pub correlation_matrix: ParameterCorrelationMatrix,
    /// Highly correlated pairs.
    pub high_correlation_pairs: Vec<(usize, usize, f64)>,
    /// Correlation clusters.
    pub clusters: Vec<CorrelationCluster>,
    /// Variance Inflation Factors.
    pub vif: Vec<f64>,
    /// Overall multicollinearity score (0 = none, 1 = severe).
    pub multicollinearity_score: f64,
}

impl CorrelationAnalysis {
    /// Create from correlation matrix.
    pub fn from_correlation_matrix(matrix: ParameterCorrelationMatrix, threshold: f64) -> Self {
        let high_pairs = matrix.find_high_correlations(threshold);
        let clusters = find_correlation_clusters(&matrix, threshold);
        let vif = compute_vif_from_correlation(&matrix);

        // Compute multicollinearity score
        let max_corr = matrix.max_off_diagonal();
        let max_vif = vif.iter().cloned().fold(1.0, f64::max);

        let corr_component = max_corr.powi(2);
        let vif_component = ((max_vif - 1.0) / 10.0).min(1.0);
        let multicollinearity_score = (corr_component + vif_component) / 2.0;

        Self {
            correlation_matrix: matrix,
            high_correlation_pairs: high_pairs,
            clusters,
            vif,
            multicollinearity_score: multicollinearity_score.clamp(0.0, 1.0),
        }
    }

    /// Check if severe multicollinearity exists.
    pub fn has_severe_multicollinearity(&self) -> bool {
        self.multicollinearity_score > 0.5
    }

    /// Get parameters with high VIF (> 5).
    pub fn high_vif_params(&self) -> Vec<usize> {
        self.vif
            .iter()
            .enumerate()
            .filter(|(_, &v)| v > 5.0)
            .map(|(i, _)| i)
            .collect()
    }

    /// Get number of non-trivial clusters.
    pub fn n_correlated_groups(&self) -> usize {
        self.clusters.iter().filter(|c| !c.is_trivial()).count()
    }

    /// Get summary string.
    pub fn summary(&self) -> String {
        let n_high = self.high_correlation_pairs.len();
        let max_vif = self.vif.iter().cloned().fold(1.0, f64::max);

        format!(
            "{} high correlations, {} groups, max VIF={:.2}, multicollinearity={:.1}%",
            n_high,
            self.n_correlated_groups(),
            max_vif,
            self.multicollinearity_score * 100.0
        )
    }
}

// ============================================================================
// HELPER FUNCTIONS
// ============================================================================

/// Find clusters of correlated parameters using union-find.
pub fn find_correlation_clusters(
    matrix: &ParameterCorrelationMatrix,
    threshold: f64,
) -> Vec<CorrelationCluster> {
    let n = matrix.n;
    if n == 0 {
        return Vec::new();
    }

    // Union-find structure
    let mut parent: Vec<usize> = (0..n).collect();

    fn find(parent: &mut [usize], i: usize) -> usize {
        if parent[i] != i {
            parent[i] = find(parent, parent[i]);
        }
        parent[i]
    }

    fn union(parent: &mut [usize], i: usize, j: usize) {
        let pi = find(parent, i);
        let pj = find(parent, j);
        if pi != pj {
            parent[pi] = pj;
        }
    }

    // Union correlated parameters
    for i in 0..n {
        for j in (i + 1)..n {
            if matrix.get(i, j).abs() > threshold {
                union(&mut parent, i, j);
            }
        }
    }

    // Build clusters
    let mut cluster_map: std::collections::HashMap<usize, Vec<usize>> =
        std::collections::HashMap::new();
    for i in 0..n {
        let root = find(&mut parent, i);
        cluster_map.entry(root).or_insert_with(Vec::new).push(i);
    }

    // Create cluster objects
    let mut clusters = Vec::new();
    for (_, members) in cluster_map {
        if members.is_empty() {
            continue;
        }

        let representative = *members.iter().min().unwrap();

        // Compute average and max correlation within cluster
        let mut sum_corr: f64 = 0.0;
        let mut max_corr: f64 = 0.0;
        let mut count = 0;

        for &i in &members {
            for &j in &members {
                if i < j {
                    let r = matrix.get(i, j).abs();
                    sum_corr += r;
                    max_corr = max_corr.max(r);
                    count += 1;
                }
            }
        }

        let avg_correlation = if count > 0 {
            sum_corr / count as f64
        } else {
            1.0
        };

        clusters.push(CorrelationCluster {
            members,
            avg_correlation,
            max_correlation: max_corr,
            representative,
        });
    }

    // Sort by size (largest first)
    clusters.sort_by(|a, b| b.size().cmp(&a.size()));

    clusters
}

/// Compute Variance Inflation Factor for each parameter.
pub fn compute_vif(covariance: &[Vec<f64>]) -> Vec<f64> {
    let matrix = ParameterCorrelationMatrix::from_covariance(covariance);
    compute_vif_from_correlation(&matrix)
}

/// Compute VIF from correlation matrix.
fn compute_vif_from_correlation(matrix: &ParameterCorrelationMatrix) -> Vec<f64> {
    let n = matrix.n;
    let mut vif = vec![1.0; n];

    for i in 0..n {
        // VIF_i = 1 / (1 - R²_i)
        // R²_i is the R² from regressing X_i on all other X_j
        // Approximation: R²_i ≈ max(r²_ij) for j ≠ i

        let max_r_sq: f64 = (0..n)
            .filter(|&j| j != i)
            .map(|j| matrix.get(i, j).powi(2))
            .fold(0.0, f64::max);

        if max_r_sq < 1.0 - 1e-10 {
            vif[i] = 1.0 / (1.0 - max_r_sq);
        } else {
            vif[i] = f64::INFINITY;
        }
    }

    vif
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_correlation_matrix_identity() {
        // Diagonal covariance = uncorrelated
        let cov = vec![
            vec![1.0, 0.0, 0.0],
            vec![0.0, 1.0, 0.0],
            vec![0.0, 0.0, 1.0],
        ];
        let corr = ParameterCorrelationMatrix::from_covariance(&cov);

        assert!((corr.get(0, 0) - 1.0).abs() < 0.01);
        assert!((corr.get(0, 1) - 0.0).abs() < 0.01);
    }

    #[test]
    fn test_correlation_matrix_perfect() {
        // Perfect positive correlation
        let cov = vec![vec![1.0, 1.0], vec![1.0, 1.0]];
        let corr = ParameterCorrelationMatrix::from_covariance(&cov);

        assert!((corr.get(0, 1) - 1.0).abs() < 0.01);
    }

    #[test]
    fn test_find_high_correlations() {
        let cov = vec![
            vec![1.0, 0.9, 0.1],
            vec![0.9, 1.0, 0.2],
            vec![0.1, 0.2, 1.0],
        ];
        let corr = ParameterCorrelationMatrix::from_covariance(&cov);
        let pairs = corr.find_high_correlations(0.8);

        assert_eq!(pairs.len(), 1);
        assert_eq!(pairs[0].0, 0);
        assert_eq!(pairs[0].1, 1);
    }

    #[test]
    fn test_correlation_cluster() {
        let cluster = CorrelationCluster {
            members: vec![0, 1, 2],
            avg_correlation: 0.9,
            max_correlation: 0.95,
            representative: 0,
        };

        assert_eq!(cluster.size(), 3);
        assert!(!cluster.is_trivial());
        assert_eq!(cluster.non_representatives(), vec![1, 2]);
    }

    #[test]
    fn test_find_clusters_independent() {
        let data = vec![
            vec![1.0, 0.0, 0.0],
            vec![0.0, 1.0, 0.0],
            vec![0.0, 0.0, 1.0],
        ];
        let matrix = ParameterCorrelationMatrix::from_raw(data);
        let clusters = find_correlation_clusters(&matrix, 0.5);

        // Each parameter in its own cluster
        assert_eq!(clusters.len(), 3);
        for cluster in &clusters {
            assert!(cluster.is_trivial());
        }
    }

    #[test]
    fn test_find_clusters_correlated() {
        let data = vec![
            vec![1.0, 0.9, 0.1],
            vec![0.9, 1.0, 0.1],
            vec![0.1, 0.1, 1.0],
        ];
        let matrix = ParameterCorrelationMatrix::from_raw(data);
        let clusters = find_correlation_clusters(&matrix, 0.5);

        // Parameters 0 and 1 should be in same cluster
        assert!(clusters
            .iter()
            .any(|c| c.members.len() == 2 && c.members.contains(&0) && c.members.contains(&1)));
    }

    #[test]
    fn test_compute_vif() {
        let cov = vec![vec![1.0, 0.0], vec![0.0, 1.0]];
        let vif = compute_vif(&cov);

        // Uncorrelated: VIF should be 1
        assert!((vif[0] - 1.0).abs() < 0.01);
        assert!((vif[1] - 1.0).abs() < 0.01);
    }

    #[test]
    fn test_correlation_analysis() {
        // Use very high correlation (0.95) to trigger severe multicollinearity
        let data = vec![
            vec![1.0, 0.95, 0.0],
            vec![0.95, 1.0, 0.0],
            vec![0.0, 0.0, 1.0],
        ];
        let matrix = ParameterCorrelationMatrix::from_raw(data);
        let analysis = CorrelationAnalysis::from_correlation_matrix(matrix, 0.7);

        assert_eq!(analysis.high_correlation_pairs.len(), 1);
        assert!(analysis.has_severe_multicollinearity());
    }

    #[test]
    fn test_max_off_diagonal() {
        let data = vec![
            vec![1.0, 0.5, 0.8],
            vec![0.5, 1.0, 0.3],
            vec![0.8, 0.3, 1.0],
        ];
        let matrix = ParameterCorrelationMatrix::from_raw(data);

        assert!((matrix.max_off_diagonal() - 0.8).abs() < 0.01);
    }
}
