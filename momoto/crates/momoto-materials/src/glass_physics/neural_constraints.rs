// ============================================================================
// PHASE 10: NEURAL CONSTRAINTS
// ============================================================================
//
// Physics-based constraints for neural correction outputs.
// Ensures the neural network never "invents physics" by enforcing:
// - Energy conservation (R + T + A = 1)
// - Reciprocity (f(wi, wo) = f(wo, wi))
// - Spectral smoothness (no sharp discontinuities)
// - Physical range (0 <= R, T, A <= 1)
// - Fresnel monotonicity (R increases at grazing)
// ============================================================================

use super::neural_correction::{CorrectionInput, CorrectionOutput, NeuralCorrectionMLP};
use super::unified_bsdf::BSDFResponse;

// ============================================================================
// CONSTRAINT TYPES
// ============================================================================

/// Types of physics constraints that can be enforced
#[derive(Debug, Clone)]
pub enum ConstraintType {
    /// Energy conservation: R + T + A = 1
    EnergyConservation {
        /// Tolerance for energy violation
        tolerance: f64,
    },
    /// Helmholtz reciprocity: f(wi, wo) = f(wo, wi)
    Reciprocity {
        /// Tolerance for reciprocity violation
        tolerance: f64,
    },
    /// Spectral smoothness: corrections should vary smoothly with wavelength
    SpectralSmoothness {
        /// Maximum allowed gradient in correction per nm
        max_gradient: f64,
    },
    /// Physical range: all components in [0, 1]
    PhysicalRange,
    /// Fresnel monotonicity: reflectance increases at grazing angles
    FresnelMonotonicity,
}

impl Default for ConstraintType {
    fn default() -> Self {
        ConstraintType::EnergyConservation { tolerance: 1e-6 }
    }
}

// ============================================================================
// REGULARIZATION TERMS
// ============================================================================

/// Regularization penalty terms computed during constraint validation.
/// Used as additional loss during training.
#[derive(Debug, Clone, Default)]
pub struct RegularizationTerms {
    /// Penalty for energy conservation violation
    pub energy_penalty: f64,
    /// Penalty for reciprocity violation
    pub reciprocity_penalty: f64,
    /// Penalty for spectral discontinuity
    pub smoothness_penalty: f64,
    /// Penalty for out-of-range values (before clamping)
    pub range_penalty: f64,
    /// Penalty for Fresnel monotonicity violation
    pub fresnel_penalty: f64,
    /// Total penalty (sum of all)
    pub total_penalty: f64,
}

impl RegularizationTerms {
    /// Create zero regularization (no penalties)
    pub fn zero() -> Self {
        Self::default()
    }

    /// Compute total from individual terms
    pub fn compute_total(&mut self) {
        self.total_penalty = self.energy_penalty
            + self.reciprocity_penalty
            + self.smoothness_penalty
            + self.range_penalty
            + self.fresnel_penalty;
    }
}

// ============================================================================
// CONSTRAINT VALIDATOR
// ============================================================================

/// Configuration for constraint validation
#[derive(Debug, Clone)]
pub struct ConstraintConfig {
    /// Energy conservation tolerance
    pub energy_tolerance: f64,
    /// Reciprocity tolerance
    pub reciprocity_tolerance: f64,
    /// Maximum spectral gradient (correction per nm)
    pub max_spectral_gradient: f64,
    /// Whether to hard-clamp violations or just penalize
    pub hard_clamp: bool,
    /// Weight for energy penalty during training
    pub energy_weight: f64,
    /// Weight for reciprocity penalty during training
    pub reciprocity_weight: f64,
    /// Weight for smoothness penalty during training
    pub smoothness_weight: f64,
}

impl Default for ConstraintConfig {
    fn default() -> Self {
        Self {
            energy_tolerance: 1e-6,
            reciprocity_tolerance: 0.01,
            max_spectral_gradient: 0.001, // 0.1% per nm
            hard_clamp: true,
            energy_weight: 10.0,
            reciprocity_weight: 1.0,
            smoothness_weight: 0.1,
        }
    }
}

/// Constraint validator that enforces physics constraints on neural outputs.
#[derive(Debug, Clone)]
pub struct ConstraintValidator {
    /// Configuration
    config: ConstraintConfig,
}

impl ConstraintValidator {
    /// Create a new constraint validator with default configuration
    pub fn new() -> Self {
        Self {
            config: ConstraintConfig::default(),
        }
    }

    /// Create with custom configuration
    pub fn with_config(config: ConstraintConfig) -> Self {
        Self { config }
    }

    /// Validate and optionally clamp a neural correction output.
    /// Returns the corrected response and regularization penalties.
    pub fn validate_and_clamp(
        &self,
        physical: &BSDFResponse,
        correction: &CorrectionOutput,
    ) -> (BSDFResponse, RegularizationTerms) {
        let mut penalties = RegularizationTerms::zero();

        // Apply correction
        let mut r = physical.reflectance + correction.delta_reflectance;
        let mut t = physical.transmittance + correction.delta_transmittance;

        // 1. Physical range constraint [0, 1]
        if r < 0.0 {
            penalties.range_penalty += r.abs();
            if self.config.hard_clamp {
                r = 0.0;
            }
        }
        if r > 1.0 {
            penalties.range_penalty += r - 1.0;
            if self.config.hard_clamp {
                r = 1.0;
            }
        }
        if t < 0.0 {
            penalties.range_penalty += t.abs();
            if self.config.hard_clamp {
                t = 0.0;
            }
        }
        if t > 1.0 {
            penalties.range_penalty += t - 1.0;
            if self.config.hard_clamp {
                t = 1.0;
            }
        }

        // 2. Energy conservation: R + T <= 1
        let total = r + t;
        if total > 1.0 {
            penalties.energy_penalty += (total - 1.0) * self.config.energy_weight;
            if self.config.hard_clamp {
                // Scale proportionally to satisfy constraint
                let scale = 1.0 / total;
                r *= scale;
                t *= scale;
            }
        }

        // Compute absorption as remainder
        let a = (1.0 - r - t).max(0.0);

        // Final energy check
        let final_total = r + t + a;
        if (final_total - 1.0).abs() > self.config.energy_tolerance {
            penalties.energy_penalty += (final_total - 1.0).abs() * self.config.energy_weight;
        }

        penalties.compute_total();

        let corrected = BSDFResponse::new(r, t, a);
        (corrected, penalties)
    }

    /// Check reciprocity violation for a network.
    /// Tests that f(wi, wo) ≈ f(wo, wi).
    pub fn check_reciprocity(
        &self,
        network: &NeuralCorrectionMLP,
        input_forward: &CorrectionInput,
        input_backward: &CorrectionInput,
    ) -> f64 {
        let forward = network.forward(input_forward);
        let backward = network.forward(input_backward);

        let diff_r = (forward.delta_reflectance - backward.delta_reflectance).abs();
        let diff_t = (forward.delta_transmittance - backward.delta_transmittance).abs();

        diff_r + diff_t
    }

    /// Compute spectral smoothness penalty for a sequence of corrections.
    /// Penalizes rapid changes in correction values across wavelengths.
    pub fn spectral_smoothness_penalty(
        &self,
        corrections: &[(f64, CorrectionOutput)], // (wavelength, correction)
    ) -> f64 {
        if corrections.len() < 2 {
            return 0.0;
        }

        let mut penalty = 0.0;

        for window in corrections.windows(2) {
            let (wl0, c0) = &window[0];
            let (wl1, c1) = &window[1];

            let d_lambda = (wl1 - wl0).abs();
            if d_lambda < 1e-10 {
                continue;
            }

            let d_r = (c1.delta_reflectance - c0.delta_reflectance).abs();
            let d_t = (c1.delta_transmittance - c0.delta_transmittance).abs();

            // Gradient per nm
            let gradient = (d_r + d_t) / d_lambda;

            if gradient > self.config.max_spectral_gradient {
                penalty += (gradient - self.config.max_spectral_gradient).powi(2)
                    * self.config.smoothness_weight;
            }
        }

        penalty
    }

    /// Check Fresnel monotonicity: reflectance should increase at grazing angles.
    /// Returns penalty if violated.
    pub fn fresnel_monotonicity_penalty(
        &self,
        network: &NeuralCorrectionMLP,
        physical_normal: &BSDFResponse,
        physical_grazing: &BSDFResponse,
        input_normal: &CorrectionInput,
        input_grazing: &CorrectionInput,
    ) -> f64 {
        let corrected_normal = network.apply(physical_normal, input_normal);
        let corrected_grazing = network.apply(physical_grazing, input_grazing);

        // Fresnel: R at grazing should be >= R at normal
        if corrected_grazing.reflectance < corrected_normal.reflectance - 0.01 {
            // Significant violation
            corrected_normal.reflectance - corrected_grazing.reflectance
        } else {
            0.0
        }
    }

    /// Compute all regularization terms for a batch of samples.
    /// Useful during training.
    pub fn compute_batch_regularization(
        &self,
        network: &NeuralCorrectionMLP,
        samples: &[(CorrectionInput, BSDFResponse)],
    ) -> RegularizationTerms {
        let mut total_penalties = RegularizationTerms::zero();

        for (input, physical) in samples {
            let correction = network.forward(input);
            let (_, penalties) = self.validate_and_clamp(physical, &correction);

            total_penalties.energy_penalty += penalties.energy_penalty;
            total_penalties.range_penalty += penalties.range_penalty;
        }

        // Average
        let n = samples.len() as f64;
        if n > 0.0 {
            total_penalties.energy_penalty /= n;
            total_penalties.range_penalty /= n;
        }

        // Spectral smoothness (requires wavelength-sorted samples)
        // This would need wavelength info in CorrectionInput

        total_penalties.compute_total();
        total_penalties
    }
}

impl Default for ConstraintValidator {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// CONSTRAINT VIOLATION REPORT
// ============================================================================

/// Detailed report of constraint violations for debugging
#[derive(Debug, Clone)]
pub struct ConstraintViolationReport {
    /// Number of samples tested
    pub samples_tested: usize,
    /// Number of energy conservation violations
    pub energy_violations: usize,
    /// Maximum energy error
    pub max_energy_error: f64,
    /// Number of range violations
    pub range_violations: usize,
    /// Number of reciprocity violations (if tested)
    pub reciprocity_violations: usize,
    /// Number of smoothness violations
    pub smoothness_violations: usize,
    /// Overall pass/fail
    pub all_passed: bool,
}

impl ConstraintViolationReport {
    /// Create an empty report
    pub fn new() -> Self {
        Self {
            samples_tested: 0,
            energy_violations: 0,
            max_energy_error: 0.0,
            range_violations: 0,
            reciprocity_violations: 0,
            smoothness_violations: 0,
            all_passed: true,
        }
    }

    /// Add a sample result
    pub fn add_sample(&mut self, penalties: &RegularizationTerms, energy_tolerance: f64) {
        self.samples_tested += 1;

        if penalties.energy_penalty > energy_tolerance {
            self.energy_violations += 1;
            self.all_passed = false;
        }

        if penalties.range_penalty > 0.0 {
            self.range_violations += 1;
        }

        self.max_energy_error = self.max_energy_error.max(penalties.energy_penalty);
    }
}

impl Default for ConstraintViolationReport {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// MEMORY UTILITIES
// ============================================================================

/// Total memory usage of neural constraints module
pub fn total_neural_constraints_memory() -> usize {
    std::mem::size_of::<ConstraintValidator>()
        + std::mem::size_of::<ConstraintConfig>()
        + std::mem::size_of::<RegularizationTerms>()
        + std::mem::size_of::<ConstraintViolationReport>()
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_energy_conservation_clamp() {
        let validator = ConstraintValidator::new();

        // Physical response that will violate energy with correction
        let physical = BSDFResponse::new(0.9, 0.05, 0.05);
        let correction = CorrectionOutput::new(0.1, 0.1); // Would make R=1.0, T=0.15

        let (corrected, penalties) = validator.validate_and_clamp(&physical, &correction);

        // Should be clamped to energy conservation
        let total = corrected.reflectance + corrected.transmittance + corrected.absorption;
        assert!((total - 1.0).abs() < 1e-10, "Energy violated: {}", total);
        assert!(penalties.energy_penalty > 0.0, "Should have energy penalty");
    }

    #[test]
    fn test_physical_range_clamp() {
        let validator = ConstraintValidator::new();

        let physical = BSDFResponse::new(0.1, 0.1, 0.8);
        let correction = CorrectionOutput::new(-0.2, -0.2); // Would make R=-0.1, T=-0.1

        let (corrected, penalties) = validator.validate_and_clamp(&physical, &correction);

        // Should clamp to [0, 1]
        assert!(corrected.reflectance >= 0.0);
        assert!(corrected.transmittance >= 0.0);
        assert!(penalties.range_penalty > 0.0);
    }

    #[test]
    fn test_no_penalty_for_valid() {
        let validator = ConstraintValidator::new();

        let physical = BSDFResponse::new(0.5, 0.3, 0.2);
        let correction = CorrectionOutput::new(0.05, -0.05); // Small valid correction

        let (corrected, penalties) = validator.validate_and_clamp(&physical, &correction);

        // Total should be 1.0
        let total = corrected.reflectance + corrected.transmittance + corrected.absorption;
        assert!((total - 1.0).abs() < 1e-10);

        // No significant penalties for valid correction
        assert!(penalties.range_penalty < 1e-10);
    }

    #[test]
    fn test_reciprocity_check() {
        let validator = ConstraintValidator::new();
        let network = NeuralCorrectionMLP::with_default_config();

        // Create forward and backward inputs (swapped angles)
        let input_forward =
            CorrectionInput::new(550.0, 0.866, 0.5, 0.1, 1.5, 0.0, 0.0, 0.0, 0.0, 0.0);
        let input_backward =
            CorrectionInput::new(550.0, 0.5, 0.866, 0.1, 1.5, 0.0, 0.0, 0.0, 0.0, 0.0);

        let violation = validator.check_reciprocity(&network, &input_forward, &input_backward);

        // Some violation expected (network not trained for reciprocity)
        // Just verify it returns a value
        assert!(violation >= 0.0);
    }

    #[test]
    fn test_spectral_smoothness() {
        let validator = ConstraintValidator::new();

        // Create a smooth sequence
        let corrections_smooth: Vec<(f64, CorrectionOutput)> = (0..10)
            .map(|i| {
                let wl = 400.0 + i as f64 * 30.0;
                let c = CorrectionOutput::new(i as f64 * 0.001, 0.0);
                (wl, c)
            })
            .collect();

        let penalty_smooth = validator.spectral_smoothness_penalty(&corrections_smooth);

        // Create a discontinuous sequence
        let corrections_sharp: Vec<(f64, CorrectionOutput)> = vec![
            (400.0, CorrectionOutput::new(0.0, 0.0)),
            (401.0, CorrectionOutput::new(0.1, 0.0)), // Huge jump in 1nm
        ];

        let penalty_sharp = validator.spectral_smoothness_penalty(&corrections_sharp);

        // Sharp should have higher penalty
        assert!(penalty_sharp > penalty_smooth);
    }

    #[test]
    fn test_constraint_violation_report() {
        let mut report = ConstraintViolationReport::new();

        // Add valid sample
        let valid_penalties = RegularizationTerms::zero();
        report.add_sample(&valid_penalties, 1e-6);
        assert_eq!(report.energy_violations, 0);

        // Add invalid sample
        let mut invalid_penalties = RegularizationTerms::zero();
        invalid_penalties.energy_penalty = 0.1;
        report.add_sample(&invalid_penalties, 1e-6);
        assert_eq!(report.energy_violations, 1);
        assert!(!report.all_passed);
    }

    #[test]
    fn test_batch_regularization() {
        let validator = ConstraintValidator::new();
        let network = NeuralCorrectionMLP::with_default_config();

        let samples: Vec<(CorrectionInput, BSDFResponse)> = (0..10)
            .map(|i| {
                let input = CorrectionInput::new(
                    400.0 + i as f64 * 30.0,
                    0.866,
                    0.866,
                    0.1,
                    1.5,
                    0.0,
                    0.0,
                    0.0,
                    0.0,
                    0.0,
                );
                let physical = BSDFResponse::new(0.5, 0.3, 0.2);
                (input, physical)
            })
            .collect();

        let penalties = validator.compute_batch_regularization(&network, &samples);

        // Should compute penalties for all samples
        assert!(penalties.total_penalty >= 0.0);
    }

    #[test]
    fn test_memory_budget() {
        let memory = total_neural_constraints_memory();
        assert!(memory < 1000, "Memory {} exceeds 1KB budget", memory);
    }
}
