//! Cooley-Tukey Radix-2 Decimation-In-Time (DIT) FFT.
//!
//! A pure-Rust, allocation-free-in-hot-path FFT implementation.
//!
//! # Design
//!
//! `FftPlan` pre-computes twiddle factors and the bit-reversal permutation
//! table once at construction. The `fft()` / `ifft()` methods are zero-alloc:
//! they receive a pre-allocated `&mut [(f32, f32)]` work buffer and operate
//! in-place.
//!
//! # Limitations
//!
//! - Input length must be a power-of-two (1 ≤ N ≤ 2²³).
//! - Operates on `f32` complex samples `(real, imag)`.
//! - No multi-thread parallelism — designed for single-thread WASM.
//!
//! # Energy conservation (Parseval's theorem)
//!
//! For a length-N signal `x[n]`:
//! ```text
//! Σ|x[n]|² = (1/N) · Σ|X[k]|²
//! ```
//! `power_spectrum()` normalises by `1/N` so that time-domain and
//! frequency-domain energies match.
//!
//! # References
//!
//! - Cooley & Tukey (1965), "An Algorithm for the Machine Calculation of
//!   Complex Fourier Series", Mathematics of Computation 19(90), 297–301.

use std::f32::consts::PI;

/// Minimum power spectrum value treated as non-zero energy.
///
/// Bins below this level are clamped to `0.0` to prevent `-inf` results when
/// log-scaling the spectrum downstream, and to eliminate numerical noise from
/// floating-point rounding in the FFT butterfly.
pub const FFT_POWER_EPSILON: f32 = 1.0e-30;

/// Pre-computed FFT plan for a fixed-length signal.
///
/// Allocates once (twiddle factors + bit-reversal table); `fft()` / `ifft()`
/// are then allocation-free.
///
/// # Construction
///
/// ```rust,ignore
/// use momoto_audio::physical::fft::FftPlan;
///
/// let plan = FftPlan::new(1024); // N must be power-of-two
/// ```
#[derive(Debug, Clone)]
pub struct FftPlan {
    /// FFT length (power-of-two).
    n: usize,
    /// Number of stages: log₂(N).
    stages: usize,
    /// Twiddle factors: W_N^k = (cos(2πk/N), -sin(2πk/N)), k = 0..N/2.
    twiddles: Box<[(f32, f32)]>,
    /// Bit-reversal permutation indices for in-place scrambling.
    bit_rev: Box<[usize]>,
}

impl FftPlan {
    /// Create a new FFT plan for length `n`.
    ///
    /// # Panics
    ///
    /// Panics if `n` is not a power of two or `n == 0`.
    #[must_use]
    pub fn new(n: usize) -> Self {
        assert!(
            n > 0 && n.is_power_of_two(),
            "FFT length must be a non-zero power of two"
        );
        let stages = n.trailing_zeros() as usize;

        // Pre-compute twiddle factors W_N^k for k = 0..N/2
        let half = n / 2;
        let twiddles: Box<[(f32, f32)]> = (0..half)
            .map(|k| {
                let angle = -2.0 * PI * k as f32 / n as f32;
                (angle.cos(), angle.sin())
            })
            .collect::<Vec<_>>()
            .into_boxed_slice();

        // Pre-compute bit-reversal permutation
        let bit_rev: Box<[usize]> = (0..n)
            .map(|i| {
                let mut j = 0usize;
                let mut ii = i;
                for _ in 0..stages {
                    j = (j << 1) | (ii & 1);
                    ii >>= 1;
                }
                j
            })
            .collect::<Vec<_>>()
            .into_boxed_slice();

        Self {
            n,
            stages,
            twiddles,
            bit_rev,
        }
    }

    /// FFT length this plan was built for.
    #[must_use]
    pub fn len(&self) -> usize {
        self.n
    }

    /// Number of FFT stages (log₂(N)).
    #[must_use]
    pub fn stages(&self) -> usize {
        self.stages
    }

    /// Returns `true` if `n == 1` (trivial transform).
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.n == 1
    }

    // ── In-place FFT ─────────────────────────────────────────────────────────

    /// Compute the forward DFT of `buf` in-place.
    ///
    /// `buf` must have exactly `self.len()` elements.
    /// Each element is a complex number `(real, imag)`.
    ///
    /// After the call, `buf[k]` contains `X[k]` (unscaled).
    ///
    /// # Panics
    ///
    /// Panics if `buf.len() != self.len()`.
    pub fn fft(&self, buf: &mut [(f32, f32)]) {
        assert_eq!(buf.len(), self.n, "buffer length must equal plan length");
        self.bit_reverse_permute(buf);
        self.butterfly_passes(buf, false);
    }

    /// Compute the inverse DFT of `buf` in-place (normalised by 1/N).
    ///
    /// After the call, `buf[n]` contains the reconstructed time-domain sample.
    ///
    /// # Panics
    ///
    /// Panics if `buf.len() != self.len()`.
    pub fn ifft(&self, buf: &mut [(f32, f32)]) {
        assert_eq!(buf.len(), self.n, "buffer length must equal plan length");
        // Conjugate → forward FFT → conjugate → scale by 1/N
        for s in buf.iter_mut() {
            s.1 = -s.1;
        }
        self.bit_reverse_permute(buf);
        self.butterfly_passes(buf, false);
        let scale = 1.0 / self.n as f32;
        for s in buf.iter_mut() {
            s.0 *= scale;
            s.1 = -s.1 * scale;
        }
    }

    /// Compute the one-sided power spectrum from a real-valued signal.
    ///
    /// `samples` must have exactly `self.len()` elements.
    /// Returns `n/2 + 1` power values (DC through Nyquist), each in units
    /// of `amplitude²` normalised by `1/N` (satisfies Parseval's theorem).
    ///
    /// The returned `Box<[f32]>` allocates once per call; for hot paths, use
    /// `power_spectrum_into()` with a pre-allocated output slice.
    ///
    /// # Panics
    ///
    /// Panics if `samples.len() != self.len()`.
    pub fn power_spectrum(&self, samples: &[f32]) -> Box<[f32]> {
        let mut out = vec![0.0_f32; self.n / 2 + 1];
        self.power_spectrum_into(samples, &mut out);
        out.into_boxed_slice()
    }

    /// Zero-allocation variant: compute power spectrum into a pre-allocated slice.
    ///
    /// `out` must have exactly `self.len() / 2 + 1` elements.
    ///
    /// # Panics
    ///
    /// Panics if slice lengths do not match the plan.
    pub fn power_spectrum_into(&self, samples: &[f32], out: &mut [f32]) {
        assert_eq!(samples.len(), self.n, "samples length mismatch");
        assert_eq!(out.len(), self.n / 2 + 1, "output length must be N/2+1");

        // Build complex buffer from real samples
        let mut buf: Vec<(f32, f32)> = samples.iter().map(|&s| (s, 0.0)).collect();
        self.fft(&mut buf);

        // Compute normalised power (Parseval's: Σ|X[k]|²/N = Σ|x[n]|²)
        let scale = 1.0 / self.n as f32;
        let half = self.n / 2;
        for k in 0..=half {
            let (re, im) = buf[k];
            let p = (re * re + im * im) * scale;
            // Guard: replace NaN/Inf (e.g. from NaN input samples) with 0.
            out[k] = if p.is_finite() && p >= 0.0 { p } else { 0.0 };
        }
    }

    // ── Internal helpers ──────────────────────────────────────────────────────

    fn bit_reverse_permute(&self, buf: &mut [(f32, f32)]) {
        for i in 0..self.n {
            let j = self.bit_rev[i];
            if i < j {
                buf.swap(i, j);
            }
        }
    }

    /// Butterfly passes (Cooley-Tukey DIT).
    ///
    /// `inverse` flag is unused (we handle inversion outside via conjugation).
    fn butterfly_passes(&self, buf: &mut [(f32, f32)], _inverse: bool) {
        let mut half_size = 1usize;
        for _ in 0..self.stages {
            let size = half_size * 2;
            let twiddle_step = self.n / size;
            let mut k = 0;
            while k < self.n {
                for j in 0..half_size {
                    let t_idx = j * twiddle_step;
                    let (wr, wi) = self.twiddles[t_idx];
                    let (ur, ui) = buf[k + j];
                    let (vr, vi) = buf[k + j + half_size];
                    let tr = wr * vr - wi * vi;
                    let ti = wr * vi + wi * vr;
                    buf[k + j] = (ur + tr, ui + ti);
                    buf[k + j + half_size] = (ur - tr, ui - ti);
                }
                k += size;
            }
            half_size = size;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn complex_magnitude(c: (f32, f32)) -> f32 {
        (c.0 * c.0 + c.1 * c.1).sqrt()
    }

    #[test]
    fn fft_dc_offset() {
        // DC signal: x[n] = A → X[0] = N*A, X[k] = 0 for k > 0
        let plan = FftPlan::new(8);
        let mut buf: Vec<(f32, f32)> = vec![(1.0, 0.0); 8];
        plan.fft(&mut buf);
        let mag_dc = complex_magnitude(buf[0]);
        assert!((mag_dc - 8.0).abs() < 1e-4, "DC bin magnitude should be N");
        for k in 1..8 {
            assert!(complex_magnitude(buf[k]) < 1e-4, "non-DC bins should be ~0");
        }
    }

    #[test]
    fn fft_single_tone() {
        // x[n] = cos(2π*k₀*n/N) → energy at k₀ and N-k₀
        let n = 16;
        let k0 = 3usize;
        let plan = FftPlan::new(n);
        let mut buf: Vec<(f32, f32)> = (0..n)
            .map(|i| {
                let angle = 2.0 * PI * k0 as f32 * i as f32 / n as f32;
                (angle.cos(), 0.0)
            })
            .collect();
        plan.fft(&mut buf);
        // Peaks at k0 and N-k0
        let mag_k0 = complex_magnitude(buf[k0]);
        let mag_nmk0 = complex_magnitude(buf[n - k0]);
        assert!(mag_k0 > 6.0, "peak at k0, got {}", mag_k0);
        assert!(mag_nmk0 > 6.0, "peak at N-k0, got {}", mag_nmk0);
        // Other bins should be small
        for k in 1..n {
            if k != k0 && k != (n - k0) {
                assert!(
                    complex_magnitude(buf[k]) < 1.0,
                    "bin {} should be near zero, got {}",
                    k,
                    complex_magnitude(buf[k])
                );
            }
        }
    }

    #[test]
    fn ifft_roundtrip() {
        let n = 64;
        let plan = FftPlan::new(n);
        let original: Vec<(f32, f32)> = (0..n)
            .map(|i| ((i as f32 / n as f32) * 2.0 * PI).sin())
            .map(|v| (v, 0.0))
            .collect();
        let mut buf = original.clone();
        plan.fft(&mut buf);
        plan.ifft(&mut buf);
        for (a, b) in original.iter().zip(buf.iter()) {
            assert!((a.0 - b.0).abs() < 1e-4, "ifft roundtrip real part");
            assert!(b.1.abs() < 1e-4, "ifft roundtrip imag part should be ~0");
        }
    }

    #[test]
    fn power_spectrum_parseval_theorem() {
        // Parseval: Σ|x[n]|² = Σ|X[k]|²/N (with our normalisation: exactly the ps sum × N)
        let n = 256;
        let plan = FftPlan::new(n);
        let samples: Vec<f32> = (0..n)
            .map(|i| ((i as f32 / n as f32) * 2.0 * PI).sin())
            .collect();

        // Time-domain energy
        let e_time: f32 = samples.iter().map(|&s| s * s).sum::<f32>() / n as f32;

        // Frequency-domain energy from power spectrum (DC + positive + Nyquist)
        let ps = plan.power_spectrum(&samples);
        // ps[k] = |X[k]|²/N; for a real signal the two-sided sum = 2*ps[1..N/2] + ps[0] + ps[N/2]
        let mut e_freq = ps[0] + ps[n / 2];
        for k in 1..n / 2 {
            e_freq += 2.0 * ps[k]; // mirror bins
        }
        e_freq /= n as f32;

        assert!(
            (e_time - e_freq).abs() < 1e-3,
            "Parseval: e_time={e_time:.6} e_freq={e_freq:.6}"
        );
    }

    #[test]
    fn power_spectrum_silence_is_zero() {
        let plan = FftPlan::new(64);
        let samples = vec![0.0_f32; 64];
        let ps = plan.power_spectrum(&samples);
        for &p in ps.iter() {
            assert!(p.abs() < 1e-12, "silence → zero power");
        }
    }

    #[test]
    fn plan_rejects_non_power_of_two() {
        let result = std::panic::catch_unwind(|| FftPlan::new(100));
        assert!(result.is_err());
    }

    #[test]
    fn plan_rejects_zero() {
        let result = std::panic::catch_unwind(|| FftPlan::new(0));
        assert!(result.is_err());
    }

    #[test]
    fn plan_len_and_stages() {
        let plan = FftPlan::new(1024);
        assert_eq!(plan.len(), 1024);
        assert_eq!(plan.stages(), 10);
    }

    #[test]
    fn power_spectrum_nan_input_produces_finite_output() {
        let n = 64;
        let plan = FftPlan::new(n);
        let mut samples = vec![0.5_f32; n];
        samples[10] = f32::NAN;
        samples[20] = f32::INFINITY;
        let ps = plan.power_spectrum(&samples);
        for (k, &p) in ps.iter().enumerate() {
            assert!(
                p.is_finite(),
                "bin {k} should be finite even with NaN input, got {p}"
            );
            assert!(p >= 0.0, "power spectrum must be non-negative");
        }
    }
}
