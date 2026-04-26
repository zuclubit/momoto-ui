//! # Spectral Coherence
//!
//! Wavelength continuity and temporal spectral stability.
//!
//! ## Overview
//!
//! This module prevents frame-to-frame spectral instability ("spectral flicker")
//! by tracking wavelength coherence across time.
//!
//! ## Key Concepts
//!
//! - **SpectralPacket**: Wavelength set with coherence metadata
//! - **Coherent Sampling**: Deterministic wavelength selection per frame
//! - **Temporal Blending**: Frame-to-frame spectral interpolation
//! - **Flicker Detection**: ΔE2000 frame-to-frame validation
//!
//! ## Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────────┐
//! │                      SpectralPacket                             │
//! │  wavelengths, values, coherence_length, temporal_phase         │
//! └─────────────────────────────────────────────────────────────────┘
//!                                  │
//!         ┌────────────────────────┼────────────────────────────┐
//!         ▼                        ▼                            ▼
//! ┌───────────────┐      ┌───────────────┐      ┌───────────────────┐
//! │CoherentSampler│      │ SpectralInterp│      │  FlickerValidator │
//! │ deterministic │      │ temporal blend│      │   ΔE2000 check   │
//! └───────────────┘      └───────────────┘      └───────────────────┘
//! ```

pub mod interpolation;
pub mod packet;
pub mod sampling;
pub mod validation;

// Re-exports
pub use packet::{CoherenceMetadata, SpectralPacket, SpectralPacketBuilder, WavelengthBand};

pub use sampling::{CoherentSampler, JitteredSampler, SamplingStrategy, StratifiedSampler};

pub use interpolation::{BlendConfig, GradientLimiter, SpectralInterpolator};

pub use validation::{
    FlickerConfig, FlickerReport, FlickerStatus, FlickerValidator, FrameComparison,
};

/// Prelude for convenient imports.
pub mod prelude {
    pub use super::interpolation::{BlendConfig, SpectralInterpolator};
    pub use super::packet::{SpectralPacket, SpectralPacketBuilder};
    pub use super::sampling::{CoherentSampler, SamplingStrategy};
    pub use super::validation::{FlickerStatus, FlickerValidator};
}

// ============================================================================
// MODULE MEMORY ESTIMATION
// ============================================================================

/// Estimate memory usage for spectral coherence module.
pub fn estimate_spectral_coherence_memory() -> usize {
    // SpectralPacket: ~320 bytes (31 wavelengths * 2 * 8 bytes + metadata)
    let packet_size = 320;

    // Sampler state: ~64 bytes
    let sampler_size = 64;

    // Interpolator buffer: ~640 bytes (2 packets for blending)
    let interpolator_size = 640;

    // Flicker validator history: ~3200 bytes (10 frame history)
    let validator_size = 3200;

    packet_size + sampler_size + interpolator_size + validator_size
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_memory_estimate() {
        let mem = estimate_spectral_coherence_memory();
        assert!(mem > 0);
        assert!(mem < 5 * 1024); // Should be under 5KB
    }

    #[test]
    fn test_module_exports() {
        // Verify types are accessible
        let _packet = SpectralPacket::default();
    }
}
