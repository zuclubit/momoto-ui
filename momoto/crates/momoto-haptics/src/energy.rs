//! Haptic actuator energy budget.
//!
//! Enforces the physical constraint that total delivered haptic energy must
//! not exceed the device's rated energy capacity. Exceeding this limit can
//! cause actuator damage or discomfort.
//!
//! # Model
//!
//! ```text
//! E_budget_joules = rated capacity (e.g. 50 mJ for a smartphone LRA)
//! E_delivered += P(t) · Δt   for each haptic event
//! Constraint:  E_delivered ≤ E_budget_joules
//! ```

/// Minimum energy value treated as a non-zero haptic event (joules).
///
/// Energy requests below this threshold are treated as no-ops to prevent
/// log-scale underflow in the energy estimator.
pub const HAPTIC_ENERGY_EPSILON: f32 = 1.0e-12;

/// Error returned when a haptic event would exceed the energy budget.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EnergyBudgetError {
    /// Energy that would be required for the requested event (joules).
    pub required_j: f32,
    /// Energy remaining in the budget (joules).
    pub available_j: f32,
}

impl core::fmt::Display for EnergyBudgetError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(
            f,
            "haptic energy budget exceeded: required {:.4} J, available {:.4} J",
            self.required_j, self.available_j
        )
    }
}

/// Haptic actuator energy budget tracker.
///
/// Tracks cumulative energy delivered to a haptic actuator and enforces
/// the rated energy capacity. Energy recovers over time via a linear
/// recharge model (charge rate in J/s).
///
/// # Example
///
/// ```rust
/// use momoto_haptics::EnergyBudget;
///
/// let mut budget = EnergyBudget::new(0.050); // 50 mJ capacity (typical LRA)
/// assert!(budget.try_consume(0.010).is_ok());  // 10 mJ event → ok
/// assert!(budget.try_consume(0.045).is_err()); // 45 mJ would exceed remaining
/// ```
#[derive(Debug, Clone)]
pub struct EnergyBudget {
    /// Rated energy capacity in joules.
    pub capacity_j: f32,
    /// Currently consumed energy in joules.
    consumed_j: f32,
    /// Recharge rate in joules per second (passive cooling / capacitor recharge).
    pub recharge_rate_j_per_s: f32,
}

impl EnergyBudget {
    /// Create a new budget with the given capacity (joules) and no recharge.
    ///
    /// For a typical smartphone LRA: `capacity_j = 0.050` (50 mJ).
    #[must_use]
    pub fn new(capacity_j: f32) -> Self {
        Self {
            capacity_j: capacity_j.max(0.0),
            consumed_j: 0.0,
            recharge_rate_j_per_s: 0.0,
        }
    }

    /// Create a budget with a passive recharge rate.
    #[must_use]
    pub fn with_recharge(capacity_j: f32, recharge_rate_j_per_s: f32) -> Self {
        Self {
            capacity_j: capacity_j.max(0.0),
            consumed_j: 0.0,
            recharge_rate_j_per_s: recharge_rate_j_per_s.max(0.0),
        }
    }

    /// Available energy remaining (joules).
    #[must_use]
    pub fn available_j(&self) -> f32 {
        (self.capacity_j - self.consumed_j).max(0.0)
    }

    /// Fraction of capacity consumed (0.0 = empty, 1.0 = full).
    #[must_use]
    pub fn load_fraction(&self) -> f32 {
        if self.capacity_j < f32::EPSILON {
            return 1.0;
        }
        (self.consumed_j / self.capacity_j).clamp(0.0, 1.0)
    }

    /// Attempt to consume `energy_j` joules from the budget.
    ///
    /// Returns `Ok(())` if the budget has sufficient capacity.
    /// Returns `Err(EnergyBudgetError)` if the event would exceed the budget.
    pub fn try_consume(&mut self, energy_j: f32) -> Result<(), EnergyBudgetError> {
        // Flush NaN/Inf energy requests — treat as zero (no-op).
        let energy_j = if energy_j.is_finite() && energy_j >= 0.0 {
            energy_j
        } else {
            0.0
        };
        let available = self.available_j();
        if energy_j > available {
            return Err(EnergyBudgetError {
                required_j: energy_j,
                available_j: available,
            });
        }
        self.consumed_j += energy_j;
        Ok(())
    }

    /// Consume energy without checking the budget (for testing / overrides).
    ///
    /// Use with caution — bypasses the safety constraint.
    pub fn consume_unchecked(&mut self, energy_j: f32) {
        self.consumed_j = (self.consumed_j + energy_j).min(self.capacity_j);
    }

    /// Advance time by `delta_secs` seconds, recovering energy via recharge.
    pub fn tick(&mut self, delta_secs: f32) {
        // Flush NaN/Inf time delta — no recharge for invalid inputs.
        let delta = if delta_secs.is_finite() {
            delta_secs.max(0.0)
        } else {
            0.0
        };
        let recovered = self.recharge_rate_j_per_s * delta;
        self.consumed_j = (self.consumed_j - recovered).max(0.0);
    }

    /// Reset consumed energy to zero (simulate a fully recharged state).
    pub fn reset(&mut self) {
        self.consumed_j = 0.0;
    }

    /// Returns `true` if the budget can accommodate `energy_j` joules.
    #[must_use]
    pub fn can_afford(&self, energy_j: f32) -> bool {
        energy_j <= self.available_j()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_budget_is_fully_available() {
        let b = EnergyBudget::new(0.050);
        assert!((b.available_j() - 0.050).abs() < 1e-6);
    }

    #[test]
    fn consume_within_budget_succeeds() {
        let mut b = EnergyBudget::new(0.050);
        assert!(b.try_consume(0.020).is_ok());
        assert!((b.available_j() - 0.030).abs() < 1e-5);
    }

    #[test]
    fn consume_exceeds_budget_fails() {
        let mut b = EnergyBudget::new(0.050);
        let err = b.try_consume(0.060).unwrap_err();
        assert!((err.required_j - 0.060).abs() < 1e-6);
    }

    #[test]
    fn can_afford_checks_without_consuming() {
        let b = EnergyBudget::new(0.050);
        assert!(b.can_afford(0.050));
        assert!(!b.can_afford(0.051));
    }

    #[test]
    fn tick_recharges_over_time() {
        let mut b = EnergyBudget::with_recharge(0.050, 0.010); // 10 mJ/s
        b.consume_unchecked(0.020); // consume 20 mJ
        b.tick(1.0); // 1 second → recover 10 mJ
        assert!((b.available_j() - 0.040).abs() < 1e-5);
    }

    #[test]
    fn tick_does_not_exceed_capacity() {
        let mut b = EnergyBudget::with_recharge(0.050, 1.0); // very fast recharge
        b.consume_unchecked(0.010);
        b.tick(100.0); // long time
        assert!((b.available_j() - 0.050).abs() < 1e-5);
    }

    #[test]
    fn reset_fully_recharges() {
        let mut b = EnergyBudget::new(0.050);
        b.consume_unchecked(0.050);
        assert!((b.available_j()).abs() < 1e-6);
        b.reset();
        assert!((b.available_j() - 0.050).abs() < 1e-6);
    }

    #[test]
    fn load_fraction_range() {
        let mut b = EnergyBudget::new(0.100);
        assert!((b.load_fraction()).abs() < 1e-6);
        b.consume_unchecked(0.050);
        assert!((b.load_fraction() - 0.5).abs() < 1e-5);
        b.consume_unchecked(0.050);
        assert!((b.load_fraction() - 1.0).abs() < 1e-5);
    }

    #[test]
    fn nan_energy_request_is_no_op() {
        let mut b = EnergyBudget::new(0.050);
        let before = b.available_j();
        let _ = b.try_consume(f32::NAN);
        assert!(
            (b.available_j() - before).abs() < 1e-9,
            "NaN request must not change budget"
        );
    }

    #[test]
    fn nan_tick_is_no_op() {
        let mut b = EnergyBudget::with_recharge(0.050, 0.010);
        b.consume_unchecked(0.020);
        let before = b.available_j();
        b.tick(f32::NAN);
        assert!(
            (b.available_j() - before).abs() < 1e-9,
            "NaN tick must not change budget"
        );
    }

    #[test]
    fn inf_energy_request_treated_as_zero() {
        let mut b = EnergyBudget::new(0.050);
        let before = b.available_j();
        let _ = b.try_consume(f32::INFINITY);
        // INFINITY → flushed to 0.0 → no-op
        assert!(
            (b.available_j() - before).abs() < 1e-9,
            "Inf request must not change budget"
        );
    }
}
