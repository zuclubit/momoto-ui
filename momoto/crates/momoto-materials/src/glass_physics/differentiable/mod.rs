//! # Differentiable BSDF Module
//!
//! Provides analytical gradient computation for all material parameters,
//! enabling gradient-based optimization and inverse rendering workflows.
//!
//! ## Overview
//!
//! This module extends the Phase 9 BSDF system with differentiation capabilities:
//!
//! - **Analytical gradients** for all physical parameters
//! - **Chain rule composition** for layered materials
//! - **Jacobian computation** for multi-output optimization
//! - **Energy-conserving gradients** that preserve R + T + A = 1
//!
//! ## Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────────┐
//! │                    DifferentiableBSDF Trait                     │
//! │  eval_with_gradients(ctx) -> (BSDFResponse, ParameterGradients) │
//! └─────────────────────────────────────────────────────────────────┘
//!                                  │
//!         ┌────────────────────────┼────────────────────────┐
//!         ▼                        ▼                        ▼
//! ┌───────────────┐      ┌───────────────┐      ┌───────────────┐
//! │DiffDielectric │      │DiffConductor  │      │DiffThinFilm   │
//! │∂R/∂n, ∂R/∂α   │      │∂R/∂n, ∂R/∂k   │      │∂R/∂d, ∂R/∂n_f │
//! └───────────────┘      └───────────────┘      └───────────────┘
//! ```
//!
//! ## Usage
//!
//! ```rust,ignore
//! use momoto_materials::glass_physics::differentiable::prelude::*;
//!
//! // Create differentiable material
//! let glass = DifferentiableDielectric::new(1.5, 0.1);
//! let ctx = BSDFContext::new_simple(0.8);
//!
//! // Evaluate with gradients
//! let result = glass.eval_with_gradients(&ctx);
//! println!("Reflectance: {}", result.response.reflectance);
//! println!("∂R/∂n: {}", result.gradients.d_ior);
//! ```
//!
//! ## Gradient Derivations
//!
//! All gradients are derived analytically from physical equations:
//!
//! - **Fresnel Schlick**: ∂F/∂n = 4(n-1)/(n+1)³ × (1 - (1-cosθ)⁵)
//! - **GGX Distribution**: ∂D/∂α via denominator differentiation
//! - **Beer-Lambert**: ∂T/∂α = -d × T, ∂T/∂d = -α × T
//! - **Airy (thin-film)**: ∂R/∂thickness via phase derivative

pub mod conductor;
pub mod dielectric;
pub mod gradients;
pub mod jacobian;
pub mod layered;
pub mod thin_film;
pub mod traits;

// Re-exports
pub use traits::{
    DifferentiableBSDF, DifferentiableResponse, GradientConfig, GradientVerification,
    ParameterBounds, ParameterGradients,
};

pub use gradients::{
    beer_lambert_gradient, fresnel_conductor_gradient, fresnel_schlick_gradient,
    ggx_distribution_gradient, smith_g_gradient, thin_film_gradient,
};

pub use conductor::DifferentiableConductor;
pub use dielectric::DifferentiableDielectric;
pub use jacobian::{Jacobian, JacobianBuilder};
pub use layered::{DifferentiableLayered, LayerConfig};
pub use thin_film::DifferentiableThinFilm;

/// Prelude for convenient imports.
pub mod prelude {
    pub use super::conductor::DifferentiableConductor;
    pub use super::dielectric::DifferentiableDielectric;
    pub use super::layered::DifferentiableLayered;
    pub use super::thin_film::DifferentiableThinFilm;
    pub use super::traits::{DifferentiableBSDF, DifferentiableResponse, ParameterGradients};
}

// ============================================================================
// MODULE MEMORY ESTIMATION
// ============================================================================

/// Estimate memory usage for differentiable module.
pub fn estimate_differentiable_memory() -> usize {
    // DifferentiableResponse: ~128 bytes (BSDFResponse + ParameterGradients)
    let response_size = 128;

    // ParameterGradients: ~80 bytes (10 f64 fields)
    let gradients_size = 80;

    // Jacobian (3x8): ~192 bytes
    let jacobian_size = 192;

    // Per-material gradient cache: ~256 bytes
    let cache_size = 256;

    response_size + gradients_size + jacobian_size + cache_size
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_memory_estimate() {
        let mem = estimate_differentiable_memory();
        assert!(mem > 0);
        assert!(mem < 2 * 1024); // Should be under 2KB per evaluation
    }

    #[test]
    fn test_module_exports() {
        // Verify all types are accessible
        let _bounds = ParameterBounds::default();
        let _config = GradientConfig::default();
    }
}
