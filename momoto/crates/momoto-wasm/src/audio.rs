// =============================================================================
// momoto-wasm: Audio Domain Bindings
// File: crates/momoto-wasm/src/audio.rs
//
// WebAssembly exports for the momoto-audio acoustic signal processing domain.
//
// # Design constraints (from architectural spec)
//   - Flat f32 only — no JSON, no nested JS objects, no heap-allocated outputs
//     except Box<[f32]> (single contiguous allocation returned as Float32Array)
//   - All functions are free functions (no stateful WASM objects for hot paths)
//   - Deterministic: same input → same output on all platforms
//   - Allocation-free in the inner loop (FftPlan, KWeightingFilter preallocated
//     by callers or constructed once and held in a WASM struct for multi-call use)
//
// # WASM API surface (JS camelCase names)
//   audio_lufs(samples, sampleRate)                     → f32 (integrated LUFS)
//   audio_momentary_lufs(samples, sampleRate)           → f32 (400 ms momentary)
//   audio_fft_power_spectrum(samples)                   → Float32Array (N/2+1)
//   audio_mel_spectrum(samples, sampleRate, nBands)     → Float32Array (nBands)
//   audio_spectral_centroid(powerSpectrum, sampleRate)  → f32 (Hz)
//   audio_spectral_brightness(ps, sampleRate, threshHz) → f32 (0..1)
//   audio_spectral_flux(prev, curr)                     → f32
//   audio_spectral_rolloff(ps, sampleRate, rollPercent) → f32 (Hz)
//   audio_spectral_flatness(ps)                         → f32 (0..1)
//   audio_validate_ebur128(integratedLufs, profile)     → bool
//   domain_process(samples, sampleRate)                 → Float32Array (mel)
//   domain_perceptual_distance(a, b)                    → f32
// =============================================================================

use wasm_bindgen::prelude::*;

use momoto_audio::{
    spectral_brightness, spectral_centroid, spectral_flatness, spectral_flux, spectral_rolloff,
    AudioDomain, EbuR128Limits, FftPlan, LufsAnalyzer, MelFilterbank,
};
use momoto_core::traits::domain::Domain;

// =============================================================================
// LUFS Loudness
// =============================================================================

/// Compute integrated LUFS loudness for a mono signal (one block = 400 ms).
///
/// Returns integrated loudness in LUFS as f32.
/// Returns `f32::NEG_INFINITY` (as 0.0 in JS) if the signal is silence or
/// falls below the absolute gate (-70 LUFS).
///
/// # Parameters
/// - `samples`: mono f32 samples (recommended: 400 ms = 19 200 samples at 48 kHz)
/// - `sample_rate`: audio sample rate (44100, 48000, or 96000 Hz)
///
/// # Returns
/// Integrated LUFS as f32, or -999.0 if sample rate is unsupported.
#[wasm_bindgen(js_name = "audioLufs")]
pub fn audio_lufs(samples: &[f32], sample_rate: u32) -> f32 {
    let Some(mut analyzer) = LufsAnalyzer::new(sample_rate, 1) else {
        return -999.0; // unsupported sample rate
    };
    analyzer.add_mono_block(samples);
    let lufs = analyzer.integrated();
    if lufs.is_finite() {
        lufs as f32
    } else {
        f32::NEG_INFINITY
    }
}

/// Compute momentary LUFS for the most recent 400 ms block.
///
/// Returns -999.0 for unsupported sample rates.
#[wasm_bindgen(js_name = "audioMomentaryLufs")]
pub fn audio_momentary_lufs(samples: &[f32], sample_rate: u32) -> f32 {
    let Some(mut analyzer) = LufsAnalyzer::new(sample_rate, 1) else {
        return -999.0;
    };
    analyzer.add_mono_block(samples);
    let m = analyzer.momentary();
    if m.is_finite() {
        m as f32
    } else {
        f32::NEG_INFINITY
    }
}

// =============================================================================
// FFT Power Spectrum
// =============================================================================

/// Compute the one-sided power spectrum of a real-valued mono signal.
///
/// The input length must be a power of two (e.g. 1024, 2048, 4096).
/// Returns a `Float32Array` of length `N/2 + 1` (DC through Nyquist).
/// Power values are normalised by `1/N` (satisfies Parseval's theorem).
///
/// Returns an empty array if `samples.len()` is not a power of two or is 0.
///
/// # JavaScript usage
/// ```javascript
/// const ps = audioFftPowerSpectrum(samples); // samples.length must be 2^k
/// ```
#[wasm_bindgen(js_name = "audioFftPowerSpectrum")]
pub fn audio_fft_power_spectrum(samples: &[f32]) -> Box<[f32]> {
    let n = samples.len();
    if n == 0 || !n.is_power_of_two() {
        return Box::new([]);
    }
    let plan = FftPlan::new(n);
    plan.power_spectrum(samples)
}

// =============================================================================
// Mel Filterbank
// =============================================================================

/// Compute mel-band energies from a mono signal.
///
/// Runs FFT + mel filterbank in one call.
/// `samples.len()` must be a power of two.
/// `n_bands` must be > 0.
///
/// Returns a `Float32Array` of `n_bands` mel-band energy values,
/// or an empty array on invalid input.
///
/// # Parameters
/// - `samples`: mono f32 samples (power-of-two length)
/// - `sample_rate`: audio sample rate (44100, 48000, 96000)
/// - `n_bands`: number of mel bands (typically 20–128)
#[wasm_bindgen(js_name = "audioMelSpectrum")]
pub fn audio_mel_spectrum(samples: &[f32], sample_rate: u32, n_bands: usize) -> Box<[f32]> {
    let n = samples.len();
    if n == 0 || !n.is_power_of_two() || n_bands == 0 {
        return Box::new([]);
    }
    let plan = FftPlan::new(n);
    let nyquist = sample_rate as f32 / 2.0;
    let fb = MelFilterbank::new(n_bands, n, sample_rate, 0.0, nyquist);
    fb.transform(samples, &plan)
}

// =============================================================================
// Spectral Features
// =============================================================================

/// Compute spectral centroid (centre-of-mass of power spectrum) in Hz.
///
/// `power_spectrum` is the one-sided spectrum returned by `audioFftPowerSpectrum`.
/// Returns 0.0 for silence.
#[wasm_bindgen(js_name = "audioSpectralCentroid")]
pub fn audio_spectral_centroid(power_spectrum: &[f32], sample_rate: u32) -> f32 {
    spectral_centroid(power_spectrum, sample_rate)
}

/// Compute spectral brightness: fraction of power above `threshold_hz`.
///
/// Returns a value in [0, 1]. Returns 0.0 for silence.
#[wasm_bindgen(js_name = "audioSpectralBrightness")]
pub fn audio_spectral_brightness(
    power_spectrum: &[f32],
    sample_rate: u32,
    threshold_hz: f32,
) -> f32 {
    spectral_brightness(power_spectrum, sample_rate, threshold_hz)
}

/// Compute spectral flux between two consecutive power spectra.
///
/// Half-wave rectified (only positive changes). Both slices must have the
/// same length. Returns 0.0 if lengths differ or are empty.
#[wasm_bindgen(js_name = "audioSpectralFlux")]
pub fn audio_spectral_flux(previous: &[f32], current: &[f32]) -> f32 {
    spectral_flux(previous, current)
}

/// Compute spectral rolloff: frequency (Hz) below which `roll_percent`×100%
/// of spectral energy lies. Common value: 0.85.
///
/// Returns 0.0 for silence or invalid `roll_percent`.
#[wasm_bindgen(js_name = "audioSpectralRolloff")]
pub fn audio_spectral_rolloff(power_spectrum: &[f32], sample_rate: u32, roll_percent: f32) -> f32 {
    spectral_rolloff(power_spectrum, sample_rate, roll_percent)
}

/// Compute spectral flatness (Wiener entropy): 1.0 = noise-like, 0.0 = tonal.
#[wasm_bindgen(js_name = "audioSpectralFlatness")]
pub fn audio_spectral_flatness(power_spectrum: &[f32]) -> f32 {
    spectral_flatness(power_spectrum)
}

// =============================================================================
// EBU R128 Compliance
// =============================================================================

/// Validate integrated LUFS against an EBU R128 profile.
///
/// `profile` must be one of: `"broadcast"`, `"streaming"`, `"podcast"`.
/// Unknown profiles default to the broadcast profile.
///
/// Returns `true` if the measured loudness is within the permitted range.
///
/// # JavaScript usage
/// ```javascript
/// const ok = audioValidateEbuR128(-23.0, "broadcast"); // → true
/// const ok2 = audioValidateEbuR128(-10.0, "streaming"); // → false (too loud)
/// ```
#[wasm_bindgen(js_name = "audioValidateEbuR128")]
pub fn audio_validate_ebur128(integrated_lufs: f64, profile: &str) -> bool {
    let limits = match profile.to_ascii_lowercase().as_str() {
        "streaming" => EbuR128Limits::STREAMING,
        "podcast" => EbuR128Limits::PODCAST,
        _ => EbuR128Limits::BROADCAST, // "broadcast" or default
    };
    limits.validate(integrated_lufs, None, None).passes
}

// =============================================================================
// Domain-level API
// =============================================================================

/// Process a mono signal through the audio domain pipeline.
///
/// One-shot: applies FFT + mel filterbank and returns `n_bands` mel-band
/// energies as a `Float32Array`. Uses 40 bands and the full 0–Nyquist range.
///
/// `samples.len()` must be a power of two (minimum 64).
/// Returns empty array on invalid input.
///
/// # JavaScript usage
/// ```javascript
/// const features = domainProcess(samples, 48000); // → Float32Array(40)
/// ```
#[wasm_bindgen(js_name = "domainProcess")]
pub fn domain_process(samples: &[f32], sample_rate: u32) -> Box<[f32]> {
    audio_mel_spectrum(samples, sample_rate, 40)
}

/// Compute the perceptual distance between two audio signals.
///
/// Computes mel spectra for both signals, then returns the L² (Euclidean)
/// distance between them in the 40-band mel feature space.
///
/// Both signals must have the same length (power of two). Returns 0.0 for
/// identical signals, or -1.0 for invalid/mismatched inputs.
///
/// # JavaScript usage
/// ```javascript
/// const dist = domainPerceptualDistance(signalA, signalB, 48000);
/// ```
#[wasm_bindgen(js_name = "domainPerceptualDistance")]
pub fn domain_perceptual_distance(a: &[f32], b: &[f32], sample_rate: u32) -> f32 {
    if a.len() != b.len() || a.is_empty() {
        return -1.0;
    }
    let mel_a = audio_mel_spectrum(a, sample_rate, 40);
    let mel_b = audio_mel_spectrum(b, sample_rate, 40);
    if mel_a.is_empty() || mel_b.is_empty() {
        return -1.0;
    }
    mel_a
        .iter()
        .zip(mel_b.iter())
        .map(|(&x, &y)| (x - y) * (x - y))
        .sum::<f32>()
        .sqrt()
}

/// Returns the audio domain name and version as a JSON string.
///
/// Useful for feature detection from JavaScript.
///
/// Returns: `{"domain":"audio","name":"momoto-audio","version":"X.Y.Z"}`
#[wasm_bindgen(js_name = "audioDomainInfo")]
pub fn audio_domain_info() -> String {
    let d = AudioDomain::at_48khz();
    format!(
        r#"{{"domain":"audio","name":"{}","version":"{}","deterministic":{}}}"#,
        d.name(),
        d.version(),
        d.is_deterministic(),
    )
}
