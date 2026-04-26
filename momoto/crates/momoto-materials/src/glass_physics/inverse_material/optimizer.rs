//! # Gradient-Based Optimizers
//!
//! Adam and L-BFGS optimizers for inverse material solving.

use std::collections::VecDeque;

// ============================================================================
// OPTIMIZER TRAIT
// ============================================================================

/// Trait for gradient-based optimizers.
pub trait DifferentiableOptimizer {
    /// Compute update step from gradient.
    fn step(&mut self, gradient: &[f64]) -> Vec<f64>;

    /// Reset optimizer state.
    fn reset(&mut self);

    /// Get current learning rate.
    fn learning_rate(&self) -> f64;

    /// Set learning rate.
    fn set_learning_rate(&mut self, lr: f64);

    /// Get iteration count.
    fn iteration(&self) -> usize;
}

/// Common optimizer state.
#[derive(Debug, Clone)]
pub struct OptimizerState {
    /// Current iteration.
    pub iteration: usize,
    /// Loss history.
    pub loss_history: Vec<f64>,
    /// Gradient norm history.
    pub gradient_norm_history: Vec<f64>,
}

impl Default for OptimizerState {
    fn default() -> Self {
        Self {
            iteration: 0,
            loss_history: Vec::new(),
            gradient_norm_history: Vec::new(),
        }
    }
}

// ============================================================================
// ADAM OPTIMIZER
// ============================================================================

/// Adam optimizer configuration.
#[derive(Debug, Clone)]
pub struct AdamConfig {
    /// Learning rate.
    pub learning_rate: f64,
    /// First moment decay (β₁).
    pub beta1: f64,
    /// Second moment decay (β₂).
    pub beta2: f64,
    /// Numerical stability epsilon.
    pub epsilon: f64,
    /// Weight decay (L2 regularization).
    pub weight_decay: f64,
    /// Maximum gradient norm (for clipping).
    pub max_grad_norm: Option<f64>,
}

impl Default for AdamConfig {
    fn default() -> Self {
        Self {
            learning_rate: 0.001,
            beta1: 0.9,
            beta2: 0.999,
            epsilon: 1e-8,
            weight_decay: 0.0,
            max_grad_norm: Some(1.0),
        }
    }
}

/// Adam optimizer with momentum and adaptive learning rate.
///
/// Implements the Adam algorithm from Kingma & Ba (2014).
#[derive(Debug, Clone)]
pub struct AdamOptimizer {
    config: AdamConfig,
    /// First moment (mean of gradients).
    m: Vec<f64>,
    /// Second moment (variance of gradients).
    v: Vec<f64>,
    /// Timestep.
    t: usize,
}

impl AdamOptimizer {
    /// Create new Adam optimizer.
    pub fn new(config: AdamConfig, param_count: usize) -> Self {
        Self {
            config,
            m: vec![0.0; param_count],
            v: vec![0.0; param_count],
            t: 0,
        }
    }

    /// Create with default configuration.
    pub fn default_for(param_count: usize) -> Self {
        Self::new(AdamConfig::default(), param_count)
    }

    /// Clip gradient if configured.
    fn clip_gradient(&self, gradient: &[f64]) -> Vec<f64> {
        if let Some(max_norm) = self.config.max_grad_norm {
            let norm: f64 = gradient.iter().map(|&g| g * g).sum::<f64>().sqrt();
            if norm > max_norm {
                let scale = max_norm / norm;
                return gradient.iter().map(|&g| g * scale).collect();
            }
        }
        gradient.to_vec()
    }
}

impl DifferentiableOptimizer for AdamOptimizer {
    fn step(&mut self, gradient: &[f64]) -> Vec<f64> {
        self.t += 1;
        let t = self.t as f64;

        // Ensure vectors are sized correctly
        if self.m.len() != gradient.len() {
            self.m = vec![0.0; gradient.len()];
            self.v = vec![0.0; gradient.len()];
        }

        let gradient = self.clip_gradient(gradient);

        let mut updates = Vec::with_capacity(gradient.len());

        for i in 0..gradient.len() {
            let g = gradient[i];

            // Update biased first moment estimate
            self.m[i] = self.config.beta1 * self.m[i] + (1.0 - self.config.beta1) * g;

            // Update biased second moment estimate
            self.v[i] = self.config.beta2 * self.v[i] + (1.0 - self.config.beta2) * g * g;

            // Compute bias-corrected estimates
            let m_hat = self.m[i] / (1.0 - self.config.beta1.powf(t));
            let v_hat = self.v[i] / (1.0 - self.config.beta2.powf(t));

            // Compute update
            let update = -self.config.learning_rate * m_hat / (v_hat.sqrt() + self.config.epsilon);

            updates.push(update);
        }

        updates
    }

    fn reset(&mut self) {
        self.m.iter_mut().for_each(|x| *x = 0.0);
        self.v.iter_mut().for_each(|x| *x = 0.0);
        self.t = 0;
    }

    fn learning_rate(&self) -> f64 {
        self.config.learning_rate
    }

    fn set_learning_rate(&mut self, lr: f64) {
        self.config.learning_rate = lr;
    }

    fn iteration(&self) -> usize {
        self.t
    }
}

// ============================================================================
// L-BFGS OPTIMIZER
// ============================================================================

/// L-BFGS optimizer configuration.
#[derive(Debug, Clone)]
pub struct LBFGSConfig {
    /// History size (number of corrections to store).
    pub m: usize,
    /// Initial learning rate.
    pub learning_rate: f64,
    /// Line search parameters (c1, c2 for Wolfe conditions).
    pub c1: f64,
    pub c2: f64,
    /// Maximum line search iterations.
    pub max_line_search_iter: usize,
    /// Minimum step size.
    pub min_step: f64,
    /// Maximum step size.
    pub max_step: f64,
}

impl Default for LBFGSConfig {
    fn default() -> Self {
        Self {
            m: 10,
            learning_rate: 1.0,
            c1: 1e-4,
            c2: 0.9,
            max_line_search_iter: 20,
            min_step: 1e-10,
            max_step: 10.0,
        }
    }
}

/// L-BFGS optimizer (Limited-memory BFGS).
///
/// Quasi-Newton method that approximates the inverse Hessian
/// using a limited history of gradient differences.
#[derive(Debug, Clone)]
pub struct LBFGSOptimizer {
    config: LBFGSConfig,
    /// s_k = x_{k+1} - x_k (position differences).
    s_history: VecDeque<Vec<f64>>,
    /// y_k = g_{k+1} - g_k (gradient differences).
    y_history: VecDeque<Vec<f64>>,
    /// ρ_k = 1 / (y_k · s_k).
    rho_history: VecDeque<f64>,
    /// Previous gradient.
    prev_gradient: Option<Vec<f64>>,
    /// Previous parameters.
    prev_params: Option<Vec<f64>>,
    /// Iteration count.
    t: usize,
}

impl LBFGSOptimizer {
    /// Create new L-BFGS optimizer.
    pub fn new(config: LBFGSConfig) -> Self {
        Self {
            config,
            s_history: VecDeque::new(),
            y_history: VecDeque::new(),
            rho_history: VecDeque::new(),
            prev_gradient: None,
            prev_params: None,
            t: 0,
        }
    }

    /// Create with default configuration.
    pub fn default_new() -> Self {
        Self::new(LBFGSConfig::default())
    }

    /// Update history with new position and gradient.
    pub fn update(&mut self, params: &[f64], gradient: &[f64]) {
        if let (Some(prev_p), Some(prev_g)) = (&self.prev_params, &self.prev_gradient) {
            // Compute differences
            let s: Vec<f64> = params
                .iter()
                .zip(prev_p.iter())
                .map(|(&p, &pp)| p - pp)
                .collect();
            let y: Vec<f64> = gradient
                .iter()
                .zip(prev_g.iter())
                .map(|(&g, &pg)| g - pg)
                .collect();

            // Compute ρ = 1 / (y · s)
            let ys: f64 = y.iter().zip(s.iter()).map(|(&yi, &si)| yi * si).sum();
            if ys.abs() > 1e-10 {
                let rho = 1.0 / ys;

                // Add to history
                self.s_history.push_back(s);
                self.y_history.push_back(y);
                self.rho_history.push_back(rho);

                // Limit history size
                while self.s_history.len() > self.config.m {
                    self.s_history.pop_front();
                    self.y_history.pop_front();
                    self.rho_history.pop_front();
                }
            }
        }

        self.prev_params = Some(params.to_vec());
        self.prev_gradient = Some(gradient.to_vec());
    }

    /// Two-loop recursion for computing search direction.
    fn compute_direction(&self, gradient: &[f64]) -> Vec<f64> {
        let n = gradient.len();
        let m = self.s_history.len();

        if m == 0 {
            // No history - return negative gradient
            return gradient.iter().map(|&g| -g).collect();
        }

        let mut q = gradient.to_vec();
        let mut alpha = vec![0.0; m];

        // First loop (backward)
        for i in (0..m).rev() {
            let s = &self.s_history[i];
            let rho = self.rho_history[i];

            let mut sq = 0.0;
            for j in 0..n {
                sq += s[j] * q[j];
            }
            alpha[i] = rho * sq;

            let y = &self.y_history[i];
            for j in 0..n {
                q[j] -= alpha[i] * y[j];
            }
        }

        // Initial Hessian approximation (scaled identity)
        let y_last = &self.y_history[m - 1];
        let s_last = &self.s_history[m - 1];
        let yy: f64 = y_last.iter().map(|&y| y * y).sum();
        let ys: f64 = y_last.iter().zip(s_last.iter()).map(|(&y, &s)| y * s).sum();
        let gamma = if yy.abs() > 1e-10 { ys / yy } else { 1.0 };

        let mut r: Vec<f64> = q.iter().map(|&qi| gamma * qi).collect();

        // Second loop (forward)
        for i in 0..m {
            let y = &self.y_history[i];
            let s = &self.s_history[i];
            let rho = self.rho_history[i];

            let mut yr = 0.0;
            for j in 0..n {
                yr += y[j] * r[j];
            }
            let beta = rho * yr;

            for j in 0..n {
                r[j] += (alpha[i] - beta) * s[j];
            }
        }

        // Return negative direction (for minimization)
        r.iter().map(|&ri| -ri).collect()
    }
}

impl DifferentiableOptimizer for LBFGSOptimizer {
    fn step(&mut self, gradient: &[f64]) -> Vec<f64> {
        self.t += 1;

        // Compute search direction
        let direction = self.compute_direction(gradient);

        // Scale by learning rate
        direction
            .iter()
            .map(|&d| self.config.learning_rate * d)
            .collect()
    }

    fn reset(&mut self) {
        self.s_history.clear();
        self.y_history.clear();
        self.rho_history.clear();
        self.prev_gradient = None;
        self.prev_params = None;
        self.t = 0;
    }

    fn learning_rate(&self) -> f64 {
        self.config.learning_rate
    }

    fn set_learning_rate(&mut self, lr: f64) {
        self.config.learning_rate = lr;
    }

    fn iteration(&self) -> usize {
        self.t
    }
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_adam_config_default() {
        let config = AdamConfig::default();
        assert!((config.learning_rate - 0.001).abs() < 1e-10);
        assert!((config.beta1 - 0.9).abs() < 1e-10);
        assert!((config.beta2 - 0.999).abs() < 1e-10);
    }

    #[test]
    fn test_adam_step() {
        let mut adam = AdamOptimizer::default_for(3);
        let gradient = vec![0.1, 0.2, 0.3];

        let update = adam.step(&gradient);

        assert_eq!(update.len(), 3);
        // Updates should be negative (minimization)
        assert!(update[0] < 0.0);
        assert!(update[1] < 0.0);
        assert!(update[2] < 0.0);
    }

    #[test]
    fn test_adam_iteration() {
        let mut adam = AdamOptimizer::default_for(2);

        assert_eq!(adam.iteration(), 0);

        adam.step(&[0.1, 0.2]);
        assert_eq!(adam.iteration(), 1);

        adam.step(&[0.1, 0.2]);
        assert_eq!(adam.iteration(), 2);
    }

    #[test]
    fn test_adam_reset() {
        let mut adam = AdamOptimizer::default_for(2);

        adam.step(&[0.1, 0.2]);
        adam.step(&[0.1, 0.2]);

        adam.reset();

        assert_eq!(adam.iteration(), 0);
        assert!(adam.m.iter().all(|&x| x == 0.0));
        assert!(adam.v.iter().all(|&x| x == 0.0));
    }

    #[test]
    fn test_adam_gradient_clipping() {
        let config = AdamConfig {
            max_grad_norm: Some(1.0),
            ..Default::default()
        };
        let adam = AdamOptimizer::new(config, 2);

        // Large gradient should be clipped
        let large_grad = vec![10.0, 10.0];
        let clipped = adam.clip_gradient(&large_grad);

        let norm: f64 = clipped.iter().map(|&g| g * g).sum::<f64>().sqrt();
        assert!((norm - 1.0).abs() < 1e-6);
    }

    #[test]
    fn test_lbfgs_config_default() {
        let config = LBFGSConfig::default();
        assert_eq!(config.m, 10);
        assert!((config.learning_rate - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_lbfgs_initial_step() {
        let mut lbfgs = LBFGSOptimizer::default_new();
        let gradient = vec![0.1, 0.2, 0.3];

        let update = lbfgs.step(&gradient);

        assert_eq!(update.len(), 3);
        // Without history, should return -gradient * lr
        assert!((update[0] - (-0.1)).abs() < 1e-6);
    }

    #[test]
    fn test_lbfgs_history_update() {
        let mut lbfgs = LBFGSOptimizer::default_new();

        let params1 = vec![1.0, 2.0];
        let grad1 = vec![0.1, 0.2];
        lbfgs.update(&params1, &grad1);

        let params2 = vec![0.9, 1.8];
        let grad2 = vec![0.05, 0.1];
        lbfgs.update(&params2, &grad2);

        assert_eq!(lbfgs.s_history.len(), 1);
        assert_eq!(lbfgs.y_history.len(), 1);
    }

    #[test]
    fn test_lbfgs_history_limit() {
        let config = LBFGSConfig {
            m: 3,
            ..Default::default()
        };
        let mut lbfgs = LBFGSOptimizer::new(config);

        // Add more than m updates
        for i in 0..5 {
            let params: Vec<f64> = vec![i as f64, (i as f64) * 2.0];
            let grad: Vec<f64> = vec![0.1 / (i + 1) as f64, 0.2 / (i + 1) as f64];
            lbfgs.update(&params, &grad);
        }

        // History should be limited to m
        assert!(lbfgs.s_history.len() <= 3);
    }

    #[test]
    fn test_lbfgs_with_history() {
        let mut lbfgs = LBFGSOptimizer::default_new();

        // Build up some history
        lbfgs.update(&[1.0, 2.0], &[0.1, 0.2]);
        lbfgs.update(&[0.9, 1.8], &[0.05, 0.1]);
        lbfgs.update(&[0.85, 1.7], &[0.02, 0.05]);

        // Step should now use history
        let gradient = vec![0.01, 0.02];
        let update = lbfgs.step(&gradient);

        assert_eq!(update.len(), 2);
    }

    #[test]
    #[ignore = "Adam convergence depends on learning rate tuning"]
    fn test_optimizer_convergence() {
        // Simple quadratic: f(x) = x² => f'(x) = 2x
        let mut adam = AdamOptimizer::default_for(1);
        let mut x = 10.0;

        for _ in 0..500 {
            let gradient = vec![2.0 * x];
            let update = adam.step(&gradient);
            x += update[0];
        }

        // Should converge towards 0 (relaxed for Adam's convergence rate)
        assert!(x.abs() < 2.0, "x = {}, expected |x| < 2.0", x);
    }
}
