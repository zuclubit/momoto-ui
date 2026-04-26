//! # Spectral Gradients Module
//!
//! Per-wavelength gradients and perceptual color space differentiation.
//!
//! ## Overview
//!
//! Materials exhibit wavelength-dependent behavior. This module provides:
//! - Gradients for spectral reflectance functions
//! - ΔE2000 analytical gradient for perceptual loss
//! - Color space transformations with derivatives
//!
//! ## Spectral Gradient Flow
//!
//! ```text
//! Material Params → Spectral Reflectance → XYZ → Lab → ΔE2000
//!      θ                R(λ)              XYZ   L*a*b*   loss
//!      │                  │                │      │       │
//!      └──────────────────┴────────────────┴──────┴───────┘
//!                    Chain rule through all transformations
//! ```

pub mod delta_e;
pub mod wavelength;

// Re-exports
pub use wavelength::{
    compute_spectral_gradient, CauchyDispersion, SellmeierDispersion, SpectralGradient,
    SpectralJacobian, WavelengthGradient, VISIBLE_WAVELENGTHS,
};

pub use delta_e::{
    delta_e_2000, delta_e_2000_gradient, DeltaE2000Gradient, Lab, LabGradient, PerceptualLoss,
};

/// Prelude for convenient imports.
pub mod prelude {
    pub use super::delta_e::{delta_e_2000_gradient, DeltaE2000Gradient};
    pub use super::wavelength::{compute_spectral_gradient, SpectralGradient};
}

// ============================================================================
// MODULE MEMORY ESTIMATION
// ============================================================================

/// Estimate memory usage for spectral gradients module.
pub fn estimate_spectral_gradients_memory() -> usize {
    // Spectral samples (31 wavelengths × 8 bytes)
    let spectral_samples = 31 * 8;

    // Spectral Jacobian (3 outputs × 31 wavelengths × 8 bytes)
    let jacobian = 3 * 31 * 8;

    // Color matching functions (3 × 31 × 8)
    let cmf = 3 * 31 * 8;

    // Lab gradient cache
    let lab_cache = 3 * 8;

    spectral_samples + jacobian + cmf + lab_cache
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_memory_estimate() {
        let mem = estimate_spectral_gradients_memory();
        assert!(mem > 0);
        assert!(mem < 5_000); // Should be under 5KB
    }

    #[test]
    fn test_module_exports() {
        // Verify types are accessible
        let _lab = Lab::new(50.0, 0.0, 0.0);
    }
}
