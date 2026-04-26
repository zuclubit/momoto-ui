//! # Time Interpolation Utilities
//!
//! Smooth interpolation and rate limiting for temporal stability.

// ============================================================================
// INTERPOLATION FUNCTIONS
// ============================================================================

/// Smoothstep interpolation (cubic Hermite).
///
/// Returns 0 for t <= 0, 1 for t >= 1, smooth curve in between.
#[inline]
pub fn smoothstep(t: f64) -> f64 {
    if t <= 0.0 {
        0.0
    } else if t >= 1.0 {
        1.0
    } else {
        t * t * (3.0 - 2.0 * t)
    }
}

/// Smootherstep interpolation (quintic Hermite).
///
/// Smoother than smoothstep with zero second derivative at edges.
#[inline]
pub fn smootherstep(t: f64) -> f64 {
    if t <= 0.0 {
        0.0
    } else if t >= 1.0 {
        1.0
    } else {
        t * t * t * (t * (t * 6.0 - 15.0) + 10.0)
    }
}

/// Ease-in-out (smooth start and end).
#[inline]
pub fn ease_in_out(t: f64) -> f64 {
    if t < 0.5 {
        2.0 * t * t
    } else {
        1.0 - (-2.0 * t + 2.0).powi(2) / 2.0
    }
}

/// Linear interpolation.
#[inline]
pub fn lerp(a: f64, b: f64, t: f64) -> f64 {
    a + (b - a) * t.clamp(0.0, 1.0)
}

/// Inverse linear interpolation.
#[inline]
pub fn inverse_lerp(a: f64, b: f64, value: f64) -> f64 {
    if (b - a).abs() < 1e-10 {
        0.0
    } else {
        ((value - a) / (b - a)).clamp(0.0, 1.0)
    }
}

/// Remap value from one range to another.
#[inline]
pub fn remap(value: f64, in_min: f64, in_max: f64, out_min: f64, out_max: f64) -> f64 {
    let t = inverse_lerp(in_min, in_max, value);
    lerp(out_min, out_max, t)
}

// ============================================================================
// INTERPOLATION MODE
// ============================================================================

/// Interpolation mode for temporal transitions.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InterpolationMode {
    /// Linear interpolation.
    Linear,
    /// Smoothstep (cubic Hermite).
    Smoothstep,
    /// Smootherstep (quintic Hermite).
    Smootherstep,
    /// Ease-in-out.
    EaseInOut,
    /// Step function (no interpolation).
    Step,
}

impl Default for InterpolationMode {
    fn default() -> Self {
        InterpolationMode::Smoothstep
    }
}

impl InterpolationMode {
    /// Apply interpolation to a normalized parameter.
    pub fn apply(&self, t: f64) -> f64 {
        match self {
            InterpolationMode::Linear => t.clamp(0.0, 1.0),
            InterpolationMode::Smoothstep => smoothstep(t),
            InterpolationMode::Smootherstep => smootherstep(t),
            InterpolationMode::EaseInOut => ease_in_out(t),
            InterpolationMode::Step => {
                if t < 0.5 {
                    0.0
                } else {
                    1.0
                }
            }
        }
    }

    /// Interpolate between two values.
    pub fn interpolate(&self, a: f64, b: f64, t: f64) -> f64 {
        let smooth_t = self.apply(t);
        lerp(a, b, smooth_t)
    }
}

/// Interpolation state for multi-value transitions.
#[derive(Debug, Clone)]
pub struct Interpolation {
    /// Start value.
    pub start: f64,
    /// End value.
    pub end: f64,
    /// Start time.
    pub start_time: f64,
    /// Duration.
    pub duration: f64,
    /// Mode.
    pub mode: InterpolationMode,
}

impl Interpolation {
    /// Create new interpolation.
    pub fn new(start: f64, end: f64, start_time: f64, duration: f64) -> Self {
        Self {
            start,
            end,
            start_time,
            duration: duration.max(0.0),
            mode: InterpolationMode::default(),
        }
    }

    /// Set interpolation mode.
    pub fn with_mode(mut self, mode: InterpolationMode) -> Self {
        self.mode = mode;
        self
    }

    /// Evaluate at time.
    pub fn evaluate(&self, time: f64) -> f64 {
        if self.duration <= 0.0 {
            return self.end;
        }

        let t = (time - self.start_time) / self.duration;
        self.mode.interpolate(self.start, self.end, t)
    }

    /// Check if interpolation is complete.
    pub fn is_complete(&self, time: f64) -> bool {
        time >= self.start_time + self.duration
    }

    /// Get progress (0 to 1).
    pub fn progress(&self, time: f64) -> f64 {
        if self.duration <= 0.0 {
            1.0
        } else {
            ((time - self.start_time) / self.duration).clamp(0.0, 1.0)
        }
    }
}

// ============================================================================
// RATE LIMITER
// ============================================================================

/// Configuration for rate limiting.
#[derive(Debug, Clone)]
pub struct RateLimitConfig {
    /// Maximum change per second.
    pub max_rate: f64,
    /// Whether to use smooth limiting.
    pub smooth: bool,
}

impl Default for RateLimitConfig {
    fn default() -> Self {
        Self {
            max_rate: 1.0,
            smooth: true,
        }
    }
}

/// Rate limiter for smooth value transitions.
#[derive(Debug, Clone)]
pub struct RateLimiter {
    /// Current value.
    current: f64,
    /// Target value.
    target: f64,
    /// Configuration.
    config: RateLimitConfig,
    /// Last update time.
    last_time: f64,
}

impl Default for RateLimiter {
    fn default() -> Self {
        Self {
            current: 0.0,
            target: 0.0,
            config: RateLimitConfig::default(),
            last_time: 0.0,
        }
    }
}

impl RateLimiter {
    /// Create new rate limiter.
    pub fn new(initial: f64, config: RateLimitConfig) -> Self {
        Self {
            current: initial,
            target: initial,
            config,
            last_time: 0.0,
        }
    }

    /// Set target value.
    pub fn set_target(&mut self, target: f64) {
        self.target = target;
    }

    /// Update and get current value.
    pub fn update(&mut self, time: f64) -> f64 {
        let dt = (time - self.last_time).max(0.0);
        self.last_time = time;

        let delta = self.target - self.current;
        let max_delta = self.config.max_rate * dt;

        if delta.abs() <= max_delta {
            self.current = self.target;
        } else if self.config.smooth {
            // Smooth approach
            let approach = 1.0 - (-dt * self.config.max_rate).exp();
            self.current += delta * approach;
        } else {
            // Linear clamp
            self.current += delta.signum() * max_delta;
        }

        self.current
    }

    /// Get current value without updating.
    pub fn current(&self) -> f64 {
        self.current
    }

    /// Get target value.
    pub fn target(&self) -> f64 {
        self.target
    }

    /// Check if at target.
    pub fn at_target(&self) -> bool {
        (self.current - self.target).abs() < 1e-6
    }

    /// Reset to a specific value.
    pub fn reset(&mut self, value: f64, time: f64) {
        self.current = value;
        self.target = value;
        self.last_time = time;
    }
}

// ============================================================================
// EXPONENTIAL MOVING AVERAGE
// ============================================================================

/// Exponential moving average for smooth value tracking.
#[derive(Debug, Clone)]
pub struct ExponentialMovingAverage {
    /// Current smoothed value.
    value: f64,
    /// Smoothing factor (0-1, higher = faster response).
    alpha: f64,
    /// Whether initialized.
    initialized: bool,
}

impl ExponentialMovingAverage {
    /// Create new EMA with smoothing factor.
    pub fn new(alpha: f64) -> Self {
        Self {
            value: 0.0,
            alpha: alpha.clamp(0.0, 1.0),
            initialized: false,
        }
    }

    /// Create EMA from time constant (tau in seconds).
    pub fn from_time_constant(tau: f64, dt: f64) -> Self {
        let alpha = if tau > 0.0 {
            1.0 - (-dt / tau).exp()
        } else {
            1.0
        };
        Self::new(alpha)
    }

    /// Update with new sample.
    pub fn update(&mut self, sample: f64) -> f64 {
        if !self.initialized {
            self.value = sample;
            self.initialized = true;
        } else {
            self.value = self.alpha * sample + (1.0 - self.alpha) * self.value;
        }
        self.value
    }

    /// Get current value.
    pub fn value(&self) -> f64 {
        self.value
    }

    /// Reset.
    pub fn reset(&mut self) {
        self.value = 0.0;
        self.initialized = false;
    }
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_smoothstep() {
        assert!((smoothstep(-0.5) - 0.0).abs() < 1e-6);
        assert!((smoothstep(0.0) - 0.0).abs() < 1e-6);
        assert!((smoothstep(0.5) - 0.5).abs() < 1e-6);
        assert!((smoothstep(1.0) - 1.0).abs() < 1e-6);
        assert!((smoothstep(1.5) - 1.0).abs() < 1e-6);
    }

    #[test]
    fn test_smootherstep() {
        assert!((smootherstep(0.0) - 0.0).abs() < 1e-6);
        assert!((smootherstep(0.5) - 0.5).abs() < 1e-6);
        assert!((smootherstep(1.0) - 1.0).abs() < 1e-6);
    }

    #[test]
    fn test_lerp() {
        assert!((lerp(0.0, 10.0, 0.5) - 5.0).abs() < 1e-6);
        assert!((lerp(0.0, 10.0, 0.0) - 0.0).abs() < 1e-6);
        assert!((lerp(0.0, 10.0, 1.0) - 10.0).abs() < 1e-6);
    }

    #[test]
    fn test_inverse_lerp() {
        assert!((inverse_lerp(0.0, 10.0, 5.0) - 0.5).abs() < 1e-6);
        assert!((inverse_lerp(0.0, 10.0, 0.0) - 0.0).abs() < 1e-6);
        assert!((inverse_lerp(0.0, 10.0, 10.0) - 1.0).abs() < 1e-6);
    }

    #[test]
    fn test_interpolation() {
        let interp = Interpolation::new(0.0, 10.0, 0.0, 1.0);

        assert!((interp.evaluate(0.0) - 0.0).abs() < 1e-6);
        assert!((interp.evaluate(1.0) - 10.0).abs() < 1e-6);
        assert!(interp.evaluate(0.5) > 0.0 && interp.evaluate(0.5) < 10.0);
    }

    #[test]
    fn test_interpolation_modes() {
        let modes = [
            InterpolationMode::Linear,
            InterpolationMode::Smoothstep,
            InterpolationMode::Smootherstep,
            InterpolationMode::EaseInOut,
            InterpolationMode::Step,
        ];

        for mode in modes {
            let result = mode.apply(0.5);
            assert!(result >= 0.0 && result <= 1.0);
        }
    }

    #[test]
    fn test_rate_limiter() {
        let config = RateLimitConfig {
            max_rate: 10.0,
            smooth: false,
        };
        let mut limiter = RateLimiter::new(0.0, config);

        limiter.set_target(100.0);
        let v1 = limiter.update(0.0);
        let v2 = limiter.update(0.1);

        assert_eq!(v1, 0.0);
        assert!(v2 > 0.0 && v2 <= 1.0); // Max 10 * 0.1 = 1.0
    }

    #[test]
    fn test_ema() {
        let mut ema = ExponentialMovingAverage::new(0.5);

        let v1 = ema.update(10.0);
        assert!((v1 - 10.0).abs() < 1e-6); // First sample

        let v2 = ema.update(20.0);
        assert!((v2 - 15.0).abs() < 1e-6); // 0.5 * 20 + 0.5 * 10 = 15
    }

    #[test]
    fn test_remap() {
        assert!((remap(5.0, 0.0, 10.0, 0.0, 100.0) - 50.0).abs() < 1e-6);
        assert!((remap(0.0, 0.0, 10.0, 100.0, 200.0) - 100.0).abs() < 1e-6);
    }
}
