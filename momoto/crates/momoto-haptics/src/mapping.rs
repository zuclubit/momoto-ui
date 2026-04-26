//! Frequency → force mapping for haptic actuator models.
//!
//! Different actuator types (LRA, ERM, piezoelectric) have different
//! frequency-force response curves. This module provides a discrete-point
//! interpolation model and pre-defined presets for common actuator types.
//!
//! # Perceptual intensity → vibration spec
//!
//! The mapping from a normalised perceptual intensity `i ∈ [0, 1]` to a
//! physical `VibrationSpec` follows the actuator's response curve:
//!
//! ```text
//! freq_hz = lerp(f_min, f_resonance, i^0.5)
//! force_n = lerp(0, f_max_n, i)
//! ```
//!
//! The square-root on frequency models human Weber's law sensitivity:
//! equal perceptual steps require larger physical steps at low intensities.

/// A single point on an actuator's frequency-force response curve.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FrequencyForcePoint {
    /// Vibration frequency in Hz.
    pub freq_hz: f32,
    /// Peak force output in Newtons.
    pub force_n: f32,
}

/// A complete vibration specification ready for actuator output.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct VibrationSpec {
    /// Target vibration frequency in Hz.
    pub freq_hz: f32,
    /// Target force amplitude in Newtons.
    pub force_n: f32,
    /// Duration in milliseconds.
    pub duration_ms: f32,
    /// Perceptual intensity that produced this spec (0.0–1.0).
    pub intensity: f32,
}

impl VibrationSpec {
    /// Estimated energy expenditure in joules (`force × distance × time`).
    ///
    /// Uses a simplified model: `E ≈ F · v_peak · t / 2`
    /// where `v_peak ≈ freq_hz * 2π * amplitude`.
    /// Amplitude is estimated from force and a nominal spring constant.
    #[must_use]
    pub fn energy_j(&self) -> f32 {
        // Simplified: E ≈ 0.5 * F² * t / k, with k=1000 N/m (nominal LRA)
        let spring_constant = 1000.0_f32;
        0.5 * self.force_n * self.force_n * (self.duration_ms / 1000.0) / spring_constant
    }
}

/// Known haptic actuator models with pre-calibrated response curves.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActuatorModel {
    /// Linear Resonant Actuator (LRA) — typical smartphone haptic.
    /// Resonant frequency: ~150–200 Hz; narrow bandwidth.
    Lra,
    /// Eccentric Rotating Mass (ERM) — older haptic motors.
    /// Variable frequency via motor speed; 50–300 Hz.
    Erm,
    /// Piezoelectric actuator — high-frequency, low-displacement.
    /// 200–400 Hz; high bandwidth, precise control.
    Piezo,
    /// Custom model (use `FrequencyForceMapper::custom()`).
    Custom,
}

/// Frequency-force mapper for a specific actuator model.
///
/// Maps a normalised intensity `[0.0, 1.0]` to a `VibrationSpec`.
#[derive(Debug, Clone)]
pub struct FrequencyForceMapper {
    /// Actuator model this mapper is calibrated for.
    pub model: ActuatorModel,
    /// Minimum output frequency (at intensity=0).
    f_min_hz: f32,
    /// Resonant / maximum frequency (at intensity=1).
    f_max_hz: f32,
    /// Maximum force output in Newtons.
    force_max_n: f32,
}

impl FrequencyForceMapper {
    /// Create a mapper for the given actuator model using preset parameters.
    #[must_use]
    pub fn new(model: ActuatorModel) -> Self {
        match model {
            ActuatorModel::Lra => Self {
                model,
                f_min_hz: 150.0,
                f_max_hz: 200.0,
                force_max_n: 0.5, // typical LRA peak force
            },
            ActuatorModel::Erm => Self {
                model,
                f_min_hz: 50.0,
                f_max_hz: 300.0,
                force_max_n: 1.2, // ERM higher force, less precise
            },
            ActuatorModel::Piezo => Self {
                model,
                f_min_hz: 200.0,
                f_max_hz: 400.0,
                force_max_n: 0.2, // piezo low force, high precision
            },
            ActuatorModel::Custom => Self {
                model,
                f_min_hz: 100.0,
                f_max_hz: 300.0,
                force_max_n: 1.0,
            },
        }
    }

    /// Create a custom mapper with explicit parameters.
    #[must_use]
    pub fn custom(f_min_hz: f32, f_max_hz: f32, force_max_n: f32) -> Self {
        Self {
            model: ActuatorModel::Custom,
            f_min_hz: f_min_hz.max(0.0),
            f_max_hz: f_max_hz.max(f_min_hz),
            force_max_n: force_max_n.max(0.0),
        }
    }

    /// Map a normalised intensity to a `VibrationSpec`.
    ///
    /// `intensity` is clamped to `[0.0, 1.0]`.
    /// `duration_ms` is the requested vibration duration in milliseconds.
    #[must_use]
    pub fn map(&self, intensity: f32, duration_ms: f32) -> VibrationSpec {
        let i = intensity.clamp(0.0, 1.0);
        // Square-root mapping for perceptual linearity (Weber's law)
        let freq_hz = self.f_min_hz + (self.f_max_hz - self.f_min_hz) * i.sqrt();
        let force_n = self.force_max_n * i;
        VibrationSpec {
            freq_hz,
            force_n,
            duration_ms: duration_ms.max(0.0),
            intensity: i,
        }
    }

    /// Returns the minimum output frequency (Hz).
    #[must_use]
    pub fn f_min_hz(&self) -> f32 {
        self.f_min_hz
    }

    /// Returns the maximum output frequency (Hz).
    #[must_use]
    pub fn f_max_hz(&self) -> f32 {
        self.f_max_hz
    }

    /// Returns the peak force output (Newtons).
    #[must_use]
    pub fn force_max_n(&self) -> f32 {
        self.force_max_n
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lra_mapper_zero_intensity_is_min_freq() {
        let m = FrequencyForceMapper::new(ActuatorModel::Lra);
        let spec = m.map(0.0, 100.0);
        assert!((spec.freq_hz - m.f_min_hz()).abs() < 1.0);
        assert!(spec.force_n.abs() < 1e-5);
    }

    #[test]
    fn lra_mapper_full_intensity_is_max_freq() {
        let m = FrequencyForceMapper::new(ActuatorModel::Lra);
        let spec = m.map(1.0, 100.0);
        assert!((spec.freq_hz - m.f_max_hz()).abs() < 1.0);
        assert!((spec.force_n - m.force_max_n()).abs() < 1e-5);
    }

    #[test]
    fn intensity_is_clamped() {
        let m = FrequencyForceMapper::new(ActuatorModel::Erm);
        let below = m.map(-0.5, 100.0);
        let above = m.map(1.5, 100.0);
        assert!((below.intensity).abs() < 1e-5);
        assert!((above.intensity - 1.0).abs() < 1e-5);
    }

    #[test]
    fn energy_estimate_positive_for_nonzero() {
        let m = FrequencyForceMapper::new(ActuatorModel::Lra);
        let spec = m.map(0.5, 100.0);
        assert!(spec.energy_j() > 0.0);
    }

    #[test]
    fn energy_zero_for_zero_intensity() {
        let m = FrequencyForceMapper::new(ActuatorModel::Lra);
        let spec = m.map(0.0, 100.0);
        assert!(spec.energy_j().abs() < 1e-9);
    }

    #[test]
    fn custom_mapper_respects_parameters() {
        let m = FrequencyForceMapper::custom(100.0, 500.0, 2.0);
        let spec = m.map(1.0, 50.0);
        assert!((spec.freq_hz - 500.0).abs() < 1.0);
        assert!((spec.force_n - 2.0).abs() < 1e-5);
    }

    #[test]
    fn all_actuator_models_construct() {
        for model in [ActuatorModel::Lra, ActuatorModel::Erm, ActuatorModel::Piezo] {
            let m = FrequencyForceMapper::new(model);
            assert!(m.f_min_hz() < m.f_max_hz());
            assert!(m.force_max_n() > 0.0);
        }
    }
}
