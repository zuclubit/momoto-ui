//! # Backpropagation Through Time (BPTT)
//!
//! Gradient computation for long temporal sequences.
//!
//! ## Overview
//!
//! BPTT computes gradients for sequences by unrolling the temporal
//! computation graph and applying the chain rule backwards through time.
//!
//! For stability over long sequences, this module provides:
//! - Gradient clipping to prevent explosion
//! - Temporal decay to weight recent frames more heavily
//! - Truncated BPTT for efficiency
//!
//! ## Gradient Flow
//!
//! ```text
//! Forward:  θ₀ → θ₁ → θ₂ → ... → θ_T
//!            ↓    ↓    ↓          ↓
//!           L₀   L₁   L₂         L_T
//!
//! Backward: ∂L/∂θ₀ ← ∂L/∂θ₁ ← ∂L/∂θ₂ ← ... ← ∂L/∂θ_T
//!                 (chain rule through evolution)
//! ```

use super::evolution_gradients::{compute_evolution_gradient, EvolutionGradients, EvolutionType};

// ============================================================================
// BPTT CONFIGURATION
// ============================================================================

/// Configuration for BPTT.
#[derive(Debug, Clone)]
pub struct BPTTConfig {
    /// Maximum sequence length to process (truncation).
    pub max_sequence_length: usize,
    /// Temporal decay factor (0 = no decay, 1 = immediate decay).
    pub temporal_decay: f64,
    /// Gradient clipping norm.
    pub gradient_clip_norm: f64,
    /// Whether to use truncated BPTT.
    pub truncated: bool,
    /// Truncation window size.
    pub truncation_window: usize,
    /// Minimum gradient norm to continue (early stopping).
    pub min_gradient_norm: f64,
}

impl Default for BPTTConfig {
    fn default() -> Self {
        Self {
            max_sequence_length: 2000,
            temporal_decay: 0.01,
            gradient_clip_norm: 1.0,
            truncated: true,
            truncation_window: 100,
            min_gradient_norm: 1e-10,
        }
    }
}

impl BPTTConfig {
    /// Create config for long sequences.
    pub fn long_sequence() -> Self {
        Self {
            max_sequence_length: 5000,
            temporal_decay: 0.005,
            truncation_window: 200,
            ..Default::default()
        }
    }

    /// Create config for high precision (no truncation).
    pub fn high_precision() -> Self {
        Self {
            truncated: false,
            temporal_decay: 0.0,
            ..Default::default()
        }
    }
}

// ============================================================================
// BPTT STATE
// ============================================================================

/// State for a single frame in BPTT.
#[derive(Debug, Clone)]
pub struct FrameState {
    /// Time of this frame.
    pub time: f64,
    /// Parameter values at this frame.
    pub params: Vec<f64>,
    /// Evolution gradients at this frame.
    pub evolution_grads: EvolutionGradients,
    /// Loss value at this frame.
    pub loss: f64,
    /// Loss gradient at this frame.
    pub loss_gradient: f64,
}

/// Accumulated state for BPTT computation.
#[derive(Debug, Clone)]
pub struct BPTTState {
    /// Forward cache of frame states.
    pub forward_cache: Vec<FrameState>,
    /// Accumulated gradients w.r.t. initial parameters.
    pub accumulated_grads: Vec<f64>,
    /// Total loss across all frames.
    pub total_loss: f64,
    /// Number of frames processed.
    pub frame_count: usize,
}

impl BPTTState {
    /// Create new BPTT state.
    pub fn new(param_count: usize) -> Self {
        Self {
            forward_cache: Vec::new(),
            accumulated_grads: vec![0.0; param_count],
            total_loss: 0.0,
            frame_count: 0,
        }
    }

    /// Clear state for new sequence.
    pub fn clear(&mut self) {
        self.forward_cache.clear();
        for g in &mut self.accumulated_grads {
            *g = 0.0;
        }
        self.total_loss = 0.0;
        self.frame_count = 0;
    }

    /// Get gradient norm.
    pub fn gradient_norm(&self) -> f64 {
        self.accumulated_grads
            .iter()
            .map(|g| g * g)
            .sum::<f64>()
            .sqrt()
    }
}

// ============================================================================
// BPTT
// ============================================================================

/// Backpropagation through time implementation.
#[derive(Debug)]
pub struct BPTT {
    /// Configuration.
    pub config: BPTTConfig,
    /// Current state.
    state: BPTTState,
    /// Stabilizer for gradient control.
    stabilizer: GradientStabilizer,
}

impl BPTT {
    /// Create new BPTT instance.
    pub fn new(param_count: usize) -> Self {
        Self {
            config: BPTTConfig::default(),
            state: BPTTState::new(param_count),
            stabilizer: GradientStabilizer::new(StabilizerConfig::default()),
        }
    }

    /// Create with configuration.
    pub fn with_config(param_count: usize, config: BPTTConfig) -> Self {
        Self {
            config,
            state: BPTTState::new(param_count),
            stabilizer: GradientStabilizer::new(StabilizerConfig::default()),
        }
    }

    /// Process forward pass for a frame.
    pub fn forward_frame(
        &mut self,
        time: f64,
        params: Vec<f64>,
        evolution: EvolutionType,
        loss: f64,
        loss_gradient: f64,
    ) {
        // Compute evolution gradients
        let initial = params.first().copied().unwrap_or(1.5);
        let (_, evolution_grads) = compute_evolution_gradient(evolution, time, initial);

        // Store frame state
        let frame = FrameState {
            time,
            params,
            evolution_grads,
            loss,
            loss_gradient,
        };

        self.state.forward_cache.push(frame);
        self.state.total_loss += loss;
        self.state.frame_count += 1;

        // Truncate if needed
        if self.config.truncated && self.state.forward_cache.len() > self.config.truncation_window {
            self.state.forward_cache.remove(0);
        }
    }

    /// Run backward pass to compute gradients.
    pub fn backward(&mut self) -> Vec<f64> {
        let n_frames = self.state.forward_cache.len();
        if n_frames == 0 {
            return self.state.accumulated_grads.clone();
        }

        // Clear accumulated gradients
        for g in &mut self.state.accumulated_grads {
            *g = 0.0;
        }

        // Backward pass through frames
        let mut upstream_grad = 0.0;

        for i in (0..n_frames).rev() {
            let frame = &self.state.forward_cache[i];

            // Temporal decay weight
            let time_weight = if self.config.temporal_decay > 0.0 {
                let frames_from_end = (n_frames - 1 - i) as f64;
                (-self.config.temporal_decay * frames_from_end).exp()
            } else {
                1.0
            };

            // Local gradient contribution
            let local_grad = frame.loss_gradient * time_weight;

            // Chain rule through evolution
            let d_initial = frame.evolution_grads.d_initial;

            // Accumulate gradient for initial parameter
            if !self.state.accumulated_grads.is_empty() {
                self.state.accumulated_grads[0] += local_grad * d_initial;
            }

            // Accumulate rate gradient if present
            if self.state.accumulated_grads.len() > 1 {
                self.state.accumulated_grads[1] += local_grad * frame.evolution_grads.d_rate;
            }

            // Propagate through time (for recurrent connections)
            upstream_grad = local_grad * d_initial + upstream_grad * d_initial;
        }

        // Apply gradient stabilization
        let norm = self.state.gradient_norm();
        if norm > self.config.gradient_clip_norm {
            let scale = self.config.gradient_clip_norm / norm;
            for g in &mut self.state.accumulated_grads {
                *g *= scale;
            }
        }

        self.stabilizer
            .record_gradient_norm(self.state.gradient_norm());

        self.state.accumulated_grads.clone()
    }

    /// Get current state.
    pub fn state(&self) -> &BPTTState {
        &self.state
    }

    /// Reset for new sequence.
    pub fn reset(&mut self) {
        self.state.clear();
    }

    /// Check if gradients are stable.
    pub fn is_stable(&self) -> bool {
        self.stabilizer.is_stable()
    }

    /// Get average gradient norm over recent history.
    pub fn average_gradient_norm(&self) -> f64 {
        self.stabilizer.average_norm()
    }
}

impl Default for BPTT {
    fn default() -> Self {
        Self::new(8)
    }
}

// ============================================================================
// TEMPORAL GRADIENT ACCUMULATOR
// ============================================================================

/// Accumulates gradients across multiple temporal sequences.
#[derive(Debug)]
pub struct TemporalGradientAccumulator {
    /// Accumulated gradients.
    gradients: Vec<f64>,
    /// Number of sequences accumulated.
    sequence_count: usize,
    /// Total frames processed.
    total_frames: usize,
}

impl TemporalGradientAccumulator {
    /// Create new accumulator.
    pub fn new(param_count: usize) -> Self {
        Self {
            gradients: vec![0.0; param_count],
            sequence_count: 0,
            total_frames: 0,
        }
    }

    /// Add gradients from a sequence.
    pub fn add_sequence(&mut self, gradients: &[f64], frame_count: usize) {
        for (acc, &g) in self.gradients.iter_mut().zip(gradients.iter()) {
            *acc += g;
        }
        self.sequence_count += 1;
        self.total_frames += frame_count;
    }

    /// Get averaged gradients.
    pub fn average(&self) -> Vec<f64> {
        if self.sequence_count == 0 {
            return self.gradients.clone();
        }
        self.gradients
            .iter()
            .map(|&g| g / self.sequence_count as f64)
            .collect()
    }

    /// Get frame-weighted average.
    pub fn frame_weighted_average(&self) -> Vec<f64> {
        if self.total_frames == 0 {
            return self.gradients.clone();
        }
        self.gradients
            .iter()
            .map(|&g| g / self.total_frames as f64)
            .collect()
    }

    /// Clear accumulator.
    pub fn clear(&mut self) {
        for g in &mut self.gradients {
            *g = 0.0;
        }
        self.sequence_count = 0;
        self.total_frames = 0;
    }

    /// Get number of sequences.
    pub fn sequence_count(&self) -> usize {
        self.sequence_count
    }
}

// ============================================================================
// GRADIENT STABILIZER
// ============================================================================

/// Configuration for gradient stabilization.
#[derive(Debug, Clone)]
pub struct StabilizerConfig {
    /// Window size for moving average.
    pub window_size: usize,
    /// Threshold for instability detection.
    pub instability_threshold: f64,
    /// Maximum gradient norm before warning.
    pub max_norm_warning: f64,
}

impl Default for StabilizerConfig {
    fn default() -> Self {
        Self {
            window_size: 100,
            instability_threshold: 10.0,
            max_norm_warning: 100.0,
        }
    }
}

/// Monitors and stabilizes gradients over time.
#[derive(Debug)]
pub struct GradientStabilizer {
    /// Configuration.
    config: StabilizerConfig,
    /// History of gradient norms.
    norm_history: Vec<f64>,
    /// Current index in circular buffer.
    current_index: usize,
    /// Running statistics.
    running_mean: f64,
    running_var: f64,
    sample_count: usize,
}

impl GradientStabilizer {
    /// Create new stabilizer.
    pub fn new(config: StabilizerConfig) -> Self {
        Self {
            norm_history: vec![0.0; config.window_size],
            current_index: 0,
            running_mean: 0.0,
            running_var: 0.0,
            sample_count: 0,
            config,
        }
    }

    /// Record a gradient norm.
    pub fn record_gradient_norm(&mut self, norm: f64) {
        self.norm_history[self.current_index] = norm;
        self.current_index = (self.current_index + 1) % self.config.window_size;

        // Update running statistics
        self.sample_count += 1;
        let delta = norm - self.running_mean;
        self.running_mean += delta / self.sample_count as f64;
        let delta2 = norm - self.running_mean;
        self.running_var += delta * delta2;
    }

    /// Get average norm over window.
    pub fn average_norm(&self) -> f64 {
        let count = self.sample_count.min(self.config.window_size);
        if count == 0 {
            return 0.0;
        }
        self.norm_history.iter().take(count).sum::<f64>() / count as f64
    }

    /// Get variance of norm.
    pub fn norm_variance(&self) -> f64 {
        if self.sample_count < 2 {
            return 0.0;
        }
        self.running_var / (self.sample_count - 1) as f64
    }

    /// Check if gradients are stable.
    pub fn is_stable(&self) -> bool {
        if self.sample_count < 10 {
            return true; // Not enough data
        }

        let variance = self.norm_variance();
        let mean = self.running_mean;

        // Check coefficient of variation
        if mean > 0.0 {
            let cv = variance.sqrt() / mean;
            cv < self.config.instability_threshold
        } else {
            true
        }
    }

    /// Check if gradient explosion is occurring.
    pub fn is_exploding(&self) -> bool {
        if self.sample_count < 2 {
            return false;
        }

        // Check if recent norms are increasing rapidly
        let recent_count = 10.min(self.sample_count);
        let recent_start = if self.current_index >= recent_count {
            self.current_index - recent_count
        } else {
            self.config.window_size - (recent_count - self.current_index)
        };

        let mut increasing_count = 0;
        for i in 0..recent_count.saturating_sub(1) {
            let idx1 = (recent_start + i) % self.config.window_size;
            let idx2 = (recent_start + i + 1) % self.config.window_size;
            if self.norm_history[idx2] > self.norm_history[idx1] * 1.5 {
                increasing_count += 1;
            }
        }

        increasing_count > recent_count / 2
    }

    /// Get suggested learning rate scale based on stability.
    pub fn suggested_lr_scale(&self) -> f64 {
        if self.is_exploding() {
            0.1 // Reduce learning rate significantly
        } else if !self.is_stable() {
            0.5 // Reduce learning rate moderately
        } else if self.average_norm() < 0.01 {
            2.0 // Increase learning rate if gradients are very small
        } else {
            1.0 // Keep learning rate
        }
    }

    /// Reset stabilizer.
    pub fn reset(&mut self) {
        for n in &mut self.norm_history {
            *n = 0.0;
        }
        self.current_index = 0;
        self.running_mean = 0.0;
        self.running_var = 0.0;
        self.sample_count = 0;
    }
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bptt_config_default() {
        let config = BPTTConfig::default();
        assert_eq!(config.max_sequence_length, 2000);
        assert!(config.truncated);
    }

    #[test]
    fn test_bptt_state_new() {
        let state = BPTTState::new(8);
        assert_eq!(state.accumulated_grads.len(), 8);
        assert_eq!(state.total_loss, 0.0);
        assert_eq!(state.frame_count, 0);
    }

    #[test]
    fn test_bptt_forward_frame() {
        let mut bptt = BPTT::new(8);

        bptt.forward_frame(
            0.0,
            vec![1.5, 0.0, 0.1, 0.0, 0.0, 0.0, 0.0, 0.0],
            EvolutionType::Linear { rate: 0.01 },
            0.1,
            0.2,
        );

        assert_eq!(bptt.state().frame_count, 1);
        assert!((bptt.state().total_loss - 0.1).abs() < 1e-10);
    }

    #[test]
    fn test_bptt_backward() {
        let mut bptt = BPTT::new(8);

        // Add multiple frames
        for i in 0..10 {
            let t = i as f64;
            bptt.forward_frame(
                t,
                vec![1.5 + 0.01 * t, 0.0, 0.1, 0.0, 0.0, 0.0, 0.0, 0.0],
                EvolutionType::Linear { rate: 0.01 },
                0.01 * t,
                0.1,
            );
        }

        let grads = bptt.backward();

        // Should have non-zero gradients
        assert!(!grads.is_empty());
        assert!(bptt.state().gradient_norm() > 0.0);
    }

    #[test]
    fn test_bptt_temporal_decay() {
        let config = BPTTConfig {
            temporal_decay: 0.1,
            ..Default::default()
        };
        let mut bptt = BPTT::with_config(8, config);

        // Add frames
        for i in 0..50 {
            bptt.forward_frame(
                i as f64,
                vec![1.5; 8],
                EvolutionType::Linear { rate: 0.01 },
                0.1,
                1.0,
            );
        }

        let grads = bptt.backward();

        // Gradients should be accumulated with decay
        assert!(grads.iter().any(|&g| g != 0.0));
    }

    #[test]
    fn test_bptt_truncation() {
        let config = BPTTConfig {
            truncated: true,
            truncation_window: 10,
            ..Default::default()
        };
        let mut bptt = BPTT::with_config(8, config);

        // Add many frames
        for i in 0..100 {
            bptt.forward_frame(
                i as f64,
                vec![1.5; 8],
                EvolutionType::Linear { rate: 0.01 },
                0.1,
                1.0,
            );
        }

        // Cache should be truncated
        assert!(bptt.state().forward_cache.len() <= 10);
    }

    #[test]
    fn test_bptt_reset() {
        let mut bptt = BPTT::new(8);

        bptt.forward_frame(
            0.0,
            vec![1.5; 8],
            EvolutionType::Linear { rate: 0.01 },
            0.1,
            1.0,
        );

        bptt.reset();

        assert_eq!(bptt.state().frame_count, 0);
        assert_eq!(bptt.state().total_loss, 0.0);
        assert!(bptt.state().forward_cache.is_empty());
    }

    #[test]
    fn test_gradient_accumulator() {
        let mut acc = TemporalGradientAccumulator::new(3);

        acc.add_sequence(&[1.0, 2.0, 3.0], 10);
        acc.add_sequence(&[2.0, 4.0, 6.0], 20);

        let avg = acc.average();
        assert!((avg[0] - 1.5).abs() < 1e-10);
        assert!((avg[1] - 3.0).abs() < 1e-10);

        let frame_avg = acc.frame_weighted_average();
        assert!((frame_avg[0] - 0.1).abs() < 1e-10); // 3.0 / 30
    }

    #[test]
    fn test_gradient_stabilizer() {
        let mut stabilizer = GradientStabilizer::new(StabilizerConfig::default());

        // Record stable gradients
        for _ in 0..50 {
            stabilizer.record_gradient_norm(1.0);
        }

        assert!(stabilizer.is_stable());
        assert!((stabilizer.average_norm() - 1.0).abs() < 0.1);
    }

    #[test]
    fn test_gradient_stabilizer_explosion_detection() {
        let mut stabilizer = GradientStabilizer::new(StabilizerConfig::default());

        // Record exploding gradients
        let mut norm = 1.0;
        for _ in 0..20 {
            stabilizer.record_gradient_norm(norm);
            norm *= 2.0; // Doubling each time
        }

        assert!(stabilizer.is_exploding());
    }

    #[test]
    fn test_stabilizer_lr_suggestion() {
        let mut stabilizer = GradientStabilizer::new(StabilizerConfig::default());

        // Stable gradients
        for _ in 0..50 {
            stabilizer.record_gradient_norm(1.0);
        }
        assert!((stabilizer.suggested_lr_scale() - 1.0).abs() < 1e-10);

        // Very small gradients
        stabilizer.reset();
        for _ in 0..50 {
            stabilizer.record_gradient_norm(0.001);
        }
        assert!(stabilizer.suggested_lr_scale() > 1.0);
    }

    #[test]
    fn test_bptt_stability() {
        let mut bptt = BPTT::new(8);

        // Simulate stable training
        for i in 0..100 {
            bptt.forward_frame(
                i as f64,
                vec![1.5; 8],
                EvolutionType::Exponential {
                    rate: 0.5,
                    asymptote: 1.0,
                },
                0.1,
                1.0,
            );
            bptt.backward();
        }

        // Should remain stable
        assert!(bptt.is_stable());
    }

    #[test]
    fn test_bptt_long_sequence() {
        let config = BPTTConfig::long_sequence();
        let mut bptt = BPTT::with_config(8, config);

        // Process long sequence
        for i in 0..2000 {
            bptt.forward_frame(
                i as f64 * 0.01,
                vec![1.5; 8],
                EvolutionType::Linear { rate: 0.001 },
                0.001,
                0.01,
            );
        }

        let grads = bptt.backward();

        // Should complete without NaN or inf
        assert!(grads.iter().all(|g| g.is_finite()));
    }
}
