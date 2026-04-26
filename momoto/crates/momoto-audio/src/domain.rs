//! `AudioDomain` — implements the `Domain` and `EnergyConserving` contracts.

use momoto_core::traits::{
    compliance::ComplianceReport,
    domain::{Domain, DomainId},
    physical::{EnergyConserving, EnergyReport},
};

use crate::compliance::ebur128::EbuR128Limits;
use crate::perceptual::lufs::LufsAnalyzer;

/// The acoustic signal processing domain.
///
/// Provides:
/// - K-weighted LUFS measurement (ITU-R BS.1770-4)
/// - FFT-based power spectrum analysis
/// - Mel filterbank feature extraction
/// - EBU R128 broadcast/streaming compliance validation
///
/// # Energy conservation model
///
/// The audio domain models linear signal processing (filtering, FFT). Under
/// this model, energy is conserved in the sense that the total RMS power of
/// a K-weighted signal is equal to the pre-filter power scaled by the
/// K-weighting frequency response. For compliance purposes, we model the
/// domain as lossless at the energy budget level — actual loudness accounting
/// happens through LUFS measurement, not energy budget tracking.
#[derive(Debug)]
pub struct AudioDomain {
    /// Default sample rate used when none is specified.
    pub default_sample_rate: u32,
}

impl AudioDomain {
    /// Create an audio domain with the given default sample rate.
    ///
    /// Returns `None` if the sample rate is unsupported by K-weighting filters.
    #[must_use]
    pub fn new(sample_rate: u32) -> Option<Self> {
        // Validate sample rate is supported
        if LufsAnalyzer::new(sample_rate, 1).is_none() {
            return None;
        }
        Some(Self {
            default_sample_rate: sample_rate,
        })
    }

    /// Create an audio domain with 48 000 Hz default (broadcast standard).
    #[must_use]
    pub fn at_48khz() -> Self {
        Self {
            default_sample_rate: 48_000,
        }
    }

    /// Create a LUFS analyzer for this domain's sample rate.
    ///
    /// Returns `None` only if the sample rate is unsupported (should not
    /// happen if `AudioDomain::new()` succeeded).
    #[must_use]
    pub fn lufs_analyzer(&self, channels: usize) -> Option<LufsAnalyzer> {
        LufsAnalyzer::new(self.default_sample_rate, channels)
    }

    /// Validate measured LUFS against the EBU R128 broadcast profile.
    #[must_use]
    pub fn validate_broadcast(&self, integrated_lufs: f64) -> ComplianceReport {
        EbuR128Limits::BROADCAST.validate(integrated_lufs, None, None)
    }

    /// Validate measured LUFS against the EBU R128 streaming profile.
    #[must_use]
    pub fn validate_streaming(&self, integrated_lufs: f64) -> ComplianceReport {
        EbuR128Limits::STREAMING.validate(integrated_lufs, None, None)
    }
}

impl Domain for AudioDomain {
    #[inline]
    fn id(&self) -> DomainId {
        DomainId::Audio
    }

    #[inline]
    fn name(&self) -> &'static str {
        "momoto-audio"
    }

    #[inline]
    fn version(&self) -> &'static str {
        env!("CARGO_PKG_VERSION")
    }

    #[inline]
    fn is_deterministic(&self) -> bool {
        true
    }

    /// Maximum inplace samples: 4 096 (matches FFT plan default and scratch buffer).
    #[inline]
    fn max_inplace_samples(&self) -> Option<usize> {
        Some(4_096)
    }
}

impl EnergyConserving for AudioDomain {
    /// Audio domain energy model: lossless signal path.
    ///
    /// Linear filters (K-weighting, FFT) are energy-preserving at the
    /// spectral level (Parseval's theorem). The domain reports lossless
    /// energy flow; loudness normalization is handled separately via LUFS.
    #[inline]
    fn energy_report(&self, input: f32) -> EnergyReport {
        EnergyReport::lossless(input)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn audio_domain_at_48khz_succeeds() {
        let d = AudioDomain::at_48khz();
        assert_eq!(d.default_sample_rate, 48000);
    }

    #[test]
    fn audio_domain_new_unsupported_rate_fails() {
        assert!(AudioDomain::new(22050).is_none());
    }

    #[test]
    fn audio_domain_id_is_audio() {
        let d = AudioDomain::at_48khz();
        assert_eq!(d.id(), DomainId::Audio);
    }

    #[test]
    fn audio_domain_is_deterministic() {
        assert!(AudioDomain::at_48khz().is_deterministic());
    }

    #[test]
    fn audio_domain_energy_conserved() {
        let d = AudioDomain::at_48khz();
        assert!(d.verify_conservation(1.0, 1e-6));
    }

    #[test]
    fn lufs_analyzer_creation_succeeds() {
        let d = AudioDomain::at_48khz();
        assert!(d.lufs_analyzer(1).is_some());
        assert!(d.lufs_analyzer(2).is_some());
    }

    #[test]
    fn validate_broadcast_target_passes() {
        let d = AudioDomain::at_48khz();
        assert!(d.validate_broadcast(-23.0).passes);
    }

    #[test]
    fn validate_streaming_target_passes() {
        let d = AudioDomain::at_48khz();
        assert!(d.validate_streaming(-14.0).passes);
    }
}
