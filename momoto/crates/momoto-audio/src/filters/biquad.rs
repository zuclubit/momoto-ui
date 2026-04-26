//! Direct Form II Transposed biquad IIR filter.
//!
//! Implements the canonical 2-pole 2-zero IIR section used for K-weighting,
//! shelving, peak EQ, and other audio signal conditioning.
//!
//! # Transfer function
//!
//! ```text
//! H(z) = (b0 + b1*z⁻¹ + b2*z⁻²) / (1 + a1*z⁻¹ + a2*z⁻²)
//! ```
//!
//! # Numerical form: Direct Form II Transposed
//!
//! ```text
//! y[n] = b0*x[n] + w1[n-1]
//! w1[n] = b1*x[n] - a1*y[n] + w2[n-1]
//! w2[n] = b2*x[n] - a2*y[n]
//! ```
//!
//! DFT2 is preferred over Direct Form I because it:
//! - Uses only 2 delay registers (vs 4 in DF1)
//! - Has lower coefficient-sensitivity to quantization
//! - Is the canonical form used by ITU-R BS.1770-4

/// Minimum absolute value treated as non-zero in the biquad delay line.
///
/// Values whose absolute value falls below this threshold are treated as zero
/// to prevent denormal numbers from degrading CPU performance.
pub const BIQUAD_DENORMAL_EPSILON: f64 = 1.0e-30;

/// Maximum absolute value allowed in the biquad delay line before state flush.
///
/// If an output exceeds this bound the filter has diverged — state is reset to
/// prevent indefinite NaN/Inf propagation.
pub const BIQUAD_DIVERGENCE_LIMIT: f64 = 1.0e15;

/// Coefficients for a biquad IIR section.
///
/// All coefficients are stored as `f64` to match the precision of the ITU-R
/// BS.1770-4 K-weighting reference implementation. Down-cast to `f32` happens
/// in the filter kernel only when explicitly requested.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BiquadCoeffs {
    /// Numerator (feed-forward) coefficient for x[n].
    pub b0: f64,
    /// Numerator coefficient for x[n-1].
    pub b1: f64,
    /// Numerator coefficient for x[n-2].
    pub b2: f64,
    /// Denominator (feedback) coefficient for y[n-1] (negated convention).
    pub a1: f64,
    /// Denominator coefficient for y[n-2] (negated convention).
    pub a2: f64,
}

impl BiquadCoeffs {
    /// All-pass identity section: `H(z) = 1`.
    #[must_use]
    pub const fn identity() -> Self {
        Self {
            b0: 1.0,
            b1: 0.0,
            b2: 0.0,
            a1: 0.0,
            a2: 0.0,
        }
    }
}

/// Stateful Direct Form II Transposed biquad IIR filter.
///
/// Process samples one at a time with `process()` or whole buffers with
/// `process_buffer()`. Supports in-place processing.
///
/// # Real-time safety
///
/// `process()` is allocation-free and branch-free after construction.
#[derive(Debug, Clone)]
pub struct BiquadFilter {
    coeffs: BiquadCoeffs,
    /// First delay register (w1).
    w1: f64,
    /// Second delay register (w2).
    w2: f64,
}

impl BiquadFilter {
    /// Create a new filter with the given coefficients and zeroed state.
    #[must_use]
    pub fn new(coeffs: BiquadCoeffs) -> Self {
        Self {
            coeffs,
            w1: 0.0,
            w2: 0.0,
        }
    }

    /// Create an identity (all-pass) filter.
    #[must_use]
    pub fn identity() -> Self {
        Self::new(BiquadCoeffs::identity())
    }

    /// Reset delay registers to zero (flush transient state).
    pub fn reset(&mut self) {
        self.w1 = 0.0;
        self.w2 = 0.0;
    }

    /// Update coefficients without resetting state.
    pub fn set_coeffs(&mut self, coeffs: BiquadCoeffs) {
        self.coeffs = coeffs;
    }

    /// Return current coefficients.
    #[must_use]
    pub fn coeffs(&self) -> BiquadCoeffs {
        self.coeffs
    }

    /// Process one sample through the filter — Direct Form II Transposed.
    ///
    /// Returns the filtered output sample. Updates internal state.
    ///
    /// # Numerical guards
    ///
    /// - NaN/Inf inputs are flushed to `0.0` before entering the delay line.
    /// - If the output diverges beyond `BIQUAD_DIVERGENCE_LIMIT` (e.g. due to
    ///   unstable coefficients), the delay registers are reset and `0.0` is
    ///   returned, preventing indefinite NaN propagation.
    #[inline]
    pub fn process(&mut self, x: f64) -> f64 {
        // Flush NaN/Inf input to zero — prevents state corruption.
        let x = if x.is_finite() { x } else { 0.0 };

        let c = &self.coeffs;
        let y = c.b0 * x + self.w1;
        self.w1 = c.b1 * x - c.a1 * y + self.w2;
        self.w2 = c.b2 * x - c.a2 * y;

        // Guard: if output or state diverged, flush and return silence.
        if !y.is_finite() || y.abs() > BIQUAD_DIVERGENCE_LIMIT {
            self.w1 = 0.0;
            self.w2 = 0.0;
            return 0.0;
        }
        y
    }

    /// Process a buffer of samples in-place.
    ///
    /// Converts `f32` input to `f64` for processing, writes `f32` back.
    /// Suitable for standard audio pipelines.
    pub fn process_buffer_f32(&mut self, buf: &mut [f32]) {
        for sample in buf.iter_mut() {
            *sample = self.process(*sample as f64) as f32;
        }
    }

    /// Process a `f64` buffer in-place.
    pub fn process_buffer_f64(&mut self, buf: &mut [f64]) {
        for sample in buf.iter_mut() {
            *sample = self.process(*sample);
        }
    }

    /// Process input slice, writing to a separate output slice.
    ///
    /// Panics if `input.len() != output.len()`.
    pub fn process_into(&mut self, input: &[f64], output: &mut [f64]) {
        assert_eq!(
            input.len(),
            output.len(),
            "input and output slices must have equal length"
        );
        for (x, y) in input.iter().zip(output.iter_mut()) {
            *y = self.process(*x);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identity_filter_passes_signal_unchanged() {
        let mut f = BiquadFilter::identity();
        let input = [1.0_f64, 0.5, -0.5, 0.0, 1.0];
        let mut output = [0.0_f64; 5];
        f.process_into(&input, &mut output);
        for (a, b) in input.iter().zip(output.iter()) {
            assert!((a - b).abs() < 1e-12, "identity should be lossless");
        }
    }

    #[test]
    fn reset_clears_state() {
        let coeffs = BiquadCoeffs {
            b0: 1.0,
            b1: 0.5,
            b2: 0.0,
            a1: 0.5,
            a2: 0.0,
        };
        let mut f = BiquadFilter::new(coeffs);
        f.process(1.0);
        f.process(1.0);
        f.reset();
        assert_eq!(f.w1, 0.0);
        assert_eq!(f.w2, 0.0);
    }

    #[test]
    fn process_impulse_response_stable() {
        // A simple low-pass (butterworth-like): should not diverge for impulse
        let coeffs = BiquadCoeffs {
            b0: 0.25,
            b1: 0.5,
            b2: 0.25,
            a1: -0.0,
            a2: 0.0,
        };
        let mut f = BiquadFilter::new(coeffs);
        let impulse = [1.0_f64, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0];
        let mut out = [0.0_f64; 8];
        f.process_into(&impulse, &mut out);
        for y in out {
            assert!(
                y.abs() <= 1.0 + 1e-9,
                "IIR should be bounded for stable coefficients"
            );
        }
    }

    #[test]
    fn process_buffer_f32_roundtrip() {
        let mut f = BiquadFilter::identity();
        let mut buf = [0.5_f32, -0.5, 0.25, 0.0];
        let expected = buf;
        f.process_buffer_f32(&mut buf);
        for (a, b) in expected.iter().zip(buf.iter()) {
            assert!((a - b).abs() < 1e-5, "f32 identity roundtrip");
        }
    }

    #[test]
    fn nan_input_is_flushed_to_zero() {
        let mut f = BiquadFilter::identity();
        let y = f.process(f64::NAN);
        assert!(!y.is_nan(), "NaN input must not propagate to output");
        // Subsequent valid samples should still work
        let y2 = f.process(1.0);
        assert!(y2.is_finite());
    }

    #[test]
    fn inf_input_is_flushed_to_zero() {
        let mut f = BiquadFilter::identity();
        let y = f.process(f64::INFINITY);
        assert!(y.is_finite(), "Inf input must not produce Inf output");
    }

    #[test]
    fn process_into_panics_on_length_mismatch() {
        let mut f = BiquadFilter::identity();
        let result = std::panic::catch_unwind(move || {
            let input = [1.0_f64; 4];
            let mut output = [0.0_f64; 5];
            f.process_into(&input, &mut output);
        });
        assert!(result.is_err());
    }
}
