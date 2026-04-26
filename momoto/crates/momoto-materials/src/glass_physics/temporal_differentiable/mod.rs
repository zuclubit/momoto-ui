//! # Temporal Differentiable Module
//!
//! Backpropagation through time for temporal material evolution.
//!
//! ## Overview
//!
//! This module provides tools for computing gradients through temporal
//! sequences, enabling optimization of evolution parameters from
//! time-series observations.
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────────┐
//! │                    Temporal Gradient Flow                        │
//! │                                                                  │
//! │   t=0      t=1      t=2      t=3      ...      t=T              │
//! │    │        │        │        │                 │               │
//! │   θ₀ ──→   θ₁ ──→   θ₂ ──→   θ₃ ──→  ...  ──→ θ_T             │
//! │    │        │        │        │                 │               │
//! │   R₀       R₁       R₂       R₃               R_T              │
//! │    │        │        │        │                 │               │
//! │   L₀       L₁       L₂       L₃               L_T              │
//! │    │        │        │        │                 │               │
//! │    └────────┴────────┴────────┴─────────────────┘               │
//! │                        │                                         │
//! │                   ∂L_total/∂θ₀ (via BPTT)                       │
//! └─────────────────────────────────────────────────────────────────┘
//! ```
//!
//! ## Key Components
//!
//! - **EvolutionGradients**: Gradients for temporal evolution models
//! - **BPTT**: Backpropagation through time for long sequences
//! - **GradientStabilization**: Techniques to prevent gradient explosion/vanishing

pub mod bptt;
pub mod evolution_gradients;

// Re-exports
pub use evolution_gradients::{
    compute_evolution_gradient, EvolutionGradient, EvolutionGradients, EvolutionType,
    ExponentialEvolutionGradient, LinearEvolutionGradient, OscillatingEvolutionGradient,
};

pub use bptt::{
    BPTTConfig, BPTTState, GradientStabilizer, StabilizerConfig, TemporalGradientAccumulator, BPTT,
};

/// Prelude for convenient imports.
pub mod prelude {
    pub use super::bptt::{BPTTConfig, GradientStabilizer, BPTT};
    pub use super::evolution_gradients::{compute_evolution_gradient, EvolutionGradient};
}

// ============================================================================
// MODULE MEMORY ESTIMATION
// ============================================================================

/// Estimate memory usage for temporal differentiable module.
pub fn estimate_temporal_differentiable_memory(sequence_length: usize) -> usize {
    // Per-frame cache
    let per_frame_cache = 8 * 10; // 10 f64 values per frame

    // Gradient accumulator
    let accumulator = 8 * 8; // 8 parameters

    // BPTT state
    let bptt_state = sequence_length * per_frame_cache;

    // Stabilizer history
    let stabilizer = 8 * 100; // 100 gradient norms

    per_frame_cache * sequence_length + accumulator + bptt_state + stabilizer
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_memory_estimate() {
        let mem_100 = estimate_temporal_differentiable_memory(100);
        let mem_1000 = estimate_temporal_differentiable_memory(1000);

        assert!(mem_1000 > mem_100);
        assert!(mem_100 < 50_000); // Should be under 50KB for 100 frames
    }

    #[test]
    fn test_module_exports() {
        // Verify types are accessible
        let _config = BPTTConfig::default();
    }
}
