//! # Phase 12: Neural Temporal Correction Layer
//!
//! Extends Phase 10 neural correction with temporal awareness.
//!
//! ## Key Extensions
//!
//! - Extended input: 10 → 14 features (add delta_time, previous_residual, frame)
//! - Cumulative drift bounding: Track total correction over time
//! - Temporal consistency: Smooth corrections across frames
//!
//! ## Architecture
//!
//! ```text
//! Input (14 features):
//!   [wavelength, angles, material params (10)] + [delta_time, prev_r, prev_t, frame]
//!
//! Network:
//!   14 → 32 (sin) → 32 (sin) → 2 (tanh)
//!
//! Constraints:
//!   - Per-frame: |ΔR|, |ΔT| ≤ max_correction (0.1)
//!   - Cumulative: Σ|ΔR|, Σ|ΔT| ≤ max_cumulative (0.05)
//! ```

use super::neural_correction::{CorrectionInput, CorrectionOutput, NeuralCorrectionConfig};
use super::temporal::TemporalContext;
use super::unified_bsdf::{BSDFContext, BSDFResponse, BSDFSample, EnergyValidation, BSDF};

// ============================================================================
// TEMPORAL CORRECTION INPUT
// ============================================================================

/// Extended input encoding for temporal neural correction.
/// 14-dimensional input: 10 material features + 4 temporal features.
#[derive(Debug, Clone, Copy, Default)]
pub struct TemporalCorrectionInput {
    /// Base material input (10 features)
    pub base: CorrectionInput,
    /// Delta time normalized: dt / 0.1 (assuming 10fps baseline)
    pub delta_time_normalized: f64,
    /// Previous frame reflectance correction
    pub prev_delta_r: f64,
    /// Previous frame transmittance correction
    pub prev_delta_t: f64,
    /// Frame index normalized: (frame % 1000) / 1000
    pub frame_normalized: f64,
}

impl TemporalCorrectionInput {
    /// Create from base input and temporal context
    pub fn new(
        base: CorrectionInput,
        delta_time: f64,
        prev_output: CorrectionOutput,
        frame: u64,
    ) -> Self {
        Self {
            base,
            delta_time_normalized: (delta_time / 0.1).clamp(0.0, 10.0) / 10.0,
            prev_delta_r: prev_output.delta_reflectance / 0.1, // Normalize by max
            prev_delta_t: prev_output.delta_transmittance / 0.1,
            frame_normalized: ((frame % 1000) as f64) / 1000.0,
        }
    }

    /// Create from temporal context
    pub fn from_temporal_context(
        ctx: &TemporalContext,
        roughness: f64,
        ior: f64,
        prev_output: CorrectionOutput,
    ) -> Self {
        let base = CorrectionInput::from_context(&ctx.base, roughness, ior);
        Self::new(base, ctx.delta_time, prev_output, ctx.frame_index)
    }

    /// Convert to 14-element vector for neural network
    pub fn to_vec(&self) -> [f64; 14] {
        let base = self.base.to_vec();
        [
            base[0],
            base[1],
            base[2],
            base[3],
            base[4],
            base[5],
            base[6],
            base[7],
            base[8],
            base[9],
            self.delta_time_normalized,
            self.prev_delta_r,
            self.prev_delta_t,
            self.frame_normalized,
        ]
    }
}

// ============================================================================
// CUMULATIVE DRIFT TRACKER
// ============================================================================

/// Configuration for cumulative drift limiting.
#[derive(Debug, Clone)]
pub struct DriftLimitConfig {
    /// Maximum cumulative correction magnitude
    pub max_cumulative: f64,
    /// Decay rate per frame (for forgetting old corrections)
    pub decay_rate: f64,
    /// Window size for drift calculation
    pub window_size: usize,
    /// Whether to disable neural when limit exceeded
    pub disable_on_exceed: bool,
}

impl Default for DriftLimitConfig {
    fn default() -> Self {
        Self {
            max_cumulative: 0.05, // 5% total drift allowed
            decay_rate: 0.99,     // Slow decay
            window_size: 100,     // Track last 100 frames
            disable_on_exceed: true,
        }
    }
}

/// Tracks cumulative neural correction drift over time.
#[derive(Debug, Clone)]
pub struct CumulativeDriftTracker {
    /// Configuration
    config: DriftLimitConfig,
    /// Cumulative reflectance correction
    cumulative_r: f64,
    /// Cumulative transmittance correction
    cumulative_t: f64,
    /// History of corrections (for windowed tracking)
    history: Vec<CorrectionOutput>,
    /// Current position in circular buffer
    position: usize,
    /// Total frames tracked
    frame_count: u64,
    /// Number of limit violations
    violations: u64,
}

impl Default for CumulativeDriftTracker {
    fn default() -> Self {
        Self::new(DriftLimitConfig::default())
    }
}

impl CumulativeDriftTracker {
    /// Create new drift tracker
    pub fn new(config: DriftLimitConfig) -> Self {
        Self {
            history: Vec::with_capacity(config.window_size),
            config,
            cumulative_r: 0.0,
            cumulative_t: 0.0,
            position: 0,
            frame_count: 0,
            violations: 0,
        }
    }

    /// Track a new correction
    pub fn track(&mut self, correction: CorrectionOutput) -> bool {
        self.frame_count += 1;

        // Apply decay
        self.cumulative_r *= self.config.decay_rate;
        self.cumulative_t *= self.config.decay_rate;

        // Add new correction
        self.cumulative_r += correction.delta_reflectance.abs();
        self.cumulative_t += correction.delta_transmittance.abs();

        // Update history
        if self.history.len() < self.config.window_size {
            self.history.push(correction);
        } else {
            self.history[self.position] = correction;
        }
        self.position = (self.position + 1) % self.config.window_size;

        // Check limit
        let exceeded = self.is_exceeded();
        if exceeded {
            self.violations += 1;
        }

        exceeded
    }

    /// Check if cumulative drift exceeds limit
    pub fn is_exceeded(&self) -> bool {
        let total = self.cumulative_r + self.cumulative_t;
        total > self.config.max_cumulative
    }

    /// Get current cumulative drift
    pub fn cumulative_drift(&self) -> f64 {
        self.cumulative_r + self.cumulative_t
    }

    /// Get drift ratio (current / max)
    pub fn drift_ratio(&self) -> f64 {
        self.cumulative_drift() / self.config.max_cumulative
    }

    /// Get violation count
    pub fn violations(&self) -> u64 {
        self.violations
    }

    /// Get most recent correction
    pub fn last_correction(&self) -> CorrectionOutput {
        if self.history.is_empty() {
            CorrectionOutput::zero()
        } else {
            let idx = if self.position == 0 {
                self.history.len() - 1
            } else {
                self.position - 1
            };
            self.history[idx]
        }
    }

    /// Reset tracker
    pub fn reset(&mut self) {
        self.cumulative_r = 0.0;
        self.cumulative_t = 0.0;
        self.history.clear();
        self.position = 0;
        self.frame_count = 0;
        self.violations = 0;
    }
}

// ============================================================================
// TEMPORAL NEURAL CORRECTION MLP
// ============================================================================

/// Configuration for temporal neural correction
#[derive(Debug, Clone)]
pub struct TemporalNeuralConfig {
    /// Base neural config
    pub base: NeuralCorrectionConfig,
    /// Drift limiting config
    pub drift: DriftLimitConfig,
    /// Temporal smoothing factor (0 = no smoothing, 1 = full previous)
    pub temporal_smoothing: f64,
}

impl Default for TemporalNeuralConfig {
    fn default() -> Self {
        Self {
            base: NeuralCorrectionConfig::default(),
            drift: DriftLimitConfig::default(),
            temporal_smoothing: 0.3, // Blend 30% previous correction
        }
    }
}

/// Simple RNG for initialization
struct SimpleRng {
    state: u64,
}

impl SimpleRng {
    fn new(seed: u64) -> Self {
        Self { state: seed.max(1) }
    }

    fn next(&mut self) -> u64 {
        self.state ^= self.state << 13;
        self.state ^= self.state >> 7;
        self.state ^= self.state << 17;
        self.state
    }

    fn uniform(&mut self, min: f64, max: f64) -> f64 {
        let t = (self.next() as f64) / (u64::MAX as f64);
        min + t * (max - min)
    }
}

/// Temporal neural correction network.
///
/// Extends Phase 10 SIREN MLP with temporal input features.
/// Architecture: 14 → 32 → 32 → 2
#[derive(Debug, Clone)]
pub struct TemporalNeuralCorrection {
    /// Weights for layer 0: [32, 14]
    w0: Vec<f64>,
    /// Biases for layer 0: [32]
    b0: Vec<f64>,
    /// Weights for layer 1: [32, 32]
    w1: Vec<f64>,
    /// Biases for layer 1: [32]
    b1: Vec<f64>,
    /// Weights for output: [2, 32]
    w_out: Vec<f64>,
    /// Biases for output: [2]
    b_out: Vec<f64>,
    /// Configuration
    config: TemporalNeuralConfig,
    /// Drift tracker
    drift_tracker: CumulativeDriftTracker,
    /// Previous output (for smoothing)
    prev_output: CorrectionOutput,
    /// Whether network is currently enabled
    enabled: bool,
}

impl TemporalNeuralCorrection {
    /// Input dimension (extended from 10 to 14)
    pub const INPUT_DIM: usize = 14;
    /// Output dimension
    pub const OUTPUT_DIM: usize = 2;
    /// Hidden dimension
    pub const HIDDEN_DIM: usize = 32;

    /// Create new temporal neural correction network
    pub fn new(config: TemporalNeuralConfig) -> Self {
        let hidden = Self::HIDDEN_DIM;
        let mut rng = SimpleRng::new(config.base.seed);

        // SIREN initialization
        let c0 = (6.0 / Self::INPUT_DIM as f64).sqrt() / config.base.omega_0;
        let w0: Vec<f64> = (0..hidden * Self::INPUT_DIM)
            .map(|_| rng.uniform(-c0, c0))
            .collect();
        let b0: Vec<f64> = (0..hidden).map(|_| rng.uniform(-c0, c0)).collect();

        let c1 = (6.0 / hidden as f64).sqrt();
        let w1: Vec<f64> = (0..hidden * hidden).map(|_| rng.uniform(-c1, c1)).collect();
        let b1: Vec<f64> = (0..hidden).map(|_| rng.uniform(-c1, c1)).collect();

        let c_out = (6.0 / hidden as f64).sqrt();
        let w_out: Vec<f64> = (0..Self::OUTPUT_DIM * hidden)
            .map(|_| rng.uniform(-c_out, c_out))
            .collect();
        let b_out: Vec<f64> = (0..Self::OUTPUT_DIM)
            .map(|_| rng.uniform(-c_out, c_out))
            .collect();

        Self {
            w0,
            b0,
            w1,
            b1,
            w_out,
            b_out,
            drift_tracker: CumulativeDriftTracker::new(config.drift.clone()),
            config,
            prev_output: CorrectionOutput::zero(),
            enabled: true,
        }
    }

    /// Create with default configuration
    pub fn with_default_config() -> Self {
        Self::new(TemporalNeuralConfig::default())
    }

    /// Total parameter count
    pub fn param_count(&self) -> usize {
        Self::HIDDEN_DIM * Self::INPUT_DIM
            + Self::HIDDEN_DIM
            + Self::HIDDEN_DIM * Self::HIDDEN_DIM
            + Self::HIDDEN_DIM
            + Self::OUTPUT_DIM * Self::HIDDEN_DIM
            + Self::OUTPUT_DIM
    }

    /// Memory usage in bytes
    pub fn memory_bytes(&self) -> usize {
        self.param_count() * std::mem::size_of::<f64>()
            + std::mem::size_of::<CumulativeDriftTracker>()
            + self.drift_tracker.config.window_size * std::mem::size_of::<CorrectionOutput>()
    }

    /// Forward pass
    pub fn forward(&self, input: &TemporalCorrectionInput) -> CorrectionOutput {
        let x = input.to_vec();
        let hidden = Self::HIDDEN_DIM;
        let omega_0 = self.config.base.omega_0;
        let max_correction = self.config.base.max_correction;

        // Layer 0: sin(omega_0 * (W0 @ x + b0))
        let mut h0 = vec![0.0; hidden];
        for i in 0..hidden {
            let mut sum = self.b0[i];
            for j in 0..Self::INPUT_DIM {
                sum += self.w0[i * Self::INPUT_DIM + j] * x[j];
            }
            h0[i] = (omega_0 * sum).sin();
        }

        // Layer 1: sin(W1 @ h0 + b1)
        let mut h1 = vec![0.0; hidden];
        for i in 0..hidden {
            let mut sum = self.b1[i];
            for j in 0..hidden {
                sum += self.w1[i * hidden + j] * h0[j];
            }
            h1[i] = sum.sin();
        }

        // Output: tanh(W_out @ h1 + b_out) * max_correction
        let mut out = [0.0; Self::OUTPUT_DIM];
        for i in 0..Self::OUTPUT_DIM {
            let mut sum = self.b_out[i];
            for j in 0..hidden {
                sum += self.w_out[i * hidden + j] * h1[j];
            }
            out[i] = sum.tanh() * max_correction;
        }

        CorrectionOutput::new(out[0], out[1])
    }

    /// Forward pass with temporal smoothing and drift tracking
    pub fn forward_temporal(&mut self, input: &TemporalCorrectionInput) -> CorrectionOutput {
        if !self.enabled {
            return CorrectionOutput::zero();
        }

        // Get raw correction
        let raw = self.forward(input);

        // Apply temporal smoothing
        let alpha = self.config.temporal_smoothing;
        let smoothed = CorrectionOutput::new(
            raw.delta_reflectance * (1.0 - alpha) + self.prev_output.delta_reflectance * alpha,
            raw.delta_transmittance * (1.0 - alpha) + self.prev_output.delta_transmittance * alpha,
        );

        // Track drift
        let exceeded = self.drift_tracker.track(smoothed);

        // Disable if exceeded and configured to do so
        if exceeded && self.config.drift.disable_on_exceed {
            self.enabled = false;
            self.prev_output = CorrectionOutput::zero();
            return CorrectionOutput::zero();
        }

        self.prev_output = smoothed;
        smoothed
    }

    /// Apply correction to physical response
    pub fn apply(
        &mut self,
        physical: &BSDFResponse,
        input: &TemporalCorrectionInput,
    ) -> BSDFResponse {
        let correction = self.forward_temporal(input);

        // Apply corrections
        let r_corrected = physical.reflectance + correction.delta_reflectance;
        let t_corrected = physical.transmittance + correction.delta_transmittance;

        // Clamp to [0, 1]
        let r_clamped = r_corrected.clamp(0.0, 1.0);
        let t_clamped = t_corrected.clamp(0.0, 1.0);

        // Energy conservation
        let total = r_clamped + t_clamped;
        if total > 1.0 {
            let scale = 1.0 / total;
            BSDFResponse::new(r_clamped * scale, t_clamped * scale, 0.0)
        } else {
            BSDFResponse::new(r_clamped, t_clamped, 1.0 - r_clamped - t_clamped)
        }
    }

    /// Check if neural correction is enabled
    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    /// Re-enable neural correction (resets drift)
    pub fn enable(&mut self) {
        self.enabled = true;
        self.drift_tracker.reset();
    }

    /// Disable neural correction
    pub fn disable(&mut self) {
        self.enabled = false;
    }

    /// Get drift tracker reference
    pub fn drift_tracker(&self) -> &CumulativeDriftTracker {
        &self.drift_tracker
    }

    /// Get previous output
    pub fn previous_output(&self) -> CorrectionOutput {
        self.prev_output
    }

    /// Reset state
    pub fn reset(&mut self) {
        self.drift_tracker.reset();
        self.prev_output = CorrectionOutput::zero();
        self.enabled = true;
    }

    /// Get all parameters as flat vector
    pub fn get_params(&self) -> Vec<f64> {
        let mut params = Vec::with_capacity(self.param_count());
        params.extend(&self.w0);
        params.extend(&self.b0);
        params.extend(&self.w1);
        params.extend(&self.b1);
        params.extend(&self.w_out);
        params.extend(&self.b_out);
        params
    }

    /// Set parameters from flat vector
    pub fn set_params(&mut self, params: &[f64]) {
        assert_eq!(params.len(), self.param_count());
        let hidden = Self::HIDDEN_DIM;

        let mut idx = 0;

        let w0_len = hidden * Self::INPUT_DIM;
        self.w0.copy_from_slice(&params[idx..idx + w0_len]);
        idx += w0_len;

        self.b0.copy_from_slice(&params[idx..idx + hidden]);
        idx += hidden;

        let w1_len = hidden * hidden;
        self.w1.copy_from_slice(&params[idx..idx + w1_len]);
        idx += w1_len;

        self.b1.copy_from_slice(&params[idx..idx + hidden]);
        idx += hidden;

        let w_out_len = Self::OUTPUT_DIM * hidden;
        self.w_out.copy_from_slice(&params[idx..idx + w_out_len]);
        idx += w_out_len;

        self.b_out
            .copy_from_slice(&params[idx..idx + Self::OUTPUT_DIM]);
    }
}

// ============================================================================
// TEMPORAL NEURAL CORRECTED BSDF
// ============================================================================

/// BSDF wrapper with temporal neural correction.
#[derive(Debug, Clone)]
pub struct TemporalNeuralCorrectedBSDF<B: BSDF + Clone> {
    /// Underlying physical BSDF
    physical: B,
    /// Temporal neural correction
    correction: TemporalNeuralCorrection,
    /// Material parameters
    roughness: f64,
    ior: f64,
}

impl<B: BSDF + Clone> TemporalNeuralCorrectedBSDF<B> {
    /// Create new temporal neural corrected BSDF
    pub fn new(
        physical: B,
        correction: TemporalNeuralCorrection,
        roughness: f64,
        ior: f64,
    ) -> Self {
        Self {
            physical,
            correction,
            roughness,
            ior,
        }
    }

    /// Create with default neural configuration
    pub fn with_default_neural(physical: B, roughness: f64, ior: f64) -> Self {
        Self::new(
            physical,
            TemporalNeuralCorrection::with_default_config(),
            roughness,
            ior,
        )
    }

    /// Evaluate at temporal context
    pub fn evaluate_temporal(&mut self, ctx: &TemporalContext) -> BSDFResponse {
        let physical_response = self.physical.evaluate(&ctx.base);

        if self.correction.is_enabled() {
            let input = TemporalCorrectionInput::from_temporal_context(
                ctx,
                self.roughness,
                self.ior,
                self.correction.previous_output(),
            );
            self.correction.apply(&physical_response, &input)
        } else {
            physical_response
        }
    }

    /// Get correction network
    pub fn correction(&self) -> &TemporalNeuralCorrection {
        &self.correction
    }

    /// Get mutable correction network
    pub fn correction_mut(&mut self) -> &mut TemporalNeuralCorrection {
        &mut self.correction
    }

    /// Get physical BSDF
    pub fn physical(&self) -> &B {
        &self.physical
    }
}

impl<B: BSDF + Clone> BSDF for TemporalNeuralCorrectedBSDF<B> {
    fn evaluate(&self, ctx: &BSDFContext) -> BSDFResponse {
        // For non-temporal evaluation, just use physical
        self.physical.evaluate(ctx)
    }

    fn sample(&self, ctx: &BSDFContext, u1: f64, u2: f64) -> BSDFSample {
        self.physical.sample(ctx, u1, u2)
    }

    fn pdf(&self, ctx: &BSDFContext) -> f64 {
        self.physical.pdf(ctx)
    }

    fn validate_energy(&self, ctx: &BSDFContext) -> EnergyValidation {
        let response = self.evaluate(ctx);
        let total = response.reflectance + response.transmittance + response.absorption;
        let error = (total - 1.0).abs();

        EnergyValidation {
            conserved: error < 1e-6,
            error,
            details: format!(
                "TemporalNeuralCorrectedBSDF: R={:.4}, T={:.4}, A={:.4}",
                response.reflectance, response.transmittance, response.absorption
            ),
        }
    }

    fn name(&self) -> &str {
        "TemporalNeuralCorrectedBSDF"
    }

    fn is_delta(&self) -> bool {
        self.physical.is_delta()
    }
}

// ============================================================================
// MEMORY ESTIMATION
// ============================================================================

/// Estimate memory for temporal neural correction
pub fn estimate_temporal_neural_memory() -> usize {
    let network = TemporalNeuralCorrection::with_default_config();
    network.memory_bytes()
        + std::mem::size_of::<TemporalNeuralConfig>()
        + std::mem::size_of::<TemporalCorrectionInput>()
        + std::mem::size_of::<CorrectionOutput>()
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::super::unified_bsdf::DielectricBSDF;
    use super::*;

    #[test]
    fn test_temporal_input() {
        let base = CorrectionInput::default();
        let input = TemporalCorrectionInput::new(
            base,
            0.016, // 60fps
            CorrectionOutput::zero(),
            100,
        );

        let vec = input.to_vec();
        assert_eq!(vec.len(), 14);
    }

    #[test]
    fn test_network_creation() {
        let network = TemporalNeuralCorrection::with_default_config();

        // 14*32 + 32 + 32*32 + 32 + 2*32 + 2 = 448 + 32 + 1024 + 32 + 64 + 2 = 1602
        assert_eq!(network.param_count(), 1602);
        assert!(network.memory_bytes() < 20000);
    }

    #[test]
    fn test_forward_bounded() {
        let network = TemporalNeuralCorrection::with_default_config();
        let input = TemporalCorrectionInput::default();

        let output = network.forward(&input);

        assert!(output.delta_reflectance.abs() <= 0.1);
        assert!(output.delta_transmittance.abs() <= 0.1);
    }

    #[test]
    fn test_drift_tracking() {
        let mut tracker = CumulativeDriftTracker::default();

        // Add small corrections
        for _ in 0..10 {
            let correction = CorrectionOutput::new(0.001, 0.001);
            tracker.track(correction);
        }

        assert!(!tracker.is_exceeded());
        assert!(tracker.cumulative_drift() > 0.0);
    }

    #[test]
    fn test_drift_limit_exceeded() {
        let config = DriftLimitConfig {
            max_cumulative: 0.01,
            decay_rate: 1.0, // No decay
            ..Default::default()
        };
        let mut tracker = CumulativeDriftTracker::new(config);

        // Add corrections that exceed limit
        for _ in 0..20 {
            let correction = CorrectionOutput::new(0.001, 0.001);
            tracker.track(correction);
        }

        assert!(tracker.is_exceeded());
        assert!(tracker.violations() > 0);
    }

    #[test]
    fn test_temporal_forward_smoothing() {
        let mut network = TemporalNeuralCorrection::with_default_config();

        let input1 = TemporalCorrectionInput::default();
        let out1 = network.forward_temporal(&input1);

        let input2 = TemporalCorrectionInput::new(CorrectionInput::default(), 0.016, out1, 1);
        let out2 = network.forward_temporal(&input2);

        // Second output should be influenced by temporal smoothing
        assert!(out1.delta_reflectance != 0.0 || out2.delta_reflectance != 0.0);
    }

    #[test]
    fn test_disable_on_exceed() {
        let config = TemporalNeuralConfig {
            drift: DriftLimitConfig {
                max_cumulative: 0.001, // Very low limit
                decay_rate: 1.0,
                disable_on_exceed: true,
                ..Default::default()
            },
            ..Default::default()
        };

        let mut network = TemporalNeuralCorrection::new(config);

        // Generate many corrections to exceed limit
        for i in 0..100 {
            let input = TemporalCorrectionInput::new(
                CorrectionInput::default(),
                0.016,
                network.previous_output(),
                i,
            );
            network.forward_temporal(&input);
        }

        // Should be disabled due to drift
        assert!(!network.is_enabled());
    }

    #[test]
    fn test_apply_energy_conservation() {
        let mut network = TemporalNeuralCorrection::with_default_config();
        let physical = BSDFResponse::new(0.5, 0.3, 0.2);
        let input = TemporalCorrectionInput::default();

        let corrected = network.apply(&physical, &input);

        let total = corrected.reflectance + corrected.transmittance + corrected.absorption;
        assert!((total - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_params_roundtrip() {
        let mut network = TemporalNeuralCorrection::with_default_config();
        let original = network.get_params();

        let mut modified = original.clone();
        modified[0] += 1.0;
        network.set_params(&modified);

        let retrieved = network.get_params();
        assert!((retrieved[0] - modified[0]).abs() < 1e-10);
    }

    #[test]
    fn test_temporal_bsdf_wrapper() {
        let physical = DielectricBSDF::new(1.5, 0.0);
        let network = TemporalNeuralCorrection::with_default_config();
        let mut corrected = TemporalNeuralCorrectedBSDF::new(physical, network, 0.0, 1.5);

        let ctx = TemporalContext::default();
        let response = corrected.evaluate_temporal(&ctx);

        let total = response.reflectance + response.transmittance + response.absorption;
        assert!((total - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_reset() {
        let mut network = TemporalNeuralCorrection::with_default_config();

        // Generate some state
        for i in 0..10 {
            let input = TemporalCorrectionInput::new(
                CorrectionInput::default(),
                0.016,
                network.previous_output(),
                i,
            );
            network.forward_temporal(&input);
        }

        assert!(network.drift_tracker().frame_count > 0);

        network.reset();

        assert_eq!(network.drift_tracker().frame_count, 0);
        assert!(network.is_enabled());
    }

    #[test]
    fn test_memory_budget() {
        let memory = estimate_temporal_neural_memory();
        assert!(memory < 20_000, "Memory {} exceeds 20KB budget", memory);
    }

    #[test]
    fn test_deterministic() {
        let config = TemporalNeuralConfig::default();
        let net1 = TemporalNeuralCorrection::new(config.clone());
        let net2 = TemporalNeuralCorrection::new(config);

        assert_eq!(net1.get_params(), net2.get_params());
    }
}
