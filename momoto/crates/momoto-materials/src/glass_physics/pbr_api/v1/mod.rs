//! # PBR API v1.0
//!
//! Stable public API for physically-based rendering.
//!
//! ## Version
//!
//! ```text
//! API Version: 1.0.0
//! Stability: Stable
//! ```
//!
//! ## Included Types
//!
//! | Type | Description |
//! |------|-------------|
//! | `Material` | High-level material wrapper |
//! | `Layer` | Material layer types |
//! | `BSDF` | Core BSDF trait |
//! | `EvaluationContext` | BSDF evaluation context |
//! | `BSDFResponse` | Energy-conserved response |
//! | `Vector3` | 3D direction vector |
//! | `DielectricBSDF` | Glass/water materials |
//! | `ConductorBSDF` | Metallic materials |
//! | `ThinFilmBSDF` | Iridescent coatings |
//! | `LayeredBSDF` | Composite materials |
//! | `AnisotropicGGX` | Brushed metal BRDF |
//! | `QualityTier` | Rendering quality levels |

// Submodules
pub mod bsdf;
pub mod context;
pub mod material;
pub mod prelude;

// Version info
/// API version as tuple (major, minor, patch).
pub const API_VERSION: (u32, u32, u32) = (1, 0, 0);

/// API version as string.
pub const API_VERSION_STRING: &str = "1.0.0";

/// Check if the API version is compatible with a required version.
pub fn is_compatible(required: (u32, u32, u32)) -> bool {
    // Major version must match
    if API_VERSION.0 != required.0 {
        return false;
    }
    // Minor version must be >= required
    if API_VERSION.1 < required.1 {
        return false;
    }
    true
}

// Re-exports from submodules
pub use bsdf::{
    BSDFResponse, BSDFSample, ConductorBSDF, DielectricBSDF, EnergyValidation, LambertianBSDF,
    LayeredBSDF, ThinFilmBSDF, BSDF,
};
pub use context::{EvaluationContext, Vector3};
pub use material::{Layer, Material, MaterialBuilder, MaterialPreset};

// Re-exports from other modules (stable types)
pub use super::super::anisotropic_brdf::AnisotropicGGX;
pub use super::super::enhanced_presets::QualityTier;

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version() {
        assert_eq!(API_VERSION, (1, 0, 0));
        assert_eq!(API_VERSION_STRING, "1.0.0");
    }

    #[test]
    fn test_compatibility() {
        // Same version is compatible
        assert!(is_compatible((1, 0, 0)));

        // Lower minor version is compatible
        assert!(is_compatible((1, 0, 0)));

        // Higher minor version is not compatible
        assert!(!is_compatible((1, 1, 0)));

        // Different major version is not compatible
        assert!(!is_compatible((2, 0, 0)));
        assert!(!is_compatible((0, 0, 0)));
    }
}
