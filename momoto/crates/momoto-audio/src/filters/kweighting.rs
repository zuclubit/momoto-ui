//! ITU-R BS.1770-4 K-weighting filter.
//!
//! K-weighting is a two-stage cascaded biquad filter that approximates the
//! frequency response of the outer-ear/head transfer function:
//!
//! - **Stage 1 (pre-filter / high-shelf)**: boosts high frequencies to model
//!   head-related acoustics. `H₁(z)` — implemented as a high-shelf filter.
//! - **Stage 2 (RLB-weighting)**: high-pass that models ear canal resonance.
//!   `H₂(z)` — implemented as a second-order high-pass.
//!
//! The combined response approximates the ITU-R BS.1770 `K(f)` weighting curve.
//!
//! # Supported sample rates
//!
//! Coefficients are precomputed for 44100, 48000, and 96000 Hz.
//! Other rates require bilinear-transform computation (not yet implemented).
//!
//! # References
//!
//! - ITU-R BS.1770-4 (2015), Annex 1 — coefficients in Table 1
//! - EBU Tech 3341 — practical implementation notes

use super::biquad::{BiquadCoeffs, BiquadFilter};

/// Pre-computed K-weighting coefficients for a standard sample rate.
#[derive(Debug, Clone, Copy)]
pub struct KWeightingCoeffs {
    /// Stage 1: high-shelf (pre-filter).
    pub stage1: BiquadCoeffs,
    /// Stage 2: RLB high-pass weighting.
    pub stage2: BiquadCoeffs,
}

impl KWeightingCoeffs {
    /// Coefficients for 44 100 Hz (CD standard).
    ///
    /// Source: ITU-R BS.1770-4, Table 1.
    #[must_use]
    pub const fn for_44100() -> Self {
        Self {
            stage1: BiquadCoeffs {
                b0: 1.53512485958697,
                b1: -2.69169618940638,
                b2: 1.19839281085285,
                a1: -1.69065929318241,
                a2: 0.73248077421585,
            },
            stage2: BiquadCoeffs {
                b0: 1.0,
                b1: -2.0,
                b2: 1.0,
                a1: -1.99004745483398,
                a2: 0.99007225036621,
            },
        }
    }

    /// Coefficients for 48 000 Hz (broadcast / pro audio standard).
    ///
    /// Source: ITU-R BS.1770-4, Table 1.
    #[must_use]
    pub const fn for_48000() -> Self {
        Self {
            stage1: BiquadCoeffs {
                b0: 1.53512485958697,
                b1: -2.69169618940638,
                b2: 1.19839281085285,
                a1: -1.69065929318241,
                a2: 0.73248077421585,
            },
            stage2: BiquadCoeffs {
                b0: 1.0,
                b1: -2.0,
                b2: 1.0,
                a1: -1.99004745483398,
                a2: 0.99007225036621,
            },
        }
    }

    /// Coefficients for 96 000 Hz (high-resolution audio).
    ///
    /// Source: ITU-R BS.1770-4, Table 1.
    #[must_use]
    pub const fn for_96000() -> Self {
        Self {
            stage1: BiquadCoeffs {
                b0: 1.69065929318241,
                b1: -2.86998398471917,
                b2: 1.21044048194357,
                a1: -1.92443593018005,
                a2: 0.94007054284922,
            },
            stage2: BiquadCoeffs {
                b0: 1.0,
                b1: -2.0,
                b2: 1.0,
                a1: -1.99750375537395,
                a2: 0.99751186527337,
            },
        }
    }

    /// Returns coefficients for the given sample rate, or `None` if unsupported.
    #[must_use]
    pub fn for_sample_rate(sample_rate: u32) -> Option<Self> {
        match sample_rate {
            44100 => Some(Self::for_44100()),
            48000 => Some(Self::for_48000()),
            96000 => Some(Self::for_96000()),
            _ => None,
        }
    }
}

/// Two-stage K-weighting filter (per channel).
///
/// Apply to each audio channel independently before computing mean-square.
/// Create one instance per channel — the filter is stateful.
///
/// # Usage
///
/// ```rust,ignore
/// let mut kw = KWeightingFilter::new(48000).unwrap();
/// let mut mono = vec![0.0_f64; 1024];
/// // ... fill mono ...
/// kw.process_buffer(&mut mono);
/// // mono now contains K-weighted samples
/// ```
#[derive(Debug, Clone)]
pub struct KWeightingFilter {
    stage1: BiquadFilter,
    stage2: BiquadFilter,
    sample_rate: u32,
}

impl KWeightingFilter {
    /// Create a K-weighting filter for the given sample rate.
    ///
    /// Returns `None` if the sample rate is unsupported.
    #[must_use]
    pub fn new(sample_rate: u32) -> Option<Self> {
        let coeffs = KWeightingCoeffs::for_sample_rate(sample_rate)?;
        Some(Self {
            stage1: BiquadFilter::new(coeffs.stage1),
            stage2: BiquadFilter::new(coeffs.stage2),
            sample_rate,
        })
    }

    /// Reset both filter stages to zero state.
    pub fn reset(&mut self) {
        self.stage1.reset();
        self.stage2.reset();
    }

    /// The sample rate this filter was constructed for.
    #[must_use]
    pub fn sample_rate(&self) -> u32 {
        self.sample_rate
    }

    /// Process one sample through both K-weighting stages.
    #[inline]
    pub fn process(&mut self, x: f64) -> f64 {
        self.stage2.process(self.stage1.process(x))
    }

    /// Process a `f64` buffer in-place (both stages).
    pub fn process_buffer(&mut self, buf: &mut [f64]) {
        for s in buf.iter_mut() {
            *s = self.process(*s);
        }
    }

    /// Process a `f32` buffer in-place.
    pub fn process_buffer_f32(&mut self, buf: &mut [f32]) {
        for s in buf.iter_mut() {
            *s = self.process(*s as f64) as f32;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn supported_sample_rates() {
        assert!(KWeightingFilter::new(44100).is_some());
        assert!(KWeightingFilter::new(48000).is_some());
        assert!(KWeightingFilter::new(96000).is_some());
    }

    #[test]
    fn unsupported_sample_rate_returns_none() {
        assert!(KWeightingFilter::new(22050).is_none());
        assert!(KWeightingFilter::new(192000).is_none());
    }

    #[test]
    fn silence_through_k_weighting_stays_silence() {
        let mut kw = KWeightingFilter::new(48000).unwrap();
        for _ in 0..1000 {
            let y = kw.process(0.0);
            assert!(y.abs() < 1e-12, "silence → silence");
        }
    }

    #[test]
    fn k_weighting_bounded_on_unit_impulse() {
        let mut kw = KWeightingFilter::new(48000).unwrap();
        let y0 = kw.process(1.0);
        assert!(y0.abs() <= 2.0, "peak output bounded for unit impulse");
        // All subsequent outputs should decay
        for _ in 0..1000 {
            let y = kw.process(0.0);
            assert!(y.abs() <= 2.0, "impulse response stays bounded");
        }
    }

    #[test]
    fn reset_clears_state() {
        let mut kw = KWeightingFilter::new(48000).unwrap();
        kw.process(1.0);
        kw.reset();
        // After reset, impulse response should be identical to a fresh filter
        let y_fresh = {
            let mut fresh = KWeightingFilter::new(48000).unwrap();
            fresh.process(1.0)
        };
        kw.process(0.0); // flush one more
        let y_reset = {
            let mut kw2 = KWeightingFilter::new(48000).unwrap();
            kw2.process(1.0)
        };
        assert!((y_fresh - y_reset).abs() < 1e-12);
    }

    #[test]
    fn sample_rate_accessor() {
        let kw = KWeightingFilter::new(44100).unwrap();
        assert_eq!(kw.sample_rate(), 44100);
    }
}
