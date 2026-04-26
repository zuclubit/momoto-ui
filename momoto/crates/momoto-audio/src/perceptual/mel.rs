//! Mel filterbank for perceptual frequency-domain analysis.
//!
//! Maps a linear-frequency power spectrum to mel-scale filter outputs,
//! approximating the nonlinear frequency resolution of the human auditory system.
//!
//! # Formula (HTK convention)
//!
//! ```text
//! mel(f) = 2595 · log₁₀(1 + f/700)
//! f(mel) = 700 · (10^(mel/2595) - 1)
//! ```
//!
//! # Filter shape
//!
//! Each filter is triangular, unit-amplitude at its centre frequency,
//! linearly ramping up from the left edge and down to the right edge.
//! Adjacent filters overlap at exactly 50% of their bandwidth (HTK default).
//!
//! # Normalisation (Slaney)
//!
//! Each filter is normalised so that its area integrates to 1, making the
//! filterbank energy-preserving (the sum of all filter outputs equals the
//! total spectral energy when inputs are power-spectrum values).

use crate::physical::fft::FftPlan;

/// Hz → mel conversion (HTK formula).
#[inline]
pub fn hz_to_mel(hz: f32) -> f32 {
    2595.0 * (1.0 + hz / 700.0).log10()
}

/// mel → Hz conversion (HTK formula).
#[inline]
pub fn mel_to_hz(mel: f32) -> f32 {
    700.0 * (10.0_f32.powf(mel / 2595.0) - 1.0)
}

/// Mel filterbank: maps a power spectrum to mel-band energies.
///
/// # Construction
///
/// ```rust,ignore
/// use momoto_audio::perceptual::mel::MelFilterbank;
///
/// let fb = MelFilterbank::new(40, 1024, 48000, 0.0, 8000.0);
/// // apply to a 513-bin power spectrum (FFT of 1024 samples)
/// let mel_energies = fb.apply(&power_spectrum);
/// ```
#[derive(Debug, Clone)]
pub struct MelFilterbank {
    /// Number of mel bands.
    n_bands: usize,
    /// Number of FFT bins (N/2 + 1).
    n_bins: usize,
    #[allow(dead_code)]
    sample_rate: u32,
    /// Sparse filterbank weights: for each band, a list of (bin_index, weight).
    ///
    /// Stored as a flat Vec to avoid inner Vec allocations.
    /// `band_offsets[b]` gives the start index in `weights` for band `b`.
    weights: Box<[(usize, f32)]>,
    /// Start index in `weights` for each band, plus a sentinel at the end.
    band_offsets: Box<[usize]>,
}

impl MelFilterbank {
    /// Create a mel filterbank.
    ///
    /// # Parameters
    ///
    /// - `n_bands`: Number of mel filter bands (typically 20–128).
    /// - `fft_size`: FFT length N (must match the power spectrum length N/2+1).
    /// - `sample_rate`: Sample rate in Hz (affects Hz→bin mapping).
    /// - `f_min`: Minimum frequency in Hz (typically 0 or 20 Hz).
    /// - `f_max`: Maximum frequency in Hz (typically sample_rate/2).
    ///
    /// # Panics
    ///
    /// Panics if `n_bands == 0`, `fft_size < 4`, or `f_min >= f_max`.
    #[must_use]
    pub fn new(n_bands: usize, fft_size: usize, sample_rate: u32, f_min: f32, f_max: f32) -> Self {
        assert!(n_bands > 0, "n_bands must be > 0");
        assert!(fft_size >= 4, "fft_size must be >= 4");
        assert!(f_min < f_max, "f_min must be < f_max");

        let n_bins = fft_size / 2 + 1;
        let nyquist = sample_rate as f32 / 2.0;
        let f_max = f_max.min(nyquist);

        // Mel-scale center frequencies (n_bands + 2 points including edges)
        let mel_min = hz_to_mel(f_min);
        let mel_max = hz_to_mel(f_max);
        let mel_points: Vec<f32> = (0..=n_bands + 1)
            .map(|i| mel_min + (mel_max - mel_min) * i as f32 / (n_bands + 1) as f32)
            .collect();

        // Convert mel points to FFT bin indices
        let bin_points: Vec<usize> = mel_points
            .iter()
            .map(|&m| {
                let hz = mel_to_hz(m);
                let bin = (hz / nyquist * (n_bins - 1) as f32).round() as usize;
                bin.min(n_bins - 1)
            })
            .collect();

        // Build sparse weight list with Slaney normalisation
        let mut raw_weights: Vec<(usize, f32)> = Vec::new();
        let mut band_offsets: Vec<usize> = Vec::with_capacity(n_bands + 1);

        for b in 0..n_bands {
            band_offsets.push(raw_weights.len());
            let left = bin_points[b];
            let center = bin_points[b + 1];
            let right = bin_points[b + 2];

            // Slaney normalisation: area = 2 / (right - left) in Hz domain
            let hz_l = mel_to_hz(mel_points[b]);
            let hz_r = mel_to_hz(mel_points[b + 2]);
            let norm = if hz_r > hz_l {
                2.0 / (hz_r - hz_l)
            } else {
                1.0
            };

            // Rising slope: left → center
            if center > left {
                for bin in left..=center {
                    let t = (bin - left) as f32 / (center - left) as f32;
                    raw_weights.push((bin, t * norm));
                }
            }
            // Falling slope: center+1 → right
            if right > center {
                for bin in (center + 1)..=right {
                    let t = (right - bin) as f32 / (right - center) as f32;
                    raw_weights.push((bin, t * norm));
                }
            }
        }
        band_offsets.push(raw_weights.len()); // sentinel

        Self {
            n_bands,
            n_bins,
            sample_rate,
            weights: raw_weights.into_boxed_slice(),
            band_offsets: band_offsets.into_boxed_slice(),
        }
    }

    /// Number of mel bands.
    #[must_use]
    pub fn n_bands(&self) -> usize {
        self.n_bands
    }

    /// Number of FFT bins this filterbank expects.
    #[must_use]
    pub fn n_bins(&self) -> usize {
        self.n_bins
    }

    /// Apply the filterbank to a power spectrum.
    ///
    /// `power_spectrum` must have exactly `self.n_bins()` elements.
    ///
    /// Returns a `Box<[f32]>` of `n_bands` mel-band energies.
    ///
    /// For zero-allocation, use `apply_into()`.
    pub fn apply(&self, power_spectrum: &[f32]) -> Box<[f32]> {
        let mut out = vec![0.0_f32; self.n_bands];
        self.apply_into(power_spectrum, &mut out);
        out.into_boxed_slice()
    }

    /// Zero-allocation variant: apply filterbank into a pre-allocated slice.
    ///
    /// # Panics
    ///
    /// Panics if slice lengths do not match.
    pub fn apply_into(&self, power_spectrum: &[f32], out: &mut [f32]) {
        assert_eq!(
            power_spectrum.len(),
            self.n_bins,
            "power_spectrum must have n_bins={} elements",
            self.n_bins
        );
        assert_eq!(
            out.len(),
            self.n_bands,
            "output must have n_bands={} elements",
            self.n_bands
        );

        for b in 0..self.n_bands {
            let start = self.band_offsets[b];
            let end = self.band_offsets[b + 1];
            let mut energy = 0.0_f32;
            for &(bin, weight) in &self.weights[start..end] {
                energy += weight * power_spectrum[bin];
            }
            out[b] = energy;
        }
    }

    /// Convenience: run FFT on `samples` and apply filterbank.
    ///
    /// `samples` length must equal the FFT size this filterbank was built for.
    /// Returns `n_bands` mel-band energies.
    pub fn transform(&self, samples: &[f32], plan: &FftPlan) -> Box<[f32]> {
        assert_eq!(
            samples.len(),
            plan.len(),
            "samples length must match FFT plan"
        );
        assert_eq!(
            plan.len() / 2 + 1,
            self.n_bins,
            "FFT plan must match filterbank n_bins"
        );
        let ps = plan.power_spectrum(samples);
        self.apply(&ps)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hz_mel_roundtrip() {
        for hz in [20.0, 100.0, 440.0, 1000.0, 4000.0, 8000.0, 20000.0_f32] {
            let roundtrip = mel_to_hz(hz_to_mel(hz));
            assert!(
                (roundtrip - hz).abs() / hz < 1e-4,
                "roundtrip failed at {hz} Hz"
            );
        }
    }

    #[test]
    fn filterbank_construction_does_not_panic() {
        let _ = MelFilterbank::new(40, 1024, 48000, 0.0, 8000.0);
    }

    #[test]
    fn filterbank_dimensions() {
        let fb = MelFilterbank::new(40, 1024, 48000, 0.0, 8000.0);
        assert_eq!(fb.n_bands(), 40);
        assert_eq!(fb.n_bins(), 513);
    }

    #[test]
    fn filterbank_silence_produces_zero_output() {
        let fb = MelFilterbank::new(40, 1024, 48000, 0.0, 24000.0);
        let ps = vec![0.0_f32; fb.n_bins()];
        let out = fb.apply(&ps);
        for &v in out.iter() {
            assert!(v.abs() < 1e-9, "silence → zero energy");
        }
    }

    #[test]
    fn filterbank_non_negative_outputs() {
        // White noise power spectrum (uniform)
        let fb = MelFilterbank::new(20, 512, 44100, 0.0, 22050.0);
        let ps = vec![1.0_f32; fb.n_bins()];
        let out = fb.apply(&ps);
        for &v in out.iter() {
            assert!(v >= 0.0, "mel energies must be non-negative");
        }
    }

    #[test]
    fn apply_into_length_mismatch_panics() {
        let fb = MelFilterbank::new(10, 256, 48000, 0.0, 8000.0);
        let ps = vec![0.0_f32; fb.n_bins()];
        let mut out = vec![0.0_f32; fb.n_bands() + 1]; // wrong length
        let result = std::panic::catch_unwind(move || {
            fb.apply_into(&ps, &mut out);
        });
        assert!(result.is_err());
    }

    #[test]
    fn filterbank_with_fft_integration() {
        use crate::physical::fft::FftPlan;
        let fft_size = 1024;
        let fb = MelFilterbank::new(40, fft_size, 48000, 0.0, 24000.0);
        let plan = FftPlan::new(fft_size);
        // Impulse signal
        let mut samples = vec![0.0_f32; fft_size];
        samples[0] = 1.0;
        let mel = fb.transform(&samples, &plan);
        assert_eq!(mel.len(), 40);
        // Energy should be spread across bands (impulse has flat spectrum)
        let total_energy: f32 = mel.iter().sum();
        assert!(
            total_energy > 0.0,
            "impulse should produce non-zero mel energy"
        );
    }
}
