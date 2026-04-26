//! # Evolution Gradients
//!
//! Analytical gradients for temporal evolution models.
//!
//! ## Overview
//!
//! Different materials evolve in different ways over time:
//! - **Linear**: Paint drying (linear increase in IOR)
//! - **Exponential**: Oxidation (approaches asymptote)
//! - **Oscillating**: Temperature cycling (periodic changes)
//!
//! This module provides analytical gradients for each evolution type,
//! enabling efficient optimization without numerical differentiation.

use std::f64::consts::PI;

// ============================================================================
// EVOLUTION GRADIENT TRAIT
// ============================================================================

/// Trait for evolution models with analytical gradients.
pub trait EvolutionGradient {
    /// Evaluate the evolution at time t.
    fn evaluate(&self, t: f64, initial: f64) -> f64;

    /// Compute gradient w.r.t. initial value.
    fn gradient_initial(&self, t: f64) -> f64;

    /// Compute gradient w.r.t. rate parameter.
    fn gradient_rate(&self, t: f64, initial: f64) -> f64;

    /// Compute gradient w.r.t. time (for adjoint methods).
    fn gradient_time(&self, t: f64, initial: f64) -> f64;

    /// Get all gradients at once.
    fn all_gradients(&self, t: f64, initial: f64) -> EvolutionGradients {
        EvolutionGradients {
            d_initial: self.gradient_initial(t),
            d_rate: self.gradient_rate(t, initial),
            d_time: self.gradient_time(t, initial),
            d_tau: None,
            d_asymptote: None,
            d_amplitude: None,
            d_frequency: None,
        }
    }
}

/// Collection of all evolution gradients.
#[derive(Debug, Clone, Default)]
pub struct EvolutionGradients {
    /// Gradient w.r.t. initial value.
    pub d_initial: f64,
    /// Gradient w.r.t. rate.
    pub d_rate: f64,
    /// Gradient w.r.t. time.
    pub d_time: f64,
    /// Gradient w.r.t. time constant (logarithmic).
    pub d_tau: Option<f64>,
    /// Gradient w.r.t. asymptote (exponential).
    pub d_asymptote: Option<f64>,
    /// Gradient w.r.t. amplitude (oscillating).
    pub d_amplitude: Option<f64>,
    /// Gradient w.r.t. frequency (oscillating).
    pub d_frequency: Option<f64>,
}

impl EvolutionGradients {
    /// Create zero gradients.
    pub fn zero() -> Self {
        Self::default()
    }

    /// Compute gradient norm.
    pub fn norm(&self) -> f64 {
        let mut sum = self.d_initial.powi(2) + self.d_rate.powi(2) + self.d_time.powi(2);

        if let Some(dt) = self.d_tau {
            sum += dt.powi(2);
        }
        if let Some(da) = self.d_asymptote {
            sum += da.powi(2);
        }
        if let Some(damp) = self.d_amplitude {
            sum += damp.powi(2);
        }
        if let Some(df) = self.d_frequency {
            sum += df.powi(2);
        }

        sum.sqrt()
    }

    /// Scale all gradients.
    pub fn scale(&mut self, factor: f64) {
        self.d_initial *= factor;
        self.d_rate *= factor;
        self.d_time *= factor;

        if let Some(ref mut dt) = self.d_tau {
            *dt *= factor;
        }
        if let Some(ref mut da) = self.d_asymptote {
            *da *= factor;
        }
        if let Some(ref mut damp) = self.d_amplitude {
            *damp *= factor;
        }
        if let Some(ref mut df) = self.d_frequency {
            *df *= factor;
        }
    }

    /// Clip gradients to maximum norm.
    pub fn clip(&mut self, max_norm: f64) {
        let norm = self.norm();
        if norm > max_norm && norm > 0.0 {
            self.scale(max_norm / norm);
        }
    }

    /// Accumulate gradients from another set.
    pub fn accumulate(&mut self, other: &EvolutionGradients, weight: f64) {
        self.d_initial += weight * other.d_initial;
        self.d_rate += weight * other.d_rate;
        self.d_time += weight * other.d_time;

        if let Some(dt) = other.d_tau {
            *self.d_tau.get_or_insert(0.0) += weight * dt;
        }
        if let Some(da) = other.d_asymptote {
            *self.d_asymptote.get_or_insert(0.0) += weight * da;
        }
        if let Some(damp) = other.d_amplitude {
            *self.d_amplitude.get_or_insert(0.0) += weight * damp;
        }
        if let Some(df) = other.d_frequency {
            *self.d_frequency.get_or_insert(0.0) += weight * df;
        }
    }
}

// ============================================================================
// LINEAR EVOLUTION
// ============================================================================

/// Linear evolution: p(t) = p₀ + rate × t
#[derive(Debug, Clone, Copy)]
pub struct LinearEvolutionGradient {
    /// Evolution rate.
    pub rate: f64,
}

impl LinearEvolutionGradient {
    /// Create new linear evolution.
    pub fn new(rate: f64) -> Self {
        Self { rate }
    }
}

impl EvolutionGradient for LinearEvolutionGradient {
    fn evaluate(&self, t: f64, initial: f64) -> f64 {
        initial + self.rate * t
    }

    fn gradient_initial(&self, _t: f64) -> f64 {
        1.0
    }

    fn gradient_rate(&self, t: f64, _initial: f64) -> f64 {
        t
    }

    fn gradient_time(&self, _t: f64, _initial: f64) -> f64 {
        self.rate
    }
}

// ============================================================================
// EXPONENTIAL EVOLUTION
// ============================================================================

/// Exponential evolution: p(t) = asymp + (p₀ - asymp) × exp(-rate × t)
#[derive(Debug, Clone, Copy)]
pub struct ExponentialEvolutionGradient {
    /// Evolution rate.
    pub rate: f64,
    /// Asymptotic value.
    pub asymptote: f64,
}

impl ExponentialEvolutionGradient {
    /// Create new exponential evolution.
    pub fn new(rate: f64, asymptote: f64) -> Self {
        Self { rate, asymptote }
    }

    /// Gradient w.r.t. asymptote.
    pub fn gradient_asymptote(&self, t: f64, _initial: f64) -> f64 {
        // ∂/∂asymp (asymp + (p₀ - asymp)×e^(-rt))
        // = 1 - e^(-rt)
        let exp_term = (-self.rate * t).exp();
        1.0 - exp_term
    }
}

impl EvolutionGradient for ExponentialEvolutionGradient {
    fn evaluate(&self, t: f64, initial: f64) -> f64 {
        let exp_term = (-self.rate * t).exp();
        self.asymptote + (initial - self.asymptote) * exp_term
    }

    fn gradient_initial(&self, t: f64) -> f64 {
        // ∂/∂p₀ = e^(-rt)
        (-self.rate * t).exp()
    }

    fn gradient_rate(&self, t: f64, initial: f64) -> f64 {
        // ∂/∂rate = -t × (p₀ - asymp) × e^(-rt)
        let exp_term = (-self.rate * t).exp();
        -t * (initial - self.asymptote) * exp_term
    }

    fn gradient_time(&self, t: f64, initial: f64) -> f64 {
        // ∂/∂t = -rate × (p₀ - asymp) × e^(-rt)
        let exp_term = (-self.rate * t).exp();
        -self.rate * (initial - self.asymptote) * exp_term
    }

    fn all_gradients(&self, t: f64, initial: f64) -> EvolutionGradients {
        let exp_term = (-self.rate * t).exp();
        let diff = initial - self.asymptote;

        EvolutionGradients {
            d_initial: exp_term,
            d_rate: -t * diff * exp_term,
            d_time: -self.rate * diff * exp_term,
            d_tau: None,
            d_asymptote: Some(1.0 - exp_term),
            d_amplitude: None,
            d_frequency: None,
        }
    }
}

// ============================================================================
// OSCILLATING EVOLUTION
// ============================================================================

/// Oscillating evolution: p(t) = p₀ + amp × sin(2π × freq × t)
#[derive(Debug, Clone, Copy)]
pub struct OscillatingEvolutionGradient {
    /// Oscillation amplitude.
    pub amplitude: f64,
    /// Oscillation frequency (Hz).
    pub frequency: f64,
    /// Phase offset (radians).
    pub phase: f64,
}

impl OscillatingEvolutionGradient {
    /// Create new oscillating evolution.
    pub fn new(amplitude: f64, frequency: f64) -> Self {
        Self {
            amplitude,
            frequency,
            phase: 0.0,
        }
    }

    /// Create with phase offset.
    pub fn with_phase(amplitude: f64, frequency: f64, phase: f64) -> Self {
        Self {
            amplitude,
            frequency,
            phase,
        }
    }

    /// Gradient w.r.t. amplitude.
    pub fn gradient_amplitude(&self, t: f64) -> f64 {
        // ∂/∂amp = sin(2πft + φ)
        (2.0 * PI * self.frequency * t + self.phase).sin()
    }

    /// Gradient w.r.t. frequency.
    pub fn gradient_frequency(&self, t: f64) -> f64 {
        // ∂/∂freq = amp × 2π × t × cos(2πft + φ)
        self.amplitude * 2.0 * PI * t * (2.0 * PI * self.frequency * t + self.phase).cos()
    }

    /// Gradient w.r.t. phase.
    pub fn gradient_phase(&self, t: f64) -> f64 {
        // ∂/∂φ = amp × cos(2πft + φ)
        self.amplitude * (2.0 * PI * self.frequency * t + self.phase).cos()
    }
}

impl EvolutionGradient for OscillatingEvolutionGradient {
    fn evaluate(&self, t: f64, initial: f64) -> f64 {
        initial + self.amplitude * (2.0 * PI * self.frequency * t + self.phase).sin()
    }

    fn gradient_initial(&self, _t: f64) -> f64 {
        1.0
    }

    fn gradient_rate(&self, _t: f64, _initial: f64) -> f64 {
        // Oscillating model doesn't have a simple "rate" parameter
        // Return gradient w.r.t. frequency instead
        0.0
    }

    fn gradient_time(&self, t: f64, _initial: f64) -> f64 {
        // ∂/∂t = amp × 2π × freq × cos(2πft + φ)
        self.amplitude
            * 2.0
            * PI
            * self.frequency
            * (2.0 * PI * self.frequency * t + self.phase).cos()
    }

    fn all_gradients(&self, t: f64, _initial: f64) -> EvolutionGradients {
        let arg = 2.0 * PI * self.frequency * t + self.phase;
        let sin_arg = arg.sin();
        let cos_arg = arg.cos();

        EvolutionGradients {
            d_initial: 1.0,
            d_rate: 0.0,
            d_time: self.amplitude * 2.0 * PI * self.frequency * cos_arg,
            d_tau: None,
            d_asymptote: None,
            d_amplitude: Some(sin_arg),
            d_frequency: Some(self.amplitude * 2.0 * PI * t * cos_arg),
        }
    }
}

// ============================================================================
// LOGARITHMIC EVOLUTION
// ============================================================================

/// Logarithmic evolution: p(t) = p₀ + rate × ln(1 + t/τ)
#[derive(Debug, Clone, Copy)]
pub struct LogarithmicEvolutionGradient {
    /// Evolution rate.
    pub rate: f64,
    /// Time constant.
    pub tau: f64,
}

impl LogarithmicEvolutionGradient {
    /// Create new logarithmic evolution.
    pub fn new(rate: f64, tau: f64) -> Self {
        Self {
            rate,
            tau: tau.max(0.001),
        }
    }

    /// Gradient w.r.t. tau.
    pub fn gradient_tau(&self, t: f64) -> f64 {
        // ∂/∂τ = rate × (-t/τ²) / (1 + t/τ)
        // = -rate × t / (τ² + τt)
        let denom = self.tau * self.tau + self.tau * t;
        if denom.abs() < 1e-10 {
            0.0
        } else {
            -self.rate * t / denom
        }
    }
}

impl EvolutionGradient for LogarithmicEvolutionGradient {
    fn evaluate(&self, t: f64, initial: f64) -> f64 {
        initial + self.rate * (1.0 + t / self.tau).ln()
    }

    fn gradient_initial(&self, _t: f64) -> f64 {
        1.0
    }

    fn gradient_rate(&self, t: f64, _initial: f64) -> f64 {
        // ∂/∂rate = ln(1 + t/τ)
        (1.0 + t / self.tau).ln()
    }

    fn gradient_time(&self, t: f64, _initial: f64) -> f64 {
        // ∂/∂t = rate / (τ + t)
        self.rate / (self.tau + t)
    }

    fn all_gradients(&self, t: f64, _initial: f64) -> EvolutionGradients {
        let log_term = (1.0 + t / self.tau).ln();

        EvolutionGradients {
            d_initial: 1.0,
            d_rate: log_term,
            d_time: self.rate / (self.tau + t),
            d_tau: Some(self.gradient_tau(t)),
            d_asymptote: None,
            d_amplitude: None,
            d_frequency: None,
        }
    }
}

// ============================================================================
// GENERIC GRADIENT COMPUTATION
// ============================================================================

/// Evolution type enum for generic computation.
#[derive(Debug, Clone, Copy)]
pub enum EvolutionType {
    /// Linear evolution.
    Linear { rate: f64 },
    /// Exponential evolution.
    Exponential { rate: f64, asymptote: f64 },
    /// Oscillating evolution.
    Oscillating {
        amplitude: f64,
        frequency: f64,
        phase: f64,
    },
    /// Logarithmic evolution.
    Logarithmic { rate: f64, tau: f64 },
}

impl Default for EvolutionType {
    fn default() -> Self {
        Self::Linear { rate: 0.0 }
    }
}

/// Compute evolution gradient for any type.
pub fn compute_evolution_gradient(
    evolution: EvolutionType,
    t: f64,
    initial: f64,
) -> (f64, EvolutionGradients) {
    match evolution {
        EvolutionType::Linear { rate } => {
            let model = LinearEvolutionGradient::new(rate);
            (model.evaluate(t, initial), model.all_gradients(t, initial))
        }
        EvolutionType::Exponential { rate, asymptote } => {
            let model = ExponentialEvolutionGradient::new(rate, asymptote);
            (model.evaluate(t, initial), model.all_gradients(t, initial))
        }
        EvolutionType::Oscillating {
            amplitude,
            frequency,
            phase,
        } => {
            let model = OscillatingEvolutionGradient::with_phase(amplitude, frequency, phase);
            (model.evaluate(t, initial), model.all_gradients(t, initial))
        }
        EvolutionType::Logarithmic { rate, tau } => {
            let model = LogarithmicEvolutionGradient::new(rate, tau);
            (model.evaluate(t, initial), model.all_gradients(t, initial))
        }
    }
}

/// Verify gradient numerically.
pub fn verify_evolution_gradient(
    evolution: EvolutionType,
    t: f64,
    initial: f64,
    epsilon: f64,
) -> GradientVerification {
    let (_, analytic) = compute_evolution_gradient(evolution, t, initial);

    // Numerical gradient w.r.t. initial
    let (v_plus, _) = compute_evolution_gradient(evolution, t, initial + epsilon);
    let (v_minus, _) = compute_evolution_gradient(evolution, t, initial - epsilon);
    let numeric_d_initial = (v_plus - v_minus) / (2.0 * epsilon);

    let error_initial = (analytic.d_initial - numeric_d_initial).abs();

    // Numerical gradient w.r.t. time
    let (v_plus_t, _) = compute_evolution_gradient(evolution, t + epsilon, initial);
    let (v_minus_t, _) = compute_evolution_gradient(evolution, (t - epsilon).max(0.0), initial);
    let numeric_d_time = (v_plus_t - v_minus_t) / (2.0 * epsilon);

    let error_time = (analytic.d_time - numeric_d_time).abs();

    GradientVerification {
        passed: error_initial < 1e-4 && error_time < 1e-4,
        max_error: error_initial.max(error_time),
        analytic,
        numeric_d_initial,
        numeric_d_time,
    }
}

/// Result of gradient verification.
#[derive(Debug)]
pub struct GradientVerification {
    /// Whether verification passed.
    pub passed: bool,
    /// Maximum error.
    pub max_error: f64,
    /// Analytic gradients.
    pub analytic: EvolutionGradients,
    /// Numeric gradient w.r.t. initial.
    pub numeric_d_initial: f64,
    /// Numeric gradient w.r.t. time.
    pub numeric_d_time: f64,
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    const EPSILON: f64 = 1e-6;
    const TOLERANCE: f64 = 1e-4;

    #[test]
    fn test_linear_evolution() {
        let model = LinearEvolutionGradient::new(0.1);

        assert!((model.evaluate(0.0, 1.5) - 1.5).abs() < 1e-10);
        assert!((model.evaluate(1.0, 1.5) - 1.6).abs() < 1e-10);
        assert!((model.evaluate(10.0, 1.5) - 2.5).abs() < 1e-10);
    }

    #[test]
    fn test_linear_gradients() {
        let model = LinearEvolutionGradient::new(0.1);

        // d/d_initial = 1
        assert!((model.gradient_initial(5.0) - 1.0).abs() < 1e-10);

        // d/d_rate = t
        assert!((model.gradient_rate(5.0, 1.5) - 5.0).abs() < 1e-10);

        // d/d_time = rate
        assert!((model.gradient_time(5.0, 1.5) - 0.1).abs() < 1e-10);
    }

    #[test]
    fn test_linear_gradient_vs_numeric() {
        let verification =
            verify_evolution_gradient(EvolutionType::Linear { rate: 0.1 }, 5.0, 1.5, EPSILON);

        assert!(verification.passed, "Max error: {}", verification.max_error);
    }

    #[test]
    fn test_exponential_evolution() {
        let model = ExponentialEvolutionGradient::new(1.0, 1.0);

        // At t=0, should equal initial
        assert!((model.evaluate(0.0, 2.0) - 2.0).abs() < 1e-10);

        // At large t, should approach asymptote
        assert!((model.evaluate(100.0, 2.0) - 1.0).abs() < 0.01);
    }

    #[test]
    fn test_exponential_gradients() {
        let model = ExponentialEvolutionGradient::new(0.5, 1.0);
        let t = 2.0;
        let initial = 2.0;

        // Numerical verification
        let eps = 1e-6;
        let v_plus = model.evaluate(t, initial + eps);
        let v_minus = model.evaluate(t, initial - eps);
        let numeric_d_initial = (v_plus - v_minus) / (2.0 * eps);

        let analytic_d_initial = model.gradient_initial(t);
        assert!(
            (analytic_d_initial - numeric_d_initial).abs() < TOLERANCE,
            "Analytic: {}, Numeric: {}",
            analytic_d_initial,
            numeric_d_initial
        );
    }

    #[test]
    fn test_exponential_gradient_vs_numeric() {
        let verification = verify_evolution_gradient(
            EvolutionType::Exponential {
                rate: 0.5,
                asymptote: 1.0,
            },
            2.0,
            2.0,
            EPSILON,
        );

        assert!(verification.passed, "Max error: {}", verification.max_error);
    }

    #[test]
    fn test_oscillating_evolution() {
        let model = OscillatingEvolutionGradient::new(0.1, 1.0);

        // At t=0, should equal initial
        assert!((model.evaluate(0.0, 1.5) - 1.5).abs() < 1e-10);

        // At t=0.25 (quarter period), should be at maximum
        assert!((model.evaluate(0.25, 1.5) - 1.6).abs() < 1e-10);

        // At t=0.5 (half period), should be back at initial
        assert!((model.evaluate(0.5, 1.5) - 1.5).abs() < 1e-6);
    }

    #[test]
    fn test_oscillating_gradients() {
        let model = OscillatingEvolutionGradient::new(0.1, 1.0);

        // d/d_time at t=0 should be amp × 2π × freq
        let expected = 0.1 * 2.0 * PI * 1.0;
        assert!((model.gradient_time(0.0, 1.5) - expected).abs() < 1e-10);
    }

    #[test]
    fn test_oscillating_gradient_vs_numeric() {
        let verification = verify_evolution_gradient(
            EvolutionType::Oscillating {
                amplitude: 0.1,
                frequency: 1.0,
                phase: 0.0,
            },
            0.3,
            1.5,
            EPSILON,
        );

        assert!(verification.passed, "Max error: {}", verification.max_error);
    }

    #[test]
    fn test_logarithmic_evolution() {
        let model = LogarithmicEvolutionGradient::new(0.1, 1.0);

        // At t=0, should equal initial
        assert!((model.evaluate(0.0, 1.5) - 1.5).abs() < 1e-10);

        // At t=1 (tau), should add rate × ln(2)
        let expected = 1.5 + 0.1 * 2.0_f64.ln();
        assert!((model.evaluate(1.0, 1.5) - expected).abs() < 1e-10);
    }

    #[test]
    fn test_logarithmic_gradient_vs_numeric() {
        let verification = verify_evolution_gradient(
            EvolutionType::Logarithmic {
                rate: 0.1,
                tau: 1.0,
            },
            2.0,
            1.5,
            EPSILON,
        );

        assert!(verification.passed, "Max error: {}", verification.max_error);
    }

    #[test]
    fn test_evolution_gradients_norm() {
        let mut grads = EvolutionGradients {
            d_initial: 3.0,
            d_rate: 4.0,
            d_time: 0.0,
            ..Default::default()
        };

        assert!((grads.norm() - 5.0).abs() < 1e-10);

        grads.clip(2.5);
        assert!((grads.norm() - 2.5).abs() < 1e-10);
    }

    #[test]
    fn test_evolution_gradients_accumulate() {
        let mut grads1 = EvolutionGradients {
            d_initial: 1.0,
            d_rate: 2.0,
            d_time: 3.0,
            ..Default::default()
        };

        let grads2 = EvolutionGradients {
            d_initial: 0.5,
            d_rate: 1.0,
            d_time: 1.5,
            ..Default::default()
        };

        grads1.accumulate(&grads2, 2.0);

        assert!((grads1.d_initial - 2.0).abs() < 1e-10);
        assert!((grads1.d_rate - 4.0).abs() < 1e-10);
        assert!((grads1.d_time - 6.0).abs() < 1e-10);
    }

    #[test]
    fn test_compute_evolution_gradient() {
        let (value, grads) =
            compute_evolution_gradient(EvolutionType::Linear { rate: 0.1 }, 5.0, 1.5);

        assert!((value - 2.0).abs() < 1e-10);
        assert!((grads.d_initial - 1.0).abs() < 1e-10);
        assert!((grads.d_rate - 5.0).abs() < 1e-10);
    }

    #[test]
    fn test_exponential_asymptote_gradient() {
        let model = ExponentialEvolutionGradient::new(0.5, 1.0);

        let d_asymp = model.gradient_asymptote(2.0, 2.0);

        // Numerical verification
        let eps = 1e-6;
        let model_plus = ExponentialEvolutionGradient::new(0.5, 1.0 + eps);
        let model_minus = ExponentialEvolutionGradient::new(0.5, 1.0 - eps);
        let numeric =
            (model_plus.evaluate(2.0, 2.0) - model_minus.evaluate(2.0, 2.0)) / (2.0 * eps);

        assert!(
            (d_asymp - numeric).abs() < TOLERANCE,
            "Analytic: {}, Numeric: {}",
            d_asymp,
            numeric
        );
    }
}
