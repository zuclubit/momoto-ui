//! # Bootstrap Resampling
//!
//! Non-parametric confidence interval estimation.

// ============================================================================
// CONFIDENCE INTERVAL
// ============================================================================

/// Confidence interval for a parameter.
#[derive(Debug, Clone, Copy)]
pub struct ConfidenceInterval {
    /// Lower bound.
    pub lower: f64,
    /// Upper bound.
    pub upper: f64,
    /// Point estimate (usually median or mean).
    pub estimate: f64,
    /// Confidence level (e.g., 0.95 for 95%).
    pub level: f64,
}

impl ConfidenceInterval {
    /// Create new confidence interval.
    pub fn new(lower: f64, upper: f64, estimate: f64, level: f64) -> Self {
        Self {
            lower,
            upper,
            estimate,
            level,
        }
    }

    /// Get interval width.
    pub fn width(&self) -> f64 {
        self.upper - self.lower
    }

    /// Check if value is within interval.
    pub fn contains(&self, value: f64) -> bool {
        value >= self.lower && value <= self.upper
    }

    /// Get relative error (half-width / estimate).
    pub fn relative_error(&self) -> f64 {
        if self.estimate.abs() < 1e-10 {
            return f64::INFINITY;
        }
        self.width() / (2.0 * self.estimate.abs())
    }

    /// Format as string.
    pub fn format(&self) -> String {
        format!(
            "{:.4} [{:.4}, {:.4}] ({:.0}% CI)",
            self.estimate,
            self.lower,
            self.upper,
            self.level * 100.0
        )
    }
}

// ============================================================================
// BOOTSTRAP CONFIG
// ============================================================================

/// Configuration for bootstrap resampling.
#[derive(Debug, Clone)]
pub struct BootstrapConfig {
    /// Number of bootstrap samples.
    pub n_samples: usize,
    /// Confidence level (e.g., 0.95).
    pub confidence_level: f64,
    /// Random seed (None for random).
    pub seed: Option<u64>,
    /// Use bias-corrected accelerated (BCa) method.
    pub use_bca: bool,
    /// Minimum samples required.
    pub min_data_points: usize,
}

impl Default for BootstrapConfig {
    fn default() -> Self {
        Self {
            n_samples: 100,
            confidence_level: 0.95,
            seed: None,
            use_bca: false,
            min_data_points: 10,
        }
    }
}

impl BootstrapConfig {
    /// Create with custom sample count.
    pub fn with_samples(n: usize) -> Self {
        Self {
            n_samples: n,
            ..Default::default()
        }
    }

    /// Set confidence level.
    pub fn with_confidence(mut self, level: f64) -> Self {
        self.confidence_level = level.clamp(0.5, 0.999);
        self
    }

    /// Set random seed.
    pub fn with_seed(mut self, seed: u64) -> Self {
        self.seed = Some(seed);
        self
    }

    /// Enable BCa method.
    pub fn with_bca(mut self) -> Self {
        self.use_bca = true;
        self
    }

    /// Quick config (fewer samples, faster).
    pub fn quick() -> Self {
        Self {
            n_samples: 50,
            confidence_level: 0.95,
            seed: None,
            use_bca: false,
            min_data_points: 5,
        }
    }

    /// Thorough config (more samples, better accuracy).
    pub fn thorough() -> Self {
        Self {
            n_samples: 1000,
            confidence_level: 0.95,
            seed: None,
            use_bca: true,
            min_data_points: 20,
        }
    }
}

// ============================================================================
// BOOTSTRAP RESULT
// ============================================================================

/// Result from bootstrap resampling.
#[derive(Debug, Clone)]
pub struct BootstrapResult {
    /// Point estimate.
    pub estimate: f64,
    /// Standard error.
    pub standard_error: f64,
    /// Confidence interval.
    pub ci: ConfidenceInterval,
    /// All bootstrap samples (sorted).
    pub samples: Vec<f64>,
    /// Bias estimate.
    pub bias: f64,
    /// Method used.
    pub method: String,
}

impl BootstrapResult {
    /// Get median of bootstrap distribution.
    pub fn median(&self) -> f64 {
        if self.samples.is_empty() {
            return self.estimate;
        }
        let n = self.samples.len();
        if n % 2 == 0 {
            (self.samples[n / 2 - 1] + self.samples[n / 2]) / 2.0
        } else {
            self.samples[n / 2]
        }
    }

    /// Get percentile from bootstrap distribution.
    pub fn percentile(&self, p: f64) -> f64 {
        if self.samples.is_empty() {
            return self.estimate;
        }
        let idx = ((p * self.samples.len() as f64) as usize).min(self.samples.len() - 1);
        self.samples[idx]
    }

    /// Check if estimate is significantly different from zero.
    pub fn is_significant(&self) -> bool {
        !self.ci.contains(0.0)
    }

    /// Get coefficient of variation.
    pub fn cv(&self) -> f64 {
        if self.estimate.abs() < 1e-10 {
            return f64::INFINITY;
        }
        self.standard_error / self.estimate.abs()
    }
}

// ============================================================================
// BOOTSTRAP RESAMPLER
// ============================================================================

/// Bootstrap resampler for confidence interval estimation.
#[derive(Debug, Clone)]
pub struct BootstrapResampler {
    /// Configuration.
    config: BootstrapConfig,
    /// PRNG state.
    rng_state: u64,
}

impl BootstrapResampler {
    /// Create new resampler with default config.
    pub fn new() -> Self {
        Self::with_config(BootstrapConfig::default())
    }

    /// Create with custom config.
    pub fn with_config(config: BootstrapConfig) -> Self {
        let rng_state = config.seed.unwrap_or_else(|| {
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos() as u64)
                .unwrap_or(12345)
        });

        Self { config, rng_state }
    }

    /// Simple PRNG (xorshift64).
    fn next_random(&mut self) -> u64 {
        self.rng_state ^= self.rng_state << 13;
        self.rng_state ^= self.rng_state >> 7;
        self.rng_state ^= self.rng_state << 17;
        self.rng_state
    }

    /// Get random index in range [0, max).
    fn random_index(&mut self, max: usize) -> usize {
        (self.next_random() % max as u64) as usize
    }

    /// Bootstrap for mean estimation.
    pub fn bootstrap_mean(&mut self, data: &[f64]) -> BootstrapResult {
        self.bootstrap_statistic(data, |sample| {
            sample.iter().sum::<f64>() / sample.len() as f64
        })
    }

    /// Bootstrap for any statistic.
    pub fn bootstrap_statistic(
        &mut self,
        data: &[f64],
        statistic: impl Fn(&[f64]) -> f64,
    ) -> BootstrapResult {
        if data.len() < self.config.min_data_points {
            let estimate = statistic(data);
            return BootstrapResult {
                estimate,
                standard_error: f64::NAN,
                ci: ConfidenceInterval::new(
                    estimate,
                    estimate,
                    estimate,
                    self.config.confidence_level,
                ),
                samples: vec![estimate],
                bias: 0.0,
                method: "insufficient_data".to_string(),
            };
        }

        let n = data.len();
        let original_estimate = statistic(data);
        let mut bootstrap_estimates = Vec::with_capacity(self.config.n_samples);

        // Generate bootstrap samples
        let mut resample = vec![0.0; n];
        for _ in 0..self.config.n_samples {
            // Resample with replacement
            for r in resample.iter_mut() {
                *r = data[self.random_index(n)];
            }
            let est = statistic(&resample);
            if est.is_finite() {
                bootstrap_estimates.push(est);
            }
        }

        // Sort for percentile computation
        bootstrap_estimates.sort_by(|a, b| a.partial_cmp(b).unwrap());

        if bootstrap_estimates.is_empty() {
            return BootstrapResult {
                estimate: original_estimate,
                standard_error: f64::NAN,
                ci: ConfidenceInterval::new(
                    original_estimate,
                    original_estimate,
                    original_estimate,
                    self.config.confidence_level,
                ),
                samples: Vec::new(),
                bias: 0.0,
                method: "failed".to_string(),
            };
        }

        // Compute statistics
        let mean = bootstrap_estimates.iter().sum::<f64>() / bootstrap_estimates.len() as f64;
        let variance = bootstrap_estimates
            .iter()
            .map(|x| (x - mean).powi(2))
            .sum::<f64>()
            / (bootstrap_estimates.len() - 1) as f64;
        let standard_error = variance.sqrt();
        let bias = mean - original_estimate;

        // Confidence interval
        let ci = if self.config.use_bca {
            self.bca_interval(&bootstrap_estimates, original_estimate, data, &statistic)
        } else {
            bootstrap_percentile(&bootstrap_estimates, self.config.confidence_level)
        };

        BootstrapResult {
            estimate: original_estimate,
            standard_error,
            ci: ConfidenceInterval::new(
                ci.0,
                ci.1,
                original_estimate,
                self.config.confidence_level,
            ),
            samples: bootstrap_estimates,
            bias,
            method: if self.config.use_bca {
                "BCa"
            } else {
                "percentile"
            }
            .to_string(),
        }
    }

    /// BCa (bias-corrected accelerated) confidence interval.
    fn bca_interval(
        &mut self,
        bootstrap_samples: &[f64],
        original: f64,
        data: &[f64],
        statistic: &impl Fn(&[f64]) -> f64,
    ) -> (f64, f64) {
        let n = bootstrap_samples.len();
        let alpha = (1.0 - self.config.confidence_level) / 2.0;

        // Bias correction factor z0
        let prop_less =
            bootstrap_samples.iter().filter(|&&x| x < original).count() as f64 / n as f64;
        let z0 = quantile_normal(prop_less);

        // Acceleration factor (from jackknife)
        let acceleration = self.compute_acceleration(data, statistic);

        // Adjusted percentiles
        let z_alpha = quantile_normal(alpha);
        let z_1_alpha = quantile_normal(1.0 - alpha);

        let alpha1 = phi(z0 + (z0 + z_alpha) / (1.0 - acceleration * (z0 + z_alpha)));
        let alpha2 = phi(z0 + (z0 + z_1_alpha) / (1.0 - acceleration * (z0 + z_1_alpha)));

        // Get adjusted percentiles
        let lower_idx = ((alpha1 * n as f64) as usize).clamp(0, n - 1);
        let upper_idx = ((alpha2 * n as f64) as usize).clamp(0, n - 1);

        (bootstrap_samples[lower_idx], bootstrap_samples[upper_idx])
    }

    /// Compute acceleration factor from jackknife.
    fn compute_acceleration(&self, data: &[f64], statistic: &impl Fn(&[f64]) -> f64) -> f64 {
        let n = data.len();
        if n < 3 {
            return 0.0;
        }

        // Jackknife estimates
        let mut jackknife = Vec::with_capacity(n);
        let mut sample = data.to_vec();

        for i in 0..n {
            let removed = sample.remove(i);
            jackknife.push(statistic(&sample));
            sample.insert(i, removed);
        }

        let mean = jackknife.iter().sum::<f64>() / n as f64;

        let num: f64 = jackknife.iter().map(|x| (mean - x).powi(3)).sum();
        let den: f64 = jackknife.iter().map(|x| (mean - x).powi(2)).sum();

        if den.abs() < 1e-15 {
            return 0.0;
        }

        num / (6.0 * den.powf(1.5))
    }
}

impl Default for BootstrapResampler {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// HELPER FUNCTIONS
// ============================================================================

/// Compute percentile-based confidence interval.
pub fn bootstrap_percentile(sorted_samples: &[f64], confidence: f64) -> (f64, f64) {
    if sorted_samples.is_empty() {
        return (0.0, 0.0);
    }

    let n = sorted_samples.len();
    let alpha = (1.0 - confidence) / 2.0;

    let lower_idx = ((alpha * n as f64) as usize).min(n - 1);
    let upper_idx = (((1.0 - alpha) * n as f64) as usize).min(n - 1);

    (sorted_samples[lower_idx], sorted_samples[upper_idx])
}

/// Compute BCa confidence interval.
pub fn bootstrap_bca(
    data: &[f64],
    statistic: impl Fn(&[f64]) -> f64,
    config: &BootstrapConfig,
) -> BootstrapResult {
    let mut resampler = BootstrapResampler::with_config(BootstrapConfig {
        use_bca: true,
        ..config.clone()
    });
    resampler.bootstrap_statistic(data, statistic)
}

/// Standard normal quantile (inverse CDF).
fn quantile_normal(p: f64) -> f64 {
    // Approximation using rational function
    if p <= 0.0 || p >= 1.0 {
        return if p <= 0.0 {
            f64::NEG_INFINITY
        } else {
            f64::INFINITY
        };
    }

    let a = [
        -3.969683028665376e+01,
        2.209460984245205e+02,
        -2.759285104469687e+02,
        1.383577518672690e+02,
        -3.066479806614716e+01,
        2.506628277459239e+00,
    ];
    let b = [
        -5.447609879822406e+01,
        1.615858368580409e+02,
        -1.556989798598866e+02,
        6.680131188771972e+01,
        -1.328068155288572e+01,
    ];
    let c = [
        -7.784894002430293e-03,
        -3.223964580411365e-01,
        -2.400758277161838e+00,
        -2.549732539343734e+00,
        4.374664141464968e+00,
        2.938163982698783e+00,
    ];
    let d = [
        7.784695709041462e-03,
        3.224671290700398e-01,
        2.445134137142996e+00,
        3.754408661907416e+00,
    ];

    let p_low = 0.02425;
    let p_high = 1.0 - p_low;

    if p < p_low {
        let q = (-2.0 * p.ln()).sqrt();
        (((((c[0] * q + c[1]) * q + c[2]) * q + c[3]) * q + c[4]) * q + c[5])
            / ((((d[0] * q + d[1]) * q + d[2]) * q + d[3]) * q + 1.0)
    } else if p <= p_high {
        let q = p - 0.5;
        let r = q * q;
        (((((a[0] * r + a[1]) * r + a[2]) * r + a[3]) * r + a[4]) * r + a[5]) * q
            / (((((b[0] * r + b[1]) * r + b[2]) * r + b[3]) * r + b[4]) * r + 1.0)
    } else {
        let q = (-2.0 * (1.0 - p).ln()).sqrt();
        -(((((c[0] * q + c[1]) * q + c[2]) * q + c[3]) * q + c[4]) * q + c[5])
            / ((((d[0] * q + d[1]) * q + d[2]) * q + d[3]) * q + 1.0)
    }
}

/// Standard normal CDF.
fn phi(x: f64) -> f64 {
    0.5 * (1.0 + erf(x / std::f64::consts::SQRT_2))
}

/// Error function approximation.
fn erf(x: f64) -> f64 {
    let a1 = 0.254829592;
    let a2 = -0.284496736;
    let a3 = 1.421413741;
    let a4 = -1.453152027;
    let a5 = 1.061405429;
    let p = 0.3275911;

    let sign = if x < 0.0 { -1.0 } else { 1.0 };
    let x = x.abs();
    let t = 1.0 / (1.0 + p * x);
    let y = 1.0 - (((((a5 * t + a4) * t) + a3) * t + a2) * t + a1) * t * (-x * x).exp();

    sign * y
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_confidence_interval() {
        let ci = ConfidenceInterval::new(0.9, 1.1, 1.0, 0.95);
        assert!((ci.width() - 0.2).abs() < 1e-10);
        assert!(ci.contains(1.0));
        assert!(!ci.contains(0.5));
    }

    #[test]
    fn test_bootstrap_config_default() {
        let config = BootstrapConfig::default();
        assert_eq!(config.n_samples, 100);
        assert!((config.confidence_level - 0.95).abs() < 1e-10);
    }

    #[test]
    fn test_bootstrap_mean() {
        let data: Vec<f64> = (0..50).map(|i| i as f64).collect();
        let mut resampler =
            BootstrapResampler::with_config(BootstrapConfig::with_samples(100).with_seed(42));

        let result = resampler.bootstrap_mean(&data);

        // Mean should be around 24.5
        assert!((result.estimate - 24.5).abs() < 0.1);
        assert!(result.standard_error > 0.0);
    }

    #[test]
    fn test_bootstrap_percentile() {
        let samples: Vec<f64> = (0..100).map(|i| i as f64).collect();
        let (lower, upper) = bootstrap_percentile(&samples, 0.95);

        // 95% CI should exclude ~5% on each side
        assert!(lower < 10.0);
        assert!(upper > 90.0);
    }

    #[test]
    fn test_bootstrap_result_median() {
        let result = BootstrapResult {
            estimate: 5.0,
            standard_error: 0.5,
            ci: ConfidenceInterval::new(4.0, 6.0, 5.0, 0.95),
            samples: vec![1.0, 2.0, 3.0, 4.0, 5.0],
            bias: 0.1,
            method: "percentile".to_string(),
        };

        assert!((result.median() - 3.0).abs() < 1e-10);
    }

    #[test]
    fn test_bootstrap_significance() {
        let significant = BootstrapResult {
            estimate: 5.0,
            standard_error: 0.5,
            ci: ConfidenceInterval::new(4.0, 6.0, 5.0, 0.95),
            samples: vec![5.0],
            bias: 0.0,
            method: "test".to_string(),
        };
        assert!(significant.is_significant());

        let not_significant = BootstrapResult {
            estimate: 0.5,
            standard_error: 1.0,
            ci: ConfidenceInterval::new(-0.5, 1.5, 0.5, 0.95),
            samples: vec![0.5],
            bias: 0.0,
            method: "test".to_string(),
        };
        assert!(!not_significant.is_significant());
    }

    #[test]
    fn test_quantile_normal() {
        // Check some known quantiles
        assert!((quantile_normal(0.5) - 0.0).abs() < 0.01);
        assert!((quantile_normal(0.975) - 1.96).abs() < 0.01);
        assert!((quantile_normal(0.025) - (-1.96)).abs() < 0.01);
    }

    #[test]
    fn test_insufficient_data() {
        let data = vec![1.0, 2.0, 3.0]; // Less than min_data_points
        let mut resampler = BootstrapResampler::new();
        let result = resampler.bootstrap_mean(&data);

        assert_eq!(result.method, "insufficient_data");
    }

    #[test]
    fn test_bootstrap_bca() {
        let data: Vec<f64> = (0..100).map(|i| i as f64).collect();
        let config = BootstrapConfig::with_samples(50).with_bca().with_seed(42);
        let result = bootstrap_bca(&data, |s| s.iter().sum::<f64>() / s.len() as f64, &config);

        assert_eq!(result.method, "BCa");
        assert!((result.estimate - 49.5).abs() < 0.1);
    }
}
