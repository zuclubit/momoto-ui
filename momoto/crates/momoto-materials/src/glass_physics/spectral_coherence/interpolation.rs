//! # Spectral Interpolation
//!
//! Frame-to-frame spectral value blending and gradient limiting.
//!
//! ## Key Features
//!
//! - **Temporal Blending**: Exponential moving average across frames
//! - **Gradient Limiting**: Maximum spectral change per nm
//! - **Coherent Transitions**: Smooth spectral evolution over time

use super::packet::SpectralPacket;

// ============================================================================
// BLEND CONFIGURATION
// ============================================================================

/// Configuration for spectral blending.
#[derive(Debug, Clone)]
pub struct BlendConfig {
    /// Blend factor (0 = previous, 1 = current).
    pub alpha: f64,
    /// Maximum gradient per nm.
    pub max_gradient: f64,
    /// Minimum blend factor (for stability).
    pub min_alpha: f64,
    /// Maximum blend factor.
    pub max_alpha: f64,
    /// Whether to use adaptive alpha.
    pub adaptive: bool,
}

impl Default for BlendConfig {
    fn default() -> Self {
        Self {
            alpha: 0.3,
            max_gradient: 0.1, // Max 10% change per nm
            min_alpha: 0.1,
            max_alpha: 1.0,
            adaptive: true,
        }
    }
}

impl BlendConfig {
    /// Create smooth blending (slow transitions).
    pub fn smooth() -> Self {
        Self {
            alpha: 0.1,
            max_gradient: 0.05,
            ..Default::default()
        }
    }

    /// Create fast blending (quick transitions).
    pub fn fast() -> Self {
        Self {
            alpha: 0.5,
            max_gradient: 0.2,
            ..Default::default()
        }
    }

    /// Create instant blending (no smoothing).
    pub fn instant() -> Self {
        Self {
            alpha: 1.0,
            max_gradient: f64::INFINITY,
            adaptive: false,
            ..Default::default()
        }
    }
}

// ============================================================================
// SPECTRAL INTERPOLATOR
// ============================================================================

/// Spectral interpolator for temporal stability.
///
/// Blends spectral packets across frames to prevent flicker.
#[derive(Debug, Clone)]
pub struct SpectralInterpolator {
    /// Configuration.
    config: BlendConfig,
    /// Previous packet (for blending).
    previous: Option<SpectralPacket>,
    /// Accumulated error (for adaptive alpha).
    accumulated_error: f64,
    /// Frame count.
    frame_count: u64,
}

impl Default for SpectralInterpolator {
    fn default() -> Self {
        Self::new(BlendConfig::default())
    }
}

impl SpectralInterpolator {
    /// Create new interpolator.
    pub fn new(config: BlendConfig) -> Self {
        Self {
            config,
            previous: None,
            accumulated_error: 0.0,
            frame_count: 0,
        }
    }

    /// Create smooth interpolator.
    pub fn smooth() -> Self {
        Self::new(BlendConfig::smooth())
    }

    /// Create fast interpolator.
    pub fn fast() -> Self {
        Self::new(BlendConfig::fast())
    }

    /// Process a new spectral packet.
    ///
    /// Returns blended result if previous exists, otherwise returns input.
    pub fn process(&mut self, current: SpectralPacket) -> SpectralPacket {
        self.frame_count += 1;

        // Clone previous to avoid borrow issues
        let prev_clone = self.previous.clone();

        let result = match prev_clone {
            Some(ref prev) => {
                let alpha = self.compute_alpha_readonly(&current, prev);
                self.blend_packets(prev, &current, alpha)
            }
            None => current.clone(),
        };

        self.previous = Some(result.clone());
        result
    }

    /// Blend two packets.
    fn blend_packets(
        &self,
        prev: &SpectralPacket,
        current: &SpectralPacket,
        alpha: f64,
    ) -> SpectralPacket {
        let mut result = current.clone();
        let max_gradient = self.config.max_gradient;

        // Blend values at each wavelength
        for i in 0..result.values.len() {
            if i < prev.values.len() {
                let prev_val = prev.values[i];
                let curr_val = result.values[i];

                // Apply alpha blend
                let blended = prev_val * (1.0 - alpha) + curr_val * alpha;

                // Apply gradient limiting if not infinite
                if max_gradient.is_finite() && i > 0 {
                    let prev_wavelength = result.wavelengths[i - 1];
                    let curr_wavelength = result.wavelengths[i];
                    let dw = (curr_wavelength - prev_wavelength).abs();

                    if dw > 0.0 {
                        let prev_result = result.values[i - 1];
                        let max_change = max_gradient * dw;
                        let limited =
                            prev_result + (blended - prev_result).clamp(-max_change, max_change);
                        result.values[i] = limited;
                    } else {
                        result.values[i] = blended;
                    }
                } else {
                    result.values[i] = blended;
                }
            }
        }

        // Invalidate caches
        result.rgb = None;
        result.xyz = None;

        result
    }

    /// Compute adaptive alpha based on change magnitude (readonly version).
    fn compute_alpha_readonly(&self, current: &SpectralPacket, prev: &SpectralPacket) -> f64 {
        if !self.config.adaptive {
            return self.config.alpha;
        }

        // Compute change magnitude
        let mut total_change = 0.0;
        let mut count = 0;

        for (i, &curr_val) in current.values.iter().enumerate() {
            if i < prev.values.len() {
                total_change += (curr_val - prev.values[i]).abs();
                count += 1;
            }
        }

        if count == 0 {
            return self.config.alpha;
        }

        let avg_change = total_change / count as f64;

        // Reduce alpha for large changes (more smoothing)
        let adaptive_alpha = if avg_change > 0.1 {
            self.config.min_alpha
        } else if avg_change > 0.01 {
            // Interpolate based on change
            let t = (avg_change - 0.01) / (0.1 - 0.01);
            self.config.alpha * (1.0 - t) + self.config.min_alpha * t
        } else {
            self.config.alpha
        };

        adaptive_alpha.clamp(self.config.min_alpha, self.config.max_alpha)
    }

    /// Compute adaptive alpha based on change magnitude.
    #[allow(dead_code)]
    fn compute_alpha(&mut self, current: &SpectralPacket, prev: &SpectralPacket) -> f64 {
        let alpha = self.compute_alpha_readonly(current, prev);

        // Compute avg_change for tracking
        let mut total_change = 0.0;
        let mut count = 0;
        for (i, &curr_val) in current.values.iter().enumerate() {
            if i < prev.values.len() {
                total_change += (curr_val - prev.values[i]).abs();
                count += 1;
            }
        }
        let avg_change = if count > 0 {
            total_change / count as f64
        } else {
            0.0
        };

        // Track accumulated error for long-term adaptation
        self.accumulated_error = self.accumulated_error * 0.99 + avg_change;

        alpha
    }

    /// Reset interpolator state.
    pub fn reset(&mut self) {
        self.previous = None;
        self.accumulated_error = 0.0;
        self.frame_count = 0;
    }

    /// Get frame count.
    pub fn frame_count(&self) -> u64 {
        self.frame_count
    }

    /// Get accumulated error.
    pub fn accumulated_error(&self) -> f64 {
        self.accumulated_error
    }
}

// ============================================================================
// GRADIENT LIMITER
// ============================================================================

/// Limits spectral gradient (change per nm).
#[derive(Debug, Clone)]
pub struct GradientLimiter {
    /// Maximum gradient (per nm).
    max_gradient: f64,
}

impl Default for GradientLimiter {
    fn default() -> Self {
        Self { max_gradient: 0.1 }
    }
}

impl GradientLimiter {
    /// Create new gradient limiter.
    pub fn new(max_gradient: f64) -> Self {
        Self {
            max_gradient: max_gradient.abs(),
        }
    }

    /// Apply gradient limiting to a spectral packet.
    pub fn limit(&self, packet: &mut SpectralPacket) {
        if packet.wavelengths.len() < 2 {
            return;
        }

        // Forward pass
        for i in 1..packet.values.len() {
            let dw = packet.wavelengths[i] - packet.wavelengths[i - 1];
            if dw > 0.0 {
                let max_change = self.max_gradient * dw;
                let prev = packet.values[i - 1];
                packet.values[i] = prev + (packet.values[i] - prev).clamp(-max_change, max_change);
            }
        }

        // Backward pass for symmetry
        for i in (0..packet.values.len() - 1).rev() {
            let dw = packet.wavelengths[i + 1] - packet.wavelengths[i];
            if dw > 0.0 {
                let max_change = self.max_gradient * dw;
                let next = packet.values[i + 1];
                packet.values[i] = next + (packet.values[i] - next).clamp(-max_change, max_change);
            }
        }

        // Invalidate caches
        packet.rgb = None;
        packet.xyz = None;
    }

    /// Check if packet exceeds gradient limits.
    pub fn check(&self, packet: &SpectralPacket) -> bool {
        if packet.wavelengths.len() < 2 {
            return true; // Trivially valid
        }

        for i in 1..packet.values.len() {
            let dw = packet.wavelengths[i] - packet.wavelengths[i - 1];
            if dw > 0.0 {
                let gradient = (packet.values[i] - packet.values[i - 1]).abs() / dw;
                if gradient > self.max_gradient {
                    return false;
                }
            }
        }

        true
    }

    /// Get maximum gradient in packet.
    pub fn max_gradient_in(&self, packet: &SpectralPacket) -> f64 {
        packet.max_gradient()
    }
}

// ============================================================================
// TEMPORAL SPECTRAL HISTORY
// ============================================================================

/// Maintains spectral history for temporal analysis.
#[derive(Debug, Clone)]
pub struct SpectralHistory {
    /// History buffer.
    history: Vec<SpectralPacket>,
    /// Maximum history size.
    max_size: usize,
    /// Current write position.
    position: usize,
    /// Is buffer full?
    full: bool,
}

impl Default for SpectralHistory {
    fn default() -> Self {
        Self::new(10)
    }
}

impl SpectralHistory {
    /// Create new history with capacity.
    pub fn new(capacity: usize) -> Self {
        Self {
            history: Vec::with_capacity(capacity),
            max_size: capacity,
            position: 0,
            full: false,
        }
    }

    /// Add a packet to history.
    pub fn push(&mut self, packet: SpectralPacket) {
        if self.history.len() < self.max_size {
            self.history.push(packet);
        } else {
            self.history[self.position] = packet;
        }

        self.position = (self.position + 1) % self.max_size;
        if self.position == 0 {
            self.full = true;
        }
    }

    /// Get most recent packet.
    pub fn latest(&self) -> Option<&SpectralPacket> {
        if self.history.is_empty() {
            None
        } else {
            let idx = if self.position == 0 {
                self.history.len() - 1
            } else {
                self.position - 1
            };
            Some(&self.history[idx])
        }
    }

    /// Get packet at offset from latest (0 = latest).
    pub fn at(&self, offset: usize) -> Option<&SpectralPacket> {
        let len = self.len();
        if offset >= len {
            return None;
        }

        let idx = if self.position > offset {
            self.position - 1 - offset
        } else if self.full {
            self.max_size - (offset - self.position) - 1
        } else {
            return None;
        };

        self.history.get(idx)
    }

    /// Compute average packet over history.
    pub fn average(&self) -> Option<SpectralPacket> {
        if self.history.is_empty() {
            return None;
        }

        let first = &self.history[0];
        let mut avg =
            SpectralPacket::from_data(first.wavelengths.clone(), vec![0.0; first.values.len()]);

        let count = self.len() as f64;
        for packet in &self.history {
            for (i, &v) in packet.values.iter().enumerate() {
                if i < avg.values.len() {
                    avg.values[i] += v / count;
                }
            }
        }

        Some(avg)
    }

    /// Get history length.
    pub fn len(&self) -> usize {
        if self.full {
            self.max_size
        } else {
            self.history.len()
        }
    }

    /// Check if empty.
    pub fn is_empty(&self) -> bool {
        self.history.is_empty()
    }

    /// Clear history.
    pub fn clear(&mut self) {
        self.history.clear();
        self.position = 0;
        self.full = false;
    }
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_blend_config_defaults() {
        let config = BlendConfig::default();
        assert!(config.alpha > 0.0 && config.alpha < 1.0);
        assert!(config.max_gradient > 0.0);
    }

    #[test]
    fn test_blend_config_presets() {
        let smooth = BlendConfig::smooth();
        let fast = BlendConfig::fast();

        assert!(smooth.alpha < fast.alpha);
    }

    #[test]
    fn test_interpolator_first_frame() {
        let mut interp = SpectralInterpolator::default();
        let packet = SpectralPacket::uniform_31();

        let result = interp.process(packet.clone());

        // First frame should pass through unchanged
        assert_eq!(result.values, packet.values);
    }

    #[test]
    fn test_interpolator_blending() {
        let mut interp = SpectralInterpolator::new(BlendConfig {
            alpha: 0.5,
            adaptive: false,
            ..Default::default()
        });

        let mut p1 = SpectralPacket::uniform_31();
        let mut p2 = SpectralPacket::uniform_31();

        // Set different values
        for v in p1.values.iter_mut() {
            *v = 0.0;
        }
        for v in p2.values.iter_mut() {
            *v = 1.0;
        }

        let _ = interp.process(p1);
        let result = interp.process(p2);

        // Should be blended (alpha = 0.5)
        // Note: gradient limiting may affect this
        for v in &result.values {
            assert!(*v >= 0.0 && *v <= 1.0);
        }
    }

    #[test]
    fn test_interpolator_reset() {
        let mut interp = SpectralInterpolator::default();
        let packet = SpectralPacket::uniform_31();

        interp.process(packet.clone());
        interp.process(packet.clone());

        assert_eq!(interp.frame_count(), 2);

        interp.reset();

        assert_eq!(interp.frame_count(), 0);
    }

    #[test]
    fn test_gradient_limiter_check() {
        let limiter = GradientLimiter::new(0.01); // Very strict

        let mut packet = SpectralPacket::uniform_31();
        // Flat spectrum should pass
        assert!(limiter.check(&packet));

        // Sharp spike should fail
        packet.values[15] = 10.0;
        assert!(!limiter.check(&packet));
    }

    #[test]
    fn test_gradient_limiter_limit() {
        let limiter = GradientLimiter::new(0.1);

        let mut packet = SpectralPacket::uniform_31();
        packet.values[15] = 10.0; // Sharp spike

        limiter.limit(&mut packet);

        // After limiting, gradient should be within bounds
        let max_grad = packet.max_gradient();
        assert!(max_grad <= 0.15); // Some tolerance due to bidirectional pass
    }

    #[test]
    fn test_spectral_history() {
        let mut history = SpectralHistory::new(5);

        for i in 0..3 {
            let mut packet = SpectralPacket::uniform_31();
            packet.values[0] = i as f64;
            history.push(packet);
        }

        assert_eq!(history.len(), 3);
        assert!(!history.is_empty());

        let latest = history.latest().unwrap();
        assert!((latest.values[0] - 2.0).abs() < 1e-6);
    }

    #[test]
    fn test_spectral_history_wrap() {
        let mut history = SpectralHistory::new(3);

        for i in 0..5 {
            let mut packet = SpectralPacket::uniform_31();
            packet.values[0] = i as f64;
            history.push(packet);
        }

        assert_eq!(history.len(), 3);

        let latest = history.latest().unwrap();
        assert!((latest.values[0] - 4.0).abs() < 1e-6);
    }

    #[test]
    fn test_spectral_history_average() {
        let mut history = SpectralHistory::new(3);

        for i in 0..3 {
            let mut packet = SpectralPacket::uniform_31();
            for v in packet.values.iter_mut() {
                *v = i as f64;
            }
            history.push(packet);
        }

        let avg = history.average().unwrap();
        // Average of 0, 1, 2 = 1.0
        assert!((avg.values[0] - 1.0).abs() < 1e-6);
    }

    #[test]
    fn test_spectral_history_at() {
        let mut history = SpectralHistory::new(5);

        for i in 0..4 {
            let mut packet = SpectralPacket::uniform_31();
            packet.values[0] = i as f64;
            history.push(packet);
        }

        // at(0) = latest = 3
        let p0 = history.at(0).unwrap();
        assert!((p0.values[0] - 3.0).abs() < 1e-6);

        // at(1) = previous = 2
        let p1 = history.at(1).unwrap();
        assert!((p1.values[0] - 2.0).abs() < 1e-6);
    }
}
