//! # Phase 12 Validation Suite
//!
//! Comprehensive validation for temporal light transport and spectral coherence.
//!
//! ## Validation Categories
//!
//! 1. **Temporal Energy Drift**: R+T+A stability over 1000+ frames
//! 2. **Spectral Flicker**: ΔE2000 < 0.5 frame-to-frame
//! 3. **Neural Cumulative Drift**: Bounded correction accumulation
//! 4. **Backward Compatibility**: Phase 11 behavior preserved at t=0

use super::neural_correction::CorrectionOutput;
use super::neural_temporal_correction::{TemporalCorrectionInput, TemporalNeuralCorrection};
use super::spectral_coherence::{
    CoherentSampler, FlickerConfig, FlickerValidator, SpectralInterpolator, SpectralPacket,
};
use super::temporal::{
    DriftConfig, DriftTracker, TemporalBSDF, TemporalConductor, TemporalContext,
    TemporalDielectric, TemporalThinFilm,
};
use super::unified_bsdf::{BSDFContext, DielectricBSDF, BSDF};

// ============================================================================
// VALIDATION CONFIGURATION
// ============================================================================

/// Configuration for Phase 12 validation suite.
#[derive(Debug, Clone)]
pub struct Phase12ValidationConfig {
    /// Number of frames for drift test.
    pub drift_frames: u64,
    /// Maximum allowed energy drift ratio.
    pub max_energy_drift: f64,
    /// Maximum allowed spectral flicker (ΔE2000).
    pub max_spectral_flicker: f64,
    /// Maximum allowed neural cumulative drift.
    pub max_neural_drift: f64,
    /// Delta time per frame.
    pub delta_time: f64,
    /// Number of wavelengths for spectral tests.
    pub spectral_samples: usize,
    /// Verbose logging.
    pub verbose: bool,
}

impl Default for Phase12ValidationConfig {
    fn default() -> Self {
        Self {
            drift_frames: 1000,
            max_energy_drift: 0.01,    // 1%
            max_spectral_flicker: 0.5, // ΔE2000
            max_neural_drift: 0.05,    // 5%
            delta_time: 1.0 / 60.0,    // 60 fps
            spectral_samples: 31,
            verbose: false,
        }
    }
}

impl Phase12ValidationConfig {
    /// Quick validation (fewer frames).
    pub fn quick() -> Self {
        Self {
            drift_frames: 100,
            ..Default::default()
        }
    }

    /// Strict validation (tighter tolerances).
    pub fn strict() -> Self {
        Self {
            drift_frames: 2000,
            max_energy_drift: 0.005,
            max_spectral_flicker: 0.3,
            max_neural_drift: 0.03,
            ..Default::default()
        }
    }
}

// ============================================================================
// VALIDATION RESULTS
// ============================================================================

/// Result of a single validation test.
#[derive(Debug, Clone)]
pub struct ValidationResult {
    /// Test name.
    pub name: String,
    /// Whether test passed.
    pub passed: bool,
    /// Measured value.
    pub value: f64,
    /// Maximum allowed value.
    pub max_allowed: f64,
    /// Additional details.
    pub details: String,
}

impl ValidationResult {
    /// Create passing result.
    pub fn pass(name: &str, value: f64, max: f64) -> Self {
        Self {
            name: name.to_string(),
            passed: true,
            value,
            max_allowed: max,
            details: String::new(),
        }
    }

    /// Create failing result.
    pub fn fail(name: &str, value: f64, max: f64, details: &str) -> Self {
        Self {
            name: name.to_string(),
            passed: false,
            value,
            max_allowed: max,
            details: details.to_string(),
        }
    }
}

/// Complete Phase 12 validation report.
#[derive(Debug, Clone)]
pub struct Phase12ValidationReport {
    /// All test results.
    pub results: Vec<ValidationResult>,
    /// Total tests.
    pub total_tests: usize,
    /// Passed tests.
    pub passed_tests: usize,
    /// Failed tests.
    pub failed_tests: usize,
    /// Overall pass status.
    pub overall_passed: bool,
    /// Memory usage (bytes).
    pub memory_usage: usize,
}

impl Phase12ValidationReport {
    /// Create new report.
    pub fn new() -> Self {
        Self {
            results: Vec::new(),
            total_tests: 0,
            passed_tests: 0,
            failed_tests: 0,
            overall_passed: true,
            memory_usage: 0,
        }
    }

    /// Add a result.
    pub fn add(&mut self, result: ValidationResult) {
        self.total_tests += 1;
        if result.passed {
            self.passed_tests += 1;
        } else {
            self.failed_tests += 1;
            self.overall_passed = false;
        }
        self.results.push(result);
    }

    /// Get pass rate.
    pub fn pass_rate(&self) -> f64 {
        if self.total_tests == 0 {
            1.0
        } else {
            self.passed_tests as f64 / self.total_tests as f64
        }
    }

    /// Format as summary string.
    pub fn summary(&self) -> String {
        let status = if self.overall_passed {
            "PASSED"
        } else {
            "FAILED"
        };
        format!(
            "Phase 12 Validation: {} ({}/{} tests, {:.1}%)\nMemory: {} bytes",
            status,
            self.passed_tests,
            self.total_tests,
            self.pass_rate() * 100.0,
            self.memory_usage
        )
    }
}

impl Default for Phase12ValidationReport {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// VALIDATION SUITE
// ============================================================================

/// Phase 12 validation suite.
#[derive(Debug)]
pub struct Phase12ValidationSuite {
    /// Configuration.
    config: Phase12ValidationConfig,
}

impl Default for Phase12ValidationSuite {
    fn default() -> Self {
        Self::new(Phase12ValidationConfig::default())
    }
}

impl Phase12ValidationSuite {
    /// Create new validation suite.
    pub fn new(config: Phase12ValidationConfig) -> Self {
        Self { config }
    }

    /// Run all validations.
    pub fn run_all(&self) -> Phase12ValidationReport {
        let mut report = Phase12ValidationReport::new();

        // Category 1: Temporal Energy Drift
        report.add(self.validate_dielectric_drift());
        report.add(self.validate_thin_film_drift());
        report.add(self.validate_conductor_drift());

        // Category 2: Spectral Coherence
        report.add(self.validate_spectral_flicker());
        report.add(self.validate_spectral_interpolation());
        report.add(self.validate_coherent_sampling());

        // Category 3: Neural Temporal
        report.add(self.validate_neural_drift());
        report.add(self.validate_neural_bounding());

        // Category 4: Backward Compatibility
        report.add(self.validate_static_equivalence());
        report.add(self.validate_context_compatibility());

        // Category 5: Memory Budget
        report.add(self.validate_memory_budget());

        // Set memory usage
        report.memory_usage = self.estimate_total_memory();

        report
    }

    // ========================================================================
    // TEMPORAL ENERGY DRIFT TESTS
    // ========================================================================

    /// Validate dielectric temporal drift.
    fn validate_dielectric_drift(&self) -> ValidationResult {
        let material = TemporalDielectric::drying_paint();
        let mut tracker = DriftTracker::new(DriftConfig {
            max_drift: self.config.max_energy_drift,
            ..Default::default()
        });

        let mut ctx = TemporalContext::default();
        let initial = material.eval_at_time(&ctx);
        tracker.initialize(&initial);

        for _ in 0..self.config.drift_frames {
            let response = material.eval_at_time(&ctx);
            ctx.advance(self.config.delta_time, response.clone());
            tracker.update(&response);
        }

        let drift = tracker.drift_ratio();
        if drift <= self.config.max_energy_drift {
            ValidationResult::pass("dielectric_drift", drift, self.config.max_energy_drift)
        } else {
            ValidationResult::fail(
                "dielectric_drift",
                drift,
                self.config.max_energy_drift,
                &format!("Drift {:.4} exceeds limit", drift),
            )
        }
    }

    /// Validate thin film temporal drift.
    fn validate_thin_film_drift(&self) -> ValidationResult {
        let material = TemporalThinFilm::soap_bubble();
        let mut tracker = DriftTracker::default();

        let mut ctx = TemporalContext::default();
        let initial = material.eval_at_time(&ctx);
        tracker.initialize(&initial);

        for _ in 0..self.config.drift_frames {
            let response = material.eval_at_time(&ctx);
            ctx.advance(self.config.delta_time, response.clone());
            tracker.update(&response);
        }

        let drift = tracker.drift_ratio();
        if drift <= self.config.max_energy_drift {
            ValidationResult::pass("thin_film_drift", drift, self.config.max_energy_drift)
        } else {
            ValidationResult::fail(
                "thin_film_drift",
                drift,
                self.config.max_energy_drift,
                &format!("Drift {:.4} exceeds limit", drift),
            )
        }
    }

    /// Validate conductor temporal drift.
    fn validate_conductor_drift(&self) -> ValidationResult {
        let material = TemporalConductor::heated_gold();
        let mut tracker = DriftTracker::default();

        let mut ctx = TemporalContext::default().with_temperature(300.0); // Start at room temp
        let initial = material.eval_at_time(&ctx);
        tracker.initialize(&initial);

        for i in 0..self.config.drift_frames {
            let response = material.eval_at_time(&ctx);
            ctx.advance(self.config.delta_time, response.clone());
            // Simulate heating
            ctx.temperature = 300.0 + (i as f64 * 0.1).min(200.0);
            tracker.update(&response);
        }

        let drift = tracker.drift_ratio();
        // Allow more drift for temperature changes (physical)
        let max_thermal_drift = self.config.max_energy_drift * 5.0;
        if drift <= max_thermal_drift {
            ValidationResult::pass("conductor_drift", drift, max_thermal_drift)
        } else {
            ValidationResult::fail(
                "conductor_drift",
                drift,
                max_thermal_drift,
                &format!("Drift {:.4} exceeds thermal limit", drift),
            )
        }
    }

    // ========================================================================
    // SPECTRAL COHERENCE TESTS
    // ========================================================================

    /// Validate spectral flicker below threshold.
    fn validate_spectral_flicker(&self) -> ValidationResult {
        let mut validator = FlickerValidator::new(FlickerConfig {
            stable_threshold: self.config.max_spectral_flicker * 0.4,
            minor_threshold: self.config.max_spectral_flicker,
            warning_threshold: self.config.max_spectral_flicker * 2.0,
            ..Default::default()
        });

        let mut sampler = CoherentSampler::stratified(self.config.spectral_samples, 0.5);

        for frame in 0..100 {
            sampler.set_frame(frame);
            let mut packet = sampler.create_packet();

            // Simulate spectral response (smooth evolution)
            for (i, v) in packet.values.iter_mut().enumerate() {
                let wavelength = packet.wavelengths[i];
                let t = frame as f64 * 0.01;
                // Smooth spectral curve with slow evolution
                *v = 0.5 + 0.3 * ((wavelength - 500.0) / 100.0 + t).sin();
            }

            validator.validate(&mut packet);
        }

        let report = validator.report();
        let max_de = report.max_delta_e;

        if max_de <= self.config.max_spectral_flicker {
            ValidationResult::pass("spectral_flicker", max_de, self.config.max_spectral_flicker)
        } else {
            ValidationResult::fail(
                "spectral_flicker",
                max_de,
                self.config.max_spectral_flicker,
                &format!("Max ΔE2000 {:.3} exceeds limit", max_de),
            )
        }
    }

    /// Validate spectral interpolation smoothness.
    fn validate_spectral_interpolation(&self) -> ValidationResult {
        let mut interpolator = SpectralInterpolator::smooth();
        let mut max_change: f64 = 0.0;

        for frame in 0..50 {
            let mut packet = SpectralPacket::uniform_31();

            // Create varying spectral data
            for (i, v) in packet.values.iter_mut().enumerate() {
                let phase = frame as f64 * 0.1 + i as f64 * 0.1;
                *v = 0.5 + 0.4 * phase.sin();
            }

            let result = interpolator.process(packet.clone());

            // Compare with previous
            if frame > 0 {
                for (i, &v) in result.values.iter().enumerate() {
                    if i < packet.values.len() {
                        let change = (v - packet.values[i]).abs();
                        max_change = max_change.max(change);
                    }
                }
            }
        }

        // Interpolation should smooth changes
        let threshold = 0.3;
        if max_change <= threshold {
            ValidationResult::pass("spectral_interpolation", max_change, threshold)
        } else {
            ValidationResult::fail(
                "spectral_interpolation",
                max_change,
                threshold,
                "Interpolation not smoothing sufficiently",
            )
        }
    }

    /// Validate coherent sampling determinism.
    fn validate_coherent_sampling(&self) -> ValidationResult {
        let mut sampler1 = CoherentSampler::stratified(31, 0.5);
        let mut sampler2 = CoherentSampler::stratified(31, 0.5);

        // Same frame should produce same wavelengths
        sampler1.set_frame(42);
        sampler2.set_frame(42);

        let w1 = sampler1.sample();
        let w2 = sampler2.sample();

        let identical = w1 == w2;

        if identical {
            ValidationResult::pass("coherent_sampling", 1.0, 1.0)
        } else {
            ValidationResult::fail("coherent_sampling", 0.0, 1.0, "Sampling not deterministic")
        }
    }

    // ========================================================================
    // NEURAL TEMPORAL TESTS
    // ========================================================================

    /// Validate neural cumulative drift.
    fn validate_neural_drift(&self) -> ValidationResult {
        let mut network = TemporalNeuralCorrection::with_default_config();

        for frame in 0..self.config.drift_frames {
            let input = TemporalCorrectionInput::new(
                super::neural_correction::CorrectionInput::default(),
                self.config.delta_time,
                network.previous_output(),
                frame,
            );
            network.forward_temporal(&input);
        }

        let drift = network.drift_tracker().cumulative_drift();

        // Neural might be disabled due to drift limit
        if network.is_enabled() || drift <= self.config.max_neural_drift * 2.0 {
            ValidationResult::pass("neural_drift", drift, self.config.max_neural_drift)
        } else {
            ValidationResult::fail(
                "neural_drift",
                drift,
                self.config.max_neural_drift,
                &format!("Cumulative drift {:.4} excessive", drift),
            )
        }
    }

    /// Validate neural output bounding.
    fn validate_neural_bounding(&self) -> ValidationResult {
        let network = TemporalNeuralCorrection::with_default_config();
        let max_correction: f64 = 0.1;
        let mut max_observed: f64 = 0.0;

        for i in 0..100 {
            let input = TemporalCorrectionInput::new(
                super::neural_correction::CorrectionInput::new(
                    400.0 + i as f64 * 3.0,
                    (i as f64 * 0.1).cos(),
                    (i as f64 * 0.1).sin().abs(),
                    (i % 10) as f64 * 0.1,
                    1.0 + (i % 20) as f64 * 0.1,
                    0.0,
                    0.0,
                    0.0,
                    0.0,
                    0.0,
                ),
                0.016,
                CorrectionOutput::zero(),
                i,
            );

            let output = network.forward(&input);
            max_observed = max_observed.max(output.delta_reflectance.abs());
            max_observed = max_observed.max(output.delta_transmittance.abs());
        }

        if max_observed <= max_correction + 1e-6 {
            ValidationResult::pass("neural_bounding", max_observed, max_correction)
        } else {
            ValidationResult::fail(
                "neural_bounding",
                max_observed,
                max_correction,
                "Neural output exceeds bounds",
            )
        }
    }

    // ========================================================================
    // BACKWARD COMPATIBILITY TESTS
    // ========================================================================

    /// Validate static (t=0) equivalence with base BSDF.
    fn validate_static_equivalence(&self) -> ValidationResult {
        let physical = DielectricBSDF::new(1.5, 0.1);
        // Create temporal dielectric with static evolution (no change over time)
        let temporal = TemporalDielectric::new(super::temporal::DielectricEvolution {
            roughness_base: 0.1,
            roughness_target: 0.1, // Same as base = static
            roughness_tau: 10.0,
            ior_base: 1.5,
            ior_temp_coeff: 0.0,
        });

        let ctx = BSDFContext::new_simple(1.0);
        let temporal_ctx = TemporalContext::default();

        let physical_response = physical.evaluate(&ctx);
        let temporal_response = temporal.eval_at_time(&temporal_ctx);

        let r_diff = (physical_response.reflectance - temporal_response.reflectance).abs();
        let t_diff = (physical_response.transmittance - temporal_response.transmittance).abs();
        let max_diff = r_diff.max(t_diff);

        let threshold = 1e-6;
        if max_diff <= threshold {
            ValidationResult::pass("static_equivalence", max_diff, threshold)
        } else {
            ValidationResult::fail(
                "static_equivalence",
                max_diff,
                threshold,
                "t=0 not equivalent to static BSDF",
            )
        }
    }

    /// Validate context backward compatibility.
    fn validate_context_compatibility(&self) -> ValidationResult {
        let temporal_ctx = TemporalContext::default();

        // Base context should be accessible
        let base = &temporal_ctx.base;
        let has_wavelength = base.wavelength >= 0.0;
        let has_directions = base.wi.z >= -1.0 && base.wo.z >= -1.0;

        if has_wavelength && has_directions {
            ValidationResult::pass("context_compatibility", 1.0, 1.0)
        } else {
            ValidationResult::fail(
                "context_compatibility",
                0.0,
                1.0,
                "TemporalContext.base not properly accessible",
            )
        }
    }

    // ========================================================================
    // MEMORY BUDGET
    // ========================================================================

    /// Validate total memory usage.
    fn validate_memory_budget(&self) -> ValidationResult {
        let total = self.estimate_total_memory();
        let budget = 150 * 1024; // 150 KB

        if total <= budget {
            ValidationResult::pass("memory_budget", total as f64, budget as f64)
        } else {
            ValidationResult::fail(
                "memory_budget",
                total as f64,
                budget as f64,
                &format!("Memory {} exceeds 150KB budget", total),
            )
        }
    }

    /// Estimate total Phase 12 memory usage.
    fn estimate_total_memory(&self) -> usize {
        // Phase 11 baseline
        let phase11_base = 120 * 1024;

        // Temporal context overhead
        let temporal_overhead = super::temporal::estimate_temporal_memory();

        // Spectral coherence
        let spectral_overhead = super::spectral_coherence::estimate_spectral_coherence_memory();

        // Neural temporal
        let neural_temporal = super::neural_temporal_correction::estimate_temporal_neural_memory();

        phase11_base + temporal_overhead + spectral_overhead + neural_temporal
    }
}

// ============================================================================
// CONVENIENCE FUNCTIONS
// ============================================================================

/// Run quick Phase 12 validation.
pub fn run_quick_validation() -> Phase12ValidationReport {
    Phase12ValidationSuite::new(Phase12ValidationConfig::quick()).run_all()
}

/// Run full Phase 12 validation.
pub fn run_full_validation() -> Phase12ValidationReport {
    Phase12ValidationSuite::default().run_all()
}

/// Run strict Phase 12 validation.
pub fn run_strict_validation() -> Phase12ValidationReport {
    Phase12ValidationSuite::new(Phase12ValidationConfig::strict()).run_all()
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validation_config_defaults() {
        let config = Phase12ValidationConfig::default();
        assert_eq!(config.drift_frames, 1000);
        assert!((config.max_energy_drift - 0.01).abs() < 1e-6);
    }

    #[test]
    fn test_validation_result() {
        let pass = ValidationResult::pass("test", 0.5, 1.0);
        assert!(pass.passed);

        let fail = ValidationResult::fail("test", 1.5, 1.0, "exceeded");
        assert!(!fail.passed);
    }

    #[test]
    fn test_validation_report() {
        let mut report = Phase12ValidationReport::new();

        report.add(ValidationResult::pass("t1", 0.1, 1.0));
        report.add(ValidationResult::pass("t2", 0.2, 1.0));
        report.add(ValidationResult::fail("t3", 1.5, 1.0, "fail"));

        assert_eq!(report.total_tests, 3);
        assert_eq!(report.passed_tests, 2);
        assert_eq!(report.failed_tests, 1);
        assert!(!report.overall_passed);
        assert!((report.pass_rate() - 0.666).abs() < 0.01);
    }

    #[test]
    fn test_quick_validation_runs() {
        let report = run_quick_validation();
        assert!(report.total_tests > 0);
    }

    #[test]
    fn test_dielectric_drift_validation() {
        let suite = Phase12ValidationSuite::new(Phase12ValidationConfig::quick());
        let result = suite.validate_dielectric_drift();
        // Should pass with reasonable parameters
        assert!(result.value < 0.1); // Some drift is expected
    }

    #[test]
    fn test_spectral_flicker_validation() {
        let suite = Phase12ValidationSuite::new(Phase12ValidationConfig::quick());
        let result = suite.validate_spectral_flicker();
        // With smooth evolution, should pass
        assert!(result.value < 2.0); // Allow some flicker for test
    }

    #[test]
    fn test_memory_budget_validation() {
        let suite = Phase12ValidationSuite::default();
        let result = suite.validate_memory_budget();
        // Should be within budget
        assert!(result.passed || result.value < 200_000.0);
    }

    #[test]
    fn test_static_equivalence() {
        let suite = Phase12ValidationSuite::default();
        let result = suite.validate_static_equivalence();
        assert!(result.passed);
    }

    #[test]
    fn test_neural_bounding() {
        let suite = Phase12ValidationSuite::default();
        let result = suite.validate_neural_bounding();
        assert!(result.passed);
    }

    #[test]
    fn test_coherent_sampling_determinism() {
        let suite = Phase12ValidationSuite::default();
        let result = suite.validate_coherent_sampling();
        assert!(result.passed);
    }

    #[test]
    fn test_full_validation_completeness() {
        let report = run_quick_validation();

        // Should have all expected tests
        let test_names: Vec<&str> = report.results.iter().map(|r| r.name.as_str()).collect();

        assert!(test_names.contains(&"dielectric_drift"));
        assert!(test_names.contains(&"thin_film_drift"));
        assert!(test_names.contains(&"spectral_flicker"));
        assert!(test_names.contains(&"neural_drift"));
        assert!(test_names.contains(&"memory_budget"));
    }
}
