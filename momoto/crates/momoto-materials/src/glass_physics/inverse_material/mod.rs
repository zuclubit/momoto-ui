//! # Inverse Material Solver
//!
//! Recover material parameters from observed reflectance data using
//! gradient-based optimization.
//!
//! ## Overview
//!
//! This module provides tools to solve the inverse problem:
//! "Given observed reflectance, what material produced it?"
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────────┐
//! │                    InverseMaterialSolver                        │
//! │  solve(reference, initial) -> MaterialParams                    │
//! └─────────────────────────────────────────────────────────────────┘
//!                                  │
//!         ┌────────────────────────┼────────────────────────────────┐
//!         │                        │                                │
//!         ▼                        ▼                                ▼
//! ┌───────────────┐      ┌─────────────────────┐      ┌─────────────────────┐
//! │AdamOptimizer  │      │LBFGSOptimizer       │      │BoundsEnforcer       │
//! │momentum + vel │      │quasi-Newton         │      │project to valid     │
//! └───────────────┘      └─────────────────────┘      └─────────────────────┘
//! ```
//!
//! ## Usage
//!
//! ```rust,ignore
//! use momoto_materials::glass_physics::inverse_material::prelude::*;
//!
//! // Reference data (observed reflectance)
//! let reference = ReferenceData::from_reflectance(&[0.04, 0.06, 0.10]);
//!
//! // Initial guess
//! let initial = MaterialParams::glass(1.4);
//!
//! // Solve
//! let mut solver = InverseMaterialSolver::with_adam();
//! let result = solver.solve(&reference, &initial)?;
//!
//! println!("Recovered IOR: {}", result.params.ior);
//! ```

pub mod bounds;
pub mod optimizer;
pub mod solver;
pub mod temporal_fitting;

// Re-exports
pub use optimizer::{
    AdamConfig, AdamOptimizer, DifferentiableOptimizer, LBFGSConfig, LBFGSOptimizer, OptimizerState,
};

pub use bounds::{BoundsConfig, BoundsEnforcer, ProjectionMethod};

pub use solver::{
    recover_ior_from_normal_reflectance, recover_roughness_from_glossiness, ConvergenceReason,
    InverseMaterialSolver, InverseResult, InverseSolverConfig, LossFunction, ReferenceData,
    ReferenceObservation,
};

pub use temporal_fitting::{
    TemporalFitResult, TemporalFitter, TemporalFitterConfig, TemporalSequence,
};

/// Prelude for convenient imports.
pub mod prelude {
    pub use super::optimizer::{AdamOptimizer, DifferentiableOptimizer, LBFGSOptimizer};
    pub use super::solver::{InverseMaterialSolver, InverseResult, ReferenceData};
    pub use super::temporal_fitting::{TemporalFitter, TemporalSequence};
}

// ============================================================================
// MODULE MEMORY ESTIMATION
// ============================================================================

/// Estimate memory usage for inverse material module.
pub fn estimate_inverse_memory() -> usize {
    // Adam state: 2 × params × f64 (m and v vectors)
    let adam_state = 2 * 8 * 8; // 8 params, 8 bytes each

    // L-BFGS history: m × (2 × params + 1) × f64
    let lbfgs_history = 10 * (2 * 8 + 1) * 8; // m=10

    // Solver state: params + history
    let solver_state = 8 * 8 + 200 * 8; // 200 history entries

    // Bounds enforcer
    let bounds = 8 * 4; // 4 pairs of f64 bounds

    adam_state + lbfgs_history + solver_state + bounds
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_memory_estimate() {
        let mem = estimate_inverse_memory();
        assert!(mem > 0);
        assert!(mem < 5 * 1024); // Should be under 5KB
    }

    #[test]
    fn test_module_exports() {
        // Verify types are accessible
        let _config = AdamConfig::default();
        let _bounds_config = BoundsConfig::default();
    }
}
