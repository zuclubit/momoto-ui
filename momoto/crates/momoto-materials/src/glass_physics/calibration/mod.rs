//! # Calibration Pipeline Module
//!
//! Multi-source calibration for Digital Material Twins.
//!
//! ## Overview
//!
//! This module provides a calibration pipeline that can fit material parameters
//! from various data sources:
//!
//! - **BRDF Measurements**: Hemispherical reflectance data
//! - **Spectral Reflectance**: Wavelength-dependent measurements
//! - **Time Series**: Temporal evolution data
//!
//! ## Loss Aggregation
//!
//! The calibration uses a weighted combination of loss terms:
//!
//! - Physical (MSE): 1.0 - Direct parameter fitting
//! - Perceptual (ΔE2000): 0.5 - Human-perceived color difference
//! - Temporal: 0.1 - Consistency across time
//! - Energy Conservation: 0.01 - Physics constraint
//!
//! ## Partial Data Handling
//!
//! Real-world data is often incomplete. The module provides:
//!
//! - Missing value imputation strategies
//! - Confidence weighting for sparse data
//! - Robust fitting with outlier handling

mod aggregation;
mod partial_data;
mod sources;

pub use sources::{
    BRDFObservation, BRDFSource, CalibrationSource, CombinedSource, SourceMetadata,
    SpectralObservation, SpectralSource, TemporalObservation, TimeSeriesSource,
};

pub use aggregation::{
    compute_energy_loss, compute_perceptual_loss, compute_physical_loss, compute_temporal_loss,
    AggregatedLoss, LossAggregator, LossComponents, LossWeights,
};

pub use partial_data::{
    detect_outliers, impute_angular, impute_spectral, DataQuality, ImputationStrategy,
    MissingDataReport, PartialDataHandler,
};

// ============================================================================
// MEMORY ESTIMATION
// ============================================================================

/// Estimate memory usage for calibration module.
///
/// Components:
/// - CalibrationSource variants: ~100 bytes each
/// - LossAggregator: ~200 bytes
/// - PartialDataHandler: ~500 bytes
/// - Typical observation set (1000 obs): ~80KB
///
/// Total typical usage: ~6KB base + observations
pub fn estimate_calibration_memory() -> usize {
    // Base module overhead
    let base = 1024;

    // Typical source metadata
    let sources = 4 * 100;

    // Aggregator and handler
    let processing = 700;

    base + sources + processing
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_memory_estimate() {
        let mem = estimate_calibration_memory();
        assert!(mem > 0);
        assert!(mem < 10_000); // Should be under 10KB base
    }

    #[test]
    fn test_module_exports() {
        // Verify all exports are accessible
        let _weights = LossWeights::default();
        let _strategy = ImputationStrategy::Linear;
        let _quality = DataQuality::High;
    }
}
