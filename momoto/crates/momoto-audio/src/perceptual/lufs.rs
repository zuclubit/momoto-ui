//! LUFS loudness measurement (ITU-R BS.1770-4).
//!
//! Implements Loudness Units relative to Full Scale (LUFS) per the
//! ITU-R BS.1770-4 standard, as adopted by EBU R128 and ATSC A/85.
//!
//! # Algorithm
//!
//! 1. **K-weighting**: Pass each channel through the two-stage biquad filter.
//! 2. **Mean-square**: Compute per-channel mean-square of K-weighted samples.
//! 3. **Channel-weighted sum**: `z[n] = G_L*y_L² + G_R*y_R² + ... `
//!    (centre and surround channels have G=1; LFE excluded).
//! 4. **Momentary loudness** (400 ms window): `M = -0.691 + 10·log₁₀(z̄)`.
//! 5. **Short-term loudness** (3 s window): same formula, 3-second sliding mean.
//! 6. **Integrated loudness** (gated): two-pass gating —
//!    - Absolute gate: discard blocks where `M < -70 LUFS`
//!    - Relative gate: discard blocks where `M < Γ_R - 10 LU`
//!      where `Γ_R` is the preliminary mean of ungated blocks.
//!
//! # Mono / stereo simplified API
//!
//! For WASM export and testing, `LufsAnalyzer` provides a mono/stereo path
//! that skips multi-channel routing: call `add_mono_block()` or
//! `add_stereo_block()` with pre-K-weighted samples.
//!
//! # References
//!
//! - ITU-R BS.1770-4 (2015)
//! - EBU Tech 3341 v3 (2016) — practical implementation notes

use crate::filters::kweighting::KWeightingFilter;

/// LUFS absolute gate threshold (ITU-R BS.1770-4 §2.8).
pub const ABSOLUTE_GATE_LUFS: f64 = -70.0;

/// LUFS relative gate offset from ungated mean (ITU-R BS.1770-4 §2.8).
pub const RELATIVE_GATE_LU: f64 = -10.0;

/// Conversion constant: `LUFS = K + 10·log₁₀(mean_square)`.
pub const LUFS_OFFSET: f64 = -0.691;

/// Minimum mean-square value treated as non-silent signal.
///
/// Values below this threshold are clamped to `f64::NEG_INFINITY` LUFS
/// (silence), preventing log₁₀(0) or log₁₀(denormal) errors.
pub const LUFS_MEAN_SQ_EPSILON: f64 = 1.0e-30;

/// Minimum valid LUFS reading (below this = silence for display purposes).
pub const LUFS_MINIMUM: f64 = -100.0;

/// Loudness values for a single analysis block.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LoudnessBlock {
    /// Momentary loudness (400 ms) in LUFS.
    pub momentary: f64,
    /// Mean-square value (before log, for gating).
    pub mean_square: f64,
}

impl LoudnessBlock {
    /// Returns `true` if this block passes the absolute gate (`M ≥ -70 LUFS`).
    #[must_use]
    pub fn passes_absolute_gate(self) -> bool {
        self.momentary >= ABSOLUTE_GATE_LUFS
    }
}

/// Integrated LUFS analyzer with gated measurement (ITU-R BS.1770-4).
///
/// # Usage
///
/// ```rust,ignore
/// let mut analyzer = LufsAnalyzer::new(48000, 1);
/// // Feed 400 ms blocks (= 19 200 samples at 48 kHz)
/// analyzer.add_mono_block(&samples_400ms);
/// let lufs = analyzer.integrated();
/// ```
#[derive(Debug, Clone)]
pub struct LufsAnalyzer {
    sample_rate: u32,
    #[allow(dead_code)]
    channels: usize,
    /// K-weighting filter per channel.
    k_filters: Vec<KWeightingFilter>,
    /// Collected loudness blocks for gated integration.
    blocks: Vec<LoudnessBlock>,
    /// Rolling sum for short-term (3 s) measurement.
    short_term_blocks: std::collections::VecDeque<LoudnessBlock>,
    /// Number of 400 ms blocks that fit in a 3-second window.
    short_term_capacity: usize,
}

impl LufsAnalyzer {
    /// Create an analyzer for the given sample rate and channel count.
    ///
    /// Returns `None` if `sample_rate` is unsupported by `KWeightingFilter`.
    #[must_use]
    pub fn new(sample_rate: u32, channels: usize) -> Option<Self> {
        let k_filters = (0..channels)
            .map(|_| KWeightingFilter::new(sample_rate))
            .collect::<Option<Vec<_>>>()?;

        // 3-second window in 400 ms blocks = 7.5 → use 8 blocks
        let short_term_capacity =
            ((3.0 * sample_rate as f64) / (0.4 * sample_rate as f64)).ceil() as usize;
        let short_term_capacity = short_term_capacity.max(1);

        Some(Self {
            sample_rate,
            channels,
            k_filters,
            blocks: Vec::new(),
            short_term_blocks: std::collections::VecDeque::with_capacity(short_term_capacity + 1),
            short_term_capacity,
        })
    }

    /// Feed a mono block of samples (one channel, 400 ms).
    ///
    /// Internally K-weights and computes mean-square, then stores the block.
    pub fn add_mono_block(&mut self, samples: &[f32]) {
        let mean_sq = self.k_weight_mono(samples);
        self.push_block(mean_sq);
    }

    /// Feed a stereo block (interleaved L R L R …, 400 ms × 2 channels).
    ///
    /// Applies K-weighting per channel and sums mean-squares equally (`G=1`).
    pub fn add_stereo_block(&mut self, interleaved: &[f32]) {
        assert!(
            interleaved.len() % 2 == 0,
            "stereo buffer must have even length"
        );
        let n = interleaved.len() / 2;
        let mut ms_l = 0.0_f64;
        let mut ms_r = 0.0_f64;
        // Process channel 0 (L) and channel 1 (R)
        for i in 0..n {
            let l = self.k_filters[0].process(interleaved[2 * i] as f64);
            let r = self.k_filters[1].process(interleaved[2 * i + 1] as f64);
            ms_l += l * l;
            ms_r += r * r;
        }
        let mean_sq = (ms_l + ms_r) / n as f64; // G_L = G_R = 1.0
        self.push_block(mean_sq);
    }

    /// Add a pre-K-weighted mean-square value directly (for testing).
    pub fn add_raw_mean_square(&mut self, mean_sq: f64) {
        self.push_block(mean_sq);
    }

    /// Momentary loudness of the last block added.
    ///
    /// Returns `f64::NEG_INFINITY` if no blocks have been added.
    #[must_use]
    pub fn momentary(&self) -> f64 {
        self.blocks
            .last()
            .map(|b| b.momentary)
            .unwrap_or(f64::NEG_INFINITY)
    }

    /// Short-term loudness (3-second sliding window).
    ///
    /// Returns `f64::NEG_INFINITY` if fewer than one block is available.
    #[must_use]
    pub fn short_term(&self) -> f64 {
        if self.short_term_blocks.is_empty() {
            return f64::NEG_INFINITY;
        }
        let mean_sq: f64 = self
            .short_term_blocks
            .iter()
            .map(|b| b.mean_square)
            .sum::<f64>()
            / self.short_term_blocks.len() as f64;
        Self::mean_sq_to_lufs(mean_sq)
    }

    /// Integrated gated loudness (ITU-R BS.1770-4 two-pass gate).
    ///
    /// Returns `f64::NEG_INFINITY` if all blocks were gated out.
    #[must_use]
    pub fn integrated(&self) -> f64 {
        // Pass 1: absolute gate (M ≥ -70 LUFS)
        let pass1: Vec<&LoudnessBlock> = self
            .blocks
            .iter()
            .filter(|b| b.passes_absolute_gate())
            .collect();

        if pass1.is_empty() {
            return f64::NEG_INFINITY;
        }

        // Preliminary mean from pass-1 blocks
        let prelim_mean: f64 =
            pass1.iter().map(|b| b.mean_square).sum::<f64>() / pass1.len() as f64;
        let gamma_r = Self::mean_sq_to_lufs(prelim_mean) + RELATIVE_GATE_LU;

        // Pass 2: relative gate
        let pass2: Vec<&LoudnessBlock> = pass1
            .into_iter()
            .filter(|b| b.momentary >= gamma_r)
            .collect();

        if pass2.is_empty() {
            return f64::NEG_INFINITY;
        }

        let final_mean: f64 = pass2.iter().map(|b| b.mean_square).sum::<f64>() / pass2.len() as f64;
        Self::mean_sq_to_lufs(final_mean)
    }

    /// Reset all state (filters + blocks).
    pub fn reset(&mut self) {
        for f in self.k_filters.iter_mut() {
            f.reset();
        }
        self.blocks.clear();
        self.short_term_blocks.clear();
    }

    /// Number of 400 ms blocks accumulated.
    #[must_use]
    pub fn block_count(&self) -> usize {
        self.blocks.len()
    }

    /// Sample rate this analyzer was constructed for.
    #[must_use]
    pub fn sample_rate(&self) -> u32 {
        self.sample_rate
    }

    // ── Private helpers ───────────────────────────────────────────────────────

    fn k_weight_mono(&mut self, samples: &[f32]) -> f64 {
        if samples.is_empty() {
            return 0.0;
        }
        let mut sum_sq = 0.0_f64;
        for &s in samples {
            // Flush NaN/Inf audio samples to silence — prevents K-filter corruption.
            let x = if s.is_finite() { s as f64 } else { 0.0_f64 };
            let y = self.k_filters[0].process(x);
            sum_sq += y * y;
        }
        let ms = sum_sq / samples.len() as f64;
        // Guard against NaN from accumulated arithmetic errors.
        if ms.is_finite() && ms >= 0.0 {
            ms
        } else {
            0.0
        }
    }

    fn push_block(&mut self, mean_sq: f64) {
        // Sanitize: clamp any non-finite mean_sq to silence.
        let mean_sq = if mean_sq.is_finite() && mean_sq >= 0.0 {
            mean_sq
        } else {
            0.0
        };
        let momentary = Self::mean_sq_to_lufs(mean_sq);
        let block = LoudnessBlock {
            momentary,
            mean_square: mean_sq,
        };
        self.blocks.push(block);

        // Update short-term window
        if self.short_term_blocks.len() >= self.short_term_capacity {
            self.short_term_blocks.pop_front();
        }
        self.short_term_blocks.push_back(block);
    }

    #[inline]
    fn mean_sq_to_lufs(mean_sq: f64) -> f64 {
        if mean_sq < LUFS_MEAN_SQ_EPSILON {
            return f64::NEG_INFINITY;
        }
        LUFS_OFFSET + 10.0 * mean_sq.log10()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sine_block(freq_hz: f64, sample_rate: u32, duration_secs: f64) -> Vec<f32> {
        let n = (sample_rate as f64 * duration_secs) as usize;
        (0..n)
            .map(|i| {
                let t = i as f64 / sample_rate as f64;
                (2.0 * std::f64::consts::PI * freq_hz * t).sin() as f32
            })
            .collect()
    }

    #[test]
    fn lufs_silence_is_neg_inf() {
        let mut a = LufsAnalyzer::new(48000, 1).unwrap();
        let silence = vec![0.0_f32; 19200]; // 400 ms at 48 kHz
        a.add_mono_block(&silence);
        assert!(a.momentary().is_infinite() && a.momentary().is_sign_negative());
    }

    #[test]
    fn lufs_full_scale_sine_below_zero() {
        // 0 dBFS sine → K-weighted LUFS should be between -25 and 0
        let mut a = LufsAnalyzer::new(48000, 1).unwrap();
        let block = sine_block(1000.0, 48000, 0.4);
        a.add_mono_block(&block);
        let m = a.momentary();
        assert!(
            m < 0.0 && m > -50.0,
            "1kHz 0dBFS sine LUFS out of range: {m}"
        );
    }

    #[test]
    fn lufs_integrated_gating_rejects_silence() {
        let mut a = LufsAnalyzer::new(48000, 1).unwrap();
        // 10 blocks of silence (all below -70 LUFS absolute gate)
        let silence = vec![0.0_f32; 19200];
        for _ in 0..10 {
            a.add_mono_block(&silence);
        }
        assert!(a.integrated().is_infinite());
    }

    #[test]
    fn lufs_integrated_signal_is_finite() {
        let mut a = LufsAnalyzer::new(48000, 1).unwrap();
        let block = sine_block(1000.0, 48000, 0.4);
        for _ in 0..20 {
            a.add_mono_block(&block);
        }
        let integrated = a.integrated();
        assert!(integrated.is_finite(), "integrated LUFS should be finite");
        assert!(integrated < 0.0, "integrated LUFS should be negative");
    }

    #[test]
    fn short_term_window_uses_recent_blocks() {
        let mut a = LufsAnalyzer::new(48000, 1).unwrap();
        // Fill with silence first
        let silence = vec![0.0_f32; 19200];
        for _ in 0..20 {
            a.add_mono_block(&silence);
        }
        // Add loud blocks — short-term should be loud now
        let loud = sine_block(1000.0, 48000, 0.4);
        for _ in 0..10 {
            a.add_mono_block(&loud);
        }
        let st = a.short_term();
        assert!(
            st.is_finite(),
            "short-term should be finite after loud blocks"
        );
    }

    #[test]
    fn reset_clears_all_state() {
        let mut a = LufsAnalyzer::new(48000, 1).unwrap();
        let block = sine_block(1000.0, 48000, 0.4);
        for _ in 0..5 {
            a.add_mono_block(&block);
        }
        a.reset();
        assert_eq!(a.block_count(), 0);
        assert!(a.momentary().is_infinite());
    }

    #[test]
    fn block_count_increments() {
        let mut a = LufsAnalyzer::new(48000, 1).unwrap();
        let block = vec![0.0_f32; 19200];
        for i in 1..=5 {
            a.add_mono_block(&block);
            assert_eq!(a.block_count(), i);
        }
    }

    #[test]
    fn raw_mean_square_api() {
        let mut a = LufsAnalyzer::new(48000, 1).unwrap();
        // mean_sq = 1.0 → LUFS = -0.691 + 10*log10(1.0) = -0.691
        a.add_raw_mean_square(1.0);
        let expected = LUFS_OFFSET;
        assert!(
            (a.momentary() - expected).abs() < 1e-6,
            "raw ms=1 → {expected} LUFS"
        );
    }

    /// Golden regression test: -23.0 LUFS reference (EBU R128 broadcast target).
    ///
    /// Derives the expected mean-square from the LUFS formula:
    /// ```text
    /// -23 = LUFS_OFFSET + 10 * log10(ms)
    /// ms  = 10^((-23 − LUFS_OFFSET) / 10)  ≈ 0.005874
    /// ```
    #[test]
    fn golden_lufs_minus_23() {
        let target_lufs = -23.0_f64;
        // Compute the mean_sq that should yield exactly -23 LUFS
        let expected_ms = 10_f64.powf((target_lufs - LUFS_OFFSET) / 10.0);

        let mut a = LufsAnalyzer::new(48000, 1).unwrap();
        a.add_raw_mean_square(expected_ms);

        let measured = a.momentary();
        assert!(
            (measured - target_lufs).abs() < 1e-4,
            "golden -23 LUFS: expected {target_lufs:.4}, got {measured:.4}"
        );
    }

    #[test]
    fn nan_samples_do_not_corrupt_analyzer() {
        let mut a = LufsAnalyzer::new(48000, 1).unwrap();
        // Feed a block containing NaN samples
        let mut block = vec![0.5_f32; 19200];
        block[100] = f32::NAN;
        block[200] = f32::INFINITY;
        // Should not panic, and should produce a finite or -inf result
        a.add_mono_block(&block);
        let m = a.momentary();
        // NaN samples flushed to 0 — result should be finite or -inf, never NaN
        assert!(!m.is_nan(), "NaN samples must not produce NaN LUFS");
    }

    #[test]
    fn nan_raw_mean_square_is_gated_out() {
        let mut a = LufsAnalyzer::new(48000, 1).unwrap();
        a.add_raw_mean_square(f64::NAN);
        // NaN mean_sq should be sanitized to 0 → silence → -inf LUFS
        let m = a.momentary();
        assert!(!m.is_nan(), "NaN mean_sq must not produce NaN LUFS");
        assert!(
            m.is_infinite() && m.is_sign_negative(),
            "silence after NaN flush → -inf"
        );
    }
}
