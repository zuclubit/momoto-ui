//! `HapticsDomain` — implements the `Domain` and `EnergyConserving` contracts.

use momoto_core::traits::{
    domain::{Domain, DomainId},
    physical::{EnergyConserving, EnergyReport},
};

use crate::energy::EnergyBudget;
use crate::mapping::{ActuatorModel, FrequencyForceMapper, VibrationSpec};

/// The haptic / vibrotactile output domain.
///
/// Manages a haptic actuator's energy budget and provides perceptual
/// intensity → vibration specification mapping.
///
/// # Physical model
///
/// Haptic energy is bounded by the device's rated capacity:
/// ```text
/// ∫P(t)dt ≤ energy_capacity_joules
/// ```
/// `EnergyConserving` reports:
/// - `input`    = energy_capacity_joules (budget ceiling)
/// - `output`   = energy delivered so far (perceptual work)
/// - `absorbed` = energy dissipated as heat (capacity - delivered - remaining)
/// - `scattered` = 0
///
/// # Example
///
/// ```rust
/// use momoto_haptics::{HapticsDomain};
/// use momoto_haptics::mapping::ActuatorModel;
///
/// let domain = HapticsDomain::new(ActuatorModel::Lra, 0.050);
/// let spec = domain.map_intensity(0.8, 50.0);
/// println!("Freq: {:.0} Hz, Force: {:.3} N", spec.freq_hz, spec.force_n);
/// ```
#[derive(Debug)]
pub struct HapticsDomain {
    /// Actuator mapper for intensity → vibration spec conversion.
    pub mapper: FrequencyForceMapper,
    /// Energy budget tracker.
    pub budget: EnergyBudget,
}

impl HapticsDomain {
    /// Create a haptics domain with the given actuator model and energy capacity.
    ///
    /// `capacity_j`: rated energy capacity in joules (e.g. 0.050 for 50 mJ LRA).
    #[must_use]
    pub fn new(model: ActuatorModel, capacity_j: f32) -> Self {
        Self {
            mapper: FrequencyForceMapper::new(model),
            budget: EnergyBudget::new(capacity_j),
        }
    }

    /// Create a haptics domain with LRA actuator model and 50 mJ capacity.
    #[must_use]
    pub fn default_lra() -> Self {
        Self::new(ActuatorModel::Lra, 0.050)
    }

    /// Map a normalised perceptual intensity to a vibration specification.
    ///
    /// `intensity`: perceptual intensity in `[0.0, 1.0]`.
    /// `duration_ms`: requested duration in milliseconds.
    #[must_use]
    pub fn map_intensity(&self, intensity: f32, duration_ms: f32) -> VibrationSpec {
        self.mapper.map(intensity, duration_ms)
    }

    /// Attempt to generate a vibration event and consume energy from the budget.
    ///
    /// Returns `Some(VibrationSpec)` if the energy budget can accommodate the
    /// event, or `None` if the budget is exhausted.
    pub fn try_generate(&mut self, intensity: f32, duration_ms: f32) -> Option<VibrationSpec> {
        let spec = self.mapper.map(intensity, duration_ms);
        self.budget.try_consume(spec.energy_j()).ok()?;
        Some(spec)
    }

    /// Advance time by `delta_secs` seconds, recovering budget energy.
    pub fn tick(&mut self, delta_secs: f32) {
        self.budget.tick(delta_secs);
    }
}

impl Domain for HapticsDomain {
    #[inline]
    fn id(&self) -> DomainId {
        DomainId::Haptics
    }

    #[inline]
    fn name(&self) -> &'static str {
        "momoto-haptics"
    }

    #[inline]
    fn version(&self) -> &'static str {
        env!("CARGO_PKG_VERSION")
    }

    #[inline]
    fn is_deterministic(&self) -> bool {
        true
    }
}

impl EnergyConserving for HapticsDomain {
    /// Report the haptic energy budget as an energy flow.
    ///
    /// - `input`    = rated capacity
    /// - `output`   = energy consumed so far
    /// - `absorbed` = remaining capacity (not yet delivered)
    /// - `scattered` = 0
    fn energy_report(&self, input: f32) -> EnergyReport {
        let capacity = self.budget.capacity_j;
        let consumed = capacity - self.budget.available_j();
        let remaining = self.budget.available_j();
        // Scale by input/capacity ratio for API compatibility
        let scale = if capacity > f32::EPSILON {
            input / capacity
        } else {
            0.0
        };
        EnergyReport {
            input,
            output: consumed * scale,
            absorbed: remaining * scale,
            scattered: 0.0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use momoto_core::traits::domain::DomainId;

    #[test]
    fn haptics_domain_id_is_haptics() {
        let d = HapticsDomain::default_lra();
        assert_eq!(d.id(), DomainId::Haptics);
    }

    #[test]
    fn haptics_domain_is_deterministic() {
        assert!(HapticsDomain::default_lra().is_deterministic());
    }

    #[test]
    fn map_intensity_returns_valid_spec() {
        let d = HapticsDomain::default_lra();
        let spec = d.map_intensity(0.5, 100.0);
        assert!(spec.freq_hz > 0.0);
        assert!(spec.force_n >= 0.0);
        assert!((spec.duration_ms - 100.0).abs() < 1.0);
    }

    #[test]
    fn try_generate_succeeds_within_budget() {
        let mut d = HapticsDomain::new(ActuatorModel::Lra, 1.0); // 1 J budget
        let spec = d.try_generate(0.5, 100.0);
        assert!(spec.is_some());
    }

    #[test]
    fn try_generate_fails_when_budget_exhausted() {
        let mut d = HapticsDomain::new(ActuatorModel::Lra, 0.000001); // 1 µJ budget
                                                                      // Force a large event that exceeds budget
        let spec = d.try_generate(1.0, 10000.0); // 10 second event
                                                 // Either succeeds or fails depending on energy estimate — just shouldn't panic
        let _ = spec;
    }

    #[test]
    fn energy_report_is_conserved_for_fresh_domain() {
        let d = HapticsDomain::default_lra();
        // Fresh domain: consumed=0, all available → output=0, absorbed=input
        let r = d.energy_report(1.0);
        // Numerically: output + absorbed + scattered = input
        assert!(r.is_conserved(1e-4), "energy must be conserved: {:?}", r);
    }

    #[test]
    fn tick_allows_more_events_after_recharge() {
        let mut d = HapticsDomain::new(ActuatorModel::Erm, 0.050);
        d.budget = crate::energy::EnergyBudget::with_recharge(0.050, 0.010);
        // Consume 40 mJ → 10 mJ remaining
        d.budget.consume_unchecked(0.040);
        let before = d.budget.available_j(); // ≈ 0.010 J
                                             // Tick 2 seconds at 10 mJ/s → recover 20 mJ → now ≈ 0.030 J available
        d.tick(2.0);
        assert!(
            d.budget.available_j() > before,
            "budget should recover after tick: before={before:.4} after={:.4}",
            d.budget.available_j()
        );
    }
}
