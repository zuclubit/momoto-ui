//! # Temporal BSDF Trait
//!
//! Extends the BSDF trait with time-awareness for temporal material evolution.

use super::super::unified_bsdf::{BSDFResponse, BSDF};
use super::context::TemporalContext;

// ============================================================================
// TEMPORAL BSDF TRAIT
// ============================================================================

/// Evolution rate for temporal parameters.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum EvolutionRate {
    /// No evolution (static).
    Static,
    /// Linear evolution over time.
    Linear { rate: f64 },
    /// Exponential decay/growth.
    Exponential { rate: f64, asymptote: f64 },
    /// Oscillating (sinusoidal).
    Oscillating { frequency: f64, amplitude: f64 },
    /// Step function at threshold time.
    Step {
        threshold: f64,
        before: f64,
        after: f64,
    },
}

impl Default for EvolutionRate {
    fn default() -> Self {
        EvolutionRate::Static
    }
}

impl EvolutionRate {
    /// Evaluate the evolution at a given time.
    pub fn evaluate(&self, time: f64, base_value: f64) -> f64 {
        match self {
            EvolutionRate::Static => base_value,

            EvolutionRate::Linear { rate } => base_value + rate * time,

            EvolutionRate::Exponential { rate, asymptote } => {
                // value = asymptote + (base - asymptote) * exp(-rate * time)
                asymptote + (base_value - asymptote) * (-rate * time).exp()
            }

            EvolutionRate::Oscillating {
                frequency,
                amplitude,
            } => {
                use std::f64::consts::TAU;
                base_value + amplitude * (frequency * TAU * time).sin()
            }

            EvolutionRate::Step {
                threshold,
                before,
                after,
            } => {
                if time < *threshold {
                    *before
                } else {
                    *after
                }
            }
        }
    }

    /// Check if this evolution is static.
    pub fn is_static(&self) -> bool {
        matches!(self, EvolutionRate::Static)
    }
}

/// Describes how a material evolves over time.
#[derive(Debug, Clone)]
pub struct TemporalEvolution {
    /// Roughness evolution.
    pub roughness: EvolutionRate,

    /// IOR evolution.
    pub ior: EvolutionRate,

    /// Extinction coefficient evolution.
    pub extinction: EvolutionRate,

    /// Thickness evolution (for thin films).
    pub thickness: EvolutionRate,

    /// Maximum allowed change per frame.
    pub max_delta_per_frame: f64,

    /// Whether evolution is reversible.
    pub reversible: bool,
}

impl Default for TemporalEvolution {
    fn default() -> Self {
        Self {
            roughness: EvolutionRate::Static,
            ior: EvolutionRate::Static,
            extinction: EvolutionRate::Static,
            thickness: EvolutionRate::Static,
            max_delta_per_frame: 0.1,
            reversible: true,
        }
    }
}

impl TemporalEvolution {
    /// Create evolution with roughness changes.
    pub fn with_roughness(mut self, rate: EvolutionRate) -> Self {
        self.roughness = rate;
        self
    }

    /// Create evolution with IOR changes.
    pub fn with_ior(mut self, rate: EvolutionRate) -> Self {
        self.ior = rate;
        self
    }

    /// Create evolution with extinction changes.
    pub fn with_extinction(mut self, rate: EvolutionRate) -> Self {
        self.extinction = rate;
        self
    }

    /// Create evolution with thickness changes.
    pub fn with_thickness(mut self, rate: EvolutionRate) -> Self {
        self.thickness = rate;
        self
    }

    /// Check if all evolutions are static.
    pub fn is_static(&self) -> bool {
        self.roughness.is_static()
            && self.ior.is_static()
            && self.extinction.is_static()
            && self.thickness.is_static()
    }
}

/// Information about a temporal BSDF.
#[derive(Debug, Clone)]
pub struct TemporalBSDFInfo {
    /// Name of the material.
    pub name: String,

    /// Whether the material supports temporal evolution.
    pub supports_temporal: bool,

    /// Evolution description.
    pub evolution: TemporalEvolution,

    /// Minimum time for valid evaluation.
    pub time_min: f64,

    /// Maximum time for valid evaluation.
    pub time_max: f64,
}

impl Default for TemporalBSDFInfo {
    fn default() -> Self {
        Self {
            name: "Unknown".to_string(),
            supports_temporal: false,
            evolution: TemporalEvolution::default(),
            time_min: 0.0,
            time_max: f64::INFINITY,
        }
    }
}

/// Trait for BSDFs that support temporal evolution.
///
/// Extends the base BSDF trait with time-awareness while maintaining
/// full backward compatibility.
pub trait TemporalBSDF: BSDF {
    /// Evaluate the BSDF at a specific time.
    ///
    /// This is the primary temporal evaluation method.
    fn eval_at_time(&self, ctx: &TemporalContext) -> BSDFResponse;

    /// Check if this BSDF supports temporal evolution.
    fn supports_temporal(&self) -> bool {
        false
    }

    /// Get information about temporal behavior.
    fn temporal_info(&self) -> TemporalBSDFInfo {
        TemporalBSDFInfo {
            name: self.name().to_string(),
            supports_temporal: self.supports_temporal(),
            ..Default::default()
        }
    }

    /// Get the evolution rate for roughness.
    fn roughness_evolution(&self) -> EvolutionRate {
        EvolutionRate::Static
    }

    /// Get the evolution rate for IOR.
    fn ior_evolution(&self) -> EvolutionRate {
        EvolutionRate::Static
    }

    /// Get the evolution rate for extinction.
    fn extinction_evolution(&self) -> EvolutionRate {
        EvolutionRate::Static
    }

    /// Evaluate parameter at time.
    fn evaluate_roughness_at(&self, _time: f64) -> f64 {
        // Default: return base roughness
        0.0
    }

    /// Evaluate IOR at time.
    fn evaluate_ior_at(&self, _time: f64) -> f64 {
        // Default: return base IOR
        1.5
    }

    /// Rate-limited update for smooth transitions.
    fn rate_limited_value(&self, current: f64, target: f64, max_delta: f64) -> f64 {
        let delta = target - current;
        if delta.abs() <= max_delta {
            target
        } else {
            current + delta.signum() * max_delta
        }
    }
}

// Note: No blanket implementation to avoid conflicts with explicit temporal materials.
// Regular BSDFs should be wrapped in temporal wrappers for time-aware evaluation.

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::super::super::unified_bsdf::DielectricBSDF;
    use super::*;

    #[test]
    fn test_evolution_rate_static() {
        let rate = EvolutionRate::Static;
        assert!(rate.is_static());
        assert!((rate.evaluate(10.0, 0.5) - 0.5).abs() < 1e-6);
    }

    #[test]
    fn test_evolution_rate_linear() {
        let rate = EvolutionRate::Linear { rate: 0.1 };
        assert!(!rate.is_static());
        // At t=2, value should be 0.5 + 0.1*2 = 0.7
        assert!((rate.evaluate(2.0, 0.5) - 0.7).abs() < 1e-6);
    }

    #[test]
    fn test_evolution_rate_exponential() {
        let rate = EvolutionRate::Exponential {
            rate: 1.0,
            asymptote: 1.0,
        };
        // At t=0, should be base value
        assert!((rate.evaluate(0.0, 0.5) - 0.5).abs() < 1e-6);
        // At t=inf, should approach asymptote
        let large_t = rate.evaluate(100.0, 0.5);
        assert!((large_t - 1.0).abs() < 0.01);
    }

    #[test]
    fn test_evolution_rate_oscillating() {
        let rate = EvolutionRate::Oscillating {
            frequency: 1.0,
            amplitude: 0.1,
        };
        // At t=0, sin(0) = 0, so value = base
        assert!((rate.evaluate(0.0, 0.5) - 0.5).abs() < 1e-6);
        // At t=0.25 (quarter period), sin(π/2) = 1
        let quarter = rate.evaluate(0.25, 0.5);
        assert!((quarter - 0.6).abs() < 1e-6);
    }

    #[test]
    fn test_evolution_rate_step() {
        let rate = EvolutionRate::Step {
            threshold: 1.0,
            before: 0.2,
            after: 0.8,
        };
        assert!((rate.evaluate(0.5, 0.5) - 0.2).abs() < 1e-6);
        assert!((rate.evaluate(1.5, 0.5) - 0.8).abs() < 1e-6);
    }

    #[test]
    fn test_temporal_evolution_default() {
        let evo = TemporalEvolution::default();
        assert!(evo.is_static());
    }

    #[test]
    fn test_temporal_evolution_builder() {
        let evo = TemporalEvolution::default().with_roughness(EvolutionRate::Linear { rate: 0.01 });
        assert!(!evo.is_static());
    }

    #[test]
    fn test_dielectric_temporal() {
        // Use temporal dielectric wrapper for temporal interface
        use super::super::materials::TemporalDielectric;

        let bsdf = TemporalDielectric::drying_paint();
        let ctx = TemporalContext::default();

        let response = bsdf.eval_at_time(&ctx);
        assert!(response.reflectance >= 0.0);
        assert!(response.transmittance >= 0.0);
        assert!(bsdf.supports_temporal());
    }

    #[test]
    fn test_temporal_info() {
        // Use temporal dielectric wrapper
        use super::super::materials::TemporalDielectric;

        let bsdf = TemporalDielectric::drying_paint();
        let info = bsdf.temporal_info();

        assert_eq!(info.name, "TemporalDielectric");
        assert!(info.supports_temporal);
    }
}
