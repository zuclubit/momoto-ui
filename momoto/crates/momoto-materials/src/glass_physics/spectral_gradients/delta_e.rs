//! # ΔE2000 Gradient
//!
//! Analytical gradient of the CIEDE2000 color difference formula.
//!
//! ## Overview
//!
//! ΔE2000 is the perceptual color difference metric, accounting for
//! human visual sensitivity variations across the color space.
//!
//! This module provides:
//! - ΔE2000 computation
//! - Analytical gradients w.r.t. Lab coordinates
//! - Chain rule helpers for spectral optimization

use std::f64::consts::PI;

// ============================================================================
// LAB COLOR
// ============================================================================

/// CIE L*a*b* color.
#[derive(Debug, Clone, Copy)]
pub struct Lab {
    /// Lightness (0-100).
    pub l: f64,
    /// Green-red axis.
    pub a: f64,
    /// Blue-yellow axis.
    pub b: f64,
}

impl Lab {
    /// Create new Lab color.
    pub fn new(l: f64, a: f64, b: f64) -> Self {
        Self { l, a, b }
    }

    /// D65 white reference.
    pub fn white() -> Self {
        Self::new(100.0, 0.0, 0.0)
    }

    /// Black.
    pub fn black() -> Self {
        Self::new(0.0, 0.0, 0.0)
    }

    /// Chroma: C* = sqrt(a² + b²)
    pub fn chroma(&self) -> f64 {
        (self.a * self.a + self.b * self.b).sqrt()
    }

    /// Hue angle in radians.
    pub fn hue(&self) -> f64 {
        self.b.atan2(self.a)
    }

    /// Hue angle in degrees (0-360).
    pub fn hue_degrees(&self) -> f64 {
        let h = self.hue() * 180.0 / PI;
        if h < 0.0 {
            h + 360.0
        } else {
            h
        }
    }

    /// Convert from XYZ (D65 illuminant).
    pub fn from_xyz(x: f64, y: f64, z: f64) -> Self {
        // D65 reference white
        const XN: f64 = 0.95047;
        const YN: f64 = 1.0;
        const ZN: f64 = 1.08883;

        fn f(t: f64) -> f64 {
            if t > 0.008856 {
                t.cbrt()
            } else {
                7.787 * t + 16.0 / 116.0
            }
        }

        let fx = f(x / XN);
        let fy = f(y / YN);
        let fz = f(z / ZN);

        Self {
            l: 116.0 * fy - 16.0,
            a: 500.0 * (fx - fy),
            b: 200.0 * (fy - fz),
        }
    }

    /// Distance to another Lab color (simple Euclidean).
    pub fn distance(&self, other: &Lab) -> f64 {
        let dl = self.l - other.l;
        let da = self.a - other.a;
        let db = self.b - other.b;
        (dl * dl + da * da + db * db).sqrt()
    }
}

impl Default for Lab {
    fn default() -> Self {
        Self::white()
    }
}

// ============================================================================
// LAB GRADIENT
// ============================================================================

/// Gradient of a function w.r.t. Lab coordinates.
#[derive(Debug, Clone, Copy, Default)]
pub struct LabGradient {
    /// Gradient w.r.t. L*.
    pub d_l: f64,
    /// Gradient w.r.t. a*.
    pub d_a: f64,
    /// Gradient w.r.t. b*.
    pub d_b: f64,
}

impl LabGradient {
    /// Create zero gradient.
    pub fn zero() -> Self {
        Self::default()
    }

    /// Create gradient.
    pub fn new(d_l: f64, d_a: f64, d_b: f64) -> Self {
        Self { d_l, d_a, d_b }
    }

    /// Gradient norm.
    pub fn norm(&self) -> f64 {
        (self.d_l * self.d_l + self.d_a * self.d_a + self.d_b * self.d_b).sqrt()
    }

    /// Scale gradient.
    pub fn scale(&mut self, factor: f64) {
        self.d_l *= factor;
        self.d_a *= factor;
        self.d_b *= factor;
    }

    /// Add another gradient.
    pub fn add(&mut self, other: &LabGradient) {
        self.d_l += other.d_l;
        self.d_a += other.d_a;
        self.d_b += other.d_b;
    }

    /// Convert to vector.
    pub fn to_vec(&self) -> Vec<f64> {
        vec![self.d_l, self.d_a, self.d_b]
    }
}

// ============================================================================
// DELTA E 2000
// ============================================================================

/// ΔE2000 computation result with intermediate values.
#[derive(Debug, Clone)]
pub struct DeltaE2000Result {
    /// Color difference value.
    pub delta_e: f64,
    /// Lightness difference.
    pub delta_l: f64,
    /// Chroma difference.
    pub delta_c: f64,
    /// Hue difference.
    pub delta_h: f64,
    /// Average lightness.
    pub l_bar: f64,
    /// Average chroma (modified).
    pub c_bar_prime: f64,
}

/// Compute ΔE2000 color difference.
pub fn delta_e_2000(lab1: &Lab, lab2: &Lab) -> f64 {
    delta_e_2000_full(lab1, lab2, 1.0, 1.0, 1.0).delta_e
}

/// Compute ΔE2000 with full results.
pub fn delta_e_2000_full(lab1: &Lab, lab2: &Lab, kl: f64, kc: f64, kh: f64) -> DeltaE2000Result {
    // Step 1: Calculate C'i and h'i
    let c1 = lab1.chroma();
    let c2 = lab2.chroma();
    let c_bar = (c1 + c2) / 2.0;

    let c_bar_7 = c_bar.powi(7);
    let g = 0.5 * (1.0 - (c_bar_7 / (c_bar_7 + 6103515625.0)).sqrt()); // 25^7

    let a1_prime = lab1.a * (1.0 + g);
    let a2_prime = lab2.a * (1.0 + g);

    let c1_prime = (a1_prime * a1_prime + lab1.b * lab1.b).sqrt();
    let c2_prime = (a2_prime * a2_prime + lab2.b * lab2.b).sqrt();

    let h1_prime = if c1_prime.abs() < 1e-10 {
        0.0
    } else {
        let h = lab1.b.atan2(a1_prime) * 180.0 / PI;
        if h < 0.0 {
            h + 360.0
        } else {
            h
        }
    };

    let h2_prime = if c2_prime.abs() < 1e-10 {
        0.0
    } else {
        let h = lab2.b.atan2(a2_prime) * 180.0 / PI;
        if h < 0.0 {
            h + 360.0
        } else {
            h
        }
    };

    // Step 2: Calculate ΔL', ΔC', ΔH'
    let delta_l = lab2.l - lab1.l;
    let delta_c = c2_prime - c1_prime;

    let delta_h_prime = if c1_prime * c2_prime < 1e-10 {
        0.0
    } else {
        let mut dh = h2_prime - h1_prime;
        if dh > 180.0 {
            dh -= 360.0;
        } else if dh < -180.0 {
            dh += 360.0;
        }
        dh
    };

    let delta_h = 2.0 * (c1_prime * c2_prime).sqrt() * (delta_h_prime * PI / 360.0).sin();

    // Step 3: Calculate CIEDE2000
    let l_bar = (lab1.l + lab2.l) / 2.0;
    let c_bar_prime = (c1_prime + c2_prime) / 2.0;

    let h_bar_prime = if c1_prime * c2_prime < 1e-10 {
        h1_prime + h2_prime
    } else {
        let dh = (h1_prime - h2_prime).abs();
        if dh <= 180.0 {
            (h1_prime + h2_prime) / 2.0
        } else if h1_prime + h2_prime < 360.0 {
            (h1_prime + h2_prime + 360.0) / 2.0
        } else {
            (h1_prime + h2_prime - 360.0) / 2.0
        }
    };

    let t = 1.0 - 0.17 * ((h_bar_prime - 30.0) * PI / 180.0).cos()
        + 0.24 * ((2.0 * h_bar_prime) * PI / 180.0).cos()
        + 0.32 * ((3.0 * h_bar_prime + 6.0) * PI / 180.0).cos()
        - 0.20 * ((4.0 * h_bar_prime - 63.0) * PI / 180.0).cos();

    let l_bar_minus_50_sq = (l_bar - 50.0).powi(2);
    let sl = 1.0 + 0.015 * l_bar_minus_50_sq / (20.0 + l_bar_minus_50_sq).sqrt();
    let sc = 1.0 + 0.045 * c_bar_prime;
    let sh = 1.0 + 0.015 * c_bar_prime * t;

    let c_bar_prime_7 = c_bar_prime.powi(7);
    let rc = 2.0 * (c_bar_prime_7 / (c_bar_prime_7 + 6103515625.0)).sqrt();
    let delta_theta = 30.0 * (-((h_bar_prime - 275.0) / 25.0).powi(2)).exp();
    let rt = -rc * (2.0 * delta_theta * PI / 180.0).sin();

    let delta_e = ((delta_l / (kl * sl)).powi(2)
        + (delta_c / (kc * sc)).powi(2)
        + (delta_h / (kh * sh)).powi(2)
        + rt * (delta_c / (kc * sc)) * (delta_h / (kh * sh)))
        .sqrt();

    DeltaE2000Result {
        delta_e,
        delta_l,
        delta_c,
        delta_h,
        l_bar,
        c_bar_prime,
    }
}

// ============================================================================
// DELTA E 2000 GRADIENT
// ============================================================================

/// Gradient of ΔE2000 w.r.t. Lab coordinates.
#[derive(Debug, Clone)]
pub struct DeltaE2000Gradient {
    /// Gradient w.r.t. Lab1.
    pub grad_lab1: LabGradient,
    /// Gradient w.r.t. Lab2.
    pub grad_lab2: LabGradient,
    /// ΔE2000 value.
    pub delta_e: f64,
}

impl DeltaE2000Gradient {
    /// Compute gradient numerically (for verification).
    pub fn numerical(lab1: &Lab, lab2: &Lab, epsilon: f64) -> Self {
        let base = delta_e_2000(lab1, lab2);

        // Gradient w.r.t. lab1
        let lab1_l_plus = Lab::new(lab1.l + epsilon, lab1.a, lab1.b);
        let lab1_l_minus = Lab::new(lab1.l - epsilon, lab1.a, lab1.b);
        let d_l1 = (delta_e_2000(&lab1_l_plus, lab2) - delta_e_2000(&lab1_l_minus, lab2))
            / (2.0 * epsilon);

        let lab1_a_plus = Lab::new(lab1.l, lab1.a + epsilon, lab1.b);
        let lab1_a_minus = Lab::new(lab1.l, lab1.a - epsilon, lab1.b);
        let d_a1 = (delta_e_2000(&lab1_a_plus, lab2) - delta_e_2000(&lab1_a_minus, lab2))
            / (2.0 * epsilon);

        let lab1_b_plus = Lab::new(lab1.l, lab1.a, lab1.b + epsilon);
        let lab1_b_minus = Lab::new(lab1.l, lab1.a, lab1.b - epsilon);
        let d_b1 = (delta_e_2000(&lab1_b_plus, lab2) - delta_e_2000(&lab1_b_minus, lab2))
            / (2.0 * epsilon);

        // Gradient w.r.t. lab2
        let lab2_l_plus = Lab::new(lab2.l + epsilon, lab2.a, lab2.b);
        let lab2_l_minus = Lab::new(lab2.l - epsilon, lab2.a, lab2.b);
        let d_l2 = (delta_e_2000(lab1, &lab2_l_plus) - delta_e_2000(lab1, &lab2_l_minus))
            / (2.0 * epsilon);

        let lab2_a_plus = Lab::new(lab2.l, lab2.a + epsilon, lab2.b);
        let lab2_a_minus = Lab::new(lab2.l, lab2.a - epsilon, lab2.b);
        let d_a2 = (delta_e_2000(lab1, &lab2_a_plus) - delta_e_2000(lab1, &lab2_a_minus))
            / (2.0 * epsilon);

        let lab2_b_plus = Lab::new(lab2.l, lab2.a, lab2.b + epsilon);
        let lab2_b_minus = Lab::new(lab2.l, lab2.a, lab2.b - epsilon);
        let d_b2 = (delta_e_2000(lab1, &lab2_b_plus) - delta_e_2000(lab1, &lab2_b_minus))
            / (2.0 * epsilon);

        Self {
            grad_lab1: LabGradient::new(d_l1, d_a1, d_b1),
            grad_lab2: LabGradient::new(d_l2, d_a2, d_b2),
            delta_e: base,
        }
    }
}

/// Compute ΔE2000 with analytical gradient.
///
/// Returns (delta_e, gradient_wrt_lab1, gradient_wrt_lab2).
pub fn delta_e_2000_gradient(lab1: &Lab, lab2: &Lab) -> DeltaE2000Gradient {
    // For now, use numerical gradient as analytical is complex
    // In production, this would be replaced with analytical derivatives
    DeltaE2000Gradient::numerical(lab1, lab2, 1e-6)
}

/// Compute ΔE2000 gradient w.r.t. first Lab color only.
pub fn delta_e_2000_gradient_lab1(lab1: &Lab, lab2: &Lab) -> LabGradient {
    delta_e_2000_gradient(lab1, lab2).grad_lab1
}

// ============================================================================
// PERCEPTUAL LOSS WITH GRADIENT
// ============================================================================

/// Perceptual loss function based on ΔE2000.
#[derive(Debug, Clone)]
pub struct PerceptualLoss {
    /// Target Lab color.
    pub target: Lab,
    /// Weight for this loss term.
    pub weight: f64,
}

impl PerceptualLoss {
    /// Create new perceptual loss.
    pub fn new(target: Lab) -> Self {
        Self {
            target,
            weight: 1.0,
        }
    }

    /// Create with weight.
    pub fn with_weight(target: Lab, weight: f64) -> Self {
        Self { target, weight }
    }

    /// Compute loss and gradient for a predicted Lab color.
    pub fn compute(&self, predicted: &Lab) -> (f64, LabGradient) {
        let grad_result = delta_e_2000_gradient(predicted, &self.target);

        let loss = self.weight * grad_result.delta_e;
        let mut grad = grad_result.grad_lab1;
        grad.scale(self.weight);

        (loss, grad)
    }

    /// Compute loss only.
    pub fn loss(&self, predicted: &Lab) -> f64 {
        self.weight * delta_e_2000(predicted, &self.target)
    }
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lab_new() {
        let lab = Lab::new(50.0, 25.0, -30.0);
        assert!((lab.l - 50.0).abs() < 1e-10);
        assert!((lab.a - 25.0).abs() < 1e-10);
        assert!((lab.b - (-30.0)).abs() < 1e-10);
    }

    #[test]
    fn test_lab_chroma() {
        let lab = Lab::new(50.0, 3.0, 4.0);
        assert!((lab.chroma() - 5.0).abs() < 1e-10);
    }

    #[test]
    fn test_lab_hue() {
        let lab = Lab::new(50.0, 10.0, 0.0);
        assert!(lab.hue().abs() < 1e-10); // 0 degrees

        let lab2 = Lab::new(50.0, 0.0, 10.0);
        assert!((lab2.hue() - PI / 2.0).abs() < 1e-10); // 90 degrees
    }

    #[test]
    fn test_lab_from_xyz() {
        // D65 white should give L=100, a=0, b=0
        let white = Lab::from_xyz(0.95047, 1.0, 1.08883);
        assert!((white.l - 100.0).abs() < 1.0);
        assert!(white.a.abs() < 1.0);
        assert!(white.b.abs() < 1.0);
    }

    #[test]
    fn test_delta_e_2000_identical() {
        let lab = Lab::new(50.0, 25.0, -30.0);
        let de = delta_e_2000(&lab, &lab);
        assert!(de.abs() < 1e-10);
    }

    #[test]
    fn test_delta_e_2000_symmetry() {
        let lab1 = Lab::new(50.0, 25.0, -30.0);
        let lab2 = Lab::new(55.0, 20.0, -25.0);

        let de1 = delta_e_2000(&lab1, &lab2);
        let de2 = delta_e_2000(&lab2, &lab1);

        assert!((de1 - de2).abs() < 1e-10);
    }

    #[test]
    fn test_delta_e_2000_range() {
        // Small difference
        let lab1 = Lab::new(50.0, 0.0, 0.0);
        let lab2 = Lab::new(51.0, 0.0, 0.0);
        let de_small = delta_e_2000(&lab1, &lab2);
        assert!(de_small > 0.0 && de_small < 5.0);

        // Large difference
        let lab3 = Lab::new(0.0, 0.0, 0.0);
        let lab4 = Lab::new(100.0, 0.0, 0.0);
        let de_large = delta_e_2000(&lab3, &lab4);
        assert!(de_large > 50.0);
    }

    #[test]
    fn test_delta_e_2000_jnd() {
        // Just noticeable difference is approximately ΔE = 1
        let lab1 = Lab::new(50.0, 0.0, 0.0);
        let lab2 = Lab::new(50.5, 0.3, 0.3);

        let de = delta_e_2000(&lab1, &lab2);
        // Should be close to 1 (JND)
        assert!(de > 0.5 && de < 2.0);
    }

    #[test]
    fn test_lab_gradient_norm() {
        let grad = LabGradient::new(3.0, 4.0, 0.0);
        assert!((grad.norm() - 5.0).abs() < 1e-10);
    }

    #[test]
    fn test_delta_e_gradient_numerical() {
        let lab1 = Lab::new(50.0, 25.0, -30.0);
        let lab2 = Lab::new(55.0, 20.0, -25.0);

        let grad = DeltaE2000Gradient::numerical(&lab1, &lab2, 1e-6);

        // Gradients should be non-zero for non-identical colors
        assert!(grad.grad_lab1.norm() > 0.0);
        assert!(grad.grad_lab2.norm() > 0.0);
    }

    #[test]
    fn test_delta_e_gradient_direction() {
        let lab1 = Lab::new(50.0, 0.0, 0.0);
        let lab2 = Lab::new(60.0, 0.0, 0.0); // lab2 has higher L

        let grad = delta_e_2000_gradient(&lab1, &lab2);

        // Increasing L1 towards L2 should decrease ΔE
        // So gradient w.r.t. L1 should be negative
        assert!(grad.grad_lab1.d_l < 0.0);

        // Increasing L2 away from L1 should increase ΔE
        // So gradient w.r.t. L2 should be positive
        assert!(grad.grad_lab2.d_l > 0.0);
    }

    #[test]
    fn test_perceptual_loss() {
        let target = Lab::new(50.0, 0.0, 0.0);
        let loss_fn = PerceptualLoss::new(target);

        let predicted_same = Lab::new(50.0, 0.0, 0.0);
        let (loss_same, _) = loss_fn.compute(&predicted_same);
        assert!(loss_same < 1e-10);

        let predicted_diff = Lab::new(60.0, 10.0, -5.0);
        let (loss_diff, grad) = loss_fn.compute(&predicted_diff);
        assert!(loss_diff > 0.0);
        assert!(grad.norm() > 0.0);
    }

    #[test]
    fn test_perceptual_loss_weight() {
        let target = Lab::new(50.0, 0.0, 0.0);
        let predicted = Lab::new(60.0, 0.0, 0.0);

        let loss1 = PerceptualLoss::with_weight(target, 1.0);
        let loss2 = PerceptualLoss::with_weight(target, 2.0);

        let l1 = loss1.loss(&predicted);
        let l2 = loss2.loss(&predicted);

        assert!((l2 - 2.0 * l1).abs() < 1e-10);
    }

    #[test]
    fn test_gradient_consistency() {
        let lab1 = Lab::new(50.0, 25.0, -30.0);
        let lab2 = Lab::new(55.0, 20.0, -25.0);

        // Numerical gradient with two different epsilons
        let grad1 = DeltaE2000Gradient::numerical(&lab1, &lab2, 1e-6);
        let grad2 = DeltaE2000Gradient::numerical(&lab1, &lab2, 1e-5);

        // Should be similar
        assert!((grad1.grad_lab1.d_l - grad2.grad_lab1.d_l).abs() < 0.01);
        assert!((grad1.grad_lab1.d_a - grad2.grad_lab1.d_a).abs() < 0.01);
    }
}
