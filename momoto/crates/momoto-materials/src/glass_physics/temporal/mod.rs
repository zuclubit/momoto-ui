//! # Temporal Material Model
//!
//! Time-aware BSDF evaluation for physically-based temporal evolution.
//!
//! ## Overview
//!
//! This module introduces time as a first-class physical parameter in material
//! evaluation, enabling:
//!
//! - Time-varying roughness (drying paint, weathering)
//! - Thin-film thickness oscillation (soap bubbles)
//! - Temperature-dependent spectral shifts (heated metals)
//! - Microfacet relaxation dynamics
//!
//! ## Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────────┐
//! │                    TemporalContext                              │
//! │  Extends BSDFContext with: time, delta_time, frame_index       │
//! └─────────────────────────────────────────────────────────────────┘
//!                                  │
//!                                  ▼
//! ┌─────────────────────────────────────────────────────────────────┐
//! │                    TemporalBSDF Trait                           │
//! │  eval_at_time(ctx) -> BSDFResponse with temporal awareness     │
//! └─────────────────────────────────────────────────────────────────┘
//!                                  │
//!         ┌────────────────────────┼────────────────────────┐
//!         ▼                        ▼                        ▼
//! ┌───────────────┐      ┌───────────────┐      ┌───────────────┐
//! │TemporalDielec │      │TemporalThinFm │      │TemporalConduc │
//! │(roughness evo)│      │(thickness osc)│      │(temp spectral)│
//! └───────────────┘      └───────────────┘      └───────────────┘
//! ```
//!
//! ## Usage
//!
//! ```rust,ignore
//! use momoto_materials::glass_physics::temporal::prelude::*;
//!
//! // Create temporal context
//! let ctx = TemporalContext::new()
//!     .at_time(1.5)  // 1.5 seconds
//!     .with_delta(0.016);  // 60 FPS
//!
//! // Evaluate temporal material
//! let drying_paint = TemporalDielectric::drying_paint();
//! let response = drying_paint.eval_at_time(&ctx);
//! ```
//!
//! ## Backward Compatibility
//!
//! All temporal materials evaluate to their Phase 11 static equivalent at t=0.
//! Existing code continues to work unchanged.

pub mod bsdf;
pub mod context;
pub mod interpolation;
pub mod materials;

// Re-exports
pub use context::{
    DriftConfig, DriftStatus, DriftTracker, TemporalContext, TemporalContextBuilder,
};

pub use bsdf::{EvolutionRate, TemporalBSDF, TemporalBSDFInfo, TemporalEvolution};

pub use materials::{
    ConductorEvolution, DielectricEvolution, TemporalConductor, TemporalDielectric,
    TemporalThinFilm, ThinFilmEvolution,
};

pub use interpolation::{
    ease_in_out, inverse_lerp, lerp, remap, smootherstep, smoothstep, ExponentialMovingAverage,
    Interpolation, InterpolationMode, RateLimitConfig, RateLimiter,
};

/// Prelude for convenient imports.
pub mod prelude {
    pub use super::bsdf::{TemporalBSDF, TemporalEvolution};
    pub use super::context::{DriftTracker, TemporalContext, TemporalContextBuilder};
    pub use super::interpolation::{ease_in_out, smootherstep, smoothstep};
    pub use super::materials::{TemporalConductor, TemporalDielectric, TemporalThinFilm};
}

// ============================================================================
// MODULE MEMORY ESTIMATION
// ============================================================================

/// Estimate memory usage for temporal module.
pub fn estimate_temporal_memory() -> usize {
    // TemporalContext: ~128 bytes (BSDFContext + temporal fields)
    let context_size = 128;

    // DriftTracker: ~64 bytes
    let drift_tracker_size = 64;

    // Material wrappers: ~256 bytes each (3 materials)
    let material_wrappers = 256 * 3;

    // Interpolation state: ~32 bytes
    let interpolation_state = 32;

    context_size + drift_tracker_size + material_wrappers + interpolation_state
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_memory_estimate() {
        let mem = estimate_temporal_memory();
        assert!(mem > 0);
        assert!(mem < 2 * 1024); // Should be under 2KB
    }

    #[test]
    fn test_module_exports() {
        // Verify all types are accessible
        let _ctx = TemporalContext::default();
        let _drift = DriftTracker::default();
    }
}
