//! # Material Twin Module
//!
//! Digital Material Twins with calibrated physical parameters and uncertainty quantification.
//!
//! ## Overview
//!
//! A Digital Material Twin is not just a fitted BSDF - it is a validated physical
//! surrogate with:
//!
//! - **Unique Identity**: UUID-based tracking across sessions
//! - **Spectral Fingerprint**: Content-based hashing for reproducibility
//! - **Uncertainty Bounds**: Every parameter includes confidence intervals
//! - **Temporal Evolution**: Materials maintain identity through changes
//!
//! ## Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────────┐
//! │                      MaterialTwin<M>                             │
//! ├─────────────────────────────────────────────────────────────────┤
//! │  TwinId (UUID)       │ Unique identifier                        │
//! │  MaterialFingerprint │ Content hash for versioning              │
//! │  physical: M         │ DifferentiableBSDF implementation        │
//! │  SpectralIdentity    │ Spectral signature for matching          │
//! │  TemporalEvolution   │ Time-dependent behavior                  │
//! │  CalibrationMetadata │ Fit quality and data source              │
//! └─────────────────────────────────────────────────────────────────┘
//! ```
//!
//! ## Usage
//!
//! ```rust,ignore
//! use momoto_materials::glass_physics::material_twin::{
//!     MaterialTwin, TwinBuilder, TwinId,
//! };
//! use momoto_materials::glass_physics::differentiable::DifferentiableDielectric;
//!
//! // Create a twin from a calibrated dielectric
//! let glass = DifferentiableDielectric::glass();
//! let twin = TwinBuilder::new(glass)
//!     .with_name("BK7 Optical Glass")
//!     .with_tag("calibrated")
//!     .build();
//!
//! // Access twin identity
//! println!("Twin ID: {}", twin.id);
//! println!("Fingerprint: {}", twin.fingerprint.short_hash());
//! ```

mod identity;
mod twin;
mod variants;

pub use twin::{CalibrationMetadata, CalibrationQuality, MaterialTwin, TwinBuilder, TwinId};

pub use variants::{
    LayeredTwinData, MeasuredTwinData, StaticTwinData, TemporalTwinData, TwinVariant,
};

pub use identity::{
    compute_spectral_distance, SpectralDistance, SpectralIdentity, SpectralSignature,
    IDENTITY_WAVELENGTHS,
};

// ============================================================================
// MEMORY ESTIMATION
// ============================================================================

/// Estimate memory usage for material_twin module.
///
/// Components:
/// - TwinId (UUID): 16 bytes
/// - MaterialFingerprint: ~68 bytes
/// - SpectralIdentity: ~200 bytes (31 wavelengths)
/// - CalibrationMetadata: ~100 bytes
/// - TwinVariant data: ~50 bytes
/// - Tags/metadata: ~200 bytes
///
/// Total per twin: ~640 bytes
/// Module overhead: ~2KB
pub fn estimate_material_twin_memory() -> usize {
    // Base module overhead
    let base = 2 * 1024;

    // Per-twin estimate (typical usage: 10 twins)
    let per_twin = 640;
    let typical_twins = 10;

    base + per_twin * typical_twins
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_memory_estimate() {
        let mem = estimate_material_twin_memory();
        assert!(mem > 0);
        assert!(mem < 20_000); // Should be under 20KB for typical usage
    }

    #[test]
    fn test_module_exports() {
        // Verify all exports are accessible
        let _id = TwinId::generate();
        let _variant = TwinVariant::Static;
        let _quality = CalibrationQuality::Uncalibrated;
    }
}
