//! # Wavelength Gradients
//!
//! Per-wavelength gradient computation for spectral materials.
//!
//! ## Overview
//!
//! Many materials have wavelength-dependent properties (dispersion).
//! This module provides gradients for:
//! - Reflectance at each wavelength
//! - Cauchy dispersion model
//! - Sellmeier dispersion
//!
//! ## Spectral Sampling
//!
//! Standard sampling uses 31 wavelengths from 400nm to 700nm.

// ============================================================================
// STANDARD WAVELENGTHS
// ============================================================================

/// Number of spectral samples.
pub const N_SPECTRAL_SAMPLES: usize = 31;

/// Wavelength range start (nm).
pub const LAMBDA_MIN: f64 = 400.0;

/// Wavelength range end (nm).
pub const LAMBDA_MAX: f64 = 700.0;

/// Standard visible wavelengths (400-700nm, 10nm spacing).
pub const VISIBLE_WAVELENGTHS: [f64; N_SPECTRAL_SAMPLES] = [
    400.0, 410.0, 420.0, 430.0, 440.0, 450.0, 460.0, 470.0, 480.0, 490.0, 500.0, 510.0, 520.0,
    530.0, 540.0, 550.0, 560.0, 570.0, 580.0, 590.0, 600.0, 610.0, 620.0, 630.0, 640.0, 650.0,
    660.0, 670.0, 680.0, 690.0, 700.0,
];

// ============================================================================
// SPECTRAL GRADIENT
// ============================================================================

/// Gradient of reflectance at a single wavelength.
#[derive(Debug, Clone, Copy)]
pub struct WavelengthGradient {
    /// Wavelength (nm).
    pub wavelength: f64,
    /// Reflectance at this wavelength.
    pub reflectance: f64,
    /// Gradient w.r.t. IOR.
    pub d_ior: f64,
    /// Gradient w.r.t. dispersion (Cauchy B coefficient).
    pub d_dispersion: f64,
    /// Gradient w.r.t. extinction.
    pub d_extinction: f64,
}

impl WavelengthGradient {
    /// Create zero gradient.
    pub fn zero(wavelength: f64) -> Self {
        Self {
            wavelength,
            reflectance: 0.0,
            d_ior: 0.0,
            d_dispersion: 0.0,
            d_extinction: 0.0,
        }
    }

    /// Scale gradient.
    pub fn scale(&mut self, factor: f64) {
        self.d_ior *= factor;
        self.d_dispersion *= factor;
        self.d_extinction *= factor;
    }

    /// Gradient norm.
    pub fn norm(&self) -> f64 {
        (self.d_ior.powi(2) + self.d_dispersion.powi(2) + self.d_extinction.powi(2)).sqrt()
    }
}

/// Gradients across all wavelengths.
#[derive(Debug, Clone)]
pub struct SpectralGradient {
    /// Per-wavelength gradients.
    pub gradients: Vec<WavelengthGradient>,
}

impl SpectralGradient {
    /// Create for standard wavelengths.
    pub fn new() -> Self {
        Self {
            gradients: VISIBLE_WAVELENGTHS
                .iter()
                .map(|&w| WavelengthGradient::zero(w))
                .collect(),
        }
    }

    /// Create with custom wavelengths.
    pub fn with_wavelengths(wavelengths: &[f64]) -> Self {
        Self {
            gradients: wavelengths
                .iter()
                .map(|&w| WavelengthGradient::zero(w))
                .collect(),
        }
    }

    /// Get gradient at index.
    pub fn get(&self, index: usize) -> Option<&WavelengthGradient> {
        self.gradients.get(index)
    }

    /// Get mutable gradient at index.
    pub fn get_mut(&mut self, index: usize) -> Option<&mut WavelengthGradient> {
        self.gradients.get_mut(index)
    }

    /// Number of wavelengths.
    pub fn len(&self) -> usize {
        self.gradients.len()
    }

    /// Check if empty.
    pub fn is_empty(&self) -> bool {
        self.gradients.is_empty()
    }

    /// Sum gradient across all wavelengths.
    pub fn sum_d_ior(&self) -> f64 {
        self.gradients.iter().map(|g| g.d_ior).sum()
    }

    /// Sum dispersion gradient.
    pub fn sum_d_dispersion(&self) -> f64 {
        self.gradients.iter().map(|g| g.d_dispersion).sum()
    }

    /// Weighted sum by luminance importance.
    pub fn luminance_weighted_d_ior(&self) -> f64 {
        // Weight by photopic luminous efficiency (peak at 555nm)
        self.gradients
            .iter()
            .map(|g| {
                let weight = luminous_efficiency(g.wavelength);
                g.d_ior * weight
            })
            .sum()
    }

    /// Get reflectance spectrum.
    pub fn reflectance_spectrum(&self) -> Vec<f64> {
        self.gradients.iter().map(|g| g.reflectance).collect()
    }
}

impl Default for SpectralGradient {
    fn default() -> Self {
        Self::new()
    }
}

/// Photopic luminous efficiency function (simplified).
fn luminous_efficiency(wavelength: f64) -> f64 {
    // Gaussian approximation centered at 555nm
    let center = 555.0;
    let sigma = 60.0;
    let diff = wavelength - center;
    (-0.5 * (diff / sigma).powi(2)).exp()
}

// ============================================================================
// SPECTRAL JACOBIAN
// ============================================================================

/// Jacobian of spectral reflectance w.r.t. parameters.
///
/// Matrix: N_wavelengths × N_params
#[derive(Debug, Clone)]
pub struct SpectralJacobian {
    /// Number of wavelengths.
    pub n_wavelengths: usize,
    /// Number of parameters.
    pub n_params: usize,
    /// Data in row-major order (wavelength × params).
    pub data: Vec<f64>,
}

impl SpectralJacobian {
    /// Create zero Jacobian.
    pub fn zeros(n_wavelengths: usize, n_params: usize) -> Self {
        Self {
            n_wavelengths,
            n_params,
            data: vec![0.0; n_wavelengths * n_params],
        }
    }

    /// Create for standard wavelengths.
    pub fn standard(n_params: usize) -> Self {
        Self::zeros(N_SPECTRAL_SAMPLES, n_params)
    }

    /// Get element at (wavelength_idx, param_idx).
    pub fn get(&self, wavelength_idx: usize, param_idx: usize) -> f64 {
        if wavelength_idx < self.n_wavelengths && param_idx < self.n_params {
            self.data[wavelength_idx * self.n_params + param_idx]
        } else {
            0.0
        }
    }

    /// Set element.
    pub fn set(&mut self, wavelength_idx: usize, param_idx: usize, value: f64) {
        if wavelength_idx < self.n_wavelengths && param_idx < self.n_params {
            self.data[wavelength_idx * self.n_params + param_idx] = value;
        }
    }

    /// Get row (gradients for one wavelength).
    pub fn row(&self, wavelength_idx: usize) -> &[f64] {
        let start = wavelength_idx * self.n_params;
        let end = start + self.n_params;
        &self.data[start..end]
    }

    /// Get column (gradients for one parameter across wavelengths).
    pub fn column(&self, param_idx: usize) -> Vec<f64> {
        (0..self.n_wavelengths)
            .map(|w| self.get(w, param_idx))
            .collect()
    }

    /// Compute J^T × J for least squares.
    pub fn jtj(&self) -> Vec<f64> {
        let n = self.n_params;
        let mut result = vec![0.0; n * n];

        for i in 0..n {
            for j in 0..n {
                let mut sum = 0.0;
                for w in 0..self.n_wavelengths {
                    sum += self.get(w, i) * self.get(w, j);
                }
                result[i * n + j] = sum;
            }
        }

        result
    }

    /// Compute J^T × residual.
    pub fn jt_residual(&self, residual: &[f64]) -> Vec<f64> {
        let n = self.n_params;
        let mut result = vec![0.0; n];

        for i in 0..n {
            let mut sum = 0.0;
            for w in 0..self.n_wavelengths.min(residual.len()) {
                sum += self.get(w, i) * residual[w];
            }
            result[i] = sum;
        }

        result
    }

    /// Frobenius norm.
    pub fn frobenius_norm(&self) -> f64 {
        self.data.iter().map(|&x| x * x).sum::<f64>().sqrt()
    }
}

// ============================================================================
// DISPERSION MODELS WITH GRADIENTS
// ============================================================================

/// Cauchy dispersion: n(λ) = A + B/λ²
#[derive(Debug, Clone, Copy)]
pub struct CauchyDispersion {
    /// Constant term A.
    pub a: f64,
    /// Dispersion term B.
    pub b: f64,
}

impl CauchyDispersion {
    /// Create new Cauchy dispersion.
    pub fn new(a: f64, b: f64) -> Self {
        Self { a, b }
    }

    /// Typical crown glass.
    pub fn crown_glass() -> Self {
        Self::new(1.5168, 4300.0)
    }

    /// Typical flint glass.
    pub fn flint_glass() -> Self {
        Self::new(1.6200, 9500.0)
    }

    /// Evaluate IOR at wavelength.
    pub fn ior(&self, wavelength: f64) -> f64 {
        self.a + self.b / (wavelength * wavelength)
    }

    /// Gradient w.r.t. A.
    pub fn d_a(&self, _wavelength: f64) -> f64 {
        1.0
    }

    /// Gradient w.r.t. B.
    pub fn d_b(&self, wavelength: f64) -> f64 {
        1.0 / (wavelength * wavelength)
    }

    /// Gradient w.r.t. wavelength.
    pub fn d_wavelength(&self, wavelength: f64) -> f64 {
        -2.0 * self.b / (wavelength * wavelength * wavelength)
    }
}

/// Sellmeier dispersion: n² - 1 = Σ (B_i × λ²) / (λ² - C_i)
#[derive(Debug, Clone)]
pub struct SellmeierDispersion {
    /// B coefficients.
    pub b: Vec<f64>,
    /// C coefficients (resonance wavelengths squared).
    pub c: Vec<f64>,
}

impl SellmeierDispersion {
    /// Create new Sellmeier dispersion.
    pub fn new(b: Vec<f64>, c: Vec<f64>) -> Self {
        assert_eq!(b.len(), c.len());
        Self { b, c }
    }

    /// BK7 optical glass.
    pub fn bk7() -> Self {
        Self::new(
            vec![1.03961212, 0.231792344, 1.01046945],
            vec![6.00069867e3, 2.00179144e4, 1.03560653e8],
        )
    }

    /// Fused silica.
    pub fn fused_silica() -> Self {
        Self::new(
            vec![0.6961663, 0.4079426, 0.8974794],
            vec![4.67914826e3, 1.35120631e4, 9.79340025e7],
        )
    }

    /// Evaluate IOR at wavelength.
    pub fn ior(&self, wavelength: f64) -> f64 {
        let lambda2 = wavelength * wavelength;
        let mut n2_minus_1 = 0.0;

        for (bi, ci) in self.b.iter().zip(self.c.iter()) {
            n2_minus_1 += bi * lambda2 / (lambda2 - ci);
        }

        (n2_minus_1 + 1.0).sqrt()
    }

    /// Gradient w.r.t. B_i coefficient.
    pub fn d_b(&self, wavelength: f64, i: usize) -> f64 {
        if i >= self.b.len() {
            return 0.0;
        }

        let lambda2 = wavelength * wavelength;
        let n = self.ior(wavelength);

        // ∂n/∂B_i = (λ² / (λ² - C_i)) / (2n)
        lambda2 / (lambda2 - self.c[i]) / (2.0 * n)
    }

    /// Gradient w.r.t. C_i coefficient.
    pub fn d_c(&self, wavelength: f64, i: usize) -> f64 {
        if i >= self.c.len() {
            return 0.0;
        }

        let lambda2 = wavelength * wavelength;
        let n = self.ior(wavelength);
        let denom = lambda2 - self.c[i];

        // ∂n/∂C_i = B_i × λ² / (λ² - C_i)² / (2n)
        self.b[i] * lambda2 / (denom * denom) / (2.0 * n)
    }
}

// ============================================================================
// COMPUTE SPECTRAL GRADIENT
// ============================================================================

/// Compute spectral gradient for a dielectric material.
pub fn compute_spectral_gradient(
    base_ior: f64,
    dispersion: Option<&CauchyDispersion>,
) -> SpectralGradient {
    let mut result = SpectralGradient::new();

    for (i, &wavelength) in VISIBLE_WAVELENGTHS.iter().enumerate() {
        let ior = if let Some(disp) = dispersion {
            disp.ior(wavelength)
        } else {
            base_ior
        };

        // Fresnel at normal incidence: R = ((n-1)/(n+1))²
        let reflectance = ((ior - 1.0) / (ior + 1.0)).powi(2);

        // ∂R/∂n = 4(n-1)/(n+1)³
        let dr_dn = 4.0 * (ior - 1.0) / (ior + 1.0).powi(3);

        // Dispersion gradient
        let d_dispersion = if let Some(disp) = dispersion {
            dr_dn * disp.d_b(wavelength)
        } else {
            0.0
        };

        if let Some(grad) = result.get_mut(i) {
            grad.wavelength = wavelength;
            grad.reflectance = reflectance;
            grad.d_ior = dr_dn;
            grad.d_dispersion = d_dispersion;
        }
    }

    result
}

/// Compute spectral Jacobian for material parameters.
pub fn compute_spectral_jacobian(
    base_ior: f64,
    extinction: f64,
    dispersion: Option<&CauchyDispersion>,
) -> SpectralJacobian {
    let n_params = 3; // IOR, extinction, dispersion_b
    let mut jacobian = SpectralJacobian::standard(n_params);

    for (w_idx, &wavelength) in VISIBLE_WAVELENGTHS.iter().enumerate() {
        let ior = if let Some(disp) = dispersion {
            disp.ior(wavelength)
        } else {
            base_ior
        };

        // ∂R/∂IOR
        let dr_dn = 4.0 * (ior - 1.0) / (ior + 1.0).powi(3);
        jacobian.set(w_idx, 0, dr_dn);

        // ∂R/∂extinction (affects absorption, small effect on R)
        let dr_dk = -2.0 * extinction / (ior * ior + 1.0); // Simplified
        jacobian.set(w_idx, 1, dr_dk);

        // ∂R/∂dispersion_b
        if let Some(disp) = dispersion {
            let dn_db = disp.d_b(wavelength);
            jacobian.set(w_idx, 2, dr_dn * dn_db);
        }
    }

    jacobian
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_visible_wavelengths() {
        assert_eq!(VISIBLE_WAVELENGTHS.len(), N_SPECTRAL_SAMPLES);
        assert!((VISIBLE_WAVELENGTHS[0] - 400.0).abs() < 1e-10);
        assert!((VISIBLE_WAVELENGTHS[30] - 700.0).abs() < 1e-10);
    }

    #[test]
    fn test_wavelength_gradient() {
        let mut grad = WavelengthGradient::zero(550.0);
        grad.d_ior = 0.1;
        grad.d_dispersion = 0.05;

        let norm = grad.norm();
        assert!(norm > 0.0);

        grad.scale(2.0);
        assert!((grad.d_ior - 0.2).abs() < 1e-10);
    }

    #[test]
    fn test_spectral_gradient() {
        let spec_grad = SpectralGradient::new();
        assert_eq!(spec_grad.len(), N_SPECTRAL_SAMPLES);
    }

    #[test]
    fn test_cauchy_dispersion() {
        let cauchy = CauchyDispersion::crown_glass();

        // IOR should decrease with wavelength
        let n_blue = cauchy.ior(450.0);
        let n_red = cauchy.ior(650.0);

        assert!(n_blue > n_red);
        assert!(n_blue > 1.0);
    }

    #[test]
    fn test_cauchy_gradient() {
        let cauchy = CauchyDispersion::new(1.5, 5000.0);
        let wavelength = 550.0;

        // Numerical gradient for B
        let eps = 1.0;
        let n_plus = CauchyDispersion::new(1.5, 5001.0).ior(wavelength);
        let n_minus = CauchyDispersion::new(1.5, 4999.0).ior(wavelength);
        let numeric_d_b = (n_plus - n_minus) / (2.0 * eps);

        let analytic_d_b = cauchy.d_b(wavelength);

        assert!((analytic_d_b - numeric_d_b).abs() < 1e-6);
    }

    #[test]
    fn test_sellmeier_dispersion() {
        let sellmeier = SellmeierDispersion::bk7();

        // BK7 should have IOR around 1.52 at 589nm
        let n_d = sellmeier.ior(589.0);
        assert!((n_d - 1.517).abs() < 0.01);
    }

    #[test]
    fn test_spectral_jacobian() {
        let jacobian = SpectralJacobian::standard(3);

        assert_eq!(jacobian.n_wavelengths, N_SPECTRAL_SAMPLES);
        assert_eq!(jacobian.n_params, 3);
        assert_eq!(jacobian.data.len(), N_SPECTRAL_SAMPLES * 3);
    }

    #[test]
    fn test_spectral_jacobian_operations() {
        let mut jacobian = SpectralJacobian::zeros(2, 2);

        jacobian.set(0, 0, 1.0);
        jacobian.set(0, 1, 2.0);
        jacobian.set(1, 0, 3.0);
        jacobian.set(1, 1, 4.0);

        assert!((jacobian.get(0, 0) - 1.0).abs() < 1e-10);
        assert!((jacobian.get(1, 1) - 4.0).abs() < 1e-10);

        let row0 = jacobian.row(0);
        assert_eq!(row0.len(), 2);
        assert!((row0[0] - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_compute_spectral_gradient() {
        let grad = compute_spectral_gradient(1.5, None);

        // All gradients should be populated
        assert_eq!(grad.len(), N_SPECTRAL_SAMPLES);

        // Check that reflectance is reasonable
        for g in &grad.gradients {
            assert!(g.reflectance > 0.0 && g.reflectance < 0.1);
            assert!(g.d_ior > 0.0); // Increasing IOR increases reflectance
        }
    }

    #[test]
    fn test_spectral_gradient_with_dispersion() {
        let cauchy = CauchyDispersion::crown_glass();
        let grad = compute_spectral_gradient(1.5, Some(&cauchy));

        // Dispersion gradient should be non-zero
        assert!(grad.gradients.iter().any(|g| g.d_dispersion != 0.0));

        // Blue wavelength should have higher reflectance (higher IOR)
        let r_blue = grad.get(4).unwrap().reflectance; // 440nm
        let r_red = grad.get(26).unwrap().reflectance; // 660nm

        assert!(r_blue > r_red);
    }

    #[test]
    fn test_luminance_weighted_gradient() {
        let mut grad = SpectralGradient::new();

        // Set uniform gradient
        for g in &mut grad.gradients {
            g.d_ior = 1.0;
        }

        let weighted = grad.luminance_weighted_d_ior();
        let unweighted = grad.sum_d_ior();

        // Weighted should be less (not all wavelengths contribute equally)
        assert!(weighted < unweighted);
        assert!(weighted > 0.0);
    }

    #[test]
    fn test_spectral_jacobian_jtj() {
        let mut jacobian = SpectralJacobian::zeros(3, 2);

        // Set up test matrix
        jacobian.set(0, 0, 1.0);
        jacobian.set(1, 0, 2.0);
        jacobian.set(2, 0, 3.0);
        jacobian.set(0, 1, 4.0);
        jacobian.set(1, 1, 5.0);
        jacobian.set(2, 1, 6.0);

        let jtj = jacobian.jtj();

        // J^T × J should be 2×2
        assert_eq!(jtj.len(), 4);

        // (0,0): 1² + 2² + 3² = 14
        assert!((jtj[0] - 14.0).abs() < 1e-10);

        // (1,1): 4² + 5² + 6² = 77
        assert!((jtj[3] - 77.0).abs() < 1e-10);
    }

    #[test]
    fn test_spectral_jacobian_jt_residual() {
        let mut jacobian = SpectralJacobian::zeros(2, 2);
        jacobian.set(0, 0, 1.0);
        jacobian.set(0, 1, 2.0);
        jacobian.set(1, 0, 3.0);
        jacobian.set(1, 1, 4.0);

        let residual = vec![1.0, 2.0];
        let result = jacobian.jt_residual(&residual);

        // J^T × r = [1 3] × [1]   = [7]
        //           [2 4]   [2]     [10]
        assert!((result[0] - 7.0).abs() < 1e-10);
        assert!((result[1] - 10.0).abs() < 1e-10);
    }
}
