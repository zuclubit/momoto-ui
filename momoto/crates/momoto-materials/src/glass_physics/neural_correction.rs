// ============================================================================
// PHASE 10: NEURAL CORRECTION LAYER
// ============================================================================
//
// Small SIREN MLP for learning physics residuals.
// Core principle: Final_Output = Physical_Model_Output + Neural_Correction
//
// The neural network NEVER replaces physics - it only learns residuals where
// physics approximations are insufficient.
//
// Architecture: 10 inputs -> 32 hidden (sin) -> 32 hidden (sin) -> 2 outputs
// Total parameters: 1,442 (~11.5 KB)
// ============================================================================

use serde::{Deserialize, Serialize};

use super::unified_bsdf::{BSDFContext, BSDFResponse, BSDFSample, EnergyValidation, BSDF};

// ============================================================================
// CORRECTION INPUT/OUTPUT
// ============================================================================

/// Input encoding for neural correction network.
/// 10-dimensional normalized input vector.
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
pub struct CorrectionInput {
    /// Wavelength normalized: (λ - 400) / 300, range [0, 1]
    pub wavelength_normalized: f64,
    /// Cosine of incident angle
    pub cos_theta_i: f64,
    /// Cosine of outgoing angle
    pub cos_theta_o: f64,
    /// Surface roughness [0, 1]
    pub roughness: f64,
    /// Index of refraction normalized: (n - 1) / 3
    pub ior_normalized: f64,
    /// Extinction coefficient normalized: k / 10
    pub k_normalized: f64,
    /// Film thickness normalized: thickness / 2000 nm
    pub thickness_normalized: f64,
    /// Absorption coefficient normalized
    pub absorption_normalized: f64,
    /// Scattering coefficient normalized
    pub scattering_normalized: f64,
    /// Henyey-Greenstein asymmetry parameter [-1, 1]
    pub g: f64,
}

impl CorrectionInput {
    /// Create input from raw material parameters
    pub fn new(
        wavelength: f64,
        cos_theta_i: f64,
        cos_theta_o: f64,
        roughness: f64,
        ior: f64,
        k: f64,
        thickness: f64,
        absorption: f64,
        scattering: f64,
        g: f64,
    ) -> Self {
        Self {
            wavelength_normalized: ((wavelength - 400.0) / 300.0).clamp(0.0, 1.0),
            cos_theta_i: cos_theta_i.clamp(-1.0, 1.0),
            cos_theta_o: cos_theta_o.clamp(-1.0, 1.0),
            roughness: roughness.clamp(0.0, 1.0),
            ior_normalized: ((ior - 1.0) / 3.0).clamp(0.0, 1.0),
            k_normalized: (k / 10.0).clamp(0.0, 1.0),
            thickness_normalized: (thickness / 2000.0).clamp(0.0, 1.0),
            absorption_normalized: (absorption / 100.0).clamp(0.0, 1.0),
            scattering_normalized: (scattering / 100.0).clamp(0.0, 1.0),
            g: g.clamp(-1.0, 1.0),
        }
    }

    /// Create from BSDF context with default material params
    pub fn from_context(ctx: &BSDFContext, roughness: f64, ior: f64) -> Self {
        Self::new(
            ctx.wavelength,
            ctx.wi.z.abs(), // cos_theta_i
            ctx.wo.z.abs(), // cos_theta_o
            roughness,
            ior,
            0.0, // k
            0.0, // thickness
            0.0, // absorption
            0.0, // scattering
            0.0, // g
        )
    }

    /// Convert to vector for neural network input
    pub fn to_vec(&self) -> [f64; 10] {
        [
            self.wavelength_normalized,
            self.cos_theta_i,
            self.cos_theta_o,
            self.roughness,
            self.ior_normalized,
            self.k_normalized,
            self.thickness_normalized,
            self.absorption_normalized,
            self.scattering_normalized,
            self.g,
        ]
    }
}

/// Output correction values with energy constraints.
/// The network outputs small corrections (±max_correction) to R and T.
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
pub struct CorrectionOutput {
    /// Correction to reflectance: ΔR ∈ [-max_correction, +max_correction]
    pub delta_reflectance: f64,
    /// Correction to transmittance: ΔT ∈ [-max_correction, +max_correction]
    pub delta_transmittance: f64,
}

impl CorrectionOutput {
    /// Create new correction output
    pub fn new(delta_r: f64, delta_t: f64) -> Self {
        Self {
            delta_reflectance: delta_r,
            delta_transmittance: delta_t,
        }
    }

    /// Zero correction (no change)
    pub fn zero() -> Self {
        Self::default()
    }

    /// Total correction magnitude
    pub fn magnitude(&self) -> f64 {
        self.delta_reflectance.abs() + self.delta_transmittance.abs()
    }
}

// ============================================================================
// NEURAL CORRECTION MLP (SIREN)
// ============================================================================

/// Configuration for the neural correction network
#[derive(Debug, Clone)]
pub struct NeuralCorrectionConfig {
    /// Hidden layer dimension (default: 32)
    pub hidden_dim: usize,
    /// Number of hidden layers (default: 2)
    pub num_hidden_layers: usize,
    /// SIREN omega_0 for first layer (default: 30.0)
    pub omega_0: f64,
    /// Maximum correction per component (default: 0.1 = 10%)
    pub max_correction: f64,
    /// Random seed for initialization
    pub seed: u64,
}

impl Default for NeuralCorrectionConfig {
    fn default() -> Self {
        Self {
            hidden_dim: 32,
            num_hidden_layers: 2,
            omega_0: 30.0,
            max_correction: 0.1,
            seed: 42,
        }
    }
}

/// Small SIREN MLP for learning physics correction residuals.
///
/// Architecture:
/// - Input: 10 features (wavelength, angles, material params)
/// - Hidden: 2 layers x 32 neurons with sin() activation
/// - Output: 2 values (ΔR, ΔT) with tanh * max_correction
///
/// Total parameters: 1,442 floats (~11.5 KB)
#[derive(Debug, Clone)]
pub struct NeuralCorrectionMLP {
    /// Weights for layer 0: [hidden_dim, input_dim] = [32, 10]
    w0: Vec<f64>,
    /// Biases for layer 0: [hidden_dim] = [32]
    b0: Vec<f64>,
    /// Weights for layer 1: [hidden_dim, hidden_dim] = [32, 32]
    w1: Vec<f64>,
    /// Biases for layer 1: [hidden_dim] = [32]
    b1: Vec<f64>,
    /// Weights for output layer: [output_dim, hidden_dim] = [2, 32]
    w_out: Vec<f64>,
    /// Biases for output layer: [output_dim] = [2]
    b_out: Vec<f64>,
    /// Configuration
    config: NeuralCorrectionConfig,
}

impl NeuralCorrectionMLP {
    /// Input dimension
    pub const INPUT_DIM: usize = 10;
    /// Output dimension (ΔR, ΔT)
    pub const OUTPUT_DIM: usize = 2;

    /// Create a new neural correction network with random initialization
    pub fn new(config: NeuralCorrectionConfig) -> Self {
        let hidden = config.hidden_dim;
        let mut rng = SimpleRng::new(config.seed);

        // SIREN initialization: uniform in [-c, c] where c = sqrt(6/fan_in) / omega_0
        // For first layer, c = sqrt(6/10) / omega_0
        let c0 = (6.0 / Self::INPUT_DIM as f64).sqrt() / config.omega_0;
        let w0: Vec<f64> = (0..hidden * Self::INPUT_DIM)
            .map(|_| rng.uniform(-c0, c0))
            .collect();
        let b0: Vec<f64> = (0..hidden).map(|_| rng.uniform(-c0, c0)).collect();

        // Hidden layers: c = sqrt(6/hidden)
        let c1 = (6.0 / hidden as f64).sqrt();
        let w1: Vec<f64> = (0..hidden * hidden).map(|_| rng.uniform(-c1, c1)).collect();
        let b1: Vec<f64> = (0..hidden).map(|_| rng.uniform(-c1, c1)).collect();

        // Output layer
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
            config,
        }
    }

    /// Create with default configuration
    pub fn with_default_config() -> Self {
        Self::new(NeuralCorrectionConfig::default())
    }

    /// Total number of parameters
    pub fn param_count(&self) -> usize {
        let hidden = self.config.hidden_dim;
        // w0: hidden * input, b0: hidden
        // w1: hidden * hidden, b1: hidden
        // w_out: output * hidden, b_out: output
        hidden * Self::INPUT_DIM
            + hidden
            + hidden * hidden
            + hidden
            + Self::OUTPUT_DIM * hidden
            + Self::OUTPUT_DIM
    }

    /// Memory usage in bytes
    pub fn memory_bytes(&self) -> usize {
        self.param_count() * std::mem::size_of::<f64>()
    }

    /// Forward pass through the network
    pub fn forward(&self, input: &CorrectionInput) -> CorrectionOutput {
        let x = input.to_vec();
        let hidden = self.config.hidden_dim;

        // Layer 0: y = sin(omega_0 * (W0 @ x + b0))
        let mut h0 = vec![0.0; hidden];
        for i in 0..hidden {
            let mut sum = self.b0[i];
            for j in 0..Self::INPUT_DIM {
                sum += self.w0[i * Self::INPUT_DIM + j] * x[j];
            }
            h0[i] = (self.config.omega_0 * sum).sin();
        }

        // Layer 1: y = sin(W1 @ h0 + b1)
        let mut h1 = vec![0.0; hidden];
        for i in 0..hidden {
            let mut sum = self.b1[i];
            for j in 0..hidden {
                sum += self.w1[i * hidden + j] * h0[j];
            }
            h1[i] = sum.sin();
        }

        // Output layer: y = tanh(W_out @ h1 + b_out) * max_correction
        let mut out = [0.0; Self::OUTPUT_DIM];
        for i in 0..Self::OUTPUT_DIM {
            let mut sum = self.b_out[i];
            for j in 0..hidden {
                sum += self.w_out[i * hidden + j] * h1[j];
            }
            out[i] = sum.tanh() * self.config.max_correction;
        }

        CorrectionOutput::new(out[0], out[1])
    }

    /// Apply correction to a physical BSDF response with energy conservation
    pub fn apply(&self, physical: &BSDFResponse, input: &CorrectionInput) -> BSDFResponse {
        let correction = self.forward(input);

        // Apply corrections
        let r_corrected = physical.reflectance + correction.delta_reflectance;
        let t_corrected = physical.transmittance + correction.delta_transmittance;

        // Clamp to physical range [0, 1]
        let r_clamped = r_corrected.clamp(0.0, 1.0);
        let t_clamped = t_corrected.clamp(0.0, 1.0);

        // Ensure energy conservation: R + T <= 1
        let total = r_clamped + t_clamped;
        if total > 1.0 {
            // Scale proportionally
            let scale = 1.0 / total;
            BSDFResponse::new(r_clamped * scale, t_clamped * scale, 0.0)
        } else {
            // Absorption is the remainder
            BSDFResponse::new(r_clamped, t_clamped, 1.0 - r_clamped - t_clamped)
        }
    }

    /// Get all parameters as a flat vector (for optimization)
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

    /// Set parameters from a flat vector
    pub fn set_params(&mut self, params: &[f64]) {
        assert_eq!(params.len(), self.param_count());
        let hidden = self.config.hidden_dim;

        let mut idx = 0;

        // w0
        let w0_len = hidden * Self::INPUT_DIM;
        self.w0.copy_from_slice(&params[idx..idx + w0_len]);
        idx += w0_len;

        // b0
        self.b0.copy_from_slice(&params[idx..idx + hidden]);
        idx += hidden;

        // w1
        let w1_len = hidden * hidden;
        self.w1.copy_from_slice(&params[idx..idx + w1_len]);
        idx += w1_len;

        // b1
        self.b1.copy_from_slice(&params[idx..idx + hidden]);
        idx += hidden;

        // w_out
        let w_out_len = Self::OUTPUT_DIM * hidden;
        self.w_out.copy_from_slice(&params[idx..idx + w_out_len]);
        idx += w_out_len;

        // b_out
        self.b_out
            .copy_from_slice(&params[idx..idx + Self::OUTPUT_DIM]);
    }

    /// Apply parameter updates (for gradient descent)
    pub fn apply_updates(&mut self, updates: &[f64]) {
        assert_eq!(updates.len(), self.param_count());
        let mut params = self.get_params();
        for (p, u) in params.iter_mut().zip(updates.iter()) {
            *p += u;
        }
        self.set_params(&params);
    }

    /// Get max correction value
    pub fn max_correction(&self) -> f64 {
        self.config.max_correction
    }
}

// ============================================================================
// NEURAL CORRECTED BSDF WRAPPER
// ============================================================================

/// BSDF wrapper that applies neural correction to any underlying physical BSDF.
///
/// This implements the core Phase 10 principle:
/// ```text
/// Final_Output = Physical_Model_Output + Neural_Correction
/// ```
///
/// The neural correction can be disabled to fall back to physics-only.
#[derive(Debug, Clone)]
pub struct NeuralCorrectedBSDF<B: BSDF + Clone> {
    /// Underlying physical BSDF
    physical: B,
    /// Neural correction network
    correction: NeuralCorrectionMLP,
    /// Whether neural correction is enabled
    enabled: bool,
    /// Material parameters for correction input
    roughness: f64,
    ior: f64,
    k: f64,
}

impl<B: BSDF + Clone> NeuralCorrectedBSDF<B> {
    /// Create a new neural-corrected BSDF
    pub fn new(physical: B, correction: NeuralCorrectionMLP, roughness: f64, ior: f64) -> Self {
        Self {
            physical,
            correction,
            enabled: true,
            roughness,
            ior,
            k: 0.0,
        }
    }

    /// Create with conductor parameters
    pub fn with_conductor(
        physical: B,
        correction: NeuralCorrectionMLP,
        roughness: f64,
        n: f64,
        k: f64,
    ) -> Self {
        Self {
            physical,
            correction,
            enabled: true,
            roughness,
            ior: n,
            k,
        }
    }

    /// Enable neural correction
    pub fn enable(&mut self) {
        self.enabled = true;
    }

    /// Disable neural correction (fall back to physics-only)
    pub fn disable(&mut self) {
        self.enabled = false;
    }

    /// Check if neural correction is enabled
    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    /// Get reference to the underlying physical BSDF
    pub fn physical(&self) -> &B {
        &self.physical
    }

    /// Get reference to the correction network
    pub fn network(&self) -> &NeuralCorrectionMLP {
        &self.correction
    }

    /// Get mutable reference to the correction network (for training)
    pub fn network_mut(&mut self) -> &mut NeuralCorrectionMLP {
        &mut self.correction
    }
}

impl<B: BSDF + Clone> BSDF for NeuralCorrectedBSDF<B> {
    fn evaluate(&self, ctx: &BSDFContext) -> BSDFResponse {
        // Get physical response
        let physical_response = self.physical.evaluate(ctx);

        if self.enabled {
            // Create correction input from context
            let input = CorrectionInput::new(
                ctx.wavelength,
                ctx.wi.z.abs(),
                ctx.wo.z.abs(),
                self.roughness,
                self.ior,
                self.k,
                0.0, // thickness
                0.0, // absorption
                0.0, // scattering
                0.0, // g
            );

            // Apply neural correction
            self.correction.apply(&physical_response, &input)
        } else {
            // Return physics-only
            physical_response
        }
    }

    fn sample(&self, ctx: &BSDFContext, u1: f64, u2: f64) -> BSDFSample {
        // Sampling uses the physical BSDF (correction doesn't change sampling distribution)
        self.physical.sample(ctx, u1, u2)
    }

    fn pdf(&self, ctx: &BSDFContext) -> f64 {
        // PDF is from physical BSDF
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
                "NeuralCorrectedBSDF: R={:.4}, T={:.4}, A={:.4}, sum={:.6}",
                response.reflectance, response.transmittance, response.absorption, total
            ),
        }
    }

    fn name(&self) -> &str {
        "NeuralCorrectedBSDF"
    }

    fn is_delta(&self) -> bool {
        self.physical.is_delta()
    }
}

// ============================================================================
// SIMPLE RNG (for deterministic initialization)
// ============================================================================

/// Simple xorshift RNG for deterministic initialization
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

// ============================================================================
// MEMORY UTILITIES
// ============================================================================

/// Total memory usage of Phase 10 neural correction module
pub fn total_neural_correction_memory() -> usize {
    // Default network memory
    let network = NeuralCorrectionMLP::with_default_config();
    network.memory_bytes()
        + std::mem::size_of::<NeuralCorrectionConfig>()
        + std::mem::size_of::<CorrectionInput>()
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
    fn test_correction_input() {
        let input = CorrectionInput::new(
            550.0, // wavelength
            0.866, // cos_theta_i (30 degrees)
            0.866, // cos_theta_o
            0.1,   // roughness
            1.5,   // ior
            0.0,   // k
            0.0,   // thickness
            0.0,   // absorption
            0.0,   // scattering
            0.0,   // g
        );

        let vec = input.to_vec();
        assert_eq!(vec.len(), 10);
        assert!((vec[0] - 0.5).abs() < 0.01); // (550-400)/300 = 0.5
    }

    #[test]
    fn test_network_creation() {
        let config = NeuralCorrectionConfig::default();
        let network = NeuralCorrectionMLP::new(config);

        assert_eq!(network.param_count(), 1474);
        assert!(network.memory_bytes() < 12000); // < 12 KB
    }

    #[test]
    fn test_forward_pass_bounded() {
        let network = NeuralCorrectionMLP::with_default_config();
        let input = CorrectionInput::default();

        let output = network.forward(&input);

        // Outputs should be bounded by max_correction
        assert!(output.delta_reflectance.abs() <= 0.1);
        assert!(output.delta_transmittance.abs() <= 0.1);
    }

    #[test]
    fn test_apply_energy_conservation() {
        let network = NeuralCorrectionMLP::with_default_config();
        let physical = BSDFResponse::new(0.5, 0.3, 0.2);
        let input = CorrectionInput::default();

        let corrected = network.apply(&physical, &input);

        // Energy must be conserved
        let total = corrected.reflectance + corrected.transmittance + corrected.absorption;
        assert!(
            (total - 1.0).abs() < 1e-10,
            "Energy not conserved: {}",
            total
        );

        // All components must be non-negative
        assert!(corrected.reflectance >= 0.0);
        assert!(corrected.transmittance >= 0.0);
        assert!(corrected.absorption >= 0.0);
    }

    #[test]
    fn test_apply_extreme_physical_values() {
        let network = NeuralCorrectionMLP::with_default_config();
        let input = CorrectionInput::default();

        // Test with physical response near boundaries
        let physical_high = BSDFResponse::new(0.95, 0.04, 0.01);
        let corrected_high = network.apply(&physical_high, &input);
        let total =
            corrected_high.reflectance + corrected_high.transmittance + corrected_high.absorption;
        assert!((total - 1.0).abs() < 1e-10);

        let physical_low = BSDFResponse::new(0.01, 0.01, 0.98);
        let corrected_low = network.apply(&physical_low, &input);
        let total =
            corrected_low.reflectance + corrected_low.transmittance + corrected_low.absorption;
        assert!((total - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_params_roundtrip() {
        let mut network = NeuralCorrectionMLP::with_default_config();
        let original_params = network.get_params();

        // Modify params
        let mut new_params = original_params.clone();
        new_params[0] += 0.5;
        new_params[100] -= 0.3;
        network.set_params(&new_params);

        // Verify change
        let retrieved = network.get_params();
        assert!((retrieved[0] - new_params[0]).abs() < 1e-10);
        assert!((retrieved[100] - new_params[100]).abs() < 1e-10);
    }

    #[test]
    fn test_deterministic_initialization() {
        let config = NeuralCorrectionConfig {
            seed: 12345,
            ..Default::default()
        };

        let network1 = NeuralCorrectionMLP::new(config.clone());
        let network2 = NeuralCorrectionMLP::new(config);

        // Same seed should produce identical networks
        assert_eq!(network1.get_params(), network2.get_params());
    }

    #[test]
    fn test_neural_corrected_bsdf() {
        let physical = DielectricBSDF::new(1.5, 0.0);
        let network = NeuralCorrectionMLP::with_default_config();
        let corrected = NeuralCorrectedBSDF::new(physical.clone(), network, 0.0, 1.5);

        let ctx = BSDFContext::new_simple(1.0); // Normal incidence

        // Test evaluation
        let response = corrected.evaluate(&ctx);
        let total = response.reflectance + response.transmittance + response.absorption;
        assert!((total - 1.0).abs() < 1e-10);

        // Test energy validation
        let validation = corrected.validate_energy(&ctx);
        assert!(validation.conserved);
    }

    #[test]
    fn test_enable_disable() {
        let physical = DielectricBSDF::new(1.5, 0.0);
        let network = NeuralCorrectionMLP::with_default_config();
        let mut corrected = NeuralCorrectedBSDF::new(physical.clone(), network, 0.0, 1.5);

        let ctx = BSDFContext::new_simple(1.0);

        // Enabled
        let response_enabled = corrected.evaluate(&ctx);

        // Disabled
        corrected.disable();
        let response_disabled = corrected.evaluate(&ctx);

        // Should match physical when disabled
        let physical_response = physical.evaluate(&ctx);
        assert!((response_disabled.reflectance - physical_response.reflectance).abs() < 1e-10);

        // Re-enable
        corrected.enable();
        let response_reenabled = corrected.evaluate(&ctx);
        assert!((response_reenabled.reflectance - response_enabled.reflectance).abs() < 1e-10);
    }

    #[test]
    fn test_correction_magnitude() {
        let network = NeuralCorrectionMLP::with_default_config();

        // Test many random inputs
        for i in 0..100 {
            let input = CorrectionInput::new(
                400.0 + (i as f64) * 3.0,
                (i as f64 * 0.01).cos(),
                (i as f64 * 0.02).cos(),
                (i as f64 % 10.0) * 0.1,
                1.0 + (i as f64 % 30.0) * 0.1,
                0.0,
                0.0,
                0.0,
                0.0,
                0.0,
            );

            let output = network.forward(&input);
            assert!(
                output.magnitude() <= 0.2 + 1e-6,
                "Correction magnitude {} exceeds limit",
                output.magnitude()
            );
        }
    }

    #[test]
    fn test_memory_budget() {
        let memory = total_neural_correction_memory();
        assert!(memory < 15_000, "Memory {} exceeds 15KB budget", memory);
    }
}
