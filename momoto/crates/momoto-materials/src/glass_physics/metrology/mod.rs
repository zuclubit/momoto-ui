//! # Metrology Layer
//!
//! Formal metrological infrastructure for certifiable measurements.
//!
//! This module provides the foundational types and utilities for traceable,
//! uncertainty-quantified measurements suitable for industrial certification.
//!
//! ## Core Types
//!
//! - [`Measurement<T>`] - Value with uncertainty, unit, and quality metadata
//! - [`Uncertainty`] - Type A (statistical) and Type B (systematic) uncertainties
//! - [`TraceabilityChain`] - Complete audit trail for measurements
//! - [`ToleranceBudget`] - Error allocation and validation
//! - [`UncertaintyPropagator`] - Forward/inverse uncertainty propagation
//!
//! ## Example
//!
//! ```ignore
//! use glass_physics::metrology::*;
//!
//! // Create a calibrated measurement
//! let wavelength = Measurement::calibrated(550.0, 0.5, Unit::Nanometers);
//!
//! // Create a tolerance budget for industrial certification
//! let mut budget = ToleranceBudget::for_certification_level(
//!     CertificationTolerance::Industrial
//! );
//!
//! // Update with actual measured errors
//! budget.update_actual("Model", 0.3);
//! budget.update_actual("Instrumental", 0.2);
//!
//! // Validate the budget
//! let validation = budget.validate();
//! assert!(validation.passed);
//! ```
//!
//! ## Module Structure
//!
//! - `units` - SI units and conversions
//! - `measurement` - Core `Measurement<T>` type with uncertainty
//! - `traceability` - Audit trail and calibration references
//! - `tolerance` - Error budgeting and validation
//! - `propagation` - Uncertainty propagation methods

pub mod measurement;
pub mod propagation;
pub mod tolerance;
pub mod traceability;
pub mod units;

// Re-exports for convenient access
pub use measurement::{
    Measurement, MeasurementArray, MeasurementId, MeasurementQuality, MeasurementSource,
    Uncertainty,
};
pub use propagation::{
    identity_correlation, uniform_correlation, validate_correlation_matrix, PropagationMethod,
    SensitivityAnalysis, UncertaintyPropagator,
};
pub use tolerance::{
    CertificationTolerance, ComponentValidation, ToleranceBudget, ToleranceCategory,
    ToleranceComponent, ToleranceValidation,
};
pub use traceability::{
    CalibrationReference, ChainMetadata, TraceabilityChain, TraceabilityEntry,
    TraceabilityOperation,
};
pub use units::{
    celsius_to_kelvin, convert_unit, deg_to_rad, fraction_to_percent, kelvin_to_celsius, nm_to_um,
    percent_to_fraction, rad_to_deg, um_to_nm, units_compatible, Unit, UnitValue,
};

// ============================================================================
// MEMORY ESTIMATION
// ============================================================================

/// Estimate memory footprint of metrology module types.
pub fn estimate_memory_footprint() -> MetrologyMemoryEstimate {
    MetrologyMemoryEstimate {
        measurement_f64_bytes: std::mem::size_of::<Measurement<f64>>(),
        measurement_array_base_bytes: std::mem::size_of::<MeasurementArray>(),
        traceability_chain_base_bytes: std::mem::size_of::<TraceabilityChain>(),
        traceability_entry_bytes: std::mem::size_of::<TraceabilityEntry>(),
        tolerance_budget_base_bytes: std::mem::size_of::<ToleranceBudget>(),
        tolerance_component_bytes: std::mem::size_of::<ToleranceComponent>(),
        propagator_bytes: std::mem::size_of::<UncertaintyPropagator>(),
    }
}

/// Memory footprint estimates for metrology types.
#[derive(Debug, Clone)]
pub struct MetrologyMemoryEstimate {
    /// Size of Measurement<f64>.
    pub measurement_f64_bytes: usize,
    /// Base size of MeasurementArray (without data).
    pub measurement_array_base_bytes: usize,
    /// Base size of TraceabilityChain (without entries).
    pub traceability_chain_base_bytes: usize,
    /// Size of single TraceabilityEntry.
    pub traceability_entry_bytes: usize,
    /// Base size of ToleranceBudget (without components).
    pub tolerance_budget_base_bytes: usize,
    /// Size of single ToleranceComponent.
    pub tolerance_component_bytes: usize,
    /// Size of UncertaintyPropagator.
    pub propagator_bytes: usize,
}

impl MetrologyMemoryEstimate {
    /// Total base memory (all types, minimal data).
    pub fn total_base(&self) -> usize {
        self.measurement_f64_bytes
            + self.measurement_array_base_bytes
            + self.traceability_chain_base_bytes
            + self.tolerance_budget_base_bytes
            + self.propagator_bytes
    }

    /// Estimate memory for typical usage scenario.
    pub fn typical_usage(&self) -> usize {
        // Assume:
        // - 10 active measurements
        // - 1 measurement array with 100 points
        // - 1 traceability chain with 20 entries
        // - 1 tolerance budget with 10 components
        // - 1 propagator

        self.measurement_f64_bytes * 10
            + self.measurement_array_base_bytes
            + 100 * 16 // 100 f64 pairs (value + uncertainty)
            + self.traceability_chain_base_bytes
            + self.traceability_entry_bytes * 20
            + self.tolerance_budget_base_bytes
            + self.tolerance_component_bytes * 10
            + self.propagator_bytes
    }

    /// Generate memory report.
    pub fn report(&self) -> String {
        format!(
            "Metrology Memory Footprint:\n\
             ├── Measurement<f64>:     {:4} bytes\n\
             ├── MeasurementArray:     {:4} bytes (base)\n\
             ├── TraceabilityChain:    {:4} bytes (base)\n\
             │   └── Entry:            {:4} bytes each\n\
             ├── ToleranceBudget:      {:4} bytes (base)\n\
             │   └── Component:        {:4} bytes each\n\
             ├── UncertaintyPropagator:{:4} bytes\n\
             ├── Total Base:           {:4} bytes\n\
             └── Typical Usage:        {:4} bytes (~{:.1} KB)",
            self.measurement_f64_bytes,
            self.measurement_array_base_bytes,
            self.traceability_chain_base_bytes,
            self.traceability_entry_bytes,
            self.tolerance_budget_base_bytes,
            self.tolerance_component_bytes,
            self.propagator_bytes,
            self.total_base(),
            self.typical_usage(),
            self.typical_usage() as f64 / 1024.0
        )
    }
}

// ============================================================================
// MODULE VALIDATION
// ============================================================================

/// Validate metrology module configuration.
pub fn validate_module() -> MetrologyValidation {
    let mut issues = Vec::new();

    // Check measurement size is reasonable
    let measurement_size = std::mem::size_of::<Measurement<f64>>();
    if measurement_size > 256 {
        issues.push(format!(
            "Measurement<f64> size {} bytes exceeds 256 byte limit",
            measurement_size
        ));
    }

    // Check unit conversions are reciprocal
    let test_deg = 45.0;
    let round_trip = rad_to_deg(deg_to_rad(test_deg));
    if (round_trip - test_deg).abs() > 1e-10 {
        issues.push("Degree/radian conversion not reciprocal".to_string());
    }

    let test_celsius = 25.0;
    let round_trip = kelvin_to_celsius(celsius_to_kelvin(test_celsius));
    if (round_trip - test_celsius).abs() > 1e-10 {
        issues.push("Celsius/Kelvin conversion not reciprocal".to_string());
    }

    MetrologyValidation {
        valid: issues.is_empty(),
        issues,
        memory_estimate: estimate_memory_footprint(),
    }
}

/// Result of metrology module validation.
#[derive(Debug)]
pub struct MetrologyValidation {
    /// Whether validation passed.
    pub valid: bool,
    /// List of issues found.
    pub issues: Vec<String>,
    /// Memory footprint estimate.
    pub memory_estimate: MetrologyMemoryEstimate,
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_module_exports() {
        // Verify all public types are accessible
        let _ = Unit::Nanometers;
        let _ = UnitValue::new(550.0, Unit::Nanometers);
        let _ = Measurement::<f64>::calibrated(1.0, 0.1, Unit::Dimensionless);
        let _ = TraceabilityChain::new();
        let _ = ToleranceBudget::new("Test", 1.0);
        let _ = UncertaintyPropagator::linear();
    }

    #[test]
    fn test_memory_estimate() {
        let estimate = estimate_memory_footprint();

        // Sanity checks
        assert!(estimate.measurement_f64_bytes > 0);
        assert!(estimate.measurement_f64_bytes < 512);
        assert!(estimate.total_base() > 0);
        assert!(estimate.typical_usage() > estimate.total_base());

        let report = estimate.report();
        assert!(report.contains("Measurement"));
        assert!(report.contains("bytes"));
    }

    #[test]
    fn test_module_validation() {
        let validation = validate_module();
        assert!(
            validation.valid,
            "Validation failed: {:?}",
            validation.issues
        );
    }

    #[test]
    fn test_memory_budget() {
        let estimate = estimate_memory_footprint();

        // Phase 15 metrology should use < 15KB typical
        assert!(
            estimate.typical_usage() < 15_000,
            "Typical usage {} exceeds 15KB budget",
            estimate.typical_usage()
        );
    }

    #[test]
    fn test_unit_conversions_exported() {
        assert!((deg_to_rad(180.0) - std::f64::consts::PI).abs() < 1e-10);
        assert!((celsius_to_kelvin(0.0) - 273.15).abs() < 1e-10);
        assert!((um_to_nm(1.0) - 1000.0).abs() < 1e-10);
    }

    #[test]
    fn test_propagator_creation() {
        let linear = UncertaintyPropagator::linear();
        assert_eq!(linear.method, PropagationMethod::Linear);

        let mc = UncertaintyPropagator::monte_carlo(5000);
        assert!(matches!(
            mc.method,
            PropagationMethod::MonteCarlo { n_samples: 5000 }
        ));
    }

    #[test]
    fn test_tolerance_budget_from_level() {
        let budget = ToleranceBudget::for_certification_level(CertificationTolerance::Industrial);
        assert_eq!(budget.target, 1.0);
        assert!(!budget.components.is_empty());
    }

    #[test]
    fn test_traceability_chain_operations() {
        let mut chain = TraceabilityChain::new();
        chain.record_measurement("Test Instrument", "Direct", MeasurementId::generate());
        assert_eq!(chain.entries.len(), 1);
    }

    #[test]
    fn test_correlation_utilities() {
        let id = identity_correlation(3);
        assert!(validate_correlation_matrix(&id).is_ok());

        let uniform = uniform_correlation(3, 0.5);
        assert!(validate_correlation_matrix(&uniform).is_ok());
    }
}
