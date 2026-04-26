//! # Identifiability Analysis Module
//!
//! Degeneracy detection and parameter freezing recommendations.
//!
//! ## Overview
//!
//! Some material parameters may not be uniquely identifiable from
//! available measurements. This module provides:
//!
//! - **Jacobian Rank Analysis**: Detect non-identifiable parameters
//! - **Parameter Correlation**: Find redundant parameter combinations
//! - **Freezing Recommendations**: Suggest which parameters to fix
//!
//! ## Key Concepts
//!
//! ### Identifiability
//!
//! A parameter is identifiable if changes in its value produce
//! distinct changes in the observable output. Non-identifiable
//! parameters have gradients that are linearly dependent on others.
//!
//! ### Structural vs Practical Identifiability
//!
//! - **Structural**: Parameter could theoretically be identified
//! - **Practical**: Parameter can be identified with available data
//!
//! ## Usage
//!
//! ```rust,ignore
//! use momoto_materials::glass_physics::identifiability::{
//!     JacobianRankAnalyzer, IdentifiabilityResult,
//! };
//!
//! // Analyze Jacobian from gradient samples
//! let analyzer = JacobianRankAnalyzer::from_gradients(&gradient_samples);
//! let result = analyzer.analyze();
//!
//! for idx in &result.non_identifiable {
//!     println!("Parameter {} is not identifiable", idx);
//! }
//! ```

mod correlation;
mod freezing;
mod jacobian;

pub use jacobian::{
    compute_condition_number, compute_effective_rank, IdentifiabilityResult, JacobianRankAnalyzer,
    RankDeficiency,
};

pub use correlation::{
    compute_vif, find_correlation_clusters, CorrelationAnalysis, CorrelationCluster,
    ParameterCorrelationMatrix,
};

pub use freezing::{
    FreezingReason, FreezingRecommendation, FreezingReport, FreezingStrategy,
    ParameterFreezingRecommender,
};

// ============================================================================
// MEMORY ESTIMATION
// ============================================================================

/// Estimate memory usage for identifiability module.
///
/// Components:
/// - JacobianRankAnalyzer (6x100): ~5KB
/// - CorrelationMatrix (6x6): ~300 bytes
/// - FreezingRecommender: ~1KB
///
/// Total typical usage: ~8KB
pub fn estimate_identifiability_memory() -> usize {
    let jacobian = 6 * 100 * 8; // 6 params, 100 observations
    let correlation = 6 * 6 * 8;
    let freezing = 1024;

    jacobian + correlation + freezing + 512 // overhead
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_memory_estimate() {
        let mem = estimate_identifiability_memory();
        assert!(mem > 0);
        assert!(mem < 15_000); // Should be under 15KB
    }

    #[test]
    fn test_module_exports() {
        let _reason = FreezingReason::HighCorrelation;
        let _strategy = FreezingStrategy::ConservativePhysics;
    }
}
