//! # Jacobian Matrix Computation
//!
//! Full Jacobian of BSDF response (R, T, A) w.r.t. material parameters.

use super::super::unified_bsdf::BSDFContext;
use super::traits::{DifferentiableBSDF, ParameterGradients};

// ============================================================================
// JACOBIAN MATRIX
// ============================================================================

/// Jacobian matrix of BSDF response w.r.t. parameters.
///
/// Represents the 3×N matrix where:
/// - Rows: R (reflectance), T (transmittance), A (absorption)
/// - Columns: Material parameters (ior, k, roughness, ...)
#[derive(Debug, Clone)]
pub struct Jacobian {
    /// Number of output dimensions (always 3: R, T, A).
    pub output_dim: usize,
    /// Number of input parameters.
    pub param_dim: usize,
    /// Matrix data in row-major order.
    pub data: Vec<f64>,
}

impl Jacobian {
    /// Create zero Jacobian.
    pub fn zeros(param_dim: usize) -> Self {
        Self {
            output_dim: 3,
            param_dim,
            data: vec![0.0; 3 * param_dim],
        }
    }

    /// Create from gradient (single-parameter case).
    pub fn from_gradient(grad: &ParameterGradients) -> Self {
        let params = grad.to_vec();
        let param_dim = params.len();

        let mut data = vec![0.0; 3 * param_dim];

        // Row 0: ∂R/∂params
        for (i, &g) in params.iter().enumerate() {
            data[i] = g * grad.d_reflectance.signum().max(0.0) + g * 0.1; // Approximate
        }

        // Row 1: ∂T/∂params = -∂R/∂params (energy conservation approximation)
        for i in 0..param_dim {
            data[param_dim + i] = -data[i];
        }

        // Row 2: ∂A/∂params = 0 for ideal dielectrics
        // Already zero

        Self {
            output_dim: 3,
            param_dim,
            data,
        }
    }

    /// Get element at (row, col).
    pub fn get(&self, row: usize, col: usize) -> f64 {
        if row < self.output_dim && col < self.param_dim {
            self.data[row * self.param_dim + col]
        } else {
            0.0
        }
    }

    /// Set element at (row, col).
    pub fn set(&mut self, row: usize, col: usize, value: f64) {
        if row < self.output_dim && col < self.param_dim {
            self.data[row * self.param_dim + col] = value;
        }
    }

    /// Get row as slice.
    pub fn row(&self, row: usize) -> &[f64] {
        let start = row * self.param_dim;
        let end = start + self.param_dim;
        &self.data[start..end]
    }

    /// Compute J^T × J (for least squares optimization).
    pub fn jtj(&self) -> Vec<f64> {
        let n = self.param_dim;
        let mut result = vec![0.0; n * n];

        for i in 0..n {
            for j in 0..n {
                let mut sum = 0.0;
                for k in 0..self.output_dim {
                    sum += self.get(k, i) * self.get(k, j);
                }
                result[i * n + j] = sum;
            }
        }

        result
    }

    /// Compute J^T × residual (for Gauss-Newton).
    pub fn jt_residual(&self, residual: &[f64]) -> Vec<f64> {
        let n = self.param_dim;
        let mut result = vec![0.0; n];

        for i in 0..n {
            let mut sum = 0.0;
            for k in 0..self.output_dim.min(residual.len()) {
                sum += self.get(k, i) * residual[k];
            }
            result[i] = sum;
        }

        result
    }

    /// Frobenius norm of the Jacobian.
    pub fn frobenius_norm(&self) -> f64 {
        self.data.iter().map(|&x| x * x).sum::<f64>().sqrt()
    }

    /// Check if Jacobian is well-conditioned.
    pub fn is_well_conditioned(&self, threshold: f64) -> bool {
        // Simple check: no row is all zeros
        for row in 0..self.output_dim {
            let row_norm: f64 = self.row(row).iter().map(|&x| x * x).sum::<f64>().sqrt();
            if row_norm < threshold {
                return false;
            }
        }
        true
    }
}

// ============================================================================
// JACOBIAN BUILDER
// ============================================================================

/// Builder for constructing Jacobians.
#[derive(Debug)]
pub struct JacobianBuilder {
    param_dim: usize,
    rows: Vec<Vec<f64>>,
}

impl JacobianBuilder {
    /// Create new builder with given parameter dimension.
    pub fn new(param_dim: usize) -> Self {
        Self {
            param_dim,
            rows: Vec::new(),
        }
    }

    /// Add reflectance gradient row.
    pub fn with_reflectance_gradient(mut self, gradient: &[f64]) -> Self {
        let mut row = vec![0.0; self.param_dim];
        for (i, &g) in gradient.iter().enumerate().take(self.param_dim) {
            row[i] = g;
        }
        if self.rows.is_empty() {
            self.rows.push(row);
        } else {
            self.rows[0] = row;
        }
        self
    }

    /// Add transmittance gradient row.
    pub fn with_transmittance_gradient(mut self, gradient: &[f64]) -> Self {
        let mut row = vec![0.0; self.param_dim];
        for (i, &g) in gradient.iter().enumerate().take(self.param_dim) {
            row[i] = g;
        }
        while self.rows.len() < 2 {
            self.rows.push(vec![0.0; self.param_dim]);
        }
        self.rows[1] = row;
        self
    }

    /// Add absorption gradient row.
    pub fn with_absorption_gradient(mut self, gradient: &[f64]) -> Self {
        let mut row = vec![0.0; self.param_dim];
        for (i, &g) in gradient.iter().enumerate().take(self.param_dim) {
            row[i] = g;
        }
        while self.rows.len() < 3 {
            self.rows.push(vec![0.0; self.param_dim]);
        }
        self.rows[2] = row;
        self
    }

    /// Enforce energy conservation: ∂A/∂p = -(∂R/∂p + ∂T/∂p).
    pub fn enforce_energy_conservation(mut self) -> Self {
        while self.rows.len() < 3 {
            self.rows.push(vec![0.0; self.param_dim]);
        }

        for i in 0..self.param_dim {
            let r_grad = self.rows.get(0).map(|r| r[i]).unwrap_or(0.0);
            let t_grad = self.rows.get(1).map(|r| r[i]).unwrap_or(0.0);
            self.rows[2][i] = -(r_grad + t_grad);
        }

        self
    }

    /// Build the Jacobian matrix.
    pub fn build(mut self) -> Jacobian {
        // Ensure we have 3 rows
        while self.rows.len() < 3 {
            self.rows.push(vec![0.0; self.param_dim]);
        }

        let mut data = Vec::with_capacity(3 * self.param_dim);
        for row in &self.rows[..3] {
            data.extend(row.iter());
        }

        Jacobian {
            output_dim: 3,
            param_dim: self.param_dim,
            data,
        }
    }
}

// ============================================================================
// NUMERICAL JACOBIAN
// ============================================================================

/// Compute Jacobian numerically via finite differences.
pub fn compute_numerical_jacobian<B: DifferentiableBSDF>(
    material: &B,
    ctx: &BSDFContext,
    epsilon: f64,
) -> Jacobian {
    let params = material.params_to_vec();
    let n = params.len();

    let mut jacobian = Jacobian::zeros(n);

    for i in 0..n {
        let mut params_plus = params.clone();
        let mut params_minus = params.clone();
        params_plus[i] += epsilon;
        params_minus[i] -= epsilon;

        let material_plus = B::from_param_vec(&params_plus);
        let material_minus = B::from_param_vec(&params_minus);

        let r_plus = material_plus.evaluate(ctx);
        let r_minus = material_minus.evaluate(ctx);

        // Central difference for each output
        let dr = (r_plus.reflectance - r_minus.reflectance) / (2.0 * epsilon);
        let dt = (r_plus.transmittance - r_minus.transmittance) / (2.0 * epsilon);
        let da = (r_plus.absorption - r_minus.absorption) / (2.0 * epsilon);

        jacobian.set(0, i, dr);
        jacobian.set(1, i, dt);
        jacobian.set(2, i, da);
    }

    jacobian
}

/// Verify analytical Jacobian against numerical.
pub fn verify_jacobian<B: DifferentiableBSDF>(
    material: &B,
    ctx: &BSDFContext,
    epsilon: f64,
    tolerance: f64,
) -> JacobianVerification {
    let analytical = Jacobian::from_gradient(&material.eval_with_gradients(ctx).gradients);
    let numerical = compute_numerical_jacobian(material, ctx, epsilon);

    let mut max_error = 0.0f64;
    let mut errors = Vec::new();

    for row in 0..3 {
        for col in 0..analytical.param_dim.min(numerical.param_dim) {
            let a = analytical.get(row, col);
            let n = numerical.get(row, col);
            let error = (a - n).abs();
            max_error = max_error.max(error);
            errors.push((row, col, error));
        }
    }

    JacobianVerification {
        passed: max_error < tolerance,
        max_error,
        element_errors: errors,
    }
}

/// Result of Jacobian verification.
#[derive(Debug, Clone)]
pub struct JacobianVerification {
    /// Whether verification passed.
    pub passed: bool,
    /// Maximum error across all elements.
    pub max_error: f64,
    /// Errors per element: (row, col, error).
    pub element_errors: Vec<(usize, usize, f64)>,
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::super::traits::ParameterGradients;
    use super::*;

    #[test]
    fn test_jacobian_zeros() {
        let jac = Jacobian::zeros(4);
        assert_eq!(jac.output_dim, 3);
        assert_eq!(jac.param_dim, 4);
        assert_eq!(jac.data.len(), 12);

        for i in 0..3 {
            for j in 0..4 {
                assert!((jac.get(i, j) - 0.0).abs() < 1e-10);
            }
        }
    }

    #[test]
    fn test_jacobian_set_get() {
        let mut jac = Jacobian::zeros(3);
        jac.set(0, 0, 1.0);
        jac.set(1, 1, 2.0);
        jac.set(2, 2, 3.0);

        assert!((jac.get(0, 0) - 1.0).abs() < 1e-10);
        assert!((jac.get(1, 1) - 2.0).abs() < 1e-10);
        assert!((jac.get(2, 2) - 3.0).abs() < 1e-10);
    }

    #[test]
    fn test_jacobian_row() {
        let mut jac = Jacobian::zeros(3);
        jac.set(0, 0, 1.0);
        jac.set(0, 1, 2.0);
        jac.set(0, 2, 3.0);

        let row = jac.row(0);
        assert_eq!(row.len(), 3);
        assert!((row[0] - 1.0).abs() < 1e-10);
        assert!((row[1] - 2.0).abs() < 1e-10);
        assert!((row[2] - 3.0).abs() < 1e-10);
    }

    #[test]
    fn test_jacobian_jtj() {
        let mut jac = Jacobian::zeros(2);
        // Simple 3x2 matrix:
        // [1 0]
        // [0 1]
        // [1 1]
        jac.set(0, 0, 1.0);
        jac.set(1, 1, 1.0);
        jac.set(2, 0, 1.0);
        jac.set(2, 1, 1.0);

        let jtj = jac.jtj();
        // J^T × J should be:
        // [1 0 1] × [1 0]   = [2 1]
        // [0 1 1]   [0 1]     [1 2]
        //           [1 1]

        assert!((jtj[0] - 2.0).abs() < 1e-10); // (0,0)
        assert!((jtj[1] - 1.0).abs() < 1e-10); // (0,1)
        assert!((jtj[2] - 1.0).abs() < 1e-10); // (1,0)
        assert!((jtj[3] - 2.0).abs() < 1e-10); // (1,1)
    }

    #[test]
    fn test_jacobian_jt_residual() {
        let mut jac = Jacobian::zeros(2);
        jac.set(0, 0, 1.0);
        jac.set(1, 1, 1.0);
        jac.set(2, 0, 1.0);
        jac.set(2, 1, 1.0);

        let residual = vec![1.0, 2.0, 3.0];
        let result = jac.jt_residual(&residual);

        // J^T × r = [1 0 1] × [1]   = [4]
        //           [0 1 1]   [2]     [5]
        //                     [3]

        assert!((result[0] - 4.0).abs() < 1e-10);
        assert!((result[1] - 5.0).abs() < 1e-10);
    }

    #[test]
    fn test_jacobian_frobenius_norm() {
        let mut jac = Jacobian::zeros(2);
        jac.set(0, 0, 3.0);
        jac.set(0, 1, 4.0);

        let norm = jac.frobenius_norm();
        assert!((norm - 5.0).abs() < 1e-10); // sqrt(9 + 16) = 5
    }

    #[test]
    fn test_jacobian_builder() {
        let jac = JacobianBuilder::new(3)
            .with_reflectance_gradient(&[1.0, 2.0, 3.0])
            .with_transmittance_gradient(&[-1.0, -2.0, -3.0])
            .enforce_energy_conservation()
            .build();

        assert_eq!(jac.output_dim, 3);
        assert_eq!(jac.param_dim, 3);

        // Check reflectance row
        assert!((jac.get(0, 0) - 1.0).abs() < 1e-10);
        assert!((jac.get(0, 1) - 2.0).abs() < 1e-10);

        // Check transmittance row
        assert!((jac.get(1, 0) - (-1.0)).abs() < 1e-10);

        // Check energy conservation (∂A/∂p = -(∂R/∂p + ∂T/∂p))
        // For param 0: -(1 + (-1)) = 0
        assert!((jac.get(2, 0) - 0.0).abs() < 1e-10);
    }

    #[test]
    fn test_jacobian_well_conditioned() {
        let mut jac = Jacobian::zeros(2);
        jac.set(0, 0, 1.0);
        jac.set(1, 1, 1.0);
        jac.set(2, 0, 0.5);

        assert!(jac.is_well_conditioned(0.1));

        // Zero row should fail
        let jac_zero_row = Jacobian::zeros(2);
        assert!(!jac_zero_row.is_well_conditioned(0.1));
    }

    #[test]
    fn test_jacobian_from_gradient() {
        let grad = ParameterGradients::dielectric(0.5, 0.3);
        let jac = Jacobian::from_gradient(&grad);

        assert_eq!(jac.output_dim, 3);
        assert!(jac.param_dim > 0);
    }
}
