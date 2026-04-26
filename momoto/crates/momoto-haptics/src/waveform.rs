//! Haptic waveform generation.
//!
//! Generates time-domain haptic waveforms (normalised to [-1, 1]) from
//! perceptual parameters. Waveforms are allocated once and returned as
//! `Box<[f32]>` for WASM compatibility.
//!
//! # Waveform kinds
//!
//! | Kind | Description | Use case |
//! |------|-------------|---------|
//! | `Sine` | Pure sinusoid | Sustained feedback, texture |
//! | `Pulse` | Single Gaussian impulse | Click, tap, notification |
//! | `Ramp` | Linear envelope × sine | Attack-decay feedback |
//! | `Buzz` | Clipped sine (rich harmonics) | Alert, error feedback |

use core::f32::consts::PI;

/// Kind of haptic waveform to generate.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WaveformKind {
    /// Pure sinusoid at a single frequency.
    Sine,
    /// Gaussian impulse (single click or tap).
    Pulse,
    /// Linearly ramping sinusoid (attack → full → decay).
    Ramp,
    /// Clipped sinusoid producing rich harmonics (buzz / alert).
    Buzz,
}

/// A generated haptic waveform and its metadata.
#[derive(Debug, Clone)]
pub struct HapticWaveform {
    /// Waveform kind.
    pub kind: WaveformKind,
    /// Vibration frequency in Hz.
    pub freq_hz: f32,
    /// Sample rate in Hz (the waveform is at this rate).
    pub sample_rate: u32,
    /// Normalised samples in [-1, 1].
    pub samples: Box<[f32]>,
}

impl HapticWaveform {
    /// Generate a haptic waveform.
    ///
    /// # Parameters
    ///
    /// - `kind`: waveform shape
    /// - `freq_hz`: vibration frequency
    /// - `duration_ms`: duration in milliseconds
    /// - `amplitude`: peak amplitude in [0, 1]
    /// - `sample_rate`: output sample rate (e.g. 8000 for typical haptic DAC)
    ///
    /// # Panics
    ///
    /// Does not panic for valid inputs. Returns a zero-length waveform if
    /// `freq_hz <= 0` or `duration_ms <= 0`.
    #[must_use]
    pub fn generate(
        kind: WaveformKind,
        freq_hz: f32,
        duration_ms: f32,
        amplitude: f32,
        sample_rate: u32,
    ) -> Self {
        let amplitude = amplitude.clamp(0.0, 1.0);
        let n_samples = ((sample_rate as f32 * duration_ms / 1000.0).round() as usize).max(0);

        let samples: Box<[f32]> = if freq_hz <= 0.0 || duration_ms <= 0.0 || n_samples == 0 {
            Box::new([])
        } else {
            match kind {
                WaveformKind::Sine => Self::sine(freq_hz, amplitude, n_samples, sample_rate),
                WaveformKind::Pulse => Self::pulse(amplitude, n_samples),
                WaveformKind::Ramp => Self::ramp(freq_hz, amplitude, n_samples, sample_rate),
                WaveformKind::Buzz => Self::buzz(freq_hz, amplitude, n_samples, sample_rate),
            }
        };

        Self {
            kind,
            freq_hz,
            sample_rate,
            samples,
        }
    }

    /// Duration of this waveform in milliseconds.
    #[must_use]
    pub fn duration_ms(&self) -> f32 {
        if self.sample_rate == 0 {
            return 0.0;
        }
        self.samples.len() as f32 / self.sample_rate as f32 * 1000.0
    }

    /// Peak amplitude in the waveform (maximum absolute value).
    #[must_use]
    pub fn peak_amplitude(&self) -> f32 {
        self.samples
            .iter()
            .map(|&s| s.abs())
            .fold(0.0_f32, f32::max)
    }

    /// RMS amplitude.
    #[must_use]
    pub fn rms_amplitude(&self) -> f32 {
        if self.samples.is_empty() {
            return 0.0;
        }
        let mean_sq: f32 =
            self.samples.iter().map(|&s| s * s).sum::<f32>() / self.samples.len() as f32;
        mean_sq.sqrt()
    }

    // ── Private generators ────────────────────────────────────────────────────

    fn sine(freq_hz: f32, amp: f32, n: usize, sr: u32) -> Box<[f32]> {
        (0..n)
            .map(|i| amp * (2.0 * PI * freq_hz * i as f32 / sr as f32).sin())
            .collect::<Vec<_>>()
            .into_boxed_slice()
    }

    fn pulse(amp: f32, n: usize) -> Box<[f32]> {
        // Gaussian centred at n/2, sigma = n/8
        let centre = n as f32 / 2.0;
        let sigma = (n as f32 / 8.0).max(1.0);
        (0..n)
            .map(|i| {
                let x = (i as f32 - centre) / sigma;
                amp * (-0.5 * x * x).exp()
            })
            .collect::<Vec<_>>()
            .into_boxed_slice()
    }

    fn ramp(freq_hz: f32, amp: f32, n: usize, sr: u32) -> Box<[f32]> {
        // Linear attack (0→1) over first half, then decay (1→0) over second half
        (0..n)
            .map(|i| {
                let env = if i < n / 2 {
                    2.0 * i as f32 / n as f32
                } else {
                    2.0 * (n - i) as f32 / n as f32
                };
                amp * env * (2.0 * PI * freq_hz * i as f32 / sr as f32).sin()
            })
            .collect::<Vec<_>>()
            .into_boxed_slice()
    }

    fn buzz(freq_hz: f32, amp: f32, n: usize, sr: u32) -> Box<[f32]> {
        // Sine clipped at ±0.7 amplitude, then normalised back to amp
        (0..n)
            .map(|i| {
                let s = (2.0 * PI * freq_hz * i as f32 / sr as f32).sin();
                amp * s.clamp(-0.7, 0.7) / 0.7
            })
            .collect::<Vec<_>>()
            .into_boxed_slice()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sine_waveform_has_correct_length() {
        let w = HapticWaveform::generate(WaveformKind::Sine, 200.0, 100.0, 1.0, 8000);
        // 8000 Hz * 0.1 s = 800 samples
        assert_eq!(w.samples.len(), 800);
    }

    #[test]
    fn sine_peak_amplitude_near_one() {
        let w = HapticWaveform::generate(WaveformKind::Sine, 200.0, 100.0, 1.0, 8000);
        assert!(w.peak_amplitude() <= 1.0 + 1e-5);
        assert!(w.peak_amplitude() > 0.9, "sine peak should be near 1.0");
    }

    #[test]
    fn pulse_is_symmetric_around_centre() {
        let w = HapticWaveform::generate(WaveformKind::Pulse, 200.0, 50.0, 1.0, 8000);
        assert!(!w.samples.is_empty());
        let n = w.samples.len();
        // Gaussian should be symmetric: samples at 1/4 and 3/4 should be equal
        let q1 = w.samples[n / 4];
        let q3 = w.samples[3 * n / 4];
        assert!((q1 - q3).abs() < 0.05, "pulse should be symmetric");
    }

    #[test]
    fn ramp_starts_and_ends_near_zero() {
        let w = HapticWaveform::generate(WaveformKind::Ramp, 200.0, 100.0, 1.0, 8000);
        assert!(w.samples[0].abs() < 0.05, "ramp should start near 0");
        assert!(
            w.samples[w.samples.len() - 1].abs() < 0.05,
            "ramp should end near 0"
        );
    }

    #[test]
    fn buzz_does_not_exceed_amplitude() {
        let w = HapticWaveform::generate(WaveformKind::Buzz, 200.0, 100.0, 0.8, 8000);
        assert!(w.peak_amplitude() <= 0.8 + 1e-5);
    }

    #[test]
    fn zero_duration_produces_empty_waveform() {
        let w = HapticWaveform::generate(WaveformKind::Sine, 200.0, 0.0, 1.0, 8000);
        assert!(w.samples.is_empty());
    }

    #[test]
    fn negative_freq_produces_empty_waveform() {
        let w = HapticWaveform::generate(WaveformKind::Sine, -1.0, 100.0, 1.0, 8000);
        assert!(w.samples.is_empty());
    }

    #[test]
    fn duration_ms_matches_generation_params() {
        let w = HapticWaveform::generate(WaveformKind::Sine, 200.0, 100.0, 1.0, 8000);
        assert!((w.duration_ms() - 100.0).abs() < 1.0);
    }

    #[test]
    fn rms_of_unit_sine_is_near_point_707() {
        // RMS of sine = amplitude / sqrt(2) ≈ 0.707
        let w = HapticWaveform::generate(WaveformKind::Sine, 200.0, 500.0, 1.0, 8000);
        let expected_rms = 1.0_f32 / 2.0_f32.sqrt();
        assert!(
            (w.rms_amplitude() - expected_rms).abs() < 0.02,
            "rms of unit sine ≈ 0.707, got {}",
            w.rms_amplitude()
        );
    }
}
