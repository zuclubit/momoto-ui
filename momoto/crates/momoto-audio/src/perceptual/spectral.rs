//! Spectral feature extraction from power spectra.
//!
//! All functions operate on one-sided power spectra (DC through Nyquist)
//! as produced by `FftPlan::power_spectrum()`. All inputs and outputs are
//! `f32` — allocation-free for use in WASM hot paths.

/// Spectral centroid: the "centre of mass" of the power spectrum (in Hz).
///
/// ```text
/// centroid = Σ(f[k] · P[k]) / Σ(P[k])
/// ```
///
/// Returns `0.0` if total power is zero (silence).
///
/// # Parameters
///
/// - `power_spectrum`: one-sided spectrum of length N/2+1
/// - `sample_rate`: audio sample rate in Hz (for bin→Hz conversion)
#[must_use]
pub fn spectral_centroid(power_spectrum: &[f32], sample_rate: u32) -> f32 {
    if power_spectrum.is_empty() {
        return 0.0;
    }
    let nyquist = sample_rate as f32 / 2.0;
    let n_bins = power_spectrum.len();
    let mut weighted_sum = 0.0_f64;
    let mut total_power = 0.0_f64;
    for (k, &p) in power_spectrum.iter().enumerate() {
        let freq = (k as f64 / (n_bins - 1) as f64) * nyquist as f64;
        weighted_sum += freq * p as f64;
        total_power += p as f64;
    }
    if total_power < 1e-30 {
        return 0.0;
    }
    (weighted_sum / total_power) as f32
}

/// Spectral brightness: fraction of total power above `threshold_hz`.
///
/// ```text
/// brightness = Σ P[k≥threshold_bin] / Σ P[k]
/// ```
///
/// Returns a value in `[0, 1]`. Returns `0.0` if total power is zero.
#[must_use]
pub fn spectral_brightness(power_spectrum: &[f32], sample_rate: u32, threshold_hz: f32) -> f32 {
    if power_spectrum.is_empty() {
        return 0.0;
    }
    let nyquist = sample_rate as f32 / 2.0;
    let n_bins = power_spectrum.len();
    let threshold_bin = ((threshold_hz / nyquist) * (n_bins - 1) as f32).round() as usize;
    let threshold_bin = threshold_bin.min(n_bins);

    let total: f32 = power_spectrum.iter().sum();
    if total < 1e-30 {
        return 0.0;
    }
    let above: f32 = power_spectrum[threshold_bin..].iter().sum();
    (above / total).clamp(0.0, 1.0)
}

/// Spectral flux: mean squared difference between consecutive power spectra.
///
/// Measures how rapidly the spectrum is changing — high flux indicates
/// transient events (attacks, note onsets).
///
/// ```text
/// flux = Σ max(P_now[k] - P_prev[k], 0)² / N_bins
/// ```
///
/// Half-wave rectification (only positive differences) emphasises onsets
/// over offsets, following Bello et al. (2005).
///
/// Returns `0.0` if `previous` and `current` have different lengths or are empty.
#[must_use]
pub fn spectral_flux(previous: &[f32], current: &[f32]) -> f32 {
    if previous.len() != current.len() || previous.is_empty() {
        return 0.0;
    }
    let sum_sq: f32 = previous
        .iter()
        .zip(current.iter())
        .map(|(&p, &c)| {
            let d = c - p;
            if d > 0.0 {
                d * d
            } else {
                0.0
            }
        })
        .sum();
    sum_sq / previous.len() as f32
}

/// Spectral rolloff: frequency below which `roll_percent`% of energy lies.
///
/// Commonly used with `roll_percent = 0.85` (85th percentile).
///
/// Returns the rolloff frequency in Hz. Returns `0.0` if total energy is zero
/// or `roll_percent` is outside `(0, 1]`.
///
/// # Parameters
///
/// - `power_spectrum`: one-sided spectrum of length N/2+1
/// - `sample_rate`: audio sample rate
/// - `roll_percent`: fraction of total energy (0 < roll_percent ≤ 1)
#[must_use]
pub fn spectral_rolloff(power_spectrum: &[f32], sample_rate: u32, roll_percent: f32) -> f32 {
    if power_spectrum.is_empty() || roll_percent <= 0.0 || roll_percent > 1.0 {
        return 0.0;
    }
    let total: f32 = power_spectrum.iter().sum();
    if total < 1e-30 {
        return 0.0;
    }
    let target = total * roll_percent;
    let mut cumsum = 0.0_f32;
    let nyquist = sample_rate as f32 / 2.0;
    let n_bins = power_spectrum.len();
    for (k, &p) in power_spectrum.iter().enumerate() {
        cumsum += p;
        if cumsum >= target {
            return (k as f32 / (n_bins - 1) as f32) * nyquist;
        }
    }
    nyquist // all energy found
}

/// Spectral flatness (Wiener entropy): ratio of geometric mean to arithmetic mean.
///
/// ```text
/// flatness = exp(Σ log(P[k]) / N) / (Σ P[k] / N)
/// ```
///
/// - Value near `1.0` → flat/noisy signal (white noise → ≈ 1)
/// - Value near `0.0` → tonal signal (pure sine → → 0)
///
/// Returns `0.0` for silence or very tonal signals.
#[must_use]
pub fn spectral_flatness(power_spectrum: &[f32]) -> f32 {
    if power_spectrum.is_empty() {
        return 0.0;
    }
    let n = power_spectrum.len() as f64;
    let arithmetic_mean: f64 = power_spectrum.iter().map(|&p| p as f64).sum::<f64>() / n;
    if arithmetic_mean < 1e-30 {
        return 0.0;
    }

    let log_sum: f64 = power_spectrum
        .iter()
        .map(|&p| if p > 0.0 { (p as f64).ln() } else { -1000.0 })
        .sum::<f64>();
    let geometric_mean = (log_sum / n).exp();
    (geometric_mean / arithmetic_mean).clamp(0.0, 1.0) as f32
}

#[cfg(test)]
mod tests {
    use super::*;

    fn flat_spectrum(n_bins: usize) -> Vec<f32> {
        vec![1.0_f32; n_bins]
    }

    #[test]
    fn centroid_of_flat_spectrum_is_nyquist_half() {
        // Flat spectrum → centroid at Nyquist/2
        let ps = flat_spectrum(513); // 1024-point FFT
        let c = spectral_centroid(&ps, 48000);
        assert!(
            (c - 12000.0).abs() < 200.0,
            "flat spectrum centroid ≈ 12 kHz, got {c}"
        );
    }

    #[test]
    fn centroid_of_silence_is_zero() {
        let ps = vec![0.0_f32; 513];
        assert_eq!(spectral_centroid(&ps, 48000), 0.0);
    }

    #[test]
    fn brightness_all_above_threshold() {
        let ps = flat_spectrum(513);
        let b = spectral_brightness(&ps, 48000, 0.0); // threshold at 0 Hz
        assert!(
            (b - 1.0).abs() < 1e-4,
            "all energy above 0 Hz → brightness = 1"
        );
    }

    #[test]
    fn brightness_nothing_above_nyquist() {
        let ps = flat_spectrum(513);
        let b = spectral_brightness(&ps, 48000, 25000.0); // above Nyquist
        assert!(b < 0.01, "no energy above Nyquist → brightness ≈ 0");
    }

    #[test]
    fn spectral_flux_identical_spectra_is_zero() {
        let ps = flat_spectrum(513);
        let flux = spectral_flux(&ps, &ps);
        assert!(flux < 1e-9, "identical spectra → zero flux");
    }

    #[test]
    fn spectral_flux_impulse_vs_silence() {
        let prev = vec![0.0_f32; 4];
        let curr = vec![1.0_f32; 4];
        let flux = spectral_flux(&prev, &curr);
        assert!(flux > 0.0, "energy increase → positive flux");
    }

    #[test]
    fn spectral_flux_length_mismatch_is_zero() {
        let a = vec![1.0_f32; 4];
        let b = vec![1.0_f32; 5];
        assert_eq!(spectral_flux(&a, &b), 0.0);
    }

    #[test]
    fn rolloff_at_100_percent_is_nyquist() {
        let ps = flat_spectrum(513);
        let r = spectral_rolloff(&ps, 48000, 1.0);
        assert!(
            (r - 24000.0).abs() < 100.0,
            "100% rolloff ≈ Nyquist, got {r}"
        );
    }

    #[test]
    fn rolloff_silence_is_zero() {
        let ps = vec![0.0_f32; 513];
        assert_eq!(spectral_rolloff(&ps, 48000, 0.85), 0.0);
    }

    #[test]
    fn flatness_white_noise_approaches_one() {
        // Uniform spectrum → flatness = 1 (approx, due to f32 precision)
        let ps = flat_spectrum(64);
        let f = spectral_flatness(&ps);
        assert!(
            (f - 1.0).abs() < 0.01,
            "uniform spectrum flatness ≈ 1, got {f}"
        );
    }

    #[test]
    fn flatness_impulse_in_spectrum_near_zero() {
        // Single non-zero bin → very tonal → flatness near 0
        let mut ps = vec![0.0_f32; 64];
        ps[10] = 1.0;
        let f = spectral_flatness(&ps);
        assert!(f < 0.1, "tonal spectrum flatness < 0.1, got {f}");
    }
}
