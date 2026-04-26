//! # Uncertainty Estimation Module
//!
//! Confidence estimation for Digital Material Twin parameters.
//!
//! ## Overview
//!
//! Every parameter in a Digital Material Twin must have uncertainty bounds.
//! This module provides:
//!
//! - **Covariance Matrix**: Parameter correlation and variance
//! - **Fisher Information**: Cramer-Rao lower bounds
//! - **Bootstrap Resampling**: Non-parametric confidence intervals
//! - **Confidence Reports**: Human-readable uncertainty summaries
//!
//! ## Key Concepts
//!
//! ### Fisher Information
//!
//! The Fisher Information matrix measures how much information the data
//! contains about each parameter. Its inverse gives the Cramer-Rao lower
//! bound on parameter variance.
//!
//! ### Bootstrap Confidence Intervals
//!
//! Bootstrap resampling provides non-parametric confidence intervals
//! that don't assume normal distributions.
//!
//! ## Usage
//!
//! ```rust,ignore
//! use momoto_materials::glass_physics::uncertainty::{
//!     ParameterCovarianceMatrix, FisherInformationEstimator,
//!     BootstrapResampler, TwinConfidenceReport,
//! };
//!
//! // Estimate covariance from optimization history
//! let covariance = ParameterCovarianceMatrix::from_samples(&param_history);
//!
//! // Compute confidence intervals
//! let report = TwinConfidenceReport::from_covariance(&covariance, &param_names);
//! println!("{}", report);
//! ```

mod bootstrap;
mod covariance;
mod fisher;
mod report;

pub use covariance::{
    estimate_covariance, shrinkage_covariance, CovarianceEstimator, ParameterCovarianceMatrix,
};

pub use fisher::{
    cramer_rao_bounds, expected_fisher_diagonal, FisherInformationEstimator,
    FisherInformationMatrix,
};

pub use bootstrap::{
    bootstrap_bca, bootstrap_percentile, BootstrapConfig, BootstrapResampler, BootstrapResult,
    ConfidenceInterval,
};

pub use report::{
    format_uncertainty, ConfidenceLevel, ConfidenceWarning, ParameterUncertainty,
    TwinConfidenceReport,
};

// ============================================================================
// MEMORY ESTIMATION
// ============================================================================

/// Estimate memory usage for uncertainty module.
///
/// Components:
/// - CovarianceMatrix (6x6): ~300 bytes
/// - FisherInformation: ~300 bytes
/// - BootstrapSamples (100): ~5KB
/// - ConfidenceReport: ~1KB
///
/// Total typical usage: ~12KB
pub fn estimate_uncertainty_memory() -> usize {
    let covariance = 6 * 6 * 8; // 6x6 f64 matrix
    let fisher = 6 * 6 * 8;
    let bootstrap = 100 * 6 * 8; // 100 samples, 6 params
    let report = 1024;

    covariance + fisher + bootstrap + report + 1024 // overhead
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_memory_estimate() {
        let mem = estimate_uncertainty_memory();
        assert!(mem > 0);
        assert!(mem < 20_000); // Should be under 20KB
    }

    #[test]
    fn test_module_exports() {
        // Verify exports are accessible
        let _level = ConfidenceLevel::P95;
        let _config = BootstrapConfig::default();
    }
}
